//! Read-only TradeLocker REST transport.
//!
//! This type intentionally has no POST/PATCH/DELETE trading surface. The only
//! POST is JWT authentication. Every trade endpoint is a GET and every refresh
//! is returned as one unpublished wire bundle for coherent decoding.

use super::tradelocker::{TradeLockerBinding, TradeLockerWireRefresh};
use super::tradelocker_runtime::{
    TradeLockerRuntimeConfig, TradeLockerRuntimeConfigError, TRADELOCKER_LIVE_ROOT,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use zeroize::Zeroize;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const RATE_LIMIT_RETRIES: usize = 3;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

struct Tokens {
    access: String,
    refresh: String,
}

impl Drop for Tokens {
    fn drop(&mut self) {
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

#[derive(Clone, Debug)]
struct SelectedAccount {
    id: u64,
    acc_num: u64,
    name: Arc<str>,
    currency: Arc<str>,
}

pub struct TradeLockerReadClient {
    config: TradeLockerRuntimeConfig,
    bindings: Box<[TradeLockerBinding]>,
    agent: ureq::Agent,
    tokens: Option<Tokens>,
    account: Option<SelectedAccount>,
}

impl TradeLockerReadClient {
    pub fn new(
        config: TradeLockerRuntimeConfig,
        bindings: Vec<TradeLockerBinding>,
    ) -> Result<Self, TradeLockerTransportError> {
        if config.rest_root.as_ref() != TRADELOCKER_LIVE_ROOT || bindings.is_empty() {
            return Err(TradeLockerTransportError::InvalidConfiguration);
        }
        let agent_config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .https_only(true)
            .build();
        Ok(Self {
            config,
            bindings: bindings.into_boxed_slice(),
            agent: agent_config.into(),
            tokens: None,
            account: None,
        })
    }

    /// Fetches every account-scoped component required for one publishable
    /// epoch. Callers must discard the bundle if decoding or persistence fails.
    pub fn refresh(&mut self) -> Result<TradeLockerWireRefresh, TradeLockerTransportError> {
        if self.tokens.is_none() {
            self.authenticate()?;
        }
        if self.account.is_none() {
            self.select_account()?;
        }
        match self.refresh_once() {
            Err(TradeLockerTransportError::AuthorizationRejected(_)) => {
                self.tokens = None;
                self.account = None;
                self.authenticate()?;
                self.select_account()?;
                self.refresh_once()
            }
            result => result,
        }
    }

    fn refresh_once(&mut self) -> Result<TradeLockerWireRefresh, TradeLockerTransportError> {
        let account = self
            .account
            .clone()
            .ok_or(TradeLockerTransportError::AccountUnavailable)?;
        let ts_started_ns = now_ns();
        let config = self.get("/trade/config", &[])?;
        let detail_spacing = route_spacing(&config, "GET_INSTRUMENT_DETAILS")?;
        let quote_spacing = route_spacing(&config, "QUOTES")?;
        let instruments_path = format!("/trade/accounts/{}/instruments", account.id);
        let instruments = self.get(&instruments_path, &[])?;
        let selected = select_instruments(&instruments, &self.bindings)?;

        let mut instrument_details = Vec::with_capacity(selected.len());
        let mut quotes = Vec::with_capacity(selected.len());
        let mut next_detail = Instant::now();
        let mut next_quote = Instant::now();
        for selected in selected {
            wait_for_slot(&mut next_detail, detail_spacing);
            let detail_path = format!("/trade/instruments/{}", selected.id);
            instrument_details.push((
                selected.id,
                self.get(
                    &detail_path,
                    &[("routeId", selected.info_route.as_str()), ("locale", "en")],
                )?,
            ));
            wait_for_slot(&mut next_quote, quote_spacing);
            quotes.push((
                selected.id,
                self.get(
                    "/trade/quotes",
                    &[
                        ("tradableInstrumentId", selected.id_text.as_str()),
                        ("routeId", selected.info_route.as_str()),
                    ],
                )?,
            ));
        }
        let account_path = format!("/trade/accounts/{}/state", account.id);
        let positions_path = format!("/trade/accounts/{}/positions", account.id);
        let orders_path = format!("/trade/accounts/{}/orders", account.id);
        let history_path = format!("/trade/accounts/{}/ordersHistory", account.id);
        let executions_path = format!("/trade/accounts/{}/executions", account.id);
        let account_state = self.get(&account_path, &[])?;
        let positions = self.get(&positions_path, &[])?;
        let orders = self.get(&orders_path, &[])?;
        let order_history = self.get(&history_path, &[])?;
        let executions = self.get(&executions_path, &[])?;
        Ok(TradeLockerWireRefresh {
            ts_started_ns,
            ts_received_ns: now_ns(),
            account_id: account.id,
            acc_num: account.acc_num,
            account_name: account.name,
            account_currency: account.currency,
            config,
            instruments,
            instrument_details: instrument_details.into_boxed_slice(),
            account_state,
            positions,
            orders,
            order_history,
            executions,
            quotes: quotes.into_boxed_slice(),
        })
    }

    fn authenticate(&mut self) -> Result<(), TradeLockerTransportError> {
        let secret = self.config.load_secret()?;
        let endpoint = format!("{}/auth/jwt/token", self.config.rest_root);
        let mut response = self
            .agent
            .post(&endpoint)
            .header("User-Agent", "Northstar/0.1 tradelocker-readonly")
            .send_json(serde_json::json!({
                "email": secret.email(),
                "password": secret.password(),
                "server": self.config.server.as_ref(),
            }))
            .map_err(TradeLockerTransportError::Http)?;
        let status = response.status().as_u16();
        if !authentication_succeeded(status) {
            return Err(TradeLockerTransportError::AuthenticationRejected(status));
        }
        let tokens: TokenResponse = response
            .body_mut()
            .read_json()
            .map_err(TradeLockerTransportError::Http)?;
        if tokens.access_token.len() < 32 || tokens.refresh_token.len() < 32 {
            return Err(TradeLockerTransportError::AuthenticationMalformed);
        }
        self.tokens = Some(Tokens {
            access: tokens.access_token,
            refresh: tokens.refresh_token,
        });
        Ok(())
    }

    fn select_account(&mut self) -> Result<(), TradeLockerTransportError> {
        let value = self.get_without_account("/auth/jwt/all-accounts", &[])?;
        let accounts = value
            .get("accounts")
            .and_then(Value::as_array)
            .ok_or(TradeLockerTransportError::Malformed("accounts"))?;
        let matches = accounts
            .iter()
            .filter_map(parse_account)
            .filter(
                |account| match (self.config.account_id, self.config.acc_num) {
                    (Some(id), None) => account.id == id,
                    (None, Some(acc_num)) => account.acc_num == acc_num,
                    (None, None) => true,
                    (Some(_), Some(_)) => false,
                },
            )
            .collect::<Vec<_>>();
        let account = match matches.as_slice() {
            [account] => account.clone(),
            [] => return Err(TradeLockerTransportError::AccountUnavailable),
            _ => {
                return Err(TradeLockerTransportError::AccountSelectionRequired(
                    matches.len(),
                ))
            }
        };
        self.account = Some(account);
        Ok(())
    }

    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, TradeLockerTransportError> {
        let acc_num = self
            .account
            .as_ref()
            .ok_or(TradeLockerTransportError::AccountUnavailable)?
            .acc_num
            .to_string();
        self.request(path, query, Some(&acc_num))
    }

    fn get_without_account(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, TradeLockerTransportError> {
        self.request(path, query, None)
    }

    fn request(
        &self,
        path: &str,
        query: &[(&str, &str)],
        acc_num: Option<&str>,
    ) -> Result<Value, TradeLockerTransportError> {
        let access = &self
            .tokens
            .as_ref()
            .ok_or(TradeLockerTransportError::NotAuthenticated)?
            .access;
        let authorization = format!("Bearer {access}");
        let endpoint = format!("{}{path}", self.config.rest_root);
        for attempt in 0..=RATE_LIMIT_RETRIES {
            let mut request = self
                .agent
                .get(&endpoint)
                .header("Authorization", &authorization)
                .header("User-Agent", "Northstar/0.1 tradelocker-readonly");
            if let Some(acc_num) = acc_num {
                request = request.header("accNum", acc_num);
            }
            for (key, value) in query {
                request = request.query(*key, *value);
            }
            let mut response = request.call().map_err(TradeLockerTransportError::Http)?;
            let status = response.status().as_u16();
            if status == 401 || status == 403 {
                return Err(TradeLockerTransportError::AuthorizationRejected(status));
            }
            if status == 429 {
                let retry_after_seconds = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse().ok());
                if attempt < RATE_LIMIT_RETRIES {
                    std::thread::sleep(rate_retry_delay(attempt, retry_after_seconds));
                    continue;
                }
                return Err(TradeLockerTransportError::RateLimited {
                    endpoint: endpoint_name(path),
                    retry_after_seconds,
                });
            }
            if !(200..300).contains(&status) {
                return Err(TradeLockerTransportError::ProviderStatus {
                    status,
                    endpoint: endpoint_name(path),
                });
            }
            return response
                .body_mut()
                .read_json()
                .map_err(TradeLockerTransportError::Http);
        }
        unreachable!("bounded rate retry loop always returns")
    }
}

#[derive(Clone, Debug)]
struct SelectedInstrument {
    id: u64,
    id_text: String,
    info_route: String,
}

fn select_instruments(
    response: &Value,
    bindings: &[TradeLockerBinding],
) -> Result<Vec<SelectedInstrument>, TradeLockerTransportError> {
    let rows = response
        .get("d")
        .and_then(|value| value.get("instruments"))
        .and_then(Value::as_array)
        .ok_or(TradeLockerTransportError::Malformed("instruments"))?;
    let mut selected = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let row = rows
            .iter()
            .find(|row| row.get("name").and_then(Value::as_str) == Some(&binding.provider_symbol))
            .ok_or_else(|| {
                TradeLockerTransportError::MissingInstrument(Arc::clone(&binding.provider_symbol))
            })?;
        let id = row
            .get("tradableInstrumentId")
            .and_then(value_u64)
            .ok_or(TradeLockerTransportError::Malformed("tradableInstrumentId"))?;
        let routes = row
            .get("routes")
            .and_then(Value::as_array)
            .ok_or(TradeLockerTransportError::Malformed("routes"))?;
        let info_route = routes
            .iter()
            .find(|route| route.get("type").and_then(Value::as_str) == Some("INFO"))
            .and_then(|route| route.get("id"))
            .and_then(value_text)
            .ok_or(TradeLockerTransportError::Malformed("INFO route"))?;
        let _trade_route = routes
            .iter()
            .find(|route| route.get("type").and_then(Value::as_str) == Some("TRADE"))
            .and_then(|route| route.get("id"))
            .and_then(value_text)
            .ok_or(TradeLockerTransportError::Malformed("TRADE route"))?;
        selected.push(SelectedInstrument {
            id,
            id_text: id.to_string(),
            info_route,
        });
    }
    Ok(selected)
}

fn parse_account(value: &Value) -> Option<SelectedAccount> {
    Some(SelectedAccount {
        id: value.get("id").and_then(value_u64)?,
        acc_num: value.get("accNum").and_then(value_u64)?,
        name: value.get("name").and_then(Value::as_str)?.into(),
        currency: value.get("currency").and_then(Value::as_str)?.into(),
    })
}

fn value_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn value_text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|value| value.to_string()))
}

fn route_spacing(
    config: &Value,
    route_name: &'static str,
) -> Result<Duration, TradeLockerTransportError> {
    let limits = config
        .get("d")
        .and_then(|value| value.get("rateLimits"))
        .and_then(Value::as_array)
        .ok_or(TradeLockerTransportError::Malformed("rateLimits"))?;
    let limit = limits
        .iter()
        .find(|limit| limit.get("rateLimitType").and_then(Value::as_str) == Some(route_name))
        .ok_or(TradeLockerTransportError::MissingRateLimit(route_name))?;
    let count = limit
        .get("limit")
        .and_then(value_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(TradeLockerTransportError::Malformed("rate limit"))?;
    let interval = limit
        .get("intervalNum")
        .and_then(value_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(TradeLockerTransportError::Malformed("rate interval"))?;
    let unit_seconds = match limit.get("measure").and_then(Value::as_str) {
        Some("SECONDS") => 1.0,
        Some("MINUTES") => 60.0,
        _ => return Err(TradeLockerTransportError::Malformed("rate measure")),
    };
    let provider_spacing = interval * unit_seconds / count;
    Ok(Duration::from_secs_f64(
        (provider_spacing * 1.10).max(0.001),
    ))
}

fn value_f64(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn wait_for_slot(next: &mut Instant, spacing: Duration) {
    let now = Instant::now();
    if now < *next {
        std::thread::sleep(*next - now);
    }
    *next = Instant::now() + spacing;
}

fn rate_retry_delay(attempt: usize, retry_after_seconds: Option<u64>) -> Duration {
    let exponential = 1u64 << attempt.min(3);
    Duration::from_secs(retry_after_seconds.unwrap_or(0).max(exponential).min(8))
}

fn endpoint_name(path: &str) -> &'static str {
    if path == "/trade/config" {
        "GET_CONFIG"
    } else if path.ends_with("/instruments") {
        "GET_INSTRUMENTS"
    } else if path.starts_with("/trade/instruments/") {
        "GET_INSTRUMENT_DETAILS"
    } else if path == "/trade/quotes" {
        "QUOTES"
    } else if path.ends_with("/state") {
        "GET_ACCOUNTS_STATE"
    } else if path.ends_with("/positions") {
        "GET_POSITIONS"
    } else if path.ends_with("/ordersHistory") {
        "GET_ORDERS_HISTORY"
    } else if path.ends_with("/orders") {
        "GET_ORDERS"
    } else if path.ends_with("/executions") {
        "GET_EXECUTIONS"
    } else if path == "/auth/jwt/all-accounts" {
        "GET_ALL_ACCOUNTS"
    } else {
        "UNKNOWN_READ_ENDPOINT"
    }
}

fn now_ns() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

const fn authentication_succeeded(status: u16) -> bool {
    status == 200 || status == 201
}

#[derive(Debug, Error)]
pub enum TradeLockerTransportError {
    #[error("TradeLocker read-only configuration is invalid")]
    InvalidConfiguration,
    #[error(transparent)]
    Runtime(#[from] TradeLockerRuntimeConfigError),
    #[error("TradeLocker HTTP transport failed: {0}")]
    Http(ureq::Error),
    #[error("TradeLocker authentication was rejected with HTTP {0}")]
    AuthenticationRejected(u16),
    #[error("TradeLocker authentication response is malformed")]
    AuthenticationMalformed,
    #[error("TradeLocker request is not authenticated")]
    NotAuthenticated,
    #[error("TradeLocker authorization was rejected with HTTP {0}")]
    AuthorizationRejected(u16),
    #[error(
        "TradeLocker rate limit reached for {endpoint} (retry-after {retry_after_seconds:?} seconds)"
    )]
    RateLimited {
        endpoint: &'static str,
        retry_after_seconds: Option<u64>,
    },
    #[error("TradeLocker returned HTTP {status} for {endpoint}")]
    ProviderStatus { status: u16, endpoint: &'static str },
    #[error("TradeLocker response has malformed {0}")]
    Malformed(&'static str),
    #[error("TradeLocker configuration omitted rate limit {0}")]
    MissingRateLimit(&'static str),
    #[error("TradeLocker account is unavailable")]
    AccountUnavailable,
    #[error("TradeLocker account selection is required; {0} accounts matched")]
    AccountSelectionRequired(usize),
    #[error("TradeLocker instrument {0} is unavailable")]
    MissingInstrument(Arc<str>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::instruments;

    #[test]
    fn jwt_accepts_documented_created_status() {
        assert!(authentication_succeeded(200));
        assert!(authentication_succeeded(201));
        assert!(!authentication_succeeded(204));
        assert!(!authentication_succeeded(400));
    }

    #[test]
    fn instrument_selection_requires_info_and_trade_routes() {
        let response = serde_json::json!({
            "d": {
                "instruments": [{
                    "name": "NAS100",
                    "tradableInstrumentId": 817,
                    "routes": [
                        {"id": 11, "type": "INFO"},
                        {"id": 12, "type": "TRADE"}
                    ]
                }]
            }
        });
        let bindings = [TradeLockerBinding {
            instrument: instruments::US100,
            provider_symbol: Arc::from("NAS100"),
            price_scale: 10_000,
        }];
        let selected = select_instruments(&response, &bindings).unwrap();
        assert_eq!(selected[0].info_route, "11");
        let missing_trade = serde_json::json!({
            "d": {"instruments": [{
                "name": "NAS100",
                "tradableInstrumentId": 817,
                "routes": [{"id": 11, "type": "INFO"}]
            }]}
        });
        assert!(select_instruments(&missing_trade, &bindings).is_err());
    }

    #[test]
    fn provider_rate_plan_spaces_bursts_from_config_truth() {
        let config = serde_json::json!({
            "d": {"rateLimits": [{
                "rateLimitType": "GET_INSTRUMENT_DETAILS",
                "measure": "SECONDS",
                "intervalNum": 1,
                "limit": 2
            }]}
        });
        assert_eq!(
            route_spacing(&config, "GET_INSTRUMENT_DETAILS").unwrap(),
            Duration::from_millis(550)
        );
    }

    #[test]
    fn diagnostic_endpoint_names_redact_account_and_instrument_ids() {
        assert_eq!(
            endpoint_name("/trade/accounts/123456/ordersHistory"),
            "GET_ORDERS_HISTORY"
        );
        assert_eq!(
            endpoint_name("/trade/instruments/3883"),
            "GET_INSTRUMENT_DETAILS"
        );
    }

    #[test]
    fn zero_retry_after_still_backs_off_without_unbounded_wait() {
        assert_eq!(rate_retry_delay(0, Some(0)), Duration::from_secs(1));
        assert_eq!(rate_retry_delay(2, Some(0)), Duration::from_secs(4));
        assert_eq!(rate_retry_delay(9, Some(60)), Duration::from_secs(8));
    }
}

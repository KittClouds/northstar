use super::runtime::{MacroRefresh, MacroRuntimeCommand, RuntimeCommand};
use super::{
    FetchedBeaReceipt, FetchedBlsCalendarReceipt, FetchedBoeReceipt, FetchedBojReceipt,
    FetchedCensusReceipt, FetchedCftcReceipt, FetchedEcbReceipt, FetchedEurostatReceipt,
    FetchedFredReceipt, FetchedMacroReceipt, FetchedOnsReceipt, BEA_REFRESH_INTERVAL_NS,
    BLS_CALENDAR_REFRESH_INTERVAL_NS, BLS_REFRESH_INTERVAL_NS, BOE_REFRESH_INTERVAL_NS,
    BOJ_REFRESH_INTERVAL_NS, CENSUS_REFRESH_INTERVAL_NS, CFTC_REFRESH_INTERVAL_NS,
    ECB_REFRESH_INTERVAL_NS, EUROSTAT_REFRESH_INTERVAL_NS, FRED_REFRESH_INTERVAL_NS,
    ONS_REFRESH_INTERVAL_NS,
};
use crate::data_plane::providers::bea::{BEA_API_ROOT, BEA_SERIES};
use crate::data_plane::providers::bls::{BLS_API_ROOT, BLS_SERIES, BLS_SOURCE};
use crate::data_plane::providers::bls_calendar::{
    BLS_RELEASE_CALENDAR_STREAM, BLS_RELEASE_CALENDAR_URL,
};
use crate::data_plane::providers::boe::{BOE_DATABASE_EXPORT, BOE_SERIES};
use crate::data_plane::providers::boj::{BOJ_API_ROOT, BOJ_BINDINGS};
use crate::data_plane::providers::census::CENSUS_MARTS_API_ROOT;
use crate::data_plane::providers::cftc::CFTC_TFF_ENDPOINT;
use crate::data_plane::providers::ecb::{ECB_API_ROOT, ECB_SERIES};
use crate::data_plane::providers::eurostat::{EUROSTAT_API_ROOT, EUROSTAT_BINDINGS};
use crate::data_plane::providers::fred::{FRED_API_ROOT, FRED_SERIES};
use crate::data_plane::providers::ons::{download_path, ONS_DOWNLOAD_ROOT, ONS_SERIES, ONS_SOURCE};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use thiserror::Error;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const INGEST_ACK_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const BLS_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const BLS_CALENDAR_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const CFTC_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const EUROSTAT_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const FRED_FAILURE_RETRY: Duration = Duration::from_secs(60 * 60);
pub(super) const BEA_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const CENSUS_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const ECB_FAILURE_RETRY: Duration = Duration::from_secs(60 * 60);
pub(super) const ONS_FAILURE_RETRY: Duration = Duration::from_secs(6 * 60 * 60);
pub(super) const BOE_FAILURE_RETRY: Duration = Duration::from_secs(60 * 60);
pub(super) const BOJ_FAILURE_RETRY: Duration = Duration::from_secs(60 * 60);

pub(super) fn bls_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        BLS_REFRESH_INTERVAL_NS,
        BLS_FAILURE_RETRY,
        "BLS",
        BLS_SOURCE,
        crate::data_plane::providers::bls::BLS_TIMESERIES_STREAM,
        fetch_complete_bls_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Bls(refresh)),
    );
}

pub(super) fn bls_calendar_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        BLS_CALENDAR_REFRESH_INTERVAL_NS,
        BLS_CALENDAR_FAILURE_RETRY,
        "BLS release calendar",
        BLS_SOURCE,
        BLS_RELEASE_CALENDAR_STREAM,
        fetch_bls_calendar,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::BlsCalendar(refresh)),
    );
}

pub(super) fn cftc_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        CFTC_REFRESH_INTERVAL_NS,
        CFTC_FAILURE_RETRY,
        "CFTC TFF",
        crate::data_plane::providers::cftc::CFTC_SOURCE,
        crate::data_plane::providers::cftc::CFTC_TFF_STREAM,
        fetch_complete_cftc_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Cftc(refresh)),
    );
}

pub(super) fn eurostat_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        EUROSTAT_REFRESH_INTERVAL_NS,
        EUROSTAT_FAILURE_RETRY,
        "Eurostat",
        crate::data_plane::providers::eurostat::EUROSTAT_SOURCE,
        crate::data_plane::providers::eurostat::EUROSTAT_STATISTICS_STREAM,
        fetch_complete_eurostat_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Eurostat(refresh)),
    );
}

pub(super) fn fred_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
    api_key: String,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        FRED_REFRESH_INTERVAL_NS,
        FRED_FAILURE_RETRY,
        "FRED rates",
        crate::data_plane::providers::fred::FRED_SOURCE,
        crate::data_plane::providers::fred::FRED_OBSERVATIONS_STREAM,
        move |agent| fetch_complete_fred_refresh(agent, &api_key),
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Fred(refresh)),
    );
}

pub(super) fn bea_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
    api_key: String,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        BEA_REFRESH_INTERVAL_NS,
        BEA_FAILURE_RETRY,
        "BEA GDP/PCE",
        crate::data_plane::providers::bea::BEA_SOURCE,
        crate::data_plane::providers::bea::BEA_NIPA_STREAM,
        move |agent| fetch_complete_bea_refresh(agent, &api_key),
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Bea(refresh)),
    );
}

pub(super) fn census_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
    api_key: String,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        CENSUS_REFRESH_INTERVAL_NS,
        CENSUS_FAILURE_RETRY,
        "Census MARTS",
        crate::data_plane::providers::census::CENSUS_SOURCE,
        crate::data_plane::providers::census::CENSUS_MARTS_STREAM,
        move |agent| fetch_census_refresh(agent, &api_key),
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Census(refresh)),
    );
}

pub(super) fn ecb_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        ECB_REFRESH_INTERVAL_NS,
        ECB_FAILURE_RETRY,
        "ECB policy rates",
        crate::data_plane::providers::ecb::ECB_SOURCE,
        crate::data_plane::providers::ecb::ECB_POLICY_RATES_STREAM,
        fetch_complete_ecb_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Ecb(refresh)),
    );
}

pub(super) fn ons_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        ONS_REFRESH_INTERVAL_NS,
        ONS_FAILURE_RETRY,
        "ONS UK macro",
        ONS_SOURCE,
        crate::data_plane::providers::ons::ONS_TIMESERIES_STREAM,
        fetch_complete_ons_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Ons(refresh)),
    );
}

pub(super) fn boe_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        BOE_REFRESH_INTERVAL_NS,
        BOE_FAILURE_RETRY,
        "Bank of England rate",
        crate::data_plane::providers::boe::BOE_SOURCE,
        crate::data_plane::providers::boe::BOE_RATES_STREAM,
        fetch_complete_boe_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Boe(refresh)),
    );
}

pub(super) fn boj_supervisor(
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
) {
    let agent = agent();
    supervise(
        &agent,
        commands,
        cancel,
        first_refresh_ns,
        BOJ_REFRESH_INTERVAL_NS,
        BOJ_FAILURE_RETRY,
        "Bank of Japan",
        crate::data_plane::providers::boj::BOJ_SOURCE,
        crate::data_plane::providers::boj::BOJ_TIMESERIES_STREAM,
        fetch_complete_boj_refresh,
        |refresh| RuntimeCommand::Macro(MacroRuntimeCommand::Boj(refresh)),
    );
}

#[allow(clippy::too_many_arguments)]
fn supervise<T, F, W>(
    agent: &ureq::Agent,
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
    first_refresh_ns: i64,
    interval_ns: i64,
    failure_retry: Duration,
    label: &'static str,
    source: crate::data_plane::ids::SourceId,
    stream: crate::data_plane::ids::StreamId,
    mut fetch: F,
    wrap: W,
) where
    F: FnMut(&ureq::Agent) -> Result<T, OfficialTransportError>,
    W: Fn(MacroRefresh<T>) -> RuntimeCommand,
{
    let mut wait = ns_until(first_refresh_ns);
    loop {
        if !wait.is_zero() {
            match cancel.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
        } else if cancel.try_recv().is_ok() {
            return;
        }
        wait = match fetch(agent) {
            Ok(payload) => {
                let next_refresh_ns = now_ns().saturating_add(interval_ns);
                let (acknowledgement, accepted) = crossbeam_channel::bounded(1);
                if commands
                    .send(wrap(MacroRefresh {
                        payload,
                        next_refresh_ns,
                        acknowledgement,
                    }))
                    .is_err()
                {
                    return;
                }
                if accepted.recv_timeout(INGEST_ACK_TIMEOUT) == Ok(true) {
                    Duration::from_nanos(u64::try_from(interval_ns).unwrap_or(u64::MAX))
                } else {
                    failure_retry
                }
            }
            Err(error) => {
                let next_refresh_ns = now_ns().saturating_add(duration_ns(failure_retry));
                if commands
                    .send(RuntimeCommand::Macro(MacroRuntimeCommand::Failure(
                        source,
                        stream,
                        format!("{label} unavailable: {error}").into(),
                        next_refresh_ns,
                    )))
                    .is_err()
                {
                    return;
                }
                failure_retry
            }
        };
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into()
}

fn fetch_complete_bls_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedMacroReceipt>, OfficialTransportError> {
    let mut receipts = Vec::with_capacity(BLS_SERIES.len());
    for binding in BLS_SERIES {
        let url = format!("{BLS_API_ROOT}/{}", binding.provider_code);
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(&url)
            .header("User-Agent", "Northstar/0.1 official-macro")
            .call()?;
        receipts.push(FetchedMacroReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_bls_calendar(
    agent: &ureq::Agent,
) -> Result<FetchedBlsCalendarReceipt, OfficialTransportError> {
    let ts_started_ns = now_ns();
    let mut response = agent
        .get(BLS_RELEASE_CALENDAR_URL)
        .header("User-Agent", "Northstar/0.1 official-release-calendar")
        .call()?;
    Ok(FetchedBlsCalendarReceipt {
        status_code: response.status().as_u16(),
        ts_started_ns,
        ts_received_ns: now_ns(),
        payload: response.body_mut().read_to_vec()?,
    })
}

fn fetch_complete_cftc_refresh(
    agent: &ureq::Agent,
) -> Result<FetchedCftcReceipt, OfficialTransportError> {
    const SELECT: &str = "id,report_date_as_yyyy_mm_dd,cftc_contract_market_code,open_interest_all,dealer_positions_long_all,dealer_positions_short_all,dealer_positions_spread_all,asset_mgr_positions_long,asset_mgr_positions_short,asset_mgr_positions_spread,lev_money_positions_long,lev_money_positions_short,lev_money_positions_spread,other_rept_positions_long,other_rept_positions_short,other_rept_positions_spread,nonrept_positions_long_all,nonrept_positions_short_all";
    const WHERE: &str = "cftc_contract_market_code in ('20974+','13874+','124603','240743')";
    const ORDER: &str = "report_date_as_yyyy_mm_dd DESC,cftc_contract_market_code ASC";
    let ts_started_ns = now_ns();
    let mut response = agent
        .get(CFTC_TFF_ENDPOINT)
        .query("$select", SELECT)
        .query("$where", WHERE)
        .query("$order", ORDER)
        .query("$limit", "416")
        .header("User-Agent", "Northstar/0.1 official-positioning")
        .call()?;
    Ok(FetchedCftcReceipt {
        status_code: response.status().as_u16(),
        ts_started_ns,
        ts_received_ns: now_ns(),
        payload: response.body_mut().read_to_vec()?,
    })
}

fn fetch_complete_eurostat_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedEurostatReceipt>, OfficialTransportError> {
    let mut receipts = Vec::with_capacity(EUROSTAT_BINDINGS.len());
    for binding in EUROSTAT_BINDINGS {
        let url = format!("{EUROSTAT_API_ROOT}/{}", binding.dataset_code);
        let ts_started_ns = now_ns();
        let mut request = agent
            .get(&url)
            .query("lang", "en")
            .query("lastTimePeriod", "36");
        for &(dimension, value) in binding.dimensions {
            request = request.query(dimension, value);
        }
        let mut response = request
            .header("User-Agent", "Northstar/0.1 official-europe-macro")
            .call()?;
        receipts.push(FetchedEurostatReceipt {
            dataset_code: binding.dataset_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_complete_fred_refresh(
    agent: &ureq::Agent,
    api_key: &str,
) -> Result<Vec<FetchedFredReceipt>, OfficialTransportError> {
    let mut receipts = Vec::with_capacity(FRED_SERIES.len());
    for binding in FRED_SERIES {
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(FRED_API_ROOT)
            .query("series_id", binding.provider_code)
            .query("api_key", api_key)
            .query("file_type", "json")
            .query("units", "lin")
            .query("output_type", "1")
            .query("sort_order", "desc")
            .query("limit", "400")
            .header("User-Agent", "Northstar/0.1 official-rates")
            .call()?;
        receipts.push(FetchedFredReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_complete_bea_refresh(
    agent: &ureq::Agent,
    api_key: &str,
) -> Result<Vec<FetchedBeaReceipt>, OfficialTransportError> {
    let year = current_utc_year();
    let years = format!("{},{},{}", year - 2, year - 1, year);
    let mut receipts = Vec::with_capacity(BEA_SERIES.len());
    for binding in BEA_SERIES {
        let mut parts = binding.provider_code.split(':');
        let table = parts
            .next()
            .ok_or_else(|| OfficialTransportError::Contract(binding.provider_code.into()))?;
        let _line = parts
            .next()
            .ok_or_else(|| OfficialTransportError::Contract(binding.provider_code.into()))?;
        let frequency = parts
            .next()
            .ok_or_else(|| OfficialTransportError::Contract(binding.provider_code.into()))?;
        if parts.next().is_some() {
            return Err(OfficialTransportError::Contract(
                binding.provider_code.into(),
            ));
        }
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(BEA_API_ROOT)
            .query("UserID", api_key)
            .query("method", "GetData")
            .query("DataSetName", "NIPA")
            .query("TableName", table)
            .query("Frequency", frequency)
            .query("Year", &years)
            .query("ResultFormat", "JSON")
            .header("User-Agent", "Northstar/0.1 official-us-macro")
            .call()?;
        receipts.push(FetchedBeaReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_census_refresh(
    agent: &ureq::Agent,
    api_key: &str,
) -> Result<FetchedCensusReceipt, OfficialTransportError> {
    let year = current_utc_year();
    let time = format!("from {}-01", year - 2);
    let ts_started_ns = now_ns();
    let mut response = agent
        .get(CENSUS_MARTS_API_ROOT)
        .query(
            "get",
            "cell_value,data_type_code,time_slot_id,time_slot_date,category_code,seasonally_adj",
        )
        .query("time", &time)
        .query("data_type_code", "SM")
        .query("category_code", "44X72")
        .query("seasonally_adj", "yes")
        .query("key", api_key)
        .header("User-Agent", "Northstar/0.1 official-us-macro")
        .call()?;
    Ok(FetchedCensusReceipt {
        status_code: response.status().as_u16(),
        ts_started_ns,
        ts_received_ns: now_ns(),
        payload: response.body_mut().read_to_vec()?,
    })
}

fn fetch_complete_ecb_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedEcbReceipt>, OfficialTransportError> {
    let mut receipts = Vec::with_capacity(ECB_SERIES.len());
    for binding in ECB_SERIES {
        let url = format!("{ECB_API_ROOT}/{}", binding.provider_code);
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(&url)
            .query("format", "csvdata")
            .query("detail", "dataonly")
            .query("lastNObservations", "400")
            .header("Accept", "text/csv")
            .header("User-Agent", "Northstar/0.1 official-europe-macro")
            .call()?;
        receipts.push(FetchedEcbReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_complete_ons_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedOnsReceipt>, OfficialTransportError> {
    let mut receipts = Vec::with_capacity(ONS_SERIES.len());
    for binding in ONS_SERIES {
        let path = download_path(binding.provider_code)
            .ok_or_else(|| OfficialTransportError::Contract(binding.provider_code.into()))?;
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(ONS_DOWNLOAD_ROOT)
            .query("format", "csv")
            .query("uri", path)
            .header("Accept", "text/csv")
            .header("User-Agent", "Northstar/0.1 official-uk-macro")
            .call()?;
        receipts.push(FetchedOnsReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_complete_boe_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedBoeReceipt>, OfficialTransportError> {
    let year = current_utc_year();
    let date_from = format!("01/Jan/{}", year - 3);
    let date_to = format!("31/Dec/{year}");
    let mut receipts = Vec::with_capacity(BOE_SERIES.len());
    for binding in BOE_SERIES {
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(BOE_DATABASE_EXPORT)
            .query("csv.x", "yes")
            .query("Datefrom", &date_from)
            .query("Dateto", &date_to)
            .query("SeriesCodes", binding.provider_code)
            .query("CSVF", "TN")
            .query("UsingCodes", "Y")
            .query("VPD", "Y")
            .query("VFD", "N")
            .header("Accept", "text/csv")
            .header("User-Agent", "Northstar/0.1 official-uk-rates")
            .call()?;
        receipts.push(FetchedBoeReceipt {
            provider_code: binding.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn fetch_complete_boj_refresh(
    agent: &ureq::Agent,
) -> Result<Vec<FetchedBojReceipt>, OfficialTransportError> {
    let year = current_utc_year();
    let start_date = format!("{}01", year - 3);
    let mut receipts = Vec::with_capacity(BOJ_BINDINGS.len());
    for binding in BOJ_BINDINGS {
        let ts_started_ns = now_ns();
        let mut response = agent
            .get(BOJ_API_ROOT)
            .query("format", "json")
            .query("lang", "en")
            .query("db", binding.database)
            .query("code", binding.catalog.provider_code)
            .query("startDate", &start_date)
            .header("Accept", "application/json")
            .header("User-Agent", "Northstar/0.1 official-japan-macro")
            .call()?;
        receipts.push(FetchedBojReceipt {
            provider_code: binding.catalog.provider_code,
            status_code: response.status().as_u16(),
            ts_started_ns,
            ts_received_ns: now_ns(),
            payload: response.body_mut().read_to_vec()?,
        });
    }
    Ok(receipts)
}

fn ns_until(timestamp_ns: i64) -> Duration {
    Duration::from_nanos(u64::try_from(timestamp_ns.saturating_sub(now_ns())).unwrap_or_default())
}

fn duration_ns(duration: Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}

fn now_ns() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

fn current_utc_year() -> i64 {
    let days = now_ns().div_euclid(86_400_000_000_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    year
}

#[derive(Debug, Error)]
enum OfficialTransportError {
    #[error(transparent)]
    Http(#[from] ureq::Error),
    #[error("frozen provider contract is invalid: {0}")]
    Contract(String),
}

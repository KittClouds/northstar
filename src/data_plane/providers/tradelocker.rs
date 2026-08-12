//! TradeLocker provider payload termination.
//!
//! Table layouts are supplied by `/trade/config`; provider array positions are
//! never compiled into Northstar. Only normalized, typed venue truth leaves
//! this module.

use super::tradelocker_schema::json_shape;
use crate::data_plane::ids::InstrumentId;
use hashbrown::{HashMap, HashSet};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

pub const TRADELOCKER_SOURCE_NAME: &str = "TradeLocker";

#[derive(Clone, Debug)]
pub struct TradeLockerWireRefresh {
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub account_id: u64,
    pub acc_num: u64,
    pub account_name: Arc<str>,
    pub account_currency: Arc<str>,
    pub config: Value,
    pub instruments: Value,
    pub instrument_details: Box<[(u64, Value)]>,
    pub account_state: Value,
    pub positions: Value,
    pub orders: Value,
    pub order_history: Value,
    pub executions: Value,
    pub quotes: Box<[(u64, Value)]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeLockerBinding {
    pub instrument: InstrumentId,
    pub provider_symbol: Arc<str>,
    pub price_scale: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VenueSide {
    Buy,
    Sell,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct VenueAccountData {
    pub account_id: u64,
    pub acc_num: u64,
    pub name: Arc<str>,
    pub currency: Arc<str>,
    pub balance_micros: i64,
    pub projected_balance_micros: i64,
    pub available_funds_micros: i64,
    pub today_net_micros: i64,
    pub today_fees_micros: i64,
    pub open_net_pnl_micros: i64,
}

#[derive(Clone, Debug)]
pub struct VenueContractData {
    pub instrument: InstrumentId,
    pub provider_symbol: Arc<str>,
    pub tradable_instrument_id: u64,
    pub info_route_id: u32,
    pub trade_route_id: u32,
    pub session_id: u32,
    pub minimum_quantity_micros: i64,
    pub quantity_step_micros: i64,
    pub price_tick_nanos: i64,
    pub price_precision: u16,
    pub session_open: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VenueQuoteData {
    pub instrument: InstrumentId,
    pub tradable_instrument_id: u64,
    pub bid_scaled: i64,
    pub ask_scaled: i64,
    pub midpoint_scaled: i64,
    pub price_scale: i64,
}

#[derive(Clone, Debug)]
pub struct VenuePositionData {
    pub id: u64,
    pub instrument: InstrumentId,
    pub tradable_instrument_id: u64,
    pub side: VenueSide,
    pub quantity_micros: i64,
    pub average_price_nanos: i64,
    pub unrealized_pnl_micros: i64,
    pub opened_ms: i64,
}

#[derive(Clone, Debug)]
pub struct VenueOrderData {
    pub id: u64,
    pub position_id: u64,
    pub instrument: InstrumentId,
    pub tradable_instrument_id: u64,
    pub side: VenueSide,
    pub order_type: Arc<str>,
    pub status: Arc<str>,
    pub quantity_micros: i64,
    pub filled_quantity_micros: i64,
    pub average_price_nanos: i64,
    pub created_ms: i64,
    pub is_open: bool,
}

#[derive(Clone, Debug)]
pub struct VenueExecutionData {
    pub id: u64,
    pub order_id: u64,
    pub position_id: u64,
    pub instrument: InstrumentId,
    pub tradable_instrument_id: u64,
    pub side: VenueSide,
    pub quantity_micros: i64,
    pub price_nanos: i64,
    pub created_ms: i64,
}

#[derive(Clone, Debug)]
pub struct VenueEpochData {
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub config_fingerprint: [u8; 16],
    pub account: VenueAccountData,
    pub contracts: Arc<[VenueContractData]>,
    pub quotes: Arc<[VenueQuoteData]>,
    pub positions: Arc<[VenuePositionData]>,
    pub open_orders: Arc<[VenueOrderData]>,
    pub final_orders: Arc<[VenueOrderData]>,
    pub executions: Arc<[VenueExecutionData]>,
    pub unknown_orders: Arc<[u64]>,
    pub unknown_positions: Arc<[u64]>,
}

pub struct TradeLockerDecoder {
    bindings: Box<[TradeLockerBinding]>,
    known_orders: HashSet<u64>,
    known_positions: HashSet<u64>,
}

impl TradeLockerDecoder {
    pub fn new(
        mut bindings: Vec<TradeLockerBinding>,
        known_orders: HashSet<u64>,
        known_positions: HashSet<u64>,
    ) -> Result<Self, TradeLockerDecodeError> {
        if bindings.is_empty() {
            return Err(TradeLockerDecodeError::NoBindings);
        }
        bindings.sort_unstable_by_key(|binding| binding.instrument);
        for binding in &bindings {
            if binding.instrument == InstrumentId::UNKNOWN
                || binding.provider_symbol.is_empty()
                || binding.price_scale <= 0
            {
                return Err(TradeLockerDecodeError::InvalidBinding);
            }
        }
        if bindings
            .windows(2)
            .any(|pair| pair[0].instrument == pair[1].instrument)
        {
            return Err(TradeLockerDecodeError::DuplicateBinding);
        }
        Ok(Self {
            bindings: bindings.into_boxed_slice(),
            known_orders,
            known_positions,
        })
    }

    pub fn decode(
        &self,
        refresh: &TradeLockerWireRefresh,
    ) -> Result<VenueEpochData, TradeLockerDecodeError> {
        if refresh.ts_started_ns <= 0 || refresh.ts_received_ns < refresh.ts_started_ns {
            return Err(TradeLockerDecodeError::InvalidTimes);
        }
        let layouts = LayoutBook::from_config(&refresh.config)?;
        let instrument_rows = response_array(&refresh.instruments, "instruments")?;
        let mut by_provider_id = HashMap::<u64, InstrumentId>::with_capacity(self.bindings.len());
        let mut provider_rows = HashMap::<u64, &Value>::with_capacity(self.bindings.len());
        for row in instrument_rows {
            let provider_id = required_u64(row, "tradableInstrumentId")?;
            provider_rows.insert(provider_id, row);
            let name = required_str(row, "name")?;
            if let Some(binding) = self
                .bindings
                .iter()
                .find(|item| item.provider_symbol.as_ref() == name)
            {
                by_provider_id.insert(provider_id, binding.instrument);
            }
        }
        if by_provider_id.len() != self.bindings.len() {
            return Err(TradeLockerDecodeError::MissingBoundInstruments {
                expected: self.bindings.len(),
                actual: by_provider_id.len(),
            });
        }

        let quotes = self.decode_quotes(&by_provider_id, &refresh.quotes)?;
        let contracts = self.decode_contracts(
            &by_provider_id,
            &provider_rows,
            &refresh.instrument_details,
            &quotes,
        )?;
        let account = decode_account(refresh, &layouts.account)?;
        let positions = decode_positions(
            response_table(&refresh.positions, "positions")?,
            &layouts.positions,
            &by_provider_id,
        )?;
        let open_orders = decode_orders(
            response_table(&refresh.orders, "orders")?,
            &layouts.orders,
            &by_provider_id,
        )?;
        let final_orders = decode_orders(
            response_table(&refresh.order_history, "ordersHistory")?,
            &layouts.order_history,
            &by_provider_id,
        )?;
        let executions = decode_executions(
            response_table(&refresh.executions, "executions")?,
            &layouts.executions,
            &by_provider_id,
        )?;
        let unknown_orders = open_orders
            .iter()
            .filter(|order| !self.known_orders.contains(&order.id))
            .map(|order| order.id)
            .collect::<Vec<_>>()
            .into();
        let unknown_positions = positions
            .iter()
            .filter(|position| !self.known_positions.contains(&position.id))
            .map(|position| position.id)
            .collect::<Vec<_>>()
            .into();
        let fingerprint = blake3::hash(&serde_json::to_vec(&refresh.config)?);
        let mut config_fingerprint = [0u8; 16];
        config_fingerprint.copy_from_slice(&fingerprint.as_bytes()[..16]);
        Ok(VenueEpochData {
            ts_started_ns: refresh.ts_started_ns,
            ts_received_ns: refresh.ts_received_ns,
            config_fingerprint,
            account,
            contracts: contracts.into(),
            quotes: quotes.into(),
            positions: positions.into(),
            open_orders: open_orders.into(),
            final_orders: final_orders.into(),
            executions: executions.into(),
            unknown_orders,
            unknown_positions,
        })
    }

    fn decode_contracts(
        &self,
        ids: &HashMap<u64, InstrumentId>,
        instrument_rows: &HashMap<u64, &Value>,
        details: &[(u64, Value)],
        quotes: &[VenueQuoteData],
    ) -> Result<Vec<VenueContractData>, TradeLockerDecodeError> {
        if details.len() != ids.len() {
            return Err(TradeLockerDecodeError::IncompleteContractCapture);
        }
        let mut output = Vec::with_capacity(ids.len());
        for (provider_id, detail_response) in details {
            let detail = response_data(detail_response)?;
            let instrument = *ids
                .get(provider_id)
                .ok_or(TradeLockerDecodeError::UnknownInstrument(*provider_id))?;
            let row = instrument_rows
                .get(provider_id)
                .ok_or(TradeLockerDecodeError::UnknownInstrument(*provider_id))?;
            let routes = row
                .get("routes")
                .and_then(Value::as_array)
                .ok_or(TradeLockerDecodeError::MissingField("routes"))?;
            let route = |kind: &str| {
                routes.iter().find_map(|value| {
                    (value.get("type").and_then(Value::as_str) == Some(kind))
                        .then(|| value.get("id").and_then(value_u64))
                        .flatten()
                })
            };
            let binding = self
                .bindings
                .iter()
                .find(|binding| binding.instrument == instrument)
                .expect("validated binding identity");
            let quote = quotes
                .iter()
                .find(|quote| quote.instrument == instrument)
                .ok_or(TradeLockerDecodeError::IncompleteQuoteCapture)?;
            let midpoint = quote.midpoint_scaled as f64 / quote.price_scale as f64;
            let tick_size = contract_tick_size(detail, midpoint)?;
            let price_precision = first_optional_u64(detail, &["pricePrecision", "precision"])
                .map(u16::try_from)
                .transpose()
                .map_err(|_| TradeLockerDecodeError::NumberOutOfRange("pricePrecision"))?
                .unwrap_or(decimal_precision(tick_size)?);
            output.push(VenueContractData {
                instrument,
                provider_symbol: Arc::clone(&binding.provider_symbol),
                tradable_instrument_id: *provider_id,
                info_route_id: u32_checked(route("INFO"), "INFO route")?,
                trade_route_id: u32_checked(route("TRADE"), "TRADE route")?,
                session_id: u32_checked(
                    first_optional_u64(detail, &["tradeSessionId", "sessionId"]),
                    "session",
                )?,
                minimum_quantity_micros: quantity_micros(first_number(
                    detail,
                    &["minLot", "minQty", "minQuantity", "lotSize"],
                )?),
                quantity_step_micros: quantity_micros(first_number(
                    detail,
                    &["qtyStep", "quantityStep", "lotStep"],
                )?),
                price_tick_nanos: price_nanos(tick_size),
                price_precision,
                session_open: contract_session_open(detail),
            });
        }
        output.sort_unstable_by_key(|contract| contract.instrument);
        Ok(output)
    }

    fn decode_quotes(
        &self,
        ids: &HashMap<u64, InstrumentId>,
        quotes: &[(u64, Value)],
    ) -> Result<Vec<VenueQuoteData>, TradeLockerDecodeError> {
        if quotes.len() != ids.len() {
            return Err(TradeLockerDecodeError::IncompleteQuoteCapture);
        }
        let mut output = Vec::with_capacity(quotes.len());
        for (provider_id, response) in quotes {
            let instrument = *ids
                .get(provider_id)
                .ok_or(TradeLockerDecodeError::UnknownInstrument(*provider_id))?;
            let data = response_data(response)?;
            let bid = required_number(data, "bp")?;
            let ask = required_number(data, "ap")?;
            if !(bid.is_finite() && ask.is_finite() && bid > 0.0 && ask >= bid) {
                return Err(TradeLockerDecodeError::CrossedQuote(*provider_id));
            }
            let scale = self
                .bindings
                .iter()
                .find(|binding| binding.instrument == instrument)
                .expect("validated binding identity")
                .price_scale;
            let bid_scaled = scaled(bid, scale);
            let ask_scaled = scaled(ask, scale);
            output.push(VenueQuoteData {
                instrument,
                tradable_instrument_id: *provider_id,
                bid_scaled,
                ask_scaled,
                midpoint_scaled: bid_scaled.saturating_add(ask_scaled) / 2,
                price_scale: scale,
            });
        }
        output.sort_unstable_by_key(|quote| quote.instrument);
        Ok(output)
    }
}

#[derive(Clone, Debug)]
struct ColumnLayout {
    columns: HashMap<Arc<str>, usize>,
}

impl ColumnLayout {
    fn from_config(config: &Value, key: &'static str) -> Result<Self, TradeLockerDecodeError> {
        let columns = config
            .get(key)
            .and_then(|value| value.get("columns"))
            .and_then(Value::as_array)
            .ok_or(TradeLockerDecodeError::MissingLayout(key))?;
        let mut index = HashMap::with_capacity(columns.len());
        for (slot, column) in columns.iter().enumerate() {
            let id: Arc<str> = column
                .get("id")
                .and_then(Value::as_str)
                .ok_or(TradeLockerDecodeError::MalformedLayout(key))?
                .into();
            if index.insert(id, slot).is_some() {
                return Err(TradeLockerDecodeError::MalformedLayout(key));
            }
        }
        Ok(Self { columns: index })
    }

    fn value<'a>(
        &self,
        row: &'a [Value],
        key: &'static str,
    ) -> Result<&'a Value, TradeLockerDecodeError> {
        let slot = self
            .columns
            .get(key)
            .ok_or(TradeLockerDecodeError::MissingColumn(key))?;
        row.get(*slot).ok_or(TradeLockerDecodeError::ShortRow)
    }
}

struct LayoutBook {
    account: ColumnLayout,
    positions: ColumnLayout,
    orders: ColumnLayout,
    order_history: ColumnLayout,
    executions: ColumnLayout,
}

impl LayoutBook {
    fn from_config(response: &Value) -> Result<Self, TradeLockerDecodeError> {
        let config = response_data(response)?;
        Ok(Self {
            account: ColumnLayout::from_config(config, "accountDetailsConfig")?,
            positions: ColumnLayout::from_config(config, "positionsConfig")?,
            orders: ColumnLayout::from_config(config, "ordersConfig")?,
            order_history: ColumnLayout::from_config(config, "ordersHistoryConfig")?,
            executions: ColumnLayout::from_config(config, "filledOrdersConfig")?,
        })
    }
}

fn decode_account(
    refresh: &TradeLockerWireRefresh,
    layout: &ColumnLayout,
) -> Result<VenueAccountData, TradeLockerDecodeError> {
    let row = response_data(&refresh.account_state)?
        .get("accountDetailsData")
        .and_then(Value::as_array)
        .ok_or(TradeLockerDecodeError::MissingField("accountDetailsData"))?;
    let money = |key| -> Result<i64, TradeLockerDecodeError> {
        Ok(money_micros(value_number(layout.value(row, key)?)?))
    };
    Ok(VenueAccountData {
        account_id: refresh.account_id,
        acc_num: refresh.acc_num,
        name: Arc::clone(&refresh.account_name),
        currency: Arc::clone(&refresh.account_currency),
        balance_micros: money("balance")?,
        projected_balance_micros: money("projectedBalance")?,
        available_funds_micros: money("availableFunds")?,
        today_net_micros: money("todayNet")?,
        today_fees_micros: money("todayFees")?,
        open_net_pnl_micros: money("openNetPnL")?,
    })
}

fn decode_positions(
    rows: &[Value],
    layout: &ColumnLayout,
    ids: &HashMap<u64, InstrumentId>,
) -> Result<Vec<VenuePositionData>, TradeLockerDecodeError> {
    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let row = row.as_array().ok_or(TradeLockerDecodeError::ShortRow)?;
        let provider_id = value_u64(layout.value(row, "tradableInstrumentId")?)
            .ok_or(TradeLockerDecodeError::InvalidField("tradableInstrumentId"))?;
        let Some(&instrument) = ids.get(&provider_id) else {
            continue;
        };
        output.push(VenuePositionData {
            id: required_layout_u64(layout, row, "id")?,
            instrument,
            tradable_instrument_id: provider_id,
            side: value_side(layout.value(row, "side")?),
            quantity_micros: quantity_micros(value_number(layout.value(row, "qty")?)?),
            average_price_nanos: price_nanos(value_number(layout.value(row, "avgPrice")?)?),
            unrealized_pnl_micros: money_micros(value_number(layout.value(row, "unrealizedPl")?)?),
            opened_ms: required_layout_i64(layout, row, "openDate")?,
        });
    }
    output.sort_unstable_by_key(|position| position.id);
    Ok(output)
}

fn decode_orders(
    rows: &[Value],
    layout: &ColumnLayout,
    ids: &HashMap<u64, InstrumentId>,
) -> Result<Vec<VenueOrderData>, TradeLockerDecodeError> {
    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let row = row.as_array().ok_or(TradeLockerDecodeError::ShortRow)?;
        let provider_id = required_layout_u64(layout, row, "tradableInstrumentId")?;
        let Some(&instrument) = ids.get(&provider_id) else {
            continue;
        };
        output.push(VenueOrderData {
            id: required_layout_u64(layout, row, "id")?,
            position_id: optional_layout_u64(layout, row, "positionId"),
            instrument,
            tradable_instrument_id: provider_id,
            side: value_side(layout.value(row, "side")?),
            order_type: value_text(layout.value(row, "type")?),
            status: value_text(layout.value(row, "status")?),
            quantity_micros: quantity_micros(value_number(layout.value(row, "qty")?)?),
            filled_quantity_micros: quantity_micros(value_number(layout.value(row, "filledQty")?)?),
            average_price_nanos: price_nanos(value_number(layout.value(row, "avgPrice")?)?),
            created_ms: required_layout_i64(layout, row, "createdDate")?,
            is_open: layout.value(row, "isOpen")?.as_bool().unwrap_or(false),
        });
    }
    output.sort_unstable_by_key(|order| order.id);
    Ok(output)
}

fn decode_executions(
    rows: &[Value],
    layout: &ColumnLayout,
    ids: &HashMap<u64, InstrumentId>,
) -> Result<Vec<VenueExecutionData>, TradeLockerDecodeError> {
    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let row = row.as_array().ok_or(TradeLockerDecodeError::ShortRow)?;
        let provider_id = required_layout_u64(layout, row, "tradableInstrumentId")?;
        let Some(&instrument) = ids.get(&provider_id) else {
            continue;
        };
        output.push(VenueExecutionData {
            id: required_layout_u64(layout, row, "id")?,
            order_id: required_layout_u64(layout, row, "orderId")?,
            position_id: required_layout_u64(layout, row, "positionId")?,
            instrument,
            tradable_instrument_id: provider_id,
            side: value_side(layout.value(row, "side")?),
            quantity_micros: quantity_micros(value_number(layout.value(row, "qty")?)?),
            price_nanos: price_nanos(value_number(layout.value(row, "price")?)?),
            created_ms: required_layout_i64(layout, row, "createdDate")?,
        });
    }
    output.sort_unstable_by_key(|execution| execution.id);
    Ok(output)
}

fn response_data(value: &Value) -> Result<&Value, TradeLockerDecodeError> {
    if value
        .get("s")
        .and_then(Value::as_str)
        .is_some_and(|status| status != "ok")
    {
        return Err(TradeLockerDecodeError::ProviderStatus);
    }
    value
        .get("d")
        .ok_or(TradeLockerDecodeError::MissingField("d"))
}

fn response_array<'a>(
    value: &'a Value,
    key: &'static str,
) -> Result<&'a [Value], TradeLockerDecodeError> {
    response_data(value)?
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(TradeLockerDecodeError::MissingField(key))
}

fn response_table<'a>(
    value: &'a Value,
    key: &'static str,
) -> Result<&'a [Value], TradeLockerDecodeError> {
    response_array(value, key)
}

fn required_str<'a>(
    value: &'a Value,
    key: &'static str,
) -> Result<&'a str, TradeLockerDecodeError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(TradeLockerDecodeError::MissingField(key))
}

fn required_u64(value: &Value, key: &'static str) -> Result<u64, TradeLockerDecodeError> {
    value
        .get(key)
        .and_then(value_u64)
        .ok_or(TradeLockerDecodeError::MissingField(key))
}

fn required_number(value: &Value, key: &'static str) -> Result<f64, TradeLockerDecodeError> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(TradeLockerDecodeError::MissingField(key))
}

fn first_optional_u64(value: &Value, keys: &[&'static str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(value_u64))
}

fn contract_tick_size(value: &Value, midpoint: f64) -> Result<f64, TradeLockerDecodeError> {
    if let Some(direct) = value
        .get("priceIncrement")
        .and_then(number_option)
        .or_else(|| value.get("tickSize").and_then(number_option))
    {
        return (direct.is_finite() && direct > 0.0)
            .then_some(direct)
            .ok_or(TradeLockerDecodeError::InvalidField("tickSize"));
    }
    let result = value
        .get("tickSize")
        .and_then(Value::as_array)
        .ok_or(TradeLockerDecodeError::MissingField("tickSize"))?
        .iter()
        .filter_map(|tier| {
            let lower = tier
                .get("leftRangeLimit")
                .and_then(number_option)
                .unwrap_or(f64::NEG_INFINITY);
            let tick = tier.get("tickSize").and_then(number_option)?;
            ((lower.is_finite() || lower == f64::NEG_INFINITY)
                && tick.is_finite()
                && tick > 0.0
                && midpoint.is_finite()
                && midpoint >= lower)
                .then_some((lower, tick))
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, tick)| tick);
    if result.is_none() {
        eprintln!(
            "NORTHSTAR_TRADELOCKER_TICK_SCHEMA tick={} detail={}",
            json_shape(value.get("tickSize")),
            json_shape(Some(value))
        );
    }
    result.ok_or(TradeLockerDecodeError::InvalidField("tickSize"))
}

fn decimal_precision(value: f64) -> Result<u16, TradeLockerDecodeError> {
    let mut scaled = value;
    for precision in 0..=9u16 {
        if (scaled - scaled.round()).abs() <= 1e-9 {
            return Ok(precision);
        }
        scaled *= 10.0;
    }
    Err(TradeLockerDecodeError::InvalidField("tickSize"))
}

fn contract_session_open(value: &Value) -> bool {
    first_bool(value, &["tradable", "isTradingAllowed", "sessionOpen"])
        .unwrap_or_else(|| value.get("symbolStatus").and_then(Value::as_str) == Some("FULLY_OPEN"))
}

fn first_number(value: &Value, keys: &[&'static str]) -> Result<f64, TradeLockerDecodeError> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(number_option))
        .ok_or(TradeLockerDecodeError::MissingField(keys[0]))
}

fn first_bool(value: &Value, keys: &[&'static str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}

fn u32_checked(value: Option<u64>, label: &'static str) -> Result<u32, TradeLockerDecodeError> {
    u32::try_from(value.ok_or(TradeLockerDecodeError::MissingField(label))?)
        .map_err(|_| TradeLockerDecodeError::NumberOutOfRange(label))
}

fn value_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn value_number(value: &Value) -> Result<f64, TradeLockerDecodeError> {
    number_option(value).ok_or(TradeLockerDecodeError::InvalidNumber)
}

fn number_option(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn required_layout_u64(
    layout: &ColumnLayout,
    row: &[Value],
    key: &'static str,
) -> Result<u64, TradeLockerDecodeError> {
    value_u64(layout.value(row, key)?).ok_or(TradeLockerDecodeError::InvalidField(key))
}

fn optional_layout_u64(layout: &ColumnLayout, row: &[Value], key: &'static str) -> u64 {
    layout.value(row, key).ok().and_then(value_u64).unwrap_or(0)
}

fn required_layout_i64(
    layout: &ColumnLayout,
    row: &[Value],
    key: &'static str,
) -> Result<i64, TradeLockerDecodeError> {
    let value = layout.value(row, key)?;
    value
        .as_i64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or(TradeLockerDecodeError::InvalidField(key))
}

fn value_side(value: &Value) -> VenueSide {
    match value.as_str() {
        Some("buy" | "BUY") => VenueSide::Buy,
        Some("sell" | "SELL") => VenueSide::Sell,
        _ => VenueSide::Unknown,
    }
}

fn value_text(value: &Value) -> Arc<str> {
    value.as_str().unwrap_or("unknown").into()
}

fn scaled(value: f64, scale: i64) -> i64 {
    (value * scale as f64).round() as i64
}
fn money_micros(value: f64) -> i64 {
    scaled(value, 1_000_000)
}
fn quantity_micros(value: f64) -> i64 {
    scaled(value, 1_000_000)
}
fn price_nanos(value: f64) -> i64 {
    scaled(value, 1_000_000_000)
}

#[derive(Debug, Error)]
pub enum TradeLockerDecodeError {
    #[error("TradeLocker binding set is empty")]
    NoBindings,
    #[error("TradeLocker binding is invalid")]
    InvalidBinding,
    #[error("TradeLocker binding identity is duplicated")]
    DuplicateBinding,
    #[error("TradeLocker refresh times are invalid")]
    InvalidTimes,
    #[error("TradeLocker response status is not ok")]
    ProviderStatus,
    #[error("TradeLocker config has no {0} layout")]
    MissingLayout(&'static str),
    #[error("TradeLocker config has malformed {0} layout")]
    MalformedLayout(&'static str),
    #[error("TradeLocker config has no {0} column")]
    MissingColumn(&'static str),
    #[error("TradeLocker response is missing {0}")]
    MissingField(&'static str),
    #[error("TradeLocker response has invalid {0}")]
    InvalidField(&'static str),
    #[error("TradeLocker response contains a short table row")]
    ShortRow,
    #[error("TradeLocker response contains an invalid number")]
    InvalidNumber,
    #[error("TradeLocker {0} is out of range")]
    NumberOutOfRange(&'static str),
    #[error(
        "TradeLocker bound instrument capture is incomplete: expected {expected}, got {actual}"
    )]
    MissingBoundInstruments { expected: usize, actual: usize },
    #[error("TradeLocker contract capture is incomplete")]
    IncompleteContractCapture,
    #[error("TradeLocker quote capture is incomplete")]
    IncompleteQuoteCapture,
    #[error("TradeLocker returned unknown instrument {0}")]
    UnknownInstrument(u64),
    #[error("TradeLocker returned crossed/invalid quote for {0}")]
    CrossedQuote(u64),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
#[path = "tradelocker_tests.rs"]
mod tests;

use super::Availability;
use crate::data_plane::ids::InstrumentId;
use crate::data_plane::providers::tradelocker::{VenueEpochData, VenueSide};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct VenueAccountSnapshot {
    pub id: u64,
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
pub struct VenueContractSnapshot {
    pub instrument: InstrumentId,
    pub symbol: Arc<str>,
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
pub struct VenueQuoteSnapshotV1 {
    pub instrument: InstrumentId,
    pub tradable_instrument_id: u64,
    pub bid_scaled: i64,
    pub ask_scaled: i64,
    pub midpoint_scaled: i64,
    pub price_scale: i64,
}

#[derive(Clone, Debug)]
pub struct VenuePositionSnapshotV1 {
    pub id: u64,
    pub instrument: InstrumentId,
    pub side: VenueSide,
    pub quantity_micros: i64,
    pub average_price_nanos: i64,
    pub unrealized_pnl_micros: i64,
    pub opened_ms: i64,
}

#[derive(Clone, Debug)]
pub struct VenueOrderSnapshotV1 {
    pub id: u64,
    pub position_id: u64,
    pub instrument: InstrumentId,
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
pub struct VenueExecutionSnapshotV1 {
    pub id: u64,
    pub order_id: u64,
    pub position_id: u64,
    pub instrument: InstrumentId,
    pub side: VenueSide,
    pub quantity_micros: i64,
    pub price_nanos: i64,
    pub created_ms: i64,
}

#[derive(Clone, Debug)]
pub struct VenueSnapshot {
    pub generation: u64,
    pub availability: Availability,
    pub as_of_ns: i64,
    pub refresh_started_ns: i64,
    pub config_fingerprint: [u8; 16],
    pub account: Option<Arc<VenueAccountSnapshot>>,
    pub contracts: Arc<[VenueContractSnapshot]>,
    pub quotes: Arc<[VenueQuoteSnapshotV1]>,
    pub positions: Arc<[VenuePositionSnapshotV1]>,
    pub open_orders: Arc<[VenueOrderSnapshotV1]>,
    pub final_orders: Arc<[VenueOrderSnapshotV1]>,
    pub executions: Arc<[VenueExecutionSnapshotV1]>,
    pub unknown_orders: Arc<[u64]>,
    pub unknown_positions: Arc<[u64]>,
    pub health: Arc<str>,
}

impl VenueSnapshot {
    pub fn unavailable(availability: Availability, health: impl Into<Arc<str>>) -> Self {
        Self {
            generation: 0,
            availability,
            as_of_ns: 0,
            refresh_started_ns: 0,
            config_fingerprint: [0; 16],
            account: None,
            contracts: Arc::from([]),
            quotes: Arc::from([]),
            positions: Arc::from([]),
            open_orders: Arc::from([]),
            final_orders: Arc::from([]),
            executions: Arc::from([]),
            unknown_orders: Arc::from([]),
            unknown_positions: Arc::from([]),
            health: health.into(),
        }
    }

    pub fn from_epoch(generation: u64, epoch: VenueEpochData) -> Self {
        let unknown = epoch.unknown_orders.len() + epoch.unknown_positions.len();
        let health: Arc<str> = if unknown == 0 {
            "coherent / account truth reconciled".into()
        } else {
            format!("coherent / {unknown} unknown venue object(s)").into()
        };
        Self {
            generation,
            availability: Availability::Live,
            as_of_ns: epoch.ts_received_ns,
            refresh_started_ns: epoch.ts_started_ns,
            config_fingerprint: epoch.config_fingerprint,
            account: Some(Arc::new(VenueAccountSnapshot {
                id: epoch.account.account_id,
                acc_num: epoch.account.acc_num,
                name: epoch.account.name,
                currency: epoch.account.currency,
                balance_micros: epoch.account.balance_micros,
                projected_balance_micros: epoch.account.projected_balance_micros,
                available_funds_micros: epoch.account.available_funds_micros,
                today_net_micros: epoch.account.today_net_micros,
                today_fees_micros: epoch.account.today_fees_micros,
                open_net_pnl_micros: epoch.account.open_net_pnl_micros,
            })),
            contracts: epoch
                .contracts
                .iter()
                .map(|value| VenueContractSnapshot {
                    instrument: value.instrument,
                    symbol: Arc::clone(&value.provider_symbol),
                    tradable_instrument_id: value.tradable_instrument_id,
                    info_route_id: value.info_route_id,
                    trade_route_id: value.trade_route_id,
                    session_id: value.session_id,
                    minimum_quantity_micros: value.minimum_quantity_micros,
                    quantity_step_micros: value.quantity_step_micros,
                    price_tick_nanos: value.price_tick_nanos,
                    price_precision: value.price_precision,
                    session_open: value.session_open,
                })
                .collect::<Vec<_>>()
                .into(),
            quotes: epoch
                .quotes
                .iter()
                .map(|value| VenueQuoteSnapshotV1 {
                    instrument: value.instrument,
                    tradable_instrument_id: value.tradable_instrument_id,
                    bid_scaled: value.bid_scaled,
                    ask_scaled: value.ask_scaled,
                    midpoint_scaled: value.midpoint_scaled,
                    price_scale: value.price_scale,
                })
                .collect::<Vec<_>>()
                .into(),
            positions: epoch
                .positions
                .iter()
                .map(|value| VenuePositionSnapshotV1 {
                    id: value.id,
                    instrument: value.instrument,
                    side: value.side,
                    quantity_micros: value.quantity_micros,
                    average_price_nanos: value.average_price_nanos,
                    unrealized_pnl_micros: value.unrealized_pnl_micros,
                    opened_ms: value.opened_ms,
                })
                .collect::<Vec<_>>()
                .into(),
            open_orders: epoch
                .open_orders
                .iter()
                .map(order_snapshot)
                .collect::<Vec<_>>()
                .into(),
            final_orders: epoch
                .final_orders
                .iter()
                .map(order_snapshot)
                .collect::<Vec<_>>()
                .into(),
            executions: epoch
                .executions
                .iter()
                .map(|value| VenueExecutionSnapshotV1 {
                    id: value.id,
                    order_id: value.order_id,
                    position_id: value.position_id,
                    instrument: value.instrument,
                    side: value.side,
                    quantity_micros: value.quantity_micros,
                    price_nanos: value.price_nanos,
                    created_ms: value.created_ms,
                })
                .collect::<Vec<_>>()
                .into(),
            unknown_orders: epoch.unknown_orders,
            unknown_positions: epoch.unknown_positions,
            health,
        }
    }

    pub fn quote(&self, instrument: InstrumentId) -> Option<VenueQuoteSnapshotV1> {
        self.quotes
            .iter()
            .copied()
            .find(|quote| quote.instrument == instrument)
    }

    pub fn exposure_for(&self, instrument: InstrumentId) -> (i64, i64) {
        self.positions
            .iter()
            .filter(|position| position.instrument == instrument)
            .fold((0i64, 0i64), |(quantity, pnl), position| {
                let signed = match position.side {
                    VenueSide::Buy => position.quantity_micros,
                    VenueSide::Sell => -position.quantity_micros,
                    VenueSide::Unknown => 0,
                };
                (
                    quantity.saturating_add(signed),
                    pnl.saturating_add(position.unrealized_pnl_micros),
                )
            })
    }
}

fn order_snapshot(
    value: &crate::data_plane::providers::tradelocker::VenueOrderData,
) -> VenueOrderSnapshotV1 {
    VenueOrderSnapshotV1 {
        id: value.id,
        position_id: value.position_id,
        instrument: value.instrument,
        side: value.side,
        order_type: Arc::clone(&value.order_type),
        status: Arc::clone(&value.status),
        quantity_micros: value.quantity_micros,
        filled_quantity_micros: value.filled_quantity_micros,
        average_price_nanos: value.average_price_nanos,
        created_ms: value.created_ms,
        is_open: value.is_open,
    }
}

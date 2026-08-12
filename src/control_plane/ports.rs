use super::{IntentId, OrderIntent, VenueOrderId};
use crate::contracts::{Bar, IndexKey};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdapterHealth {
    pub connected: bool,
    pub synchronized: bool,
    pub latency_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BarsRequest {
    pub index: IndexKey,
    pub start_ns: u64,
    pub end_ns: u64,
    pub step_seconds: u32,
}

/// Cached Northstar terms which must exactly match TradeLocker discovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrumentContractTerms {
    pub index: IndexKey,
    pub minimum_quantity_micros: u64,
    pub quantity_step_micros: u64,
    pub price_tick_nanos: u64,
    pub margin_rate_ppm: u32,
    pub price_precision: u8,
}

/// Venue truth combines immutable contract terms with current route/session state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VenueInstrumentState {
    pub terms: InstrumentContractTerms,
    pub route_open: bool,
    pub session_open: bool,
}

impl VenueInstrumentState {
    #[inline]
    pub fn agrees_with(self, cached: InstrumentContractTerms) -> bool {
        self.terms == cached && self.route_open && self.session_open
    }
}

/// Reference-data-only surface for the future Massive Nautilus `DataClient`.
pub trait MassiveDataClient {
    fn health(&self) -> AdapterHealth;
    fn subscribe_indices(&mut self, indices: &[IndexKey]) -> Result<(), EngineError>;
    fn request_bars(&mut self, request: BarsRequest) -> Result<(), EngineError>;
}

/// Northstar-facing subset implemented by the pinned Nautilus integration crate.
pub trait NautilusEnginePort {
    fn health(&self) -> AdapterHealth;
    fn publish_bars(&mut self, index: IndexKey, bars: &[Bar]) -> Result<(), EngineError>;
    fn submit_intent(&mut self, intent: OrderIntent) -> Result<(), EngineError>;
    fn begin_reconciliation(&mut self, epoch: u64) -> Result<(), EngineError>;
    fn complete_reconciliation(&mut self, epoch: u64) -> Result<(), EngineError>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExecutionCommand {
    Submit(OrderIntent),
    Cancel {
        intent_id: IntentId,
        venue_order_id: VenueOrderId,
    },
}

/// Execution-only surface for the future TradeLocker Nautilus `ExecutionClient`.
pub trait TradeLockerExecutionClient {
    fn health(&self) -> AdapterHealth;
    fn discover_instruments(&mut self, indices: &[IndexKey]) -> Result<(), EngineError>;
    fn execute(&mut self, command: ExecutionCommand) -> Result<(), EngineError>;
    fn request_execution_snapshot(&mut self, epoch: u64) -> Result<(), EngineError>;
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum EngineError {
    #[error("adapter is disconnected")]
    Disconnected,
    #[error("unsupported capability: {0}")]
    Unsupported(&'static str),
    #[error("adapter rejected request: {0}")]
    Rejected(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms() -> InstrumentContractTerms {
        InstrumentContractTerms {
            index: IndexKey::Us100,
            minimum_quantity_micros: 100_000,
            quantity_step_micros: 100_000,
            price_tick_nanos: 100_000_000,
            margin_rate_ppm: 20_000,
            price_precision: 1,
        }
    }

    #[test]
    fn contract_agreement_includes_route_and_session_truth() {
        let cached = terms();
        let mut venue = VenueInstrumentState {
            terms: cached,
            route_open: true,
            session_open: true,
        };
        assert!(venue.agrees_with(cached));
        venue.route_open = false;
        assert!(!venue.agrees_with(cached));
        venue.route_open = true;
        venue.terms.quantity_step_micros = 1_000_000;
        assert!(!venue.agrees_with(cached));
    }
}

//! Northstar-owned trading control plane.
//!
//! Nautilus supplies trading infrastructure behind these contracts. Massive is
//! the reference-data plane and TradeLocker is the execution-truth plane.

mod coordinator;
mod lineage;
mod ports;
mod reconciliation;

pub use coordinator::{
    CoreBlock, CoreCoordinator, CoreMode, OrderIntent, OrderSide, RouteDisposition,
};
pub use lineage::{LineageError, StrategyLineage};
pub use ports::{
    AdapterHealth, BarsRequest, EngineError, ExecutionCommand, InstrumentContractTerms,
    MassiveDataClient, NautilusEnginePort, TradeLockerExecutionClient, VenueInstrumentState,
};
pub use reconciliation::{
    ExecutionReconciler, FillReport, IntentId, OrderSnapshot, OrderStatus, PositionSnapshot,
    ReconciliationError, ReconciliationState, VenueOrderId, VenuePositionId,
};

//! Production-facing state derived from the canonical journal.
//!
//! This layer is deliberately UI-agnostic. Live ingestion and exact replay
//! feed the same projectors, and GPUI consumes immutable snapshots published
//! here rather than provider payloads or mutable fixture state.

mod ingest;
mod ledger;
mod ledger_commands;
mod macro_feed;
mod macro_material;
mod macro_office;
mod macro_pipeline;
mod market;
mod massive_market;
mod massive_source;
mod official_sources;
mod operator_ledger;
mod runtime;
mod snapshot;
mod tradelocker_source;
mod venue;
mod venue_ledger;

pub use ingest::{MassiveDeskPipeline, MassiveIngestError, MassiveIngestReport};
pub use ledger::{
    LedgerProjector, LedgerRowSnapshot, LedgerSnapshot, TradeCaseSummary, LEDGER_VISIBLE_ROWS,
};
pub use ledger_commands::{AutomaticCasePolicy, AutomaticLedgerEventCommand};
pub use macro_feed::{MacroFeedSnapshot, MacroFeedState};
pub use macro_material::{MacroMaterialBatch, MacroMaterialError, MacroMaterialKind};
pub use macro_office::{
    MacroPointSnapshot, MacroProjector, MacroProjectorError, MacroReleaseSnapshot,
    MacroSeriesSnapshot, MacroSnapshot, PositioningMarketSnapshot, PositioningParticipantSnapshot,
    PositioningPointSnapshot, MACRO_HISTORY_POINTS, POSITIONING_HISTORY_WEEKS,
};
pub use macro_pipeline::{
    FetchedBeaReceipt, FetchedBlsCalendarReceipt, FetchedBoeReceipt, FetchedBojReceipt,
    FetchedCensusReceipt, FetchedCftcReceipt, FetchedEcbReceipt, FetchedEurostatReceipt,
    FetchedFredReceipt, FetchedMacroReceipt, FetchedOnsReceipt, MacroIngestReport, MacroPlane,
    MacroPlaneError, BEA_REFRESH_INTERVAL_NS, BLS_CALENDAR_REFRESH_INTERVAL_NS,
    BLS_REFRESH_INTERVAL_NS, BOE_REFRESH_INTERVAL_NS, BOJ_REFRESH_INTERVAL_NS,
    CENSUS_REFRESH_INTERVAL_NS, CFTC_REFRESH_INTERVAL_NS, ECB_REFRESH_INTERVAL_NS,
    EUROSTAT_REFRESH_INTERVAL_NS, FRED_REFRESH_INTERVAL_NS, ONS_REFRESH_INTERVAL_NS,
};
pub use market::{
    IndexMarketSnapshot, MarketInstrument, MarketProjector, MarketProjectorError,
    MarketProjectorHealth, MarketSnapshotBridge, ReferenceRole, ReferenceValueSnapshot,
    VenueQuoteSnapshot,
};
pub use massive_market::{
    projector_from_massive_catalog, MassiveMarketError, MassiveMarketIngestReport,
    MassiveMarketPlane,
};
pub use operator_ledger::{
    OperatorEntryKind, OperatorLedgerCommand, OperatorLedgerError, OperatorMutation,
};
pub use runtime::{LedgerCommandPort, LedgerRuntimeError, LedgerSubmitError, NorthstarRuntime};
pub use snapshot::{
    Availability, BarSeriesSnapshot, DeskSnapshot, DomainMask, DomainSnapshot, OfficeGeneration,
    OfficeSnapshot, OfficeSnapshotPort, OperatingMode, SnapshotNotice, SnapshotPublisher,
    SnapshotStore, SnapshotSubscription, ValueMeta, BAR_SERIES_BLOCK_LEN,
};
pub use venue::{
    VenueAccountSnapshot, VenueContractSnapshot, VenueExecutionSnapshotV1, VenueOrderSnapshotV1,
    VenuePositionSnapshotV1, VenueQuoteSnapshotV1, VenueSnapshot,
};

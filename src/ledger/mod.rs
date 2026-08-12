//! Durable causal journal for machine evidence and operator memory.
//!
//! Ledger is separate from the market-data journal: it links to canonical
//! sequences instead of copying quotes or bars, and it preserves every
//! amendment as a new event.

mod store;
mod types;

pub use store::{LedgerCommit, LedgerError, LedgerMappedEntry, LedgerMappedStore, LedgerStore};
pub use types::{
    ActorId, ActorKind, CorrelationId, LedgerEntryId, LedgerEventDraft, LedgerEventHeader,
    LedgerEventKind, LedgerEventStatus, LedgerFlags, SourceEventKey, TradeCaseId,
};

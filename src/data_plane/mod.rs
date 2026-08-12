//! Northstar-owned canonical data foundation.
//!
//! Provider formats terminate at [`source`] and [`receipt`]. Everything after
//! decoding is versioned Northstar data suitable for deterministic replay.

pub mod bars;
pub mod calendar;
pub mod catalog;
pub mod event;
pub mod export;
pub mod ids;
pub mod journal;
pub mod provenance;
pub mod providers;
pub mod receipt;
pub mod replay;
pub mod source;

pub use event::{
    CanonicalBatch, CanonicalEvent, EventFlags, EventKind, ParticipantClass, TimeQuality,
};
pub use ids::*;

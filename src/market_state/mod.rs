//! Deterministic structural market state derived from canonical II-A bars.
//!
//! This module owns market geometry. Renderers receive immutable projections
//! and never calculate levels, nodes, corridors, or market location.

mod contracts;
mod engine;
mod fingerprint;
mod graph;
mod level_book;
mod location;
mod producers;
mod projection;

pub use contracts::*;
pub use engine::{StructuralEngine, StructuralEngineError, StructuralUpdate};
pub use fingerprint::{fingerprint_mutations, fingerprint_snapshot, StructuralFingerprint};
pub use graph::{NodeBook, NodeMergeSpec};
pub use level_book::{LevelBook, LevelBookError};
pub use location::locate_market;
pub use producers::{
    OpeningRangeProducer, OpeningRangeSpec, RangeProjectionProducer, RangeProjectionSpec,
    SessionStructureProducer,
};
pub use projection::{project_chart_structure, ChartBand, ChartLevelLine, ChartStructureSnapshot};

#[cfg(test)]
mod tests;

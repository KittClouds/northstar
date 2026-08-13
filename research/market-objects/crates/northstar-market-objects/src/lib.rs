//! RG3 raw market-object verification, lossless packing, and derived views.
//!
//! The raw sensor files are authority. Derived representations bind both the
//! raw corpus digest and an explicit recipe digest.

mod gate15;
mod gate155;
mod gate155_analysis;
mod gate155_graph;
mod gate155_info;
mod gate155_types;
mod gate16;
mod gate165;
mod gate165_analysis;
mod gate165_types;
mod gate16_collect;
mod gate16_compare;
mod gate16_diagnostics;
mod gate16_distance;
mod gate16_graph;
mod gate16_perf;
mod gate16_repr;
#[cfg(test)]
mod gate16_tests;
mod gate16_types;
mod pack;
mod raw;
mod view;

pub use gate15::{
    CandidateAssignment, FittedFamilySystem, Gate15Report, discover_trajectory_families,
    fit_trajectory_family_systems,
};
pub use gate16::{Gate16Inputs, Gate16Reference, build_gate16_laboratory};
pub use gate16_perf::{Gate16PerformanceReport, run_gate16_performance_fixture};
pub use gate16_types::{Gate16Package, Gate16Report};
pub use gate155::{
    AtlasEdge, AtlasNode, AuctionIntervalIntersection, ConditionedSupportAudit, Gate155Package,
    Gate155Report, LineageCoordinate, NullAuditRow, ObjectCoordinate, PhenotypeCensusRow,
    StructuralBridgeReceipt, SubcohortStability, build_gate155_atlas,
};
pub use gate155_info::{CorrespondenceCell, InformationGeometry, PairwiseCorrespondence};
pub use gate165::{Gate165Inputs, Gate165Reference, build_gate165_census};
pub use gate165_types::{DIFFERENCE_AXES, Gate165Package, Gate165Report};
pub use pack::{PackReceipt, PackedCorpus, pack_raw_corpus};
pub use raw::{DATASETS, RawCorpus, RawError, RawReport};
pub use view::{DerivedReceipt, NormalizedExpansionSample, normalize_expansion_samples};

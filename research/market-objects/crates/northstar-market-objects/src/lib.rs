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
mod pack;
mod raw;
mod view;

pub use gate15::{
    CandidateAssignment, FittedFamilySystem, Gate15Report, discover_trajectory_families,
    fit_trajectory_family_systems,
};
pub use gate155::{
    AtlasEdge, AtlasNode, AuctionIntervalIntersection, ConditionedSupportAudit, Gate155Package,
    Gate155Report, LineageCoordinate, NullAuditRow, ObjectCoordinate, PhenotypeCensusRow,
    StructuralBridgeReceipt, SubcohortStability, build_gate155_atlas,
};
pub use gate155_info::{CorrespondenceCell, InformationGeometry, PairwiseCorrespondence};
pub use pack::{PackReceipt, PackedCorpus, pack_raw_corpus};
pub use raw::{DATASETS, RawCorpus, RawError, RawReport};
pub use view::{DerivedReceipt, NormalizedExpansionSample, normalize_expansion_samples};

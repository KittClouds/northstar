use crate::FittedFamilySystem;
use crate::gate155_info::PairwiseCorrespondence;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralBridgeReceipt {
    pub receipt_id: String,
    pub object_kind: String,
    pub run_key: String,
    pub canonical_instrument: String,
    pub object_id: i64,
    pub object_event_type: String,
    pub object_event_time: i64,
    pub master_snapshot_time: Option<i64>,
    pub snapshot_age_seconds: Option<i64>,
    pub join_mode: String,
    pub max_gap_seconds: i64,
    pub availability_code: String,
    pub master_generation: Option<u64>,
    pub master_snapshot_hash: Option<String>,
    pub regional_basis_hash: Option<String>,
    pub reference_price: Option<f64>,
    pub reference_atr: Option<f64>,
    pub median_price: Option<f64>,
    pub mean_price: Option<f64>,
    pub structural_sigma: Option<f64>,
    pub cog_price: Option<f64>,
    pub price_region_code: Option<i64>,
    pub nearest_node_id: Option<String>,
    pub nearest_node_lower: Option<f64>,
    pub nearest_node_price: Option<f64>,
    pub nearest_node_upper: Option<f64>,
    pub nearest_node_region_code: Option<i64>,
    pub node_contact: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectCoordinate {
    pub object_kind: String,
    pub run_key: String,
    pub canonical_instrument: String,
    pub object_id: i64,
    pub terminal_reason_code: i64,
    pub censored: bool,
    pub compression_summary_family_id: String,
    pub compression_shape_family_id: String,
    pub compression_hybrid_family_id: String,
    pub expansion_summary_family_id: String,
    pub expansion_shape_family_id: String,
    pub expansion_hybrid_family_id: String,
    pub structural_stratum: String,
    pub terminal_bridge_receipt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageCoordinate {
    pub lineage_id: String,
    pub run_key: String,
    pub canonical_instrument: String,
    pub origin_compression_id: i64,
    pub expansion_id: i64,
    pub destination_compression_id: Option<i64>,
    pub origin_compression_summary_family_id: String,
    pub origin_compression_shape_family_id: String,
    pub origin_compression_hybrid_family_id: String,
    pub expansion_summary_family_id: String,
    pub expansion_shape_family_id: String,
    pub expansion_hybrid_family_id: String,
    pub destination_compression_summary_family_id: String,
    pub destination_compression_shape_family_id: String,
    pub destination_compression_hybrid_family_id: String,
    pub origin_structural_stratum: String,
    pub terminal_structural_stratum: String,
    pub structural_node_contacts: usize,
    pub destination_availability_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasNode {
    pub node_key: String,
    pub node_type: String,
    pub local_label: String,
    pub authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasEdge {
    pub edge_key: String,
    pub edge_type: String,
    pub source_key: String,
    pub destination_key: String,
    pub event_time: Option<i64>,
    pub weight: usize,
    pub availability_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionIntervalIntersection {
    pub object_kind: String,
    pub run_key: String,
    pub object_id: i64,
    pub object_start_time: i64,
    pub object_end_time: i64,
    pub intersection_status: String,
    pub authority_status: String,
    pub overlap_start: Option<i64>,
    pub overlap_end: Option<i64>,
    pub overlap_seconds: Option<i64>,
    pub fraction_of_object_lifetime: Option<f64>,
    pub fraction_of_episode_lifetime: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcohortStability {
    pub object_kind: String,
    pub left_representation: String,
    pub right_representation: String,
    pub cohort_type: String,
    pub cohort_id: String,
    pub population: usize,
    pub support_class: String,
    pub js_divergence_bits: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionedSupportAudit {
    pub object_kind: String,
    pub stratum: String,
    pub left_representation: String,
    pub right_representation: String,
    pub eligible_population: usize,
    pub support_class: String,
    pub run_count: usize,
    pub instrument_counts: BTreeMap<String, usize>,
    pub window_counts: BTreeMap<String, usize>,
    pub null_structural_denominator: usize,
    pub blocked_unit: String,
    pub blocked_supported_units: usize,
    pub blocked_refinement_left_to_right_p10: Option<f64>,
    pub blocked_refinement_left_to_right_median: Option<f64>,
    pub blocked_refinement_left_to_right_p90: Option<f64>,
    pub transport_support: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullAuditRow {
    pub null_code: String,
    pub object_kind: String,
    pub representation: String,
    pub count: usize,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhenotypeCensusRow {
    pub phenotype_type: String,
    pub coordinate: String,
    pub structural_coordinate: String,
    pub count: usize,
    pub support_class: String,
    pub censored_count: usize,
    pub instrument_counts: BTreeMap<String, usize>,
    pub window_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate155Checks {
    pub rg3_corpus_unchanged: bool,
    pub master_point_state_present: bool,
    pub causal_bridge_receipts_complete: bool,
    pub bridge_monotonicity: bool,
    pub six_family_systems_unchanged: bool,
    pub null_is_not_family_node: bool,
    pub namespace_qualified: bool,
    pub auction_interval_authority_explicit: bool,
    pub confirmation_unopened: bool,
    pub no_consensus_taxonomy: bool,
    pub no_trading_interpretation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate155Report {
    pub contract: String,
    pub status: String,
    pub epistemic_status: String,
    pub source_run_count: usize,
    pub source_object_count: usize,
    pub source_corpus_sha256: String,
    pub source_auction_generation: u32,
    pub master_authority: String,
    pub auction_interval_authority: String,
    pub structural_bridge_receipts: usize,
    pub exact_receipts: usize,
    pub asof_receipts: usize,
    pub null_structural_receipts: usize,
    pub lineage_count: usize,
    pub lineage_with_destination: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub global_correspondence_count: usize,
    pub conditioned_correspondence_count: usize,
    pub constraint_motion_correspondence_count: usize,
    pub object_phenotype_count: usize,
    pub lineage_phenotype_count: usize,
    pub null_counts: BTreeMap<String, usize>,
    pub checks: Gate155Checks,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate155Package {
    pub report: Gate155Report,
    pub fitted_family_systems: Vec<FittedFamilySystem>,
    pub bridge_receipts: Vec<StructuralBridgeReceipt>,
    pub object_coordinates: Vec<ObjectCoordinate>,
    pub lineage_coordinates: Vec<LineageCoordinate>,
    pub atlas_nodes: Vec<AtlasNode>,
    pub atlas_edges: Vec<AtlasEdge>,
    pub global_correspondence: Vec<PairwiseCorrespondence>,
    pub conditioned_correspondence: Vec<PairwiseCorrespondence>,
    pub constraint_motion_correspondence: Vec<PairwiseCorrespondence>,
    pub subcohort_stability: Vec<SubcohortStability>,
    pub conditioned_support_audit: Vec<ConditionedSupportAudit>,
    pub constraint_motion_support_audit: Vec<ConditionedSupportAudit>,
    pub object_phenotype_census: Vec<PhenotypeCensusRow>,
    pub lineage_phenotype_census: Vec<PhenotypeCensusRow>,
    pub null_audit: Vec<NullAuditRow>,
    pub auction_intersections: Vec<AuctionIntervalIntersection>,
}

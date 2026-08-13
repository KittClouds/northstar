use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DIFFERENCE_AXES: [&str; 13] = [
    "raw_history_sha256",
    "duration_seconds",
    "observed_bars",
    "raw_direction",
    "censor_state",
    "terminal_mechanism",
    "event_multiplicity",
    "event_order_and_type",
    "event_timing_gaps",
    "raw_summary_coordinates",
    "raw_continuous_trajectory",
    "attempt_lineage",
    "branch_merge_topology",
];

pub const AXIS_EXACT_DIFFERENT: u32 = 1;
pub const AXIS_NOT_EVALUABLE: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairCensusRow {
    pub pair_id: String,
    pub object_kind: String,
    pub left_object_key: String,
    pub right_object_key: String,
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub status: String,
    pub reason: String,
    pub distance: Option<f64>,
    pub zero_class: String,
    pub support_points: usize,
    pub support_bars: u32,
    pub support_seconds: u32,
    pub left_censored: bool,
    pub right_censored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPairSummary {
    pub object_kind: String,
    pub contract_id: String,
    pub total_same_kind_pairs: usize,
    pub comparable_pairs: usize,
    pub exact_zero_pairs: usize,
    pub epsilon_near_pairs: usize,
    pub nonzero_pairs: usize,
    pub not_comparable_pairs: usize,
    pub exhaustive_stream_blake3: String,
    pub retained_row_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDifferenceReceipt {
    pub pair_id: String,
    pub object_kind: String,
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub axis_status_bits: u32,
}

impl RawDifferenceReceipt {
    pub fn axis_status(&self, axis_index: usize) -> u32 {
        (self.axis_status_bits >> (axis_index * 2)) & 0b11
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectAuthorityRow {
    pub object_key: String,
    pub object_kind: String,
    pub raw_history_sha256: String,
    pub duration_seconds: u32,
    pub observed_bars: u32,
    pub raw_direction: i8,
    pub censored: bool,
    pub terminal_reason_code: i16,
    pub event_multiplicity: usize,
    pub event_order_and_type_blake3: String,
    pub event_timing_gaps_blake3: String,
    pub raw_summary_coordinates_blake3: String,
    pub raw_continuous_trajectory_blake3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LossProfile {
    pub object_kind: String,
    pub contract_id: String,
    pub eligible_objects: usize,
    pub total_same_kind_pairs: usize,
    pub comparable_pairs: usize,
    pub exact_zero_pairs: usize,
    pub epsilon_near_pairs: usize,
    pub nonzero_pairs: usize,
    pub not_comparable_pairs: usize,
    pub participating_objects: usize,
    pub material_history_pairs: usize,
    pub fixed_domain: bool,
    pub relation_reflexive: bool,
    pub relation_symmetric: bool,
    pub relation_transitive: String,
    pub quotient_authorized: bool,
    pub equivalence_class_count: Option<usize>,
    pub singleton_objects: Option<usize>,
    pub largest_class: Option<usize>,
    pub class_size_counts: BTreeMap<usize, usize>,
    pub raw_difference_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceClassRow {
    pub contract_id: String,
    pub object_kind: String,
    pub class_id: String,
    pub class_size: usize,
    pub object_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRepresentationSet {
    pub object_kind: String,
    pub left_contract_id: String,
    pub right_contract_id: String,
    pub left_zero_pairs: usize,
    pub right_zero_pairs: usize,
    pub intersection_pairs: usize,
    pub left_only_pairs: usize,
    pub right_only_pairs: usize,
    pub union_pairs: usize,
    pub jaccard: Option<f64>,
    pub left_containment: Option<f64>,
    pub right_containment: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizationTurnover {
    pub object_kind: String,
    pub object_key: String,
    pub raw_contract_id: String,
    pub canonical_contract_id: String,
    pub retained_neighbors: usize,
    pub entered_neighbors: usize,
    pub exited_neighbors: usize,
    pub union_neighbors: usize,
    pub membership_jaccard: f64,
    pub shared_rank_correlation: Option<f64>,
    pub raw_direction: i8,
    pub duration_seconds: u32,
    pub terminal_reason_code: i16,
    pub censored: bool,
    pub instrument: String,
    pub run_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphIncrementalAudit {
    pub object_kind: String,
    pub event_contract_id: String,
    pub graph_contract_id: String,
    pub both_zero: usize,
    pub event_only_zero: usize,
    pub graph_only_zero: usize,
    pub authoritative_graph_only_distinction: String,
    pub branch_merge_authority: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate15GeometryAudit {
    pub object_kind: String,
    pub representation_id: String,
    pub eligible_anchors: usize,
    pub same_family_nearest: usize,
    pub different_family_nearest: usize,
    pub null_family_anchors: usize,
    pub same_family_rate_excluding_null: Option<f64>,
    pub instrument_support: usize,
    pub run_support: usize,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterBlockRow {
    pub block_type: String,
    pub block_id: String,
    pub anchors: usize,
    pub same_stratum_pairs: usize,
    pub different_stratum_pairs: usize,
    pub median_anchor_delta: Option<f64>,
    pub support_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterBlockedAudit {
    pub contract_id: String,
    pub anchors_with_available_context: usize,
    pub null_structural_context: usize,
    pub exact_join_count: usize,
    pub asof_join_count: usize,
    pub unavailable_join_count: usize,
    pub pooled_same_stratum_median: Option<f64>,
    pub pooled_different_stratum_median: Option<f64>,
    pub pooled_median_difference: Option<f64>,
    pub anchor_delta_median: Option<f64>,
    pub positive_delta_fraction: Option<f64>,
    pub bootstrap_replicates: usize,
    pub bootstrap_p10: Option<f64>,
    pub bootstrap_median: Option<f64>,
    pub bootstrap_p90: Option<f64>,
    pub run_blocks: usize,
    pub instrument_blocks: usize,
    pub calendar_window_blocks: usize,
    pub supported_blocks: usize,
    pub support_status: String,
    pub block_rows: Vec<MasterBlockRow>,
    pub auction_interval_status: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate165Checks {
    pub rg3_unchanged: bool,
    pub gate15_unchanged: bool,
    pub gate15_5_unchanged: bool,
    pub gate16_unchanged: bool,
    pub confirmation_unopened: bool,
    pub exhaustive_compare_authoritative: bool,
    pub exact_zero_near_zero_separate: bool,
    pub fixed_domain_quotients_verified: bool,
    pub partial_domains_not_mislabeled: bool,
    pub all_zero_pairs_have_difference_receipts: bool,
    pub pair_accounting_reconciles: bool,
    pub input_order_invariant: bool,
    pub pair_orientation_invariant: bool,
    pub parallel_deterministic: bool,
    pub scalar_simd_parity: bool,
    pub no_representation_repaired: bool,
    pub no_representation_superiority: bool,
    pub no_new_family: bool,
    pub no_prediction: bool,
    pub no_economic_interpretation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate165Report {
    pub contract: String,
    pub status: String,
    pub epistemic_status: String,
    pub source_corpus_sha256: String,
    pub source_gate16_lab_sha256: String,
    pub source_run_count: usize,
    pub source_object_count: usize,
    pub total_same_kind_pairs: usize,
    pub contract_pair_evaluations: usize,
    pub exact_zero_relations: usize,
    pub unique_exact_zero_object_pairs: usize,
    pub unique_exact_zero_object_pairs_by_kind: BTreeMap<String, usize>,
    pub epsilon_near_relations: usize,
    pub not_comparable_relations: usize,
    pub checks: Gate165Checks,
    pub limitations: Vec<String>,
    pub final_statement: String,
}

#[derive(Debug, Clone)]
pub struct Gate165Package {
    pub report: Gate165Report,
    pub pair_census: Vec<PairCensusRow>,
    pub pair_summaries: Vec<ContractPairSummary>,
    pub raw_differences: Vec<RawDifferenceReceipt>,
    pub object_authority: Vec<ObjectAuthorityRow>,
    pub loss_profiles: Vec<LossProfile>,
    pub equivalence_classes: Vec<EquivalenceClassRow>,
    pub cross_representation_sets: Vec<CrossRepresentationSet>,
    pub canonicalization_turnover: Vec<CanonicalizationTurnover>,
    pub graph_incremental_audit: Vec<GraphIncrementalAudit>,
    pub gate15_geometry_audit: Vec<Gate15GeometryAudit>,
    pub master_blocked_audit: MasterBlockedAudit,
}

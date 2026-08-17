use obs_open_04a_g1::model::{KernelContext, KernelState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReducedKey {
    pub upper_value_bits: u64,
    pub lower_value_bits: u64,
    pub upper_giveback_bits: u64,
    pub lower_giveback_bits: u64,
    pub range_locations: Vec<Option<u8>>,
    pub upper_extensions_bits: Vec<Option<u64>>,
    pub lower_extensions_bits: Vec<Option<u64>>,
    pub window_active: bool,
    pub coverage_complete: bool,
}

#[derive(Debug, Clone)]
pub struct ReplayRecord {
    pub session_id: String,
    pub prefix_ordinal: u32,
    pub state: KernelState,
    pub context: KernelContext,
    pub reduced: ReducedKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRef {
    pub pair_id: String,
    pub fiber_id: String,
    pub left_session_id: String,
    pub left_prefix_ordinal: u32,
    pub right_session_id: String,
    pub right_prefix_ordinal: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceSignature {
    pub semantic_distinctions: Vec<String>,
    pub descriptive_not_causal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualBoundaryRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub first_protected_fracture: String,
    pub attrition_reason: String,
    pub starting_difference_signature: DifferenceSignature,
    pub fracture_ancestry_difference_subset: Vec<String>,
    pub mechanism_class_id: String,
    pub mechanism_signature: MechanismSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiberStratum {
    pub fiber_id: String,
    pub e0_lawful_pairs: u64,
    pub e1_ordinal_matched_pairs: u64,
    pub e2_boundary_equal_pairs: u64,
    pub not_g6_comparable: u64,
    pub ordinal_mismatch: u64,
    pub semantic_time_control_mismatch: u64,
    pub other_epsilon_protected_fracture: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensusSummary {
    pub schema: String,
    pub status: String,
    pub records_replayed: u64,
    pub observed_fibers: u64,
    pub collision_fibers: u64,
    pub same_fiber_candidate_pairs: u64,
    pub e0_exact_cardinality: u64,
    pub e1_exact_cardinality: u64,
    pub e2_exact_cardinality: u64,
    pub e1_minus_e2_exact_cardinality: u64,
    pub not_g6_comparable: u64,
    pub ordinal_mismatch: u64,
    pub semantic_time_control_mismatch: u64,
    pub other_epsilon_protected_fracture: u64,
    pub continuation_evaluations: u64,
    pub exact_enumeration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2PairRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub remaining_joint_steps: u64,
    pub starting_difference_signature: DifferenceSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WitnessToken {
    pub token_id: String,
    pub open_delta_ticks: i64,
    pub high_delta_ticks: i64,
    pub low_delta_ticks: i64,
    pub close_delta_ticks: i64,
    pub coverage_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MechanismSignature {
    pub fracture_class: String,
    pub ancestry_relevant_starting_distinctions: Vec<String>,
    pub first_computational_divergence_role: String,
    pub first_transition_control_divergence_class: String,
    pub first_protected_observable_fracture: String,
    pub tau_comp: String,
    pub tau_obs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixStepReceipt {
    pub step: u32,
    pub protected_equal: bool,
    pub comparison_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayedFractureRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub witness: Vec<WitnessToken>,
    pub witness_class: String,
    pub equal_prefix: Vec<PrefixStepReceipt>,
    pub tau_comp: String,
    pub tau_obs: u32,
    pub first_computational_divergence: String,
    pub first_protected_fracture: String,
    pub fracture_ancestry_difference_subset: Vec<String>,
    pub mechanism_signature: MechanismSignature,
    pub mechanism_class_id: String,
    pub verifier_receipt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchExposure {
    #[serde(flatten)]
    pub pair: PairRef,
    pub status: String,
    pub search_box_id: String,
    pub continuations_tested: u32,
    pub admissible_continuations_tested: u32,
    pub shells_completed: u32,
    pub canonical_first_witness_rank: Option<u32>,
    pub termination_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessAudit {
    pub raw_source_sha256: String,
    pub real_04a_history_reads: u64,
    pub d_a_sessions_decoded: u64,
    pub d_a_bars_replayed: u64,
    pub d_a_path_gap_sessions: u64,
    pub d_b_reads: u64,
    pub d_c_reads: u64,
    pub d_d_reads: u64,
    pub target_reads: u64,
    pub outcome_reads: u64,
    pub external_optic_reads: u64,
    pub g10_claims: u64,
}

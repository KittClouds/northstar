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
pub struct ComparabilityRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub same_04a_fiber: bool,
    pub g6_context_fiber_match: bool,
    pub reachability: String,
    pub continuation_language: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta04a {
    pub upper_id_changed: bool,
    pub upper_birth_bar_changed: bool,
    pub upper_birth_time_changed: bool,
    pub upper_age_changed: bool,
    pub lower_id_changed: bool,
    pub lower_birth_bar_changed: bool,
    pub lower_birth_time_changed: bool,
    pub lower_age_changed: bool,
    pub transition_ordinal_changed: bool,
    pub authoritative_time_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessToken {
    pub token_id: String,
    pub open_delta_ticks: i64,
    pub high_delta_ticks: i64,
    pub low_delta_ticks: i64,
    pub close_delta_ticks: i64,
    pub coverage_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractureRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub map_id: String,
    pub delta_04a: Delta04a,
    pub witness: Option<WitnessToken>,
    pub witness_class: String,
    pub equal_protected_prefix_steps: u32,
    pub first_computational_divergence: u32,
    pub first_protected_observable_divergence: u32,
    pub first_divergent_observable: String,
    pub first_divergent_transition: String,
    pub g2_role_ancestry: Vec<String>,
    pub g4_fracture_surface: Vec<String>,
    pub reachability_evidence: String,
    pub verifier_receipt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRecord {
    #[serde(flatten)]
    pub pair: PairRef,
    pub pair_status: String,
    pub epsilon_tested: bool,
    pub continuations_tested: u32,
    pub admissible_continuations_tested: u32,
    pub verified_separators: u32,
    pub bounded_separator_incidence_numerator: u32,
    pub bounded_separator_incidence_denominator: u32,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapInventoryRecord {
    pub map_id: String,
    pub source_representation: String,
    pub target_representation: String,
    pub relation_class: String,
    pub execution_status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessAudit {
    pub raw_source_sha256: String,
    pub firewall_manifest_sha256: String,
    pub source_prefix_rows_scanned: u64,
    pub source_prefix_sha256: String,
    pub real_04a_history_reads: u64,
    pub d_a_sessions_decoded: u64,
    pub d_a_bars_replayed: u64,
    pub d_a_path_gap_sessions: u64,
    pub d_a_path_gap_records: u64,
    pub next_source_row_requested: bool,
    pub inst_seal_members_verified: u64,
    pub measurement_seal_members_verified: u64,
    pub b2_seal_members_verified: u64,
    pub b2_state: String,
    pub d_b_session_ids_decoded_for_outcomes: u64,
    pub d_b_reads: u64,
    pub d_c_reads: u64,
    pub d_d_reads: u64,
    pub target_reads: u64,
    pub outcome_reads: u64,
    pub external_optic_reads_before_primary_seal: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub schema: String,
    pub status: String,
    pub map_status: String,
    pub primary_map_id: String,
    pub records_replayed: u64,
    pub observed_fibers: u64,
    pub collision_fibers: u64,
    pub same_fiber_pairs: u64,
    pub lawful_comparison_pairs: u64,
    pub immediate_fractures: u64,
    pub delayed_fractures: u64,
    pub bounded_silence_pairs: u64,
    pub not_evaluable_pairs: u64,
    pub access: AccessAudit,
    pub restrictions: Vec<String>,
}

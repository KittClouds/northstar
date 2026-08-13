use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TRAJECTORY_POINTS: usize = 21;
pub const NEIGHBOR_K: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeometryObjectKind {
    Compression,
    Expansion,
}

impl GeometryObjectKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Compression => "COMPRESSION",
            Self::Expansion => "EXPANSION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Availability {
    Available,
    CensoredSuffix,
    NotApplicable,
    NotEvaluable,
    DataGap,
    NotComparable,
}

impl Availability {
    pub fn name(self) -> &'static str {
        match self {
            Self::Available => "AVAILABLE",
            Self::CensoredSuffix => "CENSORED_SUFFIX",
            Self::NotApplicable => "NOT_APPLICABLE",
            Self::NotEvaluable => "NOT_EVALUABLE",
            Self::DataGap => "DATA_GAP",
            Self::NotComparable => "NOT_COMPARABLE",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TrajectorySample {
    pub elapsed_bars: u32,
    pub elapsed_seconds: u32,
    pub continuous_raw: Vec<f32>,
    pub continuous_canonical: Vec<f32>,
    pub state_raw: i16,
    pub state_canonical: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventToken {
    pub event_code: i16,
    pub direction: i8,
    pub state_code: i16,
    pub terminal_reason_code: i16,
    pub delta_bars: u32,
    pub delta_seconds: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct ProcessGeometryObject {
    pub kind: GeometryObjectKind,
    pub run_key: String,
    pub instrument: String,
    pub object_id: i64,
    pub start_time: i64,
    pub terminal_time: i64,
    pub terminal_reason_code: i16,
    pub direction: i8,
    pub censored: bool,
    pub raw_history_sha256: String,
    pub summary_raw: Vec<f32>,
    pub summary_canonical: Vec<f32>,
    pub categories_raw: [i16; 2],
    pub categories_canonical: [i16; 2],
    pub trajectory: Vec<TrajectorySample>,
    pub events_raw: Vec<EventToken>,
    pub events_canonical: Vec<EventToken>,
}

impl ProcessGeometryObject {
    pub fn key(&self) -> String {
        format!("{}::{}::{}", self.run_key, self.kind.name(), self.object_id)
    }

    pub fn complete(&self) -> bool {
        !self.censored
    }

    pub fn observed_bars(&self) -> u32 {
        self.trajectory
            .last()
            .map_or(0, |sample| sample.elapsed_bars)
    }

    pub fn observed_seconds(&self) -> u32 {
        self.trajectory
            .last()
            .map_or(0, |sample| sample.elapsed_seconds)
    }

    pub fn duration_seconds(&self) -> u32 {
        self.terminal_time.saturating_sub(self.start_time).max(0) as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityCensusRow {
    pub object_kind: String,
    pub coordinate: String,
    pub classification: String,
    pub source: String,
    pub availability: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentationManifest {
    pub representation_id: String,
    pub object_kind: String,
    pub intrinsic: bool,
    pub input_schema_sha256: String,
    pub feature_or_channel_order: Vec<String>,
    pub units: Vec<String>,
    pub normalization: String,
    pub resampling: String,
    pub censor_policy: String,
    pub reflection_policy: String,
    pub canonicalization_policy: String,
    pub distance_contracts: Vec<String>,
    pub missingness_policy: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustScale {
    pub object_kind: String,
    pub representation_id: String,
    pub medians: Vec<f32>,
    pub iqrs: Vec<f32>,
    pub fitted_population: usize,
    pub fit_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReceipt {
    pub left_object_key: String,
    pub right_object_key: String,
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub status: String,
    pub reason: String,
    pub distance: Option<f64>,
    pub comparable_support_points: usize,
    pub comparable_support_bars: u32,
    pub comparable_support_seconds: u32,
    pub observed_fraction_left: f64,
    pub observed_fraction_right: f64,
    pub left_censored: bool,
    pub right_censored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborRow {
    pub object_key: String,
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub rank: usize,
    pub neighbor_key: String,
    pub distance: f64,
    pub comparable_support_points: usize,
    pub comparable_support_bars: u32,
    pub comparable_support_seconds: u32,
    pub object_censored: bool,
    pub neighbor_censored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDiagnostic {
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub identity: String,
    pub nonnegative: String,
    pub symmetric: String,
    pub triangle_inequality: String,
    pub tested_pairs: usize,
    pub tested_triplets: usize,
    pub maximum_symmetry_error: f64,
    pub maximum_triangle_violation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceProbe {
    pub object_kind: String,
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub left_object_key: String,
    pub right_object_key: String,
    pub identity_distance: f64,
    pub pair_distance: f64,
    pub symmetry_error: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialDistanceCoordinate {
    pub coordinate_id: String,
    pub status: String,
    pub reason: String,
    pub distance: Option<f64>,
    pub support_points: usize,
    pub support_bars: u32,
    pub support_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialDistanceVectorRow {
    pub object_key: String,
    pub anchor_object_key: String,
    pub anchor_basis: String,
    pub coordinates: Vec<PartialDistanceCoordinate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionDiagnostic {
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub eligible_objects: usize,
    pub comparable_pairs: usize,
    pub collision_pairs: usize,
    pub materially_distinct_history_collisions: usize,
    pub epsilon: f64,
    pub audit_scope: String,
    pub examples: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalGeometryDiagnostic {
    pub representation_id: String,
    pub distance_contract: String,
    pub comparison_mode: String,
    pub eligible_objects: usize,
    pub objects_with_k_neighbors: usize,
    pub nearest_p10: Option<f64>,
    pub nearest_median: Option<f64>,
    pub nearest_p90: Option<f64>,
    pub kth_median: Option<f64>,
    pub pair_distance_p10: Option<f64>,
    pub pair_distance_median: Option<f64>,
    pub pair_distance_p90: Option<f64>,
    pub distance_concentration_ratio: Option<f64>,
    pub maximum_hub_count: usize,
    pub isolated_object_rate: f64,
    pub distance_sample_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborhoodAgreement {
    pub object_kind: String,
    pub left_representation: String,
    pub right_representation: String,
    pub comparable_objects: usize,
    pub mean_top_k_jaccard: f64,
    pub mean_shared_rank_correlation: Option<f64>,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub representation_id: String,
    pub preserves: Vec<String>,
    pub destroys_or_omits: Vec<String>,
    pub comparison_domain: String,
    pub formal_status: String,
    pub packed_bytes_per_object: Option<f64>,
    pub graph_incremental_information_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrospectiveDiagnostic {
    pub question: String,
    pub population: usize,
    pub result: String,
    pub interpretation: String,
    pub optimization_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate16Checks {
    pub rg3_unchanged: bool,
    pub gate15_unchanged: bool,
    pub gate15_5_unchanged: bool,
    pub confirmation_unopened: bool,
    pub authority_census_complete: bool,
    pub summary_contracts_frozen: bool,
    pub trajectory_contracts_frozen: bool,
    pub event_contracts_frozen: bool,
    pub graph_contracts_frozen: bool,
    pub reflection_involutive: bool,
    pub canonicalization_idempotent: bool,
    pub censored_suffix_not_fabricated: bool,
    pub shared_prefix_proven: bool,
    pub null_semantics_preserved: bool,
    pub input_order_invariant: bool,
    pub parallel_deterministic: bool,
    pub scalar_simd_parity: bool,
    pub no_universal_winner: bool,
    pub no_new_family: bool,
    pub no_trading_interpretation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate16Report {
    pub contract: String,
    pub status: String,
    pub epistemic_status: String,
    pub source_corpus_sha256: String,
    pub source_run_count: usize,
    pub source_object_count: usize,
    pub completed_objects: usize,
    pub censored_objects: usize,
    pub representation_count: usize,
    pub distance_contract_count: usize,
    pub comparison_receipt_count: usize,
    pub partial_distance_vector_count: usize,
    pub neighborhood_row_count: usize,
    pub packed_vector_bytes: usize,
    pub availability_counts: BTreeMap<String, usize>,
    pub checks: Gate16Checks,
    pub limitations: Vec<String>,
    pub final_statement: String,
}

#[derive(Debug, Clone)]
pub struct Gate16Package {
    pub report: Gate16Report,
    pub authority_census: Vec<AuthorityCensusRow>,
    pub manifests: Vec<RepresentationManifest>,
    pub scales: Vec<RobustScale>,
    pub comparison_receipts: Vec<ComparisonReceipt>,
    pub neighbors: Vec<NeighborRow>,
    pub metric_diagnostics: Vec<MetricDiagnostic>,
    pub distance_probes: Vec<DistanceProbe>,
    pub partial_distance_vectors: Vec<PartialDistanceVectorRow>,
    pub collisions: Vec<CollisionDiagnostic>,
    pub local_geometry: Vec<LocalGeometryDiagnostic>,
    pub neighborhood_agreement: Vec<NeighborhoodAgreement>,
    pub capability_profiles: Vec<CapabilityProfile>,
    pub retrospective: Vec<RetrospectiveDiagnostic>,
    pub packed_vectors: Vec<u8>,
    pub packed_vector_index: Vec<PackedVectorIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedVectorIndex {
    pub object_key: String,
    pub representation_id: String,
    pub availability: String,
    pub byte_offset: usize,
    pub byte_length: usize,
    pub dimensions: usize,
}

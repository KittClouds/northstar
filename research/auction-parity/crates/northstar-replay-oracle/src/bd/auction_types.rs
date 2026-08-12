use super::{
    input::{ExpectedFeature, SourceRef},
    topology_core::Node,
};

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub event_id: u64,
    pub event_sequence: u64,
    pub episode_id: u64,
    pub attempt_id: u64,
    pub node_id: u64,
    pub related_node_id: u64,
    pub market_time: u64,
    pub bar_time: u64,
    pub kind: i32,
    pub direction: i32,
    pub price: f64,
    pub atr: f64,
    pub distance_atr: f64,
    pub penetration_atr: f64,
    pub bar_sequence: i32,
    pub attempt_event_sequence: i32,
    pub regional: ExpectedFeature,
}

#[derive(Clone, Debug, Default)]
pub struct Attempt {
    pub active: bool,
    pub is_retest: bool,
    pub completion_status: i32,
    pub censor_reason: i32,
    pub attempt_id: u64,
    pub episode_id: u64,
    pub node_id: u64,
    pub evidence_id: u64,
    pub attempt_ordinal: i32,
    pub direction: i32,
    pub state: i32,
    pub resolution: i32,
    pub started_at: u64,
    pub contact_at: u64,
    pub break_at: u64,
    pub accepted_at: u64,
    pub resolved_at: u64,
    pub last_observed_at: u64,
    pub start_bar_sequence: i32,
    pub contact_bar_sequence: i32,
    pub resolved_bar_sequence: i32,
    pub event_count: i32,
    pub frozen_lower: f64,
    pub frozen_price: f64,
    pub frozen_upper: f64,
    pub frozen_width: f64,
    pub frozen_atr: f64,
    pub start_price: f64,
    pub contact_price: f64,
    pub last_price: f64,
    pub approach_path: f64,
    pub approach_efficiency: f64,
    pub max_penetration: f64,
    pub max_penetration_atr: f64,
    pub max_penetration_node: f64,
    pub max_above_node_atr: f64,
    pub max_below_node_atr: f64,
    pub rejection_excursion_atr: f64,
    pub inside_updates: i32,
    pub inside_seconds: i64,
    pub qualified_far_closes: i32,
    pub broke_far_boundary: bool,
    pub provisional_acceptance: bool,
    pub accepted: bool,
    pub family_mask: u64,
    pub family_count: i32,
    pub member_count: i32,
    pub developing_count: i32,
    pub frozen_count: i32,
    pub node_revision: i32,
    pub provenance_offset: i32,
    pub provenance_count: i32,
    pub node_width_atr: f64,
    pub initial_distance_atr: f64,
    pub nearest_above_id: u64,
    pub nearest_below_id: u64,
    pub corridor_up_atr: f64,
    pub corridor_down_atr: f64,
    pub corridor_up_level_count: i32,
    pub corridor_down_level_count: i32,
    pub corridor_up_noise_count: i32,
    pub corridor_down_noise_count: i32,
    pub start_region: i32,
    pub end_region: i32,
    pub node_region: i32,
    pub start_median_sigma: f64,
    pub end_median_sigma: f64,
    pub min_median_sigma: f64,
    pub max_median_sigma: f64,
    pub node_from_median_sigma: f64,
    pub node_from_cog_sigma: f64,
    pub node_width_sigma: f64,
    pub context: ExpectedFeature,
}

#[derive(Clone, Debug, Default)]
pub struct Episode {
    pub active: bool,
    pub regional_valid: bool,
    pub sigma_valid: bool,
    pub completion_status: i32,
    pub censor_reason: i32,
    pub episode_id: u64,
    pub node_id: u64,
    pub started_at: u64,
    pub ended_at: u64,
    pub start_bar_sequence: i32,
    pub last_attempt_end_bar: i32,
    pub first_direction: i32,
    pub attempts: i32,
    pub breaks: i32,
    pub reclaims: i32,
    pub retests: i32,
    pub next_node_id: u64,
    pub resolution: i32,
    pub max_up_excursion_atr: f64,
    pub max_down_excursion_atr: f64,
    pub corridor_up_atr: f64,
    pub corridor_down_atr: f64,
    pub node_width_atr: f64,
    pub family_mask: u64,
    pub family_count: i32,
    pub member_count: i32,
    pub initial_region: i32,
    pub terminal_region: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ContextRow {
    pub attempt_id: u64,
    pub episode_id: u64,
    pub node_id: u64,
    pub frozen_at: u64,
    pub source: SourceRef,
}

#[derive(Clone, Debug, Default)]
pub struct Transit {
    pub active: bool,
    pub regional_valid: bool,
    pub sigma_valid: bool,
    pub transit_id: u64,
    pub attempt_id: u64,
    pub episode_id: u64,
    pub source_node_id: u64,
    pub destination_node_id: u64,
    pub direction: i32,
    pub started_at: u64,
    pub start_bar_sequence: i32,
    pub start_price: f64,
    pub source_lower: f64,
    pub source_upper: f64,
    pub destination_lower: f64,
    pub destination_upper: f64,
    pub frozen_atr: f64,
    pub distance_atr: f64,
    pub path_length: f64,
    pub last_price: f64,
    pub max_adverse_atr: f64,
    pub ended_at: u64,
    pub end_bar_sequence: i32,
    pub end_price: f64,
    pub path_efficiency: f64,
    pub regional_basis_hash: u64,
    pub structure_snapshot_hash: u64,
    pub source_region: i32,
    pub destination_region: i32,
    pub start_price_region: i32,
    pub end_price_region: i32,
    pub start_median_price: f64,
    pub start_structural_sigma: f64,
    pub start_price_from_median_sigma: f64,
    pub end_price_from_median_sigma: f64,
    pub completion_status: i32,
    pub censor_reason: i32,
    pub resolution: i32,
}

#[derive(Clone, Debug, Default)]
pub struct NodeMemory {
    pub node_id: u64,
    pub attempt_ordinal: i32,
    pub episode_ordinal: i32,
    pub accepted_side: i32,
}

pub fn price_side(price: f64, node: &Node) -> i32 {
    if price < node.lower {
        -1
    } else if price > node.upper {
        1
    } else {
        0
    }
}

pub fn boundary_gap(price: f64, lower: f64, upper: f64) -> f64 {
    if price < lower {
        lower - price
    } else if price > upper {
        price - upper
    } else {
        0.0
    }
}

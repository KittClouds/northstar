use super::{
    input::{ExpectedFeature, Frame, Level, SourceRef},
    topology_core::{HASH_SEED, Node, hash_mix},
};

const UNAVAILABLE: i32 = 99;

#[derive(Clone, Debug)]
pub struct RegionalState {
    pub valid: bool,
    pub sigma_valid: bool,
    pub has_cog: bool,
    pub basis_changed: bool,
    pub frozen_bar_time: u64,
    pub structure_snapshot_hash: u64,
    pub regional_basis_hash: u64,
    pub structure_generation: u64,
    pub population_count: i32,
    pub population_count_delta: i32,
    pub velocity_elapsed_bars: i32,
    pub median_price: f64,
    pub mean_price: f64,
    pub structural_sigma: f64,
    pub reference_price: f64,
    pub cog_price: f64,
    pub price_from_median_atr: f64,
    pub price_from_median_sigma: f64,
    pub price_from_cog_atr: f64,
    pub price_from_cog_sigma: f64,
    pub cog_median_gap_atr: f64,
    pub cog_median_gap_sigma: f64,
    pub cog_velocity_price: f64,
    pub cog_velocity_atr: f64,
    pub cog_velocity_sigma: f64,
    pub median_velocity_price: f64,
    pub median_velocity_atr: f64,
    pub median_velocity_sigma: f64,
    pub sigma_log_change: f64,
    pub price_region: i32,
    pub cog_region: i32,
}

impl Default for RegionalState {
    fn default() -> Self {
        Self {
            price_region: UNAVAILABLE,
            cog_region: UNAVAILABLE,
            ..Self::zeroed()
        }
    }
}

impl RegionalState {
    fn zeroed() -> Self {
        Self {
            valid: false,
            sigma_valid: false,
            has_cog: false,
            basis_changed: false,
            frozen_bar_time: 0,
            structure_snapshot_hash: 0,
            regional_basis_hash: 0,
            structure_generation: 0,
            population_count: 0,
            population_count_delta: 0,
            velocity_elapsed_bars: 0,
            median_price: 0.0,
            mean_price: 0.0,
            structural_sigma: 0.0,
            reference_price: 0.0,
            cog_price: 0.0,
            price_from_median_atr: 0.0,
            price_from_median_sigma: 0.0,
            price_from_cog_atr: 0.0,
            price_from_cog_sigma: 0.0,
            cog_median_gap_atr: 0.0,
            cog_median_gap_sigma: 0.0,
            cog_velocity_price: 0.0,
            cog_velocity_atr: 0.0,
            cog_velocity_sigma: 0.0,
            median_velocity_price: 0.0,
            median_velocity_atr: 0.0,
            median_velocity_sigma: 0.0,
            sigma_log_change: 0.0,
            price_region: 0,
            cog_region: 0,
        }
    }
}

#[derive(Default)]
pub struct RegionalModel {
    state: RegionalState,
    previous_bar_time: u64,
    previous_median: f64,
    previous_sigma: f64,
    previous_cog: f64,
    previous_atr: f64,
    previous_valid: bool,
    previous_has_cog: bool,
    cog_velocity_price: f64,
    cog_velocity_atr: f64,
    cog_velocity_sigma: f64,
    median_velocity_price: f64,
    median_velocity_atr: f64,
    median_velocity_sigma: f64,
    sigma_log_change: f64,
    last_basis_hash: u64,
    last_population_count: i32,
}

impl RegionalModel {
    pub fn rebuild_basis(
        &mut self,
        nodes: &[Node],
        levels: &[Level],
        snapshot_hash: u64,
        generation: u64,
        point: f64,
    ) {
        let mut population: Vec<(f64, u64)> = nodes
            .iter()
            .filter(|node| node.existence == 1)
            .map(|node| (node.price, node.node_id))
            .collect();
        population.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let count = population.len();
        let safe_point = point.max(1e-8);
        let mut hash = hash_mix(HASH_SEED, count as u64);
        let mut mean = 0.0;
        for &(price, id) in &population {
            mean += price;
            hash = hash_mix(hash, id);
            hash = hash_mix(hash, mql_round(price / safe_point) as u64);
        }
        if count > 0 {
            mean /= count as f64;
        }
        let median = match count {
            0 => 0.0,
            value if value % 2 == 1 => population[value / 2].0,
            value => (population[value / 2 - 1].0 + population[value / 2].0) * 0.5,
        };
        let sigma = if count == 0 {
            0.0
        } else {
            (population
                .iter()
                .map(|(price, _)| (price - mean).powi(2))
                .sum::<f64>()
                / count as f64)
                .sqrt()
        };
        let cog = newest_kind(levels, 150);
        self.state.valid = count > 0;
        self.state.sigma_valid = count >= 2 && sigma > safe_point * 0.5;
        self.state.structure_snapshot_hash = snapshot_hash;
        self.state.regional_basis_hash = hash;
        self.state.structure_generation = generation;
        self.state.population_count = count as i32;
        self.state.population_count_delta = count as i32 - self.last_population_count;
        self.state.basis_changed = hash != self.last_basis_hash;
        self.state.median_price = median;
        self.state.mean_price = mean;
        self.state.structural_sigma = sigma;
        self.state.cog_price = cog.unwrap_or_default();
        self.state.has_cog = cog.is_some();
        self.last_basis_hash = hash;
        self.last_population_count = count as i32;
    }

    pub fn observe(&mut self, frame: &Frame, basis_rebuilt: bool) {
        let atr = frame.atr.max(1e-8);
        if !basis_rebuilt {
            self.state.basis_changed = false;
            self.state.population_count_delta = 0;
        }
        if frame.closed_bar_time > 0 && frame.closed_bar_time != self.previous_bar_time {
            let elapsed = 1_i32;
            self.state.velocity_elapsed_bars = elapsed;
            if self.previous_valid {
                let previous_atr = self.previous_atr.max(1e-8);
                self.median_velocity_price = self.state.median_price - self.previous_median;
                self.median_velocity_atr = self.median_velocity_price / previous_atr;
                if self.previous_sigma > 0.0 {
                    self.median_velocity_sigma = self.median_velocity_price / self.previous_sigma;
                    if self.state.structural_sigma > 0.0 {
                        self.sigma_log_change =
                            (self.state.structural_sigma / self.previous_sigma).ln();
                    }
                }
                if self.state.has_cog && self.previous_has_cog {
                    self.cog_velocity_price = self.state.cog_price - self.previous_cog;
                    self.cog_velocity_atr = self.cog_velocity_price / previous_atr;
                    if self.previous_sigma > 0.0 {
                        self.cog_velocity_sigma = self.cog_velocity_price / self.previous_sigma;
                    }
                }
            }
            self.previous_bar_time = frame.closed_bar_time;
            self.previous_median = self.state.median_price;
            self.previous_sigma = self.state.structural_sigma;
            self.previous_cog = self.state.cog_price;
            self.previous_atr = frame.atr;
            self.previous_valid = self.state.valid && self.state.sigma_valid;
            self.previous_has_cog = self.state.has_cog;
        }
        self.state.frozen_bar_time = frame.closed_bar_time;
        self.state.reference_price = frame.reference_price;
        self.state.cog_velocity_price = self.cog_velocity_price;
        self.state.cog_velocity_atr = self.cog_velocity_atr;
        self.state.cog_velocity_sigma = self.cog_velocity_sigma;
        self.state.median_velocity_price = self.median_velocity_price;
        self.state.median_velocity_atr = self.median_velocity_atr;
        self.state.median_velocity_sigma = self.median_velocity_sigma;
        self.state.sigma_log_change = self.sigma_log_change;
        self.state.price_from_median_atr = if self.state.valid {
            (frame.reference_price - self.state.median_price) / atr
        } else {
            0.0
        };
        self.state.price_from_cog_atr = if self.state.has_cog {
            (frame.reference_price - self.state.cog_price) / atr
        } else {
            0.0
        };
        self.state.cog_median_gap_atr = if self.state.valid && self.state.has_cog {
            (self.state.cog_price - self.state.median_price) / atr
        } else {
            0.0
        };
        if self.state.sigma_valid {
            let sigma = self.state.structural_sigma;
            self.state.price_from_median_sigma =
                (frame.reference_price - self.state.median_price) / sigma;
            self.state.price_region = classify(self.state.price_from_median_sigma);
            if self.state.has_cog {
                self.state.price_from_cog_sigma =
                    (frame.reference_price - self.state.cog_price) / sigma;
                self.state.cog_median_gap_sigma =
                    (self.state.cog_price - self.state.median_price) / sigma;
                self.state.cog_region = classify(self.state.cog_median_gap_sigma);
            } else {
                self.state.cog_region = UNAVAILABLE;
            }
        } else {
            self.state.price_from_median_sigma = 0.0;
            self.state.price_from_cog_sigma = 0.0;
            self.state.cog_median_gap_sigma = 0.0;
            self.state.price_region = UNAVAILABLE;
            self.state.cog_region = UNAVAILABLE;
        }
    }

    pub fn apply(&self, nodes: &mut [Node], sources: &[SourceRef], atr: f64) {
        let safe_atr = atr.max(1e-8);
        for node in nodes {
            node.region = UNAVAILABLE;
            node.median_distance_atr = 0.0;
            node.median_distance_sigma = 0.0;
            node.cog_distance_sigma = 0.0;
            node.width_sigma = 0.0;
            let begin = node.provenance_offset.max(0) as usize;
            let end = (begin + node.provenance_count.max(0) as usize).min(sources.len());
            node.contains_cog = sources[begin..end]
                .iter()
                .any(|source| source.producer == 1 && source.source_kind == 150);
            if self.state.valid {
                node.median_distance_atr = (node.price - self.state.median_price) / safe_atr;
                if self.state.sigma_valid {
                    node.median_distance_sigma =
                        (node.price - self.state.median_price) / self.state.structural_sigma;
                    node.region = classify(node.median_distance_sigma);
                    node.width_sigma =
                        (node.upper - node.lower).max(0.0) / self.state.structural_sigma;
                    if self.state.has_cog {
                        node.cog_distance_sigma =
                            (node.price - self.state.cog_price) / self.state.structural_sigma;
                    }
                }
            }
        }
    }

    pub fn state(&self) -> &RegionalState {
        &self.state
    }
}

pub struct ContextBuilder {
    previous_bar: u64,
    previous_cog: f64,
    previous_c3: f64,
    cog_velocity: f64,
    c3_velocity: f64,
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            previous_bar: 0,
            previous_cog: 0.0,
            previous_c3: 0.0,
            cog_velocity: 0.0,
            c3_velocity: 0.0,
        }
    }
}

impl ContextBuilder {
    pub fn enrich_corridors(&self, nodes: &mut [Node], levels: &[Level], labels: &[i32]) {
        for index in 0..nodes.len() {
            let (lower, upper, active) = (
                nodes[index].lower,
                nodes[index].upper,
                nodes[index].existence == 1,
            );
            nodes[index].corridor_up_levels = 0;
            nodes[index].corridor_down_levels = 0;
            nodes[index].corridor_up_noise = 0;
            nodes[index].corridor_down_noise = 0;
            if !active {
                continue;
            }
            let mut upper_edge = f64::MAX;
            let mut lower_edge = -f64::MAX;
            for (other_index, other) in nodes.iter().enumerate() {
                if other_index == index || other.existence != 1 {
                    continue;
                }
                if other.lower >= upper && other.lower < upper_edge {
                    upper_edge = other.lower;
                }
                if other.upper <= lower && other.upper > lower_edge {
                    lower_edge = other.upper;
                }
            }
            if upper_edge != f64::MAX {
                nodes[index].corridor_up_levels = count_levels(levels, upper, upper_edge) as i32;
                nodes[index].corridor_up_noise =
                    count_noise(levels, labels, upper, upper_edge) as i32;
            }
            if lower_edge != -f64::MAX {
                nodes[index].corridor_down_levels = count_levels(levels, lower_edge, lower) as i32;
                nodes[index].corridor_down_noise =
                    count_noise(levels, labels, lower_edge, lower) as i32;
            }
        }
    }

    pub fn build(
        &mut self,
        frame: &Frame,
        levels: &[Level],
        noise: usize,
        nodes: &[Node],
        regional: &RegionalState,
    ) -> ExpectedFeature {
        let atr = frame.atr.max(1e-8);
        let cog = newest_kind(levels, 150);
        let c3 = newest_kind(levels, 103);
        let c1 = newest_kind(levels, 101);
        let c5 = newest_kind(levels, 105);
        let field_lower = newest_kind(levels, 151);
        let field_upper = newest_kind(levels, 152);
        let poc = newest_kind(levels, 200);
        let vah = newest_kind(levels, 201);
        let val = newest_kind(levels, 202);
        let has_profile = poc.is_some() && vah.is_some() && val.is_some();
        if frame.calculation_bar_time > 0 && frame.calculation_bar_time != self.previous_bar {
            if self.previous_bar > 0
                && self.previous_cog > 0.0
                && let Some(value) = cog
            {
                self.cog_velocity = (value - self.previous_cog) / atr;
            }
            if self.previous_bar > 0
                && self.previous_c3 > 0.0
                && let Some(value) = c3
            {
                self.c3_velocity = (value - self.previous_c3) / atr;
            }
            if let Some(value) = cog {
                self.previous_cog = value;
            }
            if let Some(value) = c3 {
                self.previous_c3 = value;
            }
            self.previous_bar = frame.calculation_bar_time;
        }
        ExpectedFeature {
            frozen_bar_time: frame.calculation_bar_time,
            has_cog: cog.is_some(),
            has_c3: c3.is_some(),
            has_lattice: c1.is_some() && c5.is_some(),
            has_field: field_lower.is_some() && field_upper.is_some(),
            has_profile,
            cog_price: cog.unwrap_or_default(),
            c3_price: c3.unwrap_or_default(),
            cog_distance_atr: cog.map_or(0.0, |value| (value - frame.reference_price) / atr),
            c3_distance_atr: c3.map_or(0.0, |value| (value - frame.reference_price) / atr),
            cog_velocity_atr: self.cog_velocity,
            c3_velocity_atr: self.c3_velocity,
            lattice_width_atr: c1.zip(c5).map_or(0.0, |(a, b)| (b - a).abs() / atr),
            field_width_atr: field_lower
                .zip(field_upper)
                .map_or(0.0, |(a, b)| (b - a).abs() / atr),
            profile_poc: poc.unwrap_or_default(),
            profile_vah: vah.unwrap_or_default(),
            profile_val: val.unwrap_or_default(),
            poc_distance_atr: if has_profile {
                (poc.unwrap() - frame.reference_price) / atr
            } else {
                0.0
            },
            vah_distance_atr: if has_profile {
                (vah.unwrap() - frame.reference_price) / atr
            } else {
                0.0
            },
            val_distance_atr: if has_profile {
                (val.unwrap() - frame.reference_price) / atr
            } else {
                0.0
            },
            spread_atr: (frame.ask - frame.bid).max(0.0) / atr,
            raw_level_count: levels.len() as i32,
            noise_level_count: noise as i32,
            active_node_count: nodes.iter().filter(|node| node.existence == 1).count() as i32,
            regional_valid: regional.valid,
            sigma_valid: regional.sigma_valid,
            regional_has_cog: regional.has_cog,
            basis_changed: regional.basis_changed,
            regional_frozen_bar_time: regional.frozen_bar_time,
            structure_snapshot_hash: regional.structure_snapshot_hash,
            regional_basis_hash: regional.regional_basis_hash,
            structure_generation: regional.structure_generation,
            population_count: regional.population_count,
            population_count_delta: regional.population_count_delta,
            velocity_elapsed_bars: regional.velocity_elapsed_bars,
            median_price: regional.median_price,
            mean_price: regional.mean_price,
            structural_sigma: regional.structural_sigma,
            regional_reference_price: regional.reference_price,
            regional_cog_price: regional.cog_price,
            price_from_median_atr: regional.price_from_median_atr,
            price_from_median_sigma: regional.price_from_median_sigma,
            price_from_cog_atr: regional.price_from_cog_atr,
            price_from_cog_sigma: regional.price_from_cog_sigma,
            cog_median_gap_atr: regional.cog_median_gap_atr,
            cog_median_gap_sigma: regional.cog_median_gap_sigma,
            cog_velocity_price: regional.cog_velocity_price,
            regional_cog_velocity_atr: regional.cog_velocity_atr,
            cog_velocity_sigma: regional.cog_velocity_sigma,
            median_velocity_price: regional.median_velocity_price,
            median_velocity_atr: regional.median_velocity_atr,
            median_velocity_sigma: regional.median_velocity_sigma,
            sigma_log_change: regional.sigma_log_change,
            price_region: regional.price_region,
            cog_region: regional.cog_region,
        }
    }
}

fn newest_kind(levels: &[Level], source_kind: i32) -> Option<f64> {
    levels
        .iter()
        .filter(|level| level.producer == 1 && level.source_kind == source_kind)
        .max_by_key(|level| level.updated_at)
        .map(|level| level.price)
}
fn count_levels(levels: &[Level], lower: f64, upper: f64) -> usize {
    levels
        .iter()
        .filter(|level| level.price > lower && level.price < upper)
        .count()
}
fn count_noise(levels: &[Level], labels: &[i32], lower: f64, upper: f64) -> usize {
    levels
        .iter()
        .zip(labels)
        .filter(|(level, label)| **label == -1 && level.price > lower && level.price < upper)
        .count()
}
fn classify(value: f64) -> i32 {
    mql_round(value).clamp(-3, 3) as i32
}
fn mql_round(value: f64) -> i64 {
    if value >= 0.0 {
        (value + 0.5).floor() as i64
    } else {
        (value - 0.5).ceil() as i64
    }
}

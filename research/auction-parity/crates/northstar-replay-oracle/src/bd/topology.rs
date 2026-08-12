use std::path::Path;

use serde::Serialize;

use crate::{OracleError, Result};

use super::{
    auction::{AuctionEngine, AuctionLedger},
    input::{ExpectedFeature, ExpectedNode, FrameBundle, Level, SourceRef, load_oracle},
    regional::{ContextBuilder, RegionalModel},
    topology_core::{HASH_SEED, Node, build_nodes, dbscan, hash_mix, normalize_and_sort},
};

const POINT_CANDIDATES: [f64; 7] = [1.0, 0.1, 0.01, 0.001, 0.0001, 0.00001, 0.000001];

#[derive(Debug, Serialize)]
pub struct BdParityReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub frames: usize,
    pub normalized_levels_verified: usize,
    pub dbscan_rebuilds_verified: usize,
    pub stable_nodes_verified: usize,
    pub provenance_rows_verified: usize,
    pub regional_snapshots_verified: usize,
    pub causal_features_verified: usize,
    pub auction_events_verified: usize,
    pub auction_attempts_verified: usize,
    pub auction_episodes_verified: usize,
    pub auction_transits_verified: usize,
    pub auction_terminal_hash: u64,
    pub inferred_point: f64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub c1_producer_reconstruction: &'static str,
    pub c2_c5_topology_parity: &'static str,
    pub phase_d_feature_parity: &'static str,
}

pub fn verify_bd(root: &Path, ledger_prefix: &Path) -> Result<BdParityReceipt> {
    let bundles = load_oracle(root)?;
    let point = infer_point(&bundles[0])?;
    let mut tracker = NodeTracker::default();
    let mut regional = RegionalModel::default();
    let mut context = ContextBuilder::default();
    let mut auction = AuctionEngine::default();
    let mut current_nodes = Vec::new();
    let mut current_sources = Vec::new();
    let mut current_labels = Vec::new();
    let mut last_input_hash = 0_u64;
    let mut last_snapshot_hash = 0_u64;
    let mut generation = 0_u64;
    let mut normalized = 0;
    let mut rebuilds = 0;
    let mut node_rows = 0;
    let mut source_rows = 0;

    for bundle in &bundles {
        let mut levels = bundle.levels.clone();
        verify_level_normalization(bundle, &levels)?;
        normalized += levels.len();
        let input_hash = structural_input_hash(&levels, bundle.frame.atr, point);
        normalize_and_sort(&mut levels, bundle.frame.reference_price, bundle.frame.atr);
        let rebuilt = current_nodes.is_empty() || input_hash != last_input_hash;
        if rebuilt {
            let (labels, clusters) = dbscan(&levels, 0.12, 2);
            let (mut nodes, sources) = build_nodes(
                &levels,
                &labels,
                clusters,
                bundle.frame.reference_price,
                bundle.frame.atr,
            );
            tracker.reconcile(&mut nodes, bundle.frame.atr, bundle.frame.market_time);
            context.enrich_corridors(&mut nodes, &levels, &labels);
            let snapshot_hash = full_snapshot_hash(&nodes, &levels, point);
            if snapshot_hash != last_snapshot_hash {
                generation += 1;
                last_snapshot_hash = snapshot_hash;
            }
            current_nodes = nodes;
            current_sources = sources;
            current_labels = labels;
            last_input_hash = input_hash;
            rebuilds += 1;
        } else {
            refresh_cached_nodes(
                &mut current_nodes,
                &levels,
                &current_labels,
                bundle.frame.reference_price,
                bundle.frame.atr,
            );
        }

        if last_snapshot_hash != bundle.frame.structure_snapshot_hash {
            return fail(
                bundle.frame.sequence,
                "C3 snapshot hash",
                last_snapshot_hash,
                bundle.frame.structure_snapshot_hash,
            );
        }
        if generation != bundle.frame.structure_generation {
            return fail(
                bundle.frame.sequence,
                "C4 generation",
                generation,
                bundle.frame.structure_generation,
            );
        }
        regional.rebuild_basis_if(
            rebuilt,
            &current_nodes,
            &levels,
            last_snapshot_hash,
            generation,
            point,
        );
        regional.observe(&bundle.frame, rebuilt);
        regional.apply(&mut current_nodes, &current_sources, bundle.frame.atr);
        compare_nodes(bundle, &current_nodes)?;
        compare_sources(bundle, &current_sources)?;
        let feature = context.build(
            &bundle.frame,
            &levels,
            current_labels.iter().filter(|&&label| label == -1).count(),
            &current_nodes,
            regional.state(),
        );
        compare_feature(bundle.frame.sequence, &feature, &bundle.feature)?;
        if auction.event_sequence() != bundle.frame.auction_event_sequence_before {
            return fail(
                bundle.frame.sequence,
                "B auction event sequence",
                auction.event_sequence(),
                bundle.frame.auction_event_sequence_before,
            );
        }
        auction.observe(
            &mut current_nodes,
            &current_sources,
            &feature,
            &bundle.frame,
        );
        tracker.sync_attempt_counts(&current_nodes);
        node_rows += current_nodes.len();
        source_rows += current_sources.len();
    }
    let ledger = auction.finalize(&bundles.last().unwrap().frame, 1);
    verify_ledger(ledger_prefix, &ledger)?;
    Ok(BdParityReceipt {
        contract: "NORTHSTAR_PHASE12_BD_PARITY_V1",
        status: "PASS",
        frames: bundles.len(),
        normalized_levels_verified: normalized,
        dbscan_rebuilds_verified: rebuilds,
        stable_nodes_verified: node_rows,
        provenance_rows_verified: source_rows,
        regional_snapshots_verified: bundles.len(),
        causal_features_verified: bundles.len(),
        auction_events_verified: ledger.events.len(),
        auction_attempts_verified: ledger.attempts.len(),
        auction_episodes_verified: ledger.episodes.len(),
        auction_transits_verified: ledger.transits.len(),
        auction_terminal_hash: ledger.terminal_hash,
        inferred_point: point,
        first_sequence: bundles.first().unwrap().frame.sequence,
        last_sequence: bundles.last().unwrap().frame.sequence,
        c1_producer_reconstruction: "NOT_IN_SCOPE_INPUT_TAPE_STARTS_AT_RAW_PRODUCER_OUTPUT",
        c2_c5_topology_parity: "EXACT",
        phase_d_feature_parity: "EXACT_CAUSAL_FRAME_SNAPSHOT",
    })
}

trait RebuildRegional {
    fn rebuild_basis_if(
        &mut self,
        rebuilt: bool,
        nodes: &[Node],
        levels: &[Level],
        hash: u64,
        generation: u64,
        point: f64,
    );
}
impl RebuildRegional for RegionalModel {
    fn rebuild_basis_if(
        &mut self,
        rebuilt: bool,
        nodes: &[Node],
        levels: &[Level],
        hash: u64,
        generation: u64,
        point: f64,
    ) {
        if rebuilt {
            self.rebuild_basis(nodes, levels, hash, generation, point);
        }
    }
}

#[derive(Default)]
struct NodeTracker {
    tracks: Vec<Node>,
    birth_sequence: u64,
}

impl NodeTracker {
    fn reconcile(&mut self, candidates: &mut [Node], atr: f64, market_time: u64) {
        let old_count = self.tracks.len();
        let mut used = vec![false; old_count];
        for candidate in candidates.iter_mut() {
            candidate.evidence_id = candidate.node_id;
            let mut best: Option<usize> = None;
            let mut best_score = -f64::MAX;
            for (index, was_used) in used.iter().enumerate().take(old_count) {
                if *was_used {
                    continue;
                }
                let score = match_score(&self.tracks[index], candidate, atr);
                if score > best_score
                    || (score == best_score
                        && best.is_some_and(|old| {
                            self.tracks[index].node_id < self.tracks[old].node_id
                        }))
                {
                    best = Some(index);
                    best_score = score;
                }
            }
            if let Some(index) = best.filter(|_| best_score > -f64::MAX / 2.0) {
                let old = &self.tracks[index];
                candidate.node_id = old.node_id;
                candidate.existence = old.existence;
                candidate.created_at = old.created_at;
                candidate.last_seen_at = market_time;
                candidate.state_changed_at = old.state_changed_at;
                candidate.attempt_count = old.attempt_count;
                candidate.revision = old.revision + 1;
                candidate.primary_parent_id = old.primary_parent_id;
                candidate.secondary_parent_id = old.secondary_parent_id;
                candidate.missed_rebuilds = 0;
                candidate.existence = 1;
                self.tracks[index] = candidate.clone();
                used[index] = true;
            } else {
                self.birth_sequence += 1;
                let mut id = hash_mix(
                    hash_mix(candidate.evidence_id, market_time),
                    self.birth_sequence,
                );
                for retry in 0..8 {
                    if id != 0 && !self.tracks.iter().any(|track| track.node_id == id) {
                        break;
                    }
                    id = hash_mix(id, retry + 1);
                }
                candidate.node_id = if id == 0 { 1 } else { id };
                candidate.existence = 1;
                candidate.created_at = market_time;
                candidate.last_seen_at = market_time;
                candidate.state_changed_at = market_time;
                candidate.revision = 1;
                self.tracks.push(candidate.clone());
            }
        }
        for candidate in candidates.iter_mut() {
            let first_parent = self.tracks[..old_count]
                .iter()
                .find(|track| {
                    track.node_id != candidate.node_id && lineage_compatible(track, candidate, atr)
                })
                .map(|track| track.node_id)
                .unwrap_or(0);
            if first_parent != 0 {
                candidate.primary_parent_id = first_parent;
                if let Some(track) = self
                    .tracks
                    .iter_mut()
                    .find(|track| track.node_id == candidate.node_id)
                {
                    track.primary_parent_id = first_parent;
                }
            }
        }
        for (index, was_used) in used.iter().enumerate().take(old_count) {
            if *was_used {
                continue;
            }
            self.tracks[index].missed_rebuilds += 1;
            if self.tracks[index].missed_rebuilds >= 3 {
                self.tracks[index].existence = 3;
            }
        }
        self.tracks.retain(|track| track.existence != 3);
    }

    fn sync_attempt_counts(&mut self, nodes: &[Node]) {
        for node in nodes {
            if let Some(track) = self
                .tracks
                .iter_mut()
                .find(|track| track.node_id == node.node_id)
            {
                track.attempt_count = node.attempt_count;
            }
        }
    }
}

fn verify_ledger(prefix: &Path, ledger: &AuctionLedger) -> Result<()> {
    super::auction_verify::verify(prefix, ledger)
}

fn geometry_compatible(track: &Node, candidate: &Node, atr: f64) -> bool {
    if track.existence == 3 {
        return false;
    }
    let safe_atr = atr.max(1e-8);
    let gap = if candidate.lower > track.upper {
        candidate.lower - track.upper
    } else if track.lower > candidate.upper {
        track.lower - candidate.upper
    } else {
        0.0
    };
    track.family_mask & candidate.family_mask != 0
        && gap / safe_atr <= 0.2
        && (candidate.price - track.price).abs() / safe_atr <= 0.4
}
fn lineage_compatible(track: &Node, candidate: &Node, atr: f64) -> bool {
    geometry_compatible(track, candidate, atr)
        && ((track.upper.min(candidate.upper) - track.lower.max(candidate.lower) >= 0.0)
            || (track.price - candidate.price).abs() / atr.max(1e-8) <= 0.1)
}
fn match_score(track: &Node, candidate: &Node, atr: f64) -> f64 {
    if !geometry_compatible(track, candidate, atr) {
        return -f64::MAX;
    }
    let safe_atr = atr.max(1e-8);
    let mut score = if track.evidence_id == candidate.evidence_id {
        1000.0
    } else {
        0.0
    };
    score += 10.0 - ((track.price - candidate.price).abs() / safe_atr).min(10.0);
    let overlap = (track.upper.min(candidate.upper) - track.lower.max(candidate.lower)).max(0.0);
    let span =
        (track.upper.max(candidate.upper) - track.lower.min(candidate.lower)).max(safe_atr * 1e-6);
    score += 10.0 * overlap / span;
    score += (track.family_mask & candidate.family_mask).count_ones() as f64;
    if track.role == candidate.role {
        score += 1.0;
    }
    score
}

fn refresh_cached_nodes(
    nodes: &mut [Node],
    levels: &[Level],
    labels: &[i32],
    reference: f64,
    atr: f64,
) {
    for node in nodes.iter_mut() {
        node.normalized_lower = (node.lower - reference) / atr;
        node.normalized_price = (node.price - reference) / atr;
        node.normalized_upper = (node.upper - reference) / atr;
        node.width_atr = (node.upper - node.lower).max(0.0) / atr;
        node.distance_atr = node.normalized_price;
        node.role = if node.normalized_price < -0.15 {
            -1
        } else if node.normalized_price > 0.15 {
            1
        } else {
            0
        };
        node.newest_update_time = 0;
    }
    for (level, &cluster) in levels.iter().zip(labels) {
        if let Ok(index) = usize::try_from(cluster)
            && let Some(node) = nodes.get_mut(index)
        {
            node.newest_update_time = node.newest_update_time.max(level.updated_at);
        }
    }
}

fn infer_point(bundle: &FrameBundle) -> Result<f64> {
    for point in POINT_CANDIDATES {
        let mut levels = bundle.levels.clone();
        normalize_and_sort(&mut levels, bundle.frame.reference_price, bundle.frame.atr);
        let (labels, clusters) = dbscan(&levels, 0.12, 2);
        let (mut nodes, _) = build_nodes(
            &levels,
            &labels,
            clusters,
            bundle.frame.reference_price,
            bundle.frame.atr,
        );
        let mut tracker = NodeTracker::default();
        tracker.reconcile(&mut nodes, bundle.frame.atr, bundle.frame.market_time);
        let context = ContextBuilder::default();
        context.enrich_corridors(&mut nodes, &levels, &labels);
        if full_snapshot_hash(&nodes, &levels, point) == bundle.frame.structure_snapshot_hash {
            return Ok(point);
        }
    }
    Err(OracleError::Contract(
        "could not infer symbol point from first structural snapshot".into(),
    ))
}

fn structural_input_hash(levels: &[Level], atr: f64, point: f64) -> u64 {
    let fine = point.max(1e-8) * 1e-6;
    let mut hash = hash_mix(HASH_SEED, levels.len() as u64);
    hash = hash_mix(hash, mql_round(atr / fine) as u64);
    for level in levels {
        for value in [
            level.source_key,
            level.producer as u64,
            level.producer_instance as u64,
            level.local_id,
            level.family as u64,
            level.source_kind as u64,
            level.role as u64,
            mql_round(level.lower / point) as u64,
            mql_round(level.price / point) as u64,
            mql_round(level.upper / point) as u64,
            level.created_at,
            level.developing as u64,
            level.frozen as u64,
            level.state as u64,
            mql_round(level.evidence_weight / fine) as u64,
        ] {
            hash = hash_mix(hash, value);
        }
    }
    hash
}
fn full_snapshot_hash(nodes: &[Node], levels: &[Level], point: f64) -> u64 {
    let mut hash = hash_mix(HASH_SEED, nodes.len() as u64);
    for node in nodes {
        for value in [
            node.node_id,
            mql_round(node.price / point) as u64,
            mql_round(node.lower / point) as u64,
            mql_round(node.upper / point) as u64,
        ] {
            hash = hash_mix(hash, value);
        }
    }
    hash = hash_mix(hash, levels.len() as u64);
    for level in levels {
        for value in [
            level.source_key,
            mql_round(level.lower / point) as u64,
            mql_round(level.price / point) as u64,
            mql_round(level.upper / point) as u64,
            level.state as u64,
        ] {
            hash = hash_mix(hash, value);
        }
    }
    hash
}

fn verify_level_normalization(bundle: &FrameBundle, levels: &[Level]) -> Result<()> {
    for level in levels {
        let atr = bundle.frame.atr;
        close(
            bundle.frame.sequence,
            "C2 normalized_lower",
            (level.lower - bundle.frame.reference_price) / atr,
            level.normalized_lower,
        )?;
        close(
            bundle.frame.sequence,
            "C2 normalized_price",
            (level.price - bundle.frame.reference_price) / atr,
            level.normalized_price,
        )?;
        close(
            bundle.frame.sequence,
            "C2 normalized_upper",
            (level.upper - bundle.frame.reference_price) / atr,
            level.normalized_upper,
        )?;
        close(
            bundle.frame.sequence,
            "C2 width_atr",
            (level.upper - level.lower).max(0.0) / atr,
            level.width_atr,
        )?;
    }
    Ok(())
}
fn compare_nodes(bundle: &FrameBundle, nodes: &[Node]) -> Result<()> {
    if nodes.len() != bundle.nodes.len() {
        return fail(
            bundle.frame.sequence,
            "C4 node count",
            nodes.len(),
            bundle.nodes.len(),
        );
    }
    for (actual, expected) in nodes.iter().zip(&bundle.nodes) {
        compare_node(bundle.frame.sequence, actual, expected)?;
    }
    Ok(())
}
fn compare_node(sequence: u64, a: &Node, e: &ExpectedNode) -> Result<()> {
    macro_rules! eq {
        ($field:ident) => {
            if a.$field != e.$field {
                return fail(
                    sequence,
                    concat!("C4 node ", stringify!($field)),
                    a.$field,
                    e.$field,
                );
            }
        };
    }
    macro_rules! fl {
        ($field:ident) => {
            close(
                sequence,
                concat!("C4 node ", stringify!($field)),
                a.$field,
                e.$field,
            )?;
        };
    }
    eq!(node_id);
    eq!(evidence_id);
    eq!(cluster_id);
    fl!(lower);
    fl!(price);
    fl!(upper);
    fl!(normalized_lower);
    fl!(normalized_price);
    fl!(normalized_upper);
    fl!(width_atr);
    fl!(distance_atr);
    eq!(region);
    fl!(median_distance_atr);
    fl!(median_distance_sigma);
    fl!(cog_distance_sigma);
    fl!(width_sigma);
    eq!(contains_cog);
    eq!(role);
    eq!(family_mask);
    eq!(role_mask);
    eq!(family_count);
    eq!(member_count);
    eq!(developing_count);
    eq!(frozen_count);
    eq!(broken_count);
    eq!(corridor_up_levels);
    eq!(corridor_down_levels);
    eq!(corridor_up_noise);
    eq!(corridor_down_noise);
    eq!(oldest_source_time);
    eq!(newest_update_time);
    eq!(provenance_offset);
    eq!(provenance_count);
    eq!(existence);
    eq!(created_at);
    eq!(last_seen_at);
    eq!(state_changed_at);
    eq!(missed_rebuilds);
    eq!(revision);
    eq!(primary_parent_id);
    eq!(secondary_parent_id);
    Ok(())
}
fn compare_sources(bundle: &FrameBundle, actual: &[SourceRef]) -> Result<()> {
    if actual != bundle.sources {
        return Err(OracleError::Contract(format!(
            "B-D divergence frame={} layer=C3 provenance",
            bundle.frame.sequence
        )));
    }
    Ok(())
}
fn compare_feature(sequence: u64, a: &ExpectedFeature, e: &ExpectedFeature) -> Result<()> {
    macro_rules! eq {
        ($field:ident) => {
            if a.$field != e.$field {
                return fail(
                    sequence,
                    concat!("D feature ", stringify!($field)),
                    a.$field,
                    e.$field,
                );
            }
        };
    }
    macro_rules! fl {
        ($field:ident) => {
            close(
                sequence,
                concat!("D feature ", stringify!($field)),
                a.$field,
                e.$field,
            )?;
        };
    }
    eq!(frozen_bar_time);
    eq!(has_cog);
    eq!(has_c3);
    eq!(has_lattice);
    eq!(has_field);
    eq!(has_profile);
    fl!(cog_price);
    fl!(c3_price);
    fl!(cog_distance_atr);
    fl!(c3_distance_atr);
    fl!(cog_velocity_atr);
    fl!(c3_velocity_atr);
    fl!(lattice_width_atr);
    fl!(field_width_atr);
    fl!(profile_poc);
    fl!(profile_vah);
    fl!(profile_val);
    fl!(poc_distance_atr);
    fl!(vah_distance_atr);
    fl!(val_distance_atr);
    fl!(spread_atr);
    eq!(raw_level_count);
    eq!(noise_level_count);
    eq!(active_node_count);
    eq!(regional_valid);
    eq!(sigma_valid);
    eq!(regional_has_cog);
    eq!(basis_changed);
    eq!(regional_frozen_bar_time);
    eq!(structure_snapshot_hash);
    eq!(regional_basis_hash);
    eq!(structure_generation);
    eq!(population_count);
    eq!(population_count_delta);
    eq!(velocity_elapsed_bars);
    fl!(median_price);
    fl!(mean_price);
    fl!(structural_sigma);
    fl!(regional_reference_price);
    fl!(regional_cog_price);
    fl!(price_from_median_atr);
    fl!(price_from_median_sigma);
    fl!(price_from_cog_atr);
    fl!(price_from_cog_sigma);
    fl!(cog_median_gap_atr);
    fl!(cog_median_gap_sigma);
    fl!(cog_velocity_price);
    fl!(regional_cog_velocity_atr);
    fl!(cog_velocity_sigma);
    fl!(median_velocity_price);
    fl!(median_velocity_atr);
    fl!(median_velocity_sigma);
    fl!(sigma_log_change);
    eq!(price_region);
    eq!(cog_region);
    Ok(())
}
fn close(sequence: u64, layer: &str, actual: f64, expected: f64) -> Result<()> {
    let scale = actual.abs().max(expected.abs()).max(1.0);
    if (actual - expected).abs() > 2e-10 * scale {
        return fail(sequence, layer, actual, expected);
    }
    Ok(())
}
fn fail<T, A: std::fmt::Display, E: std::fmt::Display>(
    sequence: u64,
    layer: &str,
    actual: A,
    expected: E,
) -> Result<T> {
    Err(OracleError::Contract(format!(
        "B-D divergence frame={sequence} layer={layer} actual={actual} expected={expected}"
    )))
}
fn mql_round(value: f64) -> i64 {
    if value >= 0.0 {
        (value + 0.5).floor() as i64
    } else {
        (value - 0.5).ceil() as i64
    }
}

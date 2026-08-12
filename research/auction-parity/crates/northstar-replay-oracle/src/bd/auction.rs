use super::{
    auction_types::{
        Attempt, ContextRow, Episode, Event, NodeMemory, Transit, boundary_gap, price_side,
    },
    input::{ExpectedFeature, Frame, SourceRef},
    topology_core::{HASH_SEED, Node, hash_mix},
};

const ACTIVE: i32 = 0;
const RESOLVED: i32 = 1;
const CENSORED: i32 = 2;

#[derive(Default)]
pub struct AuctionLedger {
    pub events: Vec<Event>,
    pub attempts: Vec<Attempt>,
    pub episodes: Vec<Episode>,
    pub context: Vec<ContextRow>,
    pub features: Vec<(u64, ExpectedFeature)>,
    pub transits: Vec<Transit>,
    pub terminal_hash: u64,
    pub event_sequence: u64,
    pub attempts_started: u64,
    pub attempts_resolved: u64,
    pub attempts_censored: u64,
    pub episodes_started: u64,
    pub episodes_resolved: u64,
    pub episodes_censored: u64,
    pub transits_started: u64,
    pub transits_resolved: u64,
    pub transits_censored: u64,
}

pub struct AuctionEngine {
    active_attempts: Vec<Attempt>,
    active_episodes: Vec<Episode>,
    active_transits: Vec<Transit>,
    memory: Vec<NodeMemory>,
    feature: ExpectedFeature,
    last_closed_bar_time: u64,
    bar_sequence: i32,
    ledger: AuctionLedger,
}

impl Default for AuctionEngine {
    fn default() -> Self {
        Self {
            active_attempts: Vec::with_capacity(32),
            active_episodes: Vec::with_capacity(16),
            active_transits: Vec::with_capacity(8),
            memory: Vec::with_capacity(64),
            feature: ExpectedFeature::default(),
            last_closed_bar_time: 0,
            bar_sequence: 0,
            ledger: AuctionLedger {
                terminal_hash: HASH_SEED,
                ..AuctionLedger::default()
            },
        }
    }
}

impl AuctionEngine {
    pub fn observe(
        &mut self,
        nodes: &mut [Node],
        sources: &[SourceRef],
        feature: &ExpectedFeature,
        frame: &Frame,
    ) {
        self.feature = feature.clone();
        let new_bar =
            frame.closed_bar_time > 0 && frame.closed_bar_time > self.last_closed_bar_time;
        if new_bar {
            self.last_closed_bar_time = frame.closed_bar_time;
            self.bar_sequence += 1;
        }
        let safe_atr = frame.atr.max(1e-8);

        for index in 0..self.active_attempts.len() {
            let present = nodes.iter().any(|node| {
                node.existence == 1 && node.node_id == self.active_attempts[index].node_id
            });
            if present {
                continue;
            }
            let mut orphan = self.active_attempts[index].clone();
            let distance = boundary_gap(
                frame.reference_price,
                orphan.frozen_lower,
                orphan.frozen_upper,
            ) / safe_atr;
            let penetration = orphan.max_penetration_atr;
            self.push_event(
                &mut orphan,
                16,
                frame,
                frame.reference_price,
                distance,
                penetration,
                0,
            );
            self.complete_attempt(&mut orphan, 9, frame.market_time);
            self.active_attempts[index] = orphan;
        }
        self.observe_transits(frame);

        for node_index in 0..nodes.len() {
            if nodes[node_index].existence != 1 {
                continue;
            }
            let node_id = nodes[node_index].node_id;
            let mut attempt_index = self
                .active_attempts
                .iter()
                .position(|attempt| attempt.active && attempt.node_id == node_id);
            let side = price_side(frame.reference_price, &nodes[node_index]);
            if attempt_index.is_none() {
                let gap = boundary_gap(
                    frame.reference_price,
                    nodes[node_index].lower,
                    nodes[node_index].upper,
                ) / safe_atr;
                if side != 0 && gap <= 0.50 {
                    attempt_index =
                        Some(self.start_attempt(node_index, nodes, sources, frame, side));
                } else {
                    continue;
                }
            }
            let index = attempt_index.unwrap();
            let mut attempt = self.active_attempts[index].clone();
            let previous_price = attempt.last_price;
            let previous_observed = attempt.last_observed_at;
            attempt.approach_path += (frame.reference_price - previous_price).abs();
            attempt.last_price = frame.reference_price;
            attempt.last_observed_at = frame.market_time;
            if feature.sigma_valid {
                let z = feature.price_from_median_sigma;
                attempt.end_region = feature.price_region;
                attempt.end_median_sigma = z;
                attempt.min_median_sigma = attempt.min_median_sigma.min(z);
                attempt.max_median_sigma = attempt.max_median_sigma.max(z);
            }
            let frozen_side = side_of(
                frame.reference_price,
                attempt.frozen_lower,
                attempt.frozen_upper,
            );
            let inside = frozen_side == 0;
            if inside {
                attempt.inside_updates += 1;
                if attempt.last_observed_at > 0 {
                    attempt.inside_seconds +=
                        frame.market_time.saturating_sub(previous_observed) as i64;
                }
            }
            attempt.max_above_node_atr = attempt
                .max_above_node_atr
                .max((frame.reference_price - attempt.frozen_upper).max(0.0) / safe_atr);
            attempt.max_below_node_atr = attempt
                .max_below_node_atr
                .max((attempt.frozen_lower - frame.reference_price).max(0.0) / safe_atr);
            let crossed_near = if attempt.direction == -1 {
                previous_price <= attempt.frozen_lower
                    && frame.reference_price >= attempt.frozen_lower
            } else {
                previous_price >= attempt.frozen_upper
                    && frame.reference_price <= attempt.frozen_upper
            };
            if attempt.contact_at == 0 && (inside || crossed_near) {
                attempt.contact_at = frame.market_time;
                attempt.contact_bar_sequence = self.bar_sequence;
                attempt.contact_price = frame.reference_price;
                attempt.state = if attempt.is_retest { 7 } else { 2 };
                let net = (frame.reference_price - attempt.start_price).abs();
                attempt.approach_efficiency = if attempt.approach_path > 0.0 {
                    net / attempt.approach_path
                } else {
                    1.0
                };
                self.push_event(&mut attempt, 2, frame, frame.reference_price, 0.0, 0.0, 0);
                if attempt.is_retest {
                    self.push_event(&mut attempt, 9, frame, frame.reference_price, 0.0, 0.0, 0);
                }
            }

            if attempt.contact_at > 0 {
                self.observe_contact(&mut attempt, nodes, frame, safe_atr, frozen_side);
            }
            if attempt.active && new_bar && attempt.contact_at > 0 {
                self.observe_closed_bar(&mut attempt, frame, safe_atr);
            }
            if attempt.active && new_bar && self.bar_sequence - attempt.start_bar_sequence > 24 {
                let distance = boundary_gap(
                    frame.reference_price,
                    attempt.frozen_lower,
                    attempt.frozen_upper,
                ) / safe_atr;
                let penetration = attempt.max_penetration_atr;
                self.push_event(
                    &mut attempt,
                    16,
                    frame,
                    frame.reference_price,
                    distance,
                    penetration,
                    0,
                );
                self.complete_attempt(&mut attempt, 8, frame.market_time);
            }
            self.active_attempts[index] = attempt;
        }
        if new_bar {
            self.close_expired_episodes(frame.market_time);
        }
        self.active_attempts.retain(|attempt| attempt.active);
        self.active_episodes.retain(|episode| episode.active);
    }

    fn observe_contact(
        &mut self,
        attempt: &mut Attempt,
        nodes: &[Node],
        frame: &Frame,
        safe_atr: f64,
        frozen_side: i32,
    ) {
        let penetration = if attempt.direction == -1 {
            frame.reference_price - attempt.frozen_lower
        } else {
            attempt.frozen_upper - frame.reference_price
        }
        .max(0.0);
        if penetration > attempt.max_penetration {
            let first = attempt.max_penetration <= 0.0 && penetration > 0.0;
            attempt.max_penetration = penetration;
            attempt.max_penetration_atr = penetration / safe_atr;
            attempt.max_penetration_node = if attempt.frozen_width > 0.0 {
                penetration / attempt.frozen_width
            } else {
                0.0
            };
            if first {
                self.push_event(
                    attempt,
                    3,
                    frame,
                    frame.reference_price,
                    0.0,
                    attempt.max_penetration_atr,
                    0,
                );
            }
        }
        let far_cross = if attempt.direction == -1 {
            frame.reference_price > attempt.frozen_upper
        } else {
            frame.reference_price < attempt.frozen_lower
        };
        if attempt.is_retest && far_cross {
            self.push_event(
                attempt,
                11,
                frame,
                frame.reference_price,
                boundary_gap(
                    frame.reference_price,
                    attempt.frozen_lower,
                    attempt.frozen_upper,
                ) / safe_atr,
                attempt.max_penetration_atr,
                0,
            );
            self.push_event(
                attempt,
                8,
                frame,
                frame.reference_price,
                0.0,
                attempt.max_penetration_atr,
                0,
            );
            self.ensure_memory(attempt.node_id).accepted_side = 0;
            self.complete_attempt(attempt, 5, frame.market_time);
        } else if !attempt.is_retest && !attempt.broke_far_boundary && far_cross {
            attempt.broke_far_boundary = true;
            attempt.break_at = frame.market_time;
            attempt.state = 4;
            self.push_event(
                attempt,
                4,
                frame,
                frame.reference_price,
                boundary_gap(
                    frame.reference_price,
                    attempt.frozen_lower,
                    attempt.frozen_upper,
                ) / safe_atr,
                attempt.max_penetration_atr,
                0,
            );
        }
        let origin_gap = if attempt.direction == -1 {
            attempt.frozen_lower - frame.reference_price
        } else {
            frame.reference_price - attempt.frozen_upper
        };
        let accepted_gap = if attempt.direction == -1 {
            frame.reference_price - attempt.frozen_upper
        } else {
            attempt.frozen_lower - frame.reference_price
        };
        if attempt.active
            && attempt.is_retest
            && !far_cross
            && frozen_side == attempt.direction
            && origin_gap / safe_atr >= 0.20
        {
            attempt.rejection_excursion_atr = origin_gap / safe_atr;
            self.push_event(
                attempt,
                10,
                frame,
                frame.reference_price,
                attempt.rejection_excursion_atr,
                attempt.max_penetration_atr,
                0,
            );
            self.complete_attempt(attempt, 4, frame.market_time);
        } else if attempt.active
            && !attempt.is_retest
            && !attempt.broke_far_boundary
            && frozen_side == attempt.direction
            && origin_gap / safe_atr >= 0.20
        {
            attempt.rejection_excursion_atr = origin_gap / safe_atr;
            self.push_event(
                attempt,
                7,
                frame,
                frame.reference_price,
                attempt.rejection_excursion_atr,
                attempt.max_penetration_atr,
                0,
            );
            self.push_event(
                attempt,
                12,
                frame,
                frame.reference_price,
                attempt.rejection_excursion_atr,
                attempt.max_penetration_atr,
                0,
            );
            self.complete_attempt(attempt, 1, frame.market_time);
        } else if attempt.active && attempt.accepted && accepted_gap / safe_atr >= 0.25 {
            self.push_event(
                attempt,
                12,
                frame,
                frame.reference_price,
                accepted_gap / safe_atr,
                attempt.max_penetration_atr,
                0,
            );
            self.ensure_memory(attempt.node_id).accepted_side = -attempt.direction;
            self.start_transit(attempt, nodes, frame);
            self.complete_attempt(attempt, 2, frame.market_time);
        }
    }

    fn observe_closed_bar(&mut self, attempt: &mut Attempt, frame: &Frame, safe_atr: f64) {
        let close_side = side_of(
            frame.closed_bar_price,
            attempt.frozen_lower,
            attempt.frozen_upper,
        );
        let far_side = -attempt.direction;
        let far_distance = if far_side > 0 {
            frame.closed_bar_price - attempt.frozen_upper
        } else {
            attempt.frozen_lower - frame.closed_bar_price
        };
        if attempt.broke_far_boundary && close_side == far_side && far_distance / safe_atr >= 0.0 {
            attempt.qualified_far_closes += 1;
            if !attempt.provisional_acceptance {
                attempt.provisional_acceptance = true;
                attempt.state = 5;
                self.push_event(
                    attempt,
                    5,
                    frame,
                    frame.closed_bar_price,
                    far_distance / safe_atr,
                    attempt.max_penetration_atr,
                    0,
                );
            }
            if !attempt.accepted && attempt.qualified_far_closes >= 2 {
                attempt.accepted = true;
                attempt.accepted_at = frame.closed_bar_time;
                attempt.state = 6;
                self.push_event(
                    attempt,
                    6,
                    frame,
                    frame.closed_bar_price,
                    far_distance / safe_atr,
                    attempt.max_penetration_atr,
                    0,
                );
            }
        } else if attempt.broke_far_boundary && close_side == attempt.direction {
            let reclaim = if attempt.direction == -1 {
                attempt.frozen_lower - frame.closed_bar_price
            } else {
                frame.closed_bar_price - attempt.frozen_upper
            };
            if reclaim / safe_atr >= 0.05 {
                self.push_event(
                    attempt,
                    8,
                    frame,
                    frame.closed_bar_price,
                    reclaim / safe_atr,
                    attempt.max_penetration_atr,
                    0,
                );
                self.complete_attempt(attempt, 3, frame.market_time);
            }
        }
    }

    fn start_attempt(
        &mut self,
        node_index: usize,
        nodes: &mut [Node],
        sources: &[SourceRef],
        frame: &Frame,
        side: i32,
    ) -> usize {
        let node = nodes[node_index].clone();
        let memory_index = self.ensure_memory_index(node.node_id);
        self.memory[memory_index].attempt_ordinal += 1;
        let episode_index = self.open_episode(&node, frame.market_time, side, frame.atr);
        let ordinal = self.memory[memory_index].attempt_ordinal;
        let mut attempt = Attempt {
            active: true,
            completion_status: ACTIVE,
            is_retest: self.memory[memory_index].accepted_side == side,
            attempt_ordinal: ordinal,
            attempt_id: attempt_id(node.node_id, frame.market_time, side, ordinal),
            episode_id: self.active_episodes[episode_index].episode_id,
            node_id: node.node_id,
            evidence_id: node.evidence_id,
            direction: side,
            state: 1,
            started_at: frame.market_time,
            last_observed_at: frame.market_time,
            start_bar_sequence: self.bar_sequence,
            frozen_lower: node.lower,
            frozen_price: node.price,
            frozen_upper: node.upper,
            frozen_width: (node.upper - node.lower).max(0.0),
            frozen_atr: frame.atr,
            start_price: frame.reference_price,
            last_price: frame.reference_price,
            family_mask: node.family_mask,
            family_count: node.family_count,
            member_count: node.member_count,
            developing_count: node.developing_count,
            frozen_count: node.frozen_count,
            node_revision: node.revision,
            provenance_offset: node.provenance_offset,
            provenance_count: node.provenance_count,
            corridor_up_level_count: node.corridor_up_levels,
            corridor_down_level_count: node.corridor_down_levels,
            corridor_up_noise_count: node.corridor_up_noise,
            corridor_down_noise_count: node.corridor_down_noise,
            context: self.feature.clone(),
            start_region: self.feature.price_region,
            end_region: self.feature.price_region,
            node_region: node.region,
            start_median_sigma: self.feature.price_from_median_sigma,
            end_median_sigma: self.feature.price_from_median_sigma,
            min_median_sigma: self.feature.price_from_median_sigma,
            max_median_sigma: self.feature.price_from_median_sigma,
            node_from_median_sigma: node.median_distance_sigma,
            node_from_cog_sigma: node.cog_distance_sigma,
            node_width_sigma: node.width_sigma,
            node_width_atr: (node.upper - node.lower).max(0.0) / frame.atr.max(1e-8),
            initial_distance_atr: boundary_gap(frame.reference_price, node.lower, node.upper)
                / frame.atr.max(1e-8),
            ..Attempt::default()
        };
        self.freeze_corridors(&mut attempt, nodes, frame.atr);
        self.active_episodes[episode_index].attempts += 1;
        self.active_episodes[episode_index].corridor_up_atr = attempt.corridor_up_atr;
        self.active_episodes[episode_index].corridor_down_atr = attempt.corridor_down_atr;
        nodes[node_index].attempt_count += 1;
        let begin = attempt.provenance_offset.max(0) as usize;
        let end = (begin + attempt.provenance_count.max(0) as usize).min(sources.len());
        for source in &sources[begin..end] {
            self.ledger.context.push(ContextRow {
                attempt_id: attempt.attempt_id,
                episode_id: attempt.episode_id,
                node_id: attempt.node_id,
                frozen_at: frame.market_time,
                source: source.clone(),
            });
        }
        self.ledger
            .features
            .push((attempt.attempt_id, self.feature.clone()));
        self.ledger.attempts_started += 1;
        let distance = attempt.initial_distance_atr;
        self.push_event(
            &mut attempt,
            1,
            frame,
            frame.reference_price,
            distance,
            0.0,
            0,
        );
        self.active_attempts.push(attempt);
        self.active_attempts.len() - 1
    }

    fn open_episode(&mut self, node: &Node, market_time: u64, direction: i32, atr: f64) -> usize {
        if let Some(index) = self.active_episodes.iter().position(|episode| {
            episode.node_id == node.node_id
                && (episode.last_attempt_end_bar <= 0
                    || self.bar_sequence - episode.last_attempt_end_bar <= 12)
        }) {
            return index;
        }
        let memory_index = self.ensure_memory_index(node.node_id);
        self.memory[memory_index].episode_ordinal += 1;
        let ordinal = self.memory[memory_index].episode_ordinal;
        self.active_episodes.push(Episode {
            active: true,
            regional_valid: self.feature.regional_valid,
            sigma_valid: self.feature.sigma_valid,
            completion_status: ACTIVE,
            episode_id: episode_id(node.node_id, market_time, ordinal),
            node_id: node.node_id,
            started_at: market_time,
            start_bar_sequence: self.bar_sequence,
            first_direction: direction,
            node_width_atr: (node.upper - node.lower).max(0.0) / atr.max(1e-8),
            family_mask: node.family_mask,
            family_count: node.family_count,
            member_count: node.member_count,
            initial_region: self.feature.price_region,
            terminal_region: self.feature.price_region,
            ..Episode::default()
        });
        self.ledger.episodes_started += 1;
        self.active_episodes.len() - 1
    }

    fn complete_attempt(&mut self, attempt: &mut Attempt, resolution: i32, market_time: u64) {
        if !attempt.active {
            return;
        }
        attempt.active = false;
        attempt.state = 8;
        attempt.resolution = resolution;
        attempt.completion_status = RESOLVED;
        attempt.censor_reason = 0;
        attempt.resolved_at = market_time;
        attempt.resolved_bar_sequence = self.bar_sequence;
        attempt.end_region = self.feature.price_region;
        attempt.end_median_sigma = self.feature.price_from_median_sigma;
        if let Some(episode) = self
            .active_episodes
            .iter_mut()
            .find(|episode| episode.episode_id == attempt.episode_id)
        {
            episode.last_attempt_end_bar = self.bar_sequence;
            episode.ended_at = market_time;
            episode.resolution = resolution;
            episode.breaks += i32::from(attempt.broke_far_boundary);
            episode.reclaims += i32::from(resolution == 3);
            episode.retests += i32::from(attempt.is_retest);
            episode.max_up_excursion_atr =
                episode.max_up_excursion_atr.max(attempt.max_above_node_atr);
            episode.max_down_excursion_atr = episode
                .max_down_excursion_atr
                .max(attempt.max_below_node_atr);
            episode.terminal_region = attempt.end_region;
        }
        self.ledger.terminal_hash = hash_mix(
            hash_mix(self.ledger.terminal_hash, attempt.attempt_id),
            resolution as u64,
        );
        self.ledger.attempts_resolved += 1;
        self.ledger.attempts.push(attempt.clone());
    }

    #[allow(clippy::too_many_arguments)]
    fn push_event(
        &mut self,
        attempt: &mut Attempt,
        kind: i32,
        frame: &Frame,
        price: f64,
        distance: f64,
        penetration: f64,
        related: u64,
    ) {
        self.push_event_with_atr(
            attempt,
            kind,
            frame,
            price,
            frame.atr,
            distance,
            penetration,
            related,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_event_with_atr(
        &mut self,
        attempt: &mut Attempt,
        kind: i32,
        frame: &Frame,
        price: f64,
        atr: f64,
        distance: f64,
        penetration: f64,
        related: u64,
    ) {
        attempt.event_count += 1;
        self.ledger.event_sequence += 1;
        let event_id = hash_mix(
            hash_mix(
                hash_mix(attempt.attempt_id, attempt.event_count as u64),
                kind as u64,
            ),
            frame.market_time,
        )
        .max(1);
        self.ledger.events.push(Event {
            event_id,
            event_sequence: self.ledger.event_sequence,
            episode_id: attempt.episode_id,
            attempt_id: attempt.attempt_id,
            node_id: attempt.node_id,
            related_node_id: related,
            market_time: frame.market_time,
            bar_time: frame.closed_bar_time,
            kind,
            direction: attempt.direction,
            price,
            atr,
            distance_atr: distance,
            penetration_atr: penetration,
            bar_sequence: self.bar_sequence,
            attempt_event_sequence: attempt.event_count,
            regional: self.feature.clone(),
        });
        for value in [
            event_id,
            kind as u64,
            attempt.node_id,
            self.feature.regional_basis_hash,
            mql_round(self.feature.price_from_median_sigma * 100_000_000.0) as u64,
            (self.feature.price_region + 100) as u64,
        ] {
            self.ledger.terminal_hash = hash_mix(self.ledger.terminal_hash, value);
        }
    }

    fn freeze_corridors(&self, attempt: &mut Attempt, nodes: &[Node], atr: f64) {
        let mut up = f64::MAX;
        let mut down = f64::MAX;
        for node in nodes
            .iter()
            .filter(|node| node.existence == 1 && node.node_id != attempt.node_id)
        {
            if node.lower >= attempt.frozen_upper && node.lower - attempt.frozen_upper < up {
                up = node.lower - attempt.frozen_upper;
                attempt.nearest_above_id = node.node_id;
            }
            if node.upper <= attempt.frozen_lower && attempt.frozen_lower - node.upper < down {
                down = attempt.frozen_lower - node.upper;
                attempt.nearest_below_id = node.node_id;
            }
        }
        attempt.corridor_up_atr = if up == f64::MAX {
            -1.0
        } else {
            up / atr.max(1e-8)
        };
        attempt.corridor_down_atr = if down == f64::MAX {
            -1.0
        } else {
            down / atr.max(1e-8)
        };
    }

    fn start_transit(&mut self, attempt: &Attempt, nodes: &[Node], frame: &Frame) {
        let destination = if attempt.direction == -1 {
            attempt.nearest_above_id
        } else {
            attempt.nearest_below_id
        };
        let Some(node) = nodes
            .iter()
            .find(|node| destination != 0 && node.node_id == destination)
        else {
            return;
        };
        self.active_transits.push(Transit {
            active: true,
            regional_valid: attempt.context.regional_valid,
            sigma_valid: attempt.context.sigma_valid,
            transit_id: hash_mix(attempt.attempt_id, destination).max(1),
            attempt_id: attempt.attempt_id,
            episode_id: attempt.episode_id,
            source_node_id: attempt.node_id,
            destination_node_id: destination,
            direction: attempt.direction,
            started_at: frame.market_time,
            start_bar_sequence: self.bar_sequence,
            start_price: frame.reference_price,
            last_price: frame.reference_price,
            source_lower: attempt.frozen_lower,
            source_upper: attempt.frozen_upper,
            destination_lower: node.lower,
            destination_upper: node.upper,
            frozen_atr: attempt.frozen_atr,
            distance_atr: (node.price - attempt.frozen_price).abs() / attempt.frozen_atr.max(1e-8),
            regional_basis_hash: attempt.context.regional_basis_hash,
            structure_snapshot_hash: attempt.context.structure_snapshot_hash,
            source_region: attempt.node_region,
            destination_region: node.region,
            start_price_region: attempt.start_region,
            end_price_region: attempt.start_region,
            start_median_price: attempt.context.median_price,
            start_structural_sigma: attempt.context.structural_sigma,
            start_price_from_median_sigma: attempt.start_median_sigma,
            end_price_from_median_sigma: attempt.start_median_sigma,
            completion_status: ACTIVE,
            ..Transit::default()
        });
        self.ledger.transits_started += 1;
    }

    fn observe_transits(&mut self, frame: &Frame) {
        for index in 0..self.active_transits.len() {
            let mut transit = self.active_transits[index].clone();
            let previous = transit.last_price;
            transit.path_length += (frame.reference_price - previous).abs();
            transit.last_price = frame.reference_price;
            let adverse = if transit.direction == -1 {
                transit.start_price - frame.reference_price
            } else {
                frame.reference_price - transit.start_price
            };
            transit.max_adverse_atr = transit
                .max_adverse_atr
                .max((adverse / transit.frozen_atr.max(1e-8)).max(0.0));
            let result = if touches(
                previous,
                frame.reference_price,
                transit.destination_lower,
                transit.destination_upper,
            ) {
                Some(6)
            } else if touches(
                previous,
                frame.reference_price,
                transit.source_lower,
                transit.source_upper,
            ) {
                Some(7)
            } else if self.bar_sequence - transit.start_bar_sequence > 48 {
                Some(8)
            } else {
                None
            };
            if let Some(resolution) = result {
                self.complete_transit(&mut transit, resolution, RESOLVED, 0, frame);
            }
            self.active_transits[index] = transit;
        }
        self.active_transits.retain(|transit| transit.active);
    }

    fn complete_transit(
        &mut self,
        transit: &mut Transit,
        resolution: i32,
        status: i32,
        censor: i32,
        frame: &Frame,
    ) {
        transit.active = false;
        transit.resolution = resolution;
        transit.completion_status = status;
        transit.censor_reason = censor;
        transit.ended_at = frame.market_time;
        transit.end_bar_sequence = self.bar_sequence;
        transit.end_price = frame.reference_price;
        transit.end_price_region = self.feature.price_region;
        transit.end_price_from_median_sigma = self.feature.price_from_median_sigma;
        let net = (frame.reference_price - transit.start_price).abs();
        transit.path_efficiency = if transit.path_length > 0.0 {
            net / transit.path_length
        } else {
            1.0
        };
        let mut synthetic = Attempt {
            attempt_id: transit.attempt_id,
            episode_id: transit.episode_id,
            node_id: transit.source_node_id,
            direction: transit.direction,
            ..Attempt::default()
        };
        let (kind, related) = if status == CENSORED {
            (15, 0)
        } else if resolution == 6 {
            (13, transit.destination_node_id)
        } else if resolution == 7 {
            (14, transit.source_node_id)
        } else {
            (16, 0)
        };
        self.push_event_with_atr(
            &mut synthetic,
            kind,
            frame,
            transit.end_price,
            transit.frozen_atr,
            transit.distance_atr,
            0.0,
            related,
        );
        if status != CENSORED {
            if let Some(episode) = self
                .active_episodes
                .iter_mut()
                .find(|episode| episode.episode_id == transit.episode_id)
            {
                if resolution == 6 {
                    episode.next_node_id = transit.destination_node_id;
                }
                episode.resolution = resolution;
                episode.ended_at = frame.market_time;
                episode.terminal_region = transit.end_price_region;
            }
            self.ledger.transits_resolved += 1;
        } else {
            self.ledger.transits_censored += 1;
        }
        for value in [
            transit.transit_id,
            resolution as u64,
            status as u64,
            censor as u64,
        ] {
            self.ledger.terminal_hash = hash_mix(self.ledger.terminal_hash, value);
        }
        self.ledger.transits.push(transit.clone());
    }

    fn close_expired_episodes(&mut self, market_time: u64) {
        for index in 0..self.active_episodes.len() {
            let episode = &self.active_episodes[index];
            let has_attempt = self
                .active_attempts
                .iter()
                .any(|attempt| attempt.active && attempt.node_id == episode.node_id);
            let has_transit = self
                .active_transits
                .iter()
                .any(|transit| transit.active && transit.episode_id == episode.episode_id);
            if !episode.active
                || episode.last_attempt_end_bar <= 0
                || has_attempt
                || has_transit
                || self.bar_sequence - episode.last_attempt_end_bar <= 12
            {
                continue;
            }
            let episode = &mut self.active_episodes[index];
            episode.active = false;
            episode.completion_status = RESOLVED;
            episode.censor_reason = 0;
            if episode.ended_at == 0 {
                episode.ended_at = market_time;
            }
            self.ledger.terminal_hash = hash_mix(
                hash_mix(self.ledger.terminal_hash, episode.episode_id),
                episode.resolution as u64,
            );
            self.ledger.episodes_resolved += 1;
            self.ledger.episodes.push(episode.clone());
        }
    }

    pub fn finalize(mut self, frame: &Frame, reason: i32) -> AuctionLedger {
        for index in 0..self.active_transits.len() {
            let mut transit = self.active_transits[index].clone();
            self.complete_transit(&mut transit, 0, CENSORED, reason, frame);
        }
        for index in 0..self.active_attempts.len() {
            let mut attempt = self.active_attempts[index].clone();
            if !attempt.active {
                continue;
            }
            let distance = boundary_gap(
                frame.reference_price,
                attempt.frozen_lower,
                attempt.frozen_upper,
            ) / frame.atr.max(1e-8);
            let penetration = attempt.max_penetration_atr;
            self.push_event(
                &mut attempt,
                15,
                frame,
                frame.reference_price,
                distance,
                penetration,
                0,
            );
            attempt.active = false;
            attempt.state = 8;
            attempt.completion_status = CENSORED;
            attempt.censor_reason = reason;
            attempt.resolution = 0;
            attempt.resolved_at = frame.market_time;
            attempt.resolved_bar_sequence = self.bar_sequence;
            attempt.end_region = self.feature.price_region;
            attempt.end_median_sigma = self.feature.price_from_median_sigma;
            self.ledger.terminal_hash = hash_mix(
                hash_mix(self.ledger.terminal_hash, attempt.attempt_id),
                reason as u64,
            );
            self.ledger.attempts_censored += 1;
            self.ledger.attempts.push(attempt);
        }
        for index in 0..self.active_episodes.len() {
            let episode = &mut self.active_episodes[index];
            if !episode.active {
                continue;
            }
            episode.active = false;
            episode.completion_status = CENSORED;
            episode.censor_reason = reason;
            episode.ended_at = frame.market_time;
            episode.terminal_region = self.feature.price_region;
            self.ledger.terminal_hash = hash_mix(
                hash_mix(self.ledger.terminal_hash, episode.episode_id),
                reason as u64,
            );
            self.ledger.episodes_censored += 1;
            self.ledger.episodes.push(episode.clone());
        }
        self.ledger
    }

    pub fn event_sequence(&self) -> u64 {
        self.ledger.event_sequence
    }
    fn ensure_memory_index(&mut self, node_id: u64) -> usize {
        if let Some(index) = self
            .memory
            .iter()
            .position(|memory| memory.node_id == node_id)
        {
            return index;
        }
        self.memory.push(NodeMemory {
            node_id,
            ..NodeMemory::default()
        });
        self.memory.len() - 1
    }
    fn ensure_memory(&mut self, node_id: u64) -> &mut NodeMemory {
        let index = self.ensure_memory_index(node_id);
        &mut self.memory[index]
    }
}

fn side_of(price: f64, lower: f64, upper: f64) -> i32 {
    if price < lower {
        -1
    } else if price > upper {
        1
    } else {
        0
    }
}
fn attempt_id(node: u64, time: u64, direction: i32, ordinal: i32) -> u64 {
    hash_mix(
        hash_mix(
            hash_mix(hash_mix(HASH_SEED, node), time),
            (direction + 2) as u64,
        ),
        ordinal as u64,
    )
    .max(1)
}
fn episode_id(node: u64, time: u64, ordinal: i32) -> u64 {
    hash_mix(
        hash_mix(hash_mix(1_099_511_628_211, node), time),
        ordinal as u64,
    )
    .max(1)
}
fn touches(previous: f64, current: f64, lower: f64, upper: f64) -> bool {
    (current >= lower && current <= upper)
        || previous.min(current) <= upper && previous.max(current) >= lower
}
fn mql_round(value: f64) -> i64 {
    if value >= 0.0 {
        (value + 0.5).floor() as i64
    } else {
        (value - 0.5).ceil() as i64
    }
}

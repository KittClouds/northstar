use northstar_auction_contract::{AuctionResolution, CensorReason, CompletionStatus, EventType};

mod scenarios;

pub(crate) use scenarios::verify_exact_golden;

const LOWER: f64 = 100.0;
const UPPER: f64 = 102.0;
const ATR: f64 = 10.0;
const EPS: f64 = 0.0001;
const BASE_TIME: u64 = 1_700_000_000;
const SOURCE_NODE: u64 = 1001;
const DESTINATION_NODE: u64 = 2002;
const HASH_SEED: u64 = 1_469_598_103_934_665_603;

struct EventRow {
    sequence: u64,
    id: u64,
    attempt_id: u64,
    episode_id: u64,
    related_node_id: u64,
    kind: EventType,
    direction: i8,
    market_time: u64,
    bar_time: u64,
    price: f64,
    distance_atr: f64,
    penetration_atr: f64,
}

#[derive(Clone)]
struct Attempt {
    id: u64,
    episode_id: u64,
    direction: i8,
    is_retest: bool,
    started_at: u64,
    contact_at: u64,
    break_at: u64,
    accepted_at: u64,
    resolved_at: u64,
    start_bar: i32,
    event_count: u64,
    last_price: f64,
    max_penetration: f64,
    max_penetration_atr: f64,
    max_above_atr: f64,
    max_below_atr: f64,
    rejection_atr: f64,
    broke: bool,
    provisional: bool,
    accepted: bool,
    qualified_far_closes: u8,
    resolution: AuctionResolution,
    censor: CensorReason,
}

#[derive(Clone)]
struct Episode {
    id: u64,
    started_at: u64,
    ended_at: u64,
    last_attempt_end_bar: i32,
    attempts: u32,
    breaks: u32,
    reclaims: u32,
    retests: u32,
    next_node_id: u64,
    resolution: AuctionResolution,
    completion: CompletionStatus,
    censor: CensorReason,
}

#[derive(Clone)]
struct Transit {
    id: u64,
    attempt_id: u64,
    episode_id: u64,
    direction: i8,
    started_at: u64,
    ended_at: u64,
    start_price: f64,
    last_price: f64,
    path_length: f64,
    max_adverse_atr: f64,
    end_price: f64,
    path_efficiency: f64,
    resolution: AuctionResolution,
    completion: CompletionStatus,
    censor: CensorReason,
}

struct ExactLedger {
    events: Vec<EventRow>,
    attempts: Vec<Attempt>,
    episodes: Vec<Episode>,
    transits: Vec<Transit>,
    terminal_hash: u64,
    event_sequence: u64,
}

struct ExactEngine {
    events: Vec<EventRow>,
    completed_attempts: Vec<Attempt>,
    completed_episodes: Vec<Episode>,
    completed_transits: Vec<Transit>,
    attempt: Option<Attempt>,
    episode: Option<Episode>,
    transit: Option<Transit>,
    attempt_ordinal: u64,
    episode_ordinal: u64,
    accepted_side: i8,
    bar_sequence: i32,
    last_bar_slot: i32,
    last_bar_time: u64,
    market_time: u64,
    event_sequence: u64,
    terminal_hash: u64,
}

impl ExactEngine {
    fn new() -> Self {
        Self {
            events: Vec::with_capacity(16),
            completed_attempts: Vec::with_capacity(2),
            completed_episodes: Vec::with_capacity(1),
            completed_transits: Vec::with_capacity(1),
            attempt: None,
            episode: None,
            transit: None,
            attempt_ordinal: 0,
            episode_ordinal: 0,
            accepted_side: 0,
            bar_sequence: 0,
            last_bar_slot: -1,
            last_bar_time: BASE_TIME - 60,
            market_time: BASE_TIME,
            event_sequence: 0,
            terminal_hash: HASH_SEED,
        }
    }

    fn observe(
        &mut self,
        price: f64,
        bar_slot: Option<i32>,
        closed_price: f64,
        source_present: bool,
    ) {
        let new_bar = bar_slot.is_some_and(|slot| slot > self.last_bar_slot);
        if let Some(slot) = bar_slot {
            self.last_bar_time = BASE_TIME - 60 + u64::try_from(slot).expect("positive slot") * 60;
            if new_bar {
                self.last_bar_slot = slot;
                self.bar_sequence += 1;
            }
        }

        if !source_present && let Some(mut attempt) = self.attempt.take() {
            let distance = boundary_gap(price) / ATR;
            let penetration = attempt.max_penetration_atr;
            self.emit_attempt(
                &mut attempt,
                EventType::Expire,
                price,
                distance,
                penetration,
                0,
            );
            self.complete_attempt(attempt, AuctionResolution::NodeRetired);
        }

        self.observe_transit(price);
        if source_present {
            self.observe_source(price, closed_price, new_bar);
        }
        if new_bar {
            self.close_expired_episode();
        }
    }

    fn observe_source(&mut self, price: f64, closed_price: f64, new_bar: bool) {
        if self.attempt.is_none() {
            let side = price_side(price);
            if side == 0 || boundary_gap(price) / ATR > 0.50 {
                return;
            }
            self.start_attempt(price, side);
        }

        let Some(mut attempt) = self.attempt.take() else {
            return;
        };
        attempt.last_price = price;
        attempt.max_above_atr = attempt.max_above_atr.max((price - UPPER).max(0.0) / ATR);
        attempt.max_below_atr = attempt.max_below_atr.max((LOWER - price).max(0.0) / ATR);
        let side = price_side(price);

        if attempt.contact_at == 0 && side == 0 {
            attempt.contact_at = self.market_time;
            self.emit_attempt(&mut attempt, EventType::Contact, price, 0.0, 0.0, 0);
            if attempt.is_retest {
                self.emit_attempt(&mut attempt, EventType::Retest, price, 0.0, 0.0, 0);
            }
        }

        if attempt.contact_at > 0 {
            let penetration = if attempt.direction < 0 {
                (price - LOWER).max(0.0)
            } else {
                (UPPER - price).max(0.0)
            };
            if penetration > attempt.max_penetration {
                let first = attempt.max_penetration <= 0.0;
                attempt.max_penetration = penetration;
                attempt.max_penetration_atr = penetration / ATR;
                if first {
                    let value = attempt.max_penetration_atr;
                    self.emit_attempt(&mut attempt, EventType::Penetration, price, 0.0, value, 0);
                }
            }

            let far_cross = if attempt.direction < 0 {
                price > UPPER + ATR * 0.10
            } else {
                price < LOWER - ATR * 0.10
            };
            if attempt.is_retest && far_cross {
                let distance = boundary_gap(price) / ATR;
                let penetration = attempt.max_penetration_atr;
                self.emit_attempt(
                    &mut attempt,
                    EventType::RetestFailure,
                    price,
                    distance,
                    penetration,
                    0,
                );
                self.emit_attempt(&mut attempt, EventType::Reclaim, price, 0.0, penetration, 0);
                self.accepted_side = 0;
                self.complete_attempt(attempt, AuctionResolution::AcceptAndFailRetest);
                return;
            }
            if !attempt.is_retest && !attempt.broke && far_cross {
                attempt.broke = true;
                attempt.break_at = self.market_time;
                let distance = boundary_gap(price) / ATR;
                let penetration = attempt.max_penetration_atr;
                self.emit_attempt(
                    &mut attempt,
                    EventType::Break,
                    price,
                    distance,
                    penetration,
                    0,
                );
            }

            let origin_gap = if attempt.direction < 0 {
                LOWER - price
            } else {
                price - UPPER
            };
            let accepted_gap = if attempt.direction < 0 {
                price - UPPER
            } else {
                LOWER - price
            };
            if attempt.is_retest
                && !far_cross
                && side == attempt.direction
                && origin_gap / ATR >= 0.20
            {
                attempt.rejection_atr = origin_gap / ATR;
                let rejection = attempt.rejection_atr;
                let penetration = attempt.max_penetration_atr;
                self.emit_attempt(
                    &mut attempt,
                    EventType::Hold,
                    price,
                    rejection,
                    penetration,
                    0,
                );
                self.complete_attempt(attempt, AuctionResolution::AcceptAndHoldRetest);
                return;
            }
            if !attempt.is_retest
                && !attempt.broke
                && side == attempt.direction
                && origin_gap / ATR >= 0.20
            {
                attempt.rejection_atr = origin_gap / ATR;
                let rejection = attempt.rejection_atr;
                let penetration = attempt.max_penetration_atr;
                self.emit_attempt(
                    &mut attempt,
                    EventType::Rejection,
                    price,
                    rejection,
                    penetration,
                    0,
                );
                self.emit_attempt(
                    &mut attempt,
                    EventType::Departure,
                    price,
                    rejection,
                    penetration,
                    0,
                );
                self.complete_attempt(attempt, AuctionResolution::RejectToOrigin);
                return;
            }
            if attempt.accepted && accepted_gap / ATR >= 0.25 {
                let distance = accepted_gap / ATR;
                let penetration = attempt.max_penetration_atr;
                self.emit_attempt(
                    &mut attempt,
                    EventType::Departure,
                    price,
                    distance,
                    penetration,
                    0,
                );
                self.accepted_side = -attempt.direction;
                self.start_transit(&attempt, price);
                self.complete_attempt(attempt, AuctionResolution::AcceptThroughNode);
                return;
            }
        }

        if new_bar && attempt.contact_at > 0 {
            let close_side = price_side(closed_price);
            let far_side = -attempt.direction;
            let far_distance = if far_side > 0 {
                closed_price - UPPER
            } else {
                LOWER - closed_price
            };
            if attempt.broke && close_side == far_side && far_distance / ATR >= 0.10 {
                attempt.qualified_far_closes += 1;
                if !attempt.provisional {
                    attempt.provisional = true;
                    let penetration = attempt.max_penetration_atr;
                    self.emit_attempt(
                        &mut attempt,
                        EventType::ProvisionalAcceptance,
                        closed_price,
                        far_distance / ATR,
                        penetration,
                        0,
                    );
                }
                if !attempt.accepted && attempt.qualified_far_closes >= 2 {
                    attempt.accepted = true;
                    attempt.accepted_at = self.last_bar_time;
                    let penetration = attempt.max_penetration_atr;
                    self.emit_attempt(
                        &mut attempt,
                        EventType::Acceptance,
                        closed_price,
                        far_distance / ATR,
                        penetration,
                        0,
                    );
                }
            } else if attempt.broke && close_side == attempt.direction {
                let reclaim_distance = if attempt.direction < 0 {
                    LOWER - closed_price
                } else {
                    closed_price - UPPER
                };
                if reclaim_distance / ATR >= 0.05 {
                    let penetration = attempt.max_penetration_atr;
                    self.emit_attempt(
                        &mut attempt,
                        EventType::Reclaim,
                        closed_price,
                        reclaim_distance / ATR,
                        penetration,
                        0,
                    );
                    self.complete_attempt(attempt, AuctionResolution::ReclaimAfterBreak);
                    return;
                }
            }
        }

        if new_bar && self.bar_sequence - attempt.start_bar > 3 {
            let distance = boundary_gap(price) / ATR;
            let penetration = attempt.max_penetration_atr;
            self.emit_attempt(
                &mut attempt,
                EventType::Expire,
                price,
                distance,
                penetration,
                0,
            );
            self.complete_attempt(attempt, AuctionResolution::Timeout);
            return;
        }
        self.attempt = Some(attempt);
    }

    fn start_attempt(&mut self, price: f64, direction: i8) {
        self.attempt_ordinal += 1;
        if self.episode.is_none() {
            self.episode_ordinal += 1;
            self.episode = Some(Episode {
                id: episode_id(SOURCE_NODE, self.market_time, self.episode_ordinal),
                started_at: self.market_time,
                ended_at: 0,
                last_attempt_end_bar: 0,
                attempts: 0,
                breaks: 0,
                reclaims: 0,
                retests: 0,
                next_node_id: 0,
                resolution: AuctionResolution::None,
                completion: CompletionStatus::Active,
                censor: CensorReason::None,
            });
        }
        let episode = self.episode.as_mut().expect("episode was opened");
        episode.attempts += 1;
        let mut attempt = Attempt {
            id: attempt_id(
                SOURCE_NODE,
                self.market_time,
                direction,
                self.attempt_ordinal,
            ),
            episode_id: episode.id,
            direction,
            is_retest: self.accepted_side == direction,
            started_at: self.market_time,
            contact_at: 0,
            break_at: 0,
            accepted_at: 0,
            resolved_at: 0,
            start_bar: self.bar_sequence,
            event_count: 0,
            last_price: price,
            max_penetration: 0.0,
            max_penetration_atr: 0.0,
            max_above_atr: 0.0,
            max_below_atr: 0.0,
            rejection_atr: 0.0,
            broke: false,
            provisional: false,
            accepted: false,
            qualified_far_closes: 0,
            resolution: AuctionResolution::None,
            censor: CensorReason::None,
        };
        let distance = boundary_gap(price) / ATR;
        self.emit_attempt(&mut attempt, EventType::Approach, price, distance, 0.0, 0);
        self.attempt = Some(attempt);
    }

    fn emit_attempt(
        &mut self,
        attempt: &mut Attempt,
        kind: EventType,
        price: f64,
        distance_atr: f64,
        penetration_atr: f64,
        related_node_id: u64,
    ) {
        attempt.event_count += 1;
        self.emit_event(
            attempt.id,
            attempt.episode_id,
            attempt.direction,
            attempt.event_count,
            kind,
            price,
            distance_atr,
            penetration_atr,
            related_node_id,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_event(
        &mut self,
        attempt_id: u64,
        episode_id: u64,
        direction: i8,
        attempt_event_count: u64,
        kind: EventType,
        price: f64,
        distance_atr: f64,
        penetration_atr: f64,
        related_node_id: u64,
    ) {
        self.event_sequence += 1;
        let id = event_id(attempt_id, attempt_event_count, kind, self.market_time);
        self.events.push(EventRow {
            sequence: self.event_sequence,
            id,
            attempt_id,
            episode_id,
            related_node_id,
            kind,
            direction,
            market_time: self.market_time,
            bar_time: self.last_bar_time,
            price,
            distance_atr,
            penetration_atr,
        });
        for value in [id, u64::from(kind.code()), SOURCE_NODE, 0, 0, 100] {
            self.terminal_hash = hash_mix(self.terminal_hash, value);
        }
    }

    fn complete_attempt(&mut self, mut attempt: Attempt, resolution: AuctionResolution) {
        attempt.resolution = resolution;
        attempt.censor = CensorReason::None;
        attempt.resolved_at = self.market_time;
        if let Some(episode) = self.episode.as_mut() {
            episode.last_attempt_end_bar = self.bar_sequence;
            episode.ended_at = attempt.resolved_at;
            episode.resolution = resolution;
            episode.breaks += u32::from(attempt.broke);
            episode.reclaims += u32::from(resolution == AuctionResolution::ReclaimAfterBreak);
            episode.retests += u32::from(attempt.is_retest);
        }
        self.terminal_hash = hash_mix(self.terminal_hash, attempt.id);
        self.terminal_hash = hash_mix(self.terminal_hash, u64::from(resolution.code()));
        self.completed_attempts.push(attempt);
    }

    fn censor_attempt(&mut self, mut attempt: Attempt, reason: CensorReason) {
        attempt.resolution = AuctionResolution::None;
        attempt.censor = reason;
        attempt.resolved_at = self.market_time;
        self.terminal_hash = hash_mix(self.terminal_hash, attempt.id);
        self.terminal_hash = hash_mix(self.terminal_hash, u64::from(reason.code()));
        self.completed_attempts.push(attempt);
    }

    fn start_transit(&mut self, attempt: &Attempt, price: f64) {
        self.transit = Some(Transit {
            id: hash_mix(attempt.id, DESTINATION_NODE).max(1),
            attempt_id: attempt.id,
            episode_id: attempt.episode_id,
            direction: attempt.direction,
            started_at: self.market_time,
            ended_at: 0,
            start_price: price,
            last_price: price,
            path_length: 0.0,
            max_adverse_atr: 0.0,
            end_price: price,
            path_efficiency: 0.0,
            resolution: AuctionResolution::None,
            completion: CompletionStatus::Active,
            censor: CensorReason::None,
        });
    }

    fn observe_transit(&mut self, price: f64) {
        let Some(mut transit) = self.transit.take() else {
            return;
        };
        let previous = transit.last_price;
        transit.path_length += (price - previous).abs();
        transit.last_price = price;
        let adverse = if transit.direction < 0 {
            transit.start_price - price
        } else {
            price - transit.start_price
        };
        transit.max_adverse_atr = transit.max_adverse_atr.max((adverse / ATR).max(0.0));
        let destination = if transit.direction < 0 {
            (120.0, 122.0)
        } else {
            (80.0, 82.0)
        };
        if touches(previous, price, destination.0, destination.1) {
            self.complete_transit(
                transit,
                price,
                AuctionResolution::TransitToNextNode,
                CompletionStatus::Resolved,
                CensorReason::None,
            );
        } else if touches(previous, price, LOWER, UPPER) {
            self.complete_transit(
                transit,
                price,
                AuctionResolution::ReturnToSourceNode,
                CompletionStatus::Resolved,
                CensorReason::None,
            );
        } else {
            self.transit = Some(transit);
        }
    }

    fn complete_transit(
        &mut self,
        mut transit: Transit,
        price: f64,
        resolution: AuctionResolution,
        completion: CompletionStatus,
        censor: CensorReason,
    ) {
        transit.resolution = resolution;
        transit.completion = completion;
        transit.censor = censor;
        transit.ended_at = self.market_time;
        transit.end_price = price;
        let net = (price - transit.start_price).abs();
        transit.path_efficiency = if transit.path_length > 0.0 {
            net / transit.path_length
        } else {
            1.0
        };
        let kind = if completion == CompletionStatus::RightCensored {
            EventType::Censor
        } else if resolution == AuctionResolution::TransitToNextNode {
            EventType::Transit
        } else {
            EventType::ReturnToSource
        };
        let related = match kind {
            EventType::Transit => DESTINATION_NODE,
            EventType::ReturnToSource => SOURCE_NODE,
            _ => 0,
        };
        self.emit_event(
            transit.attempt_id,
            transit.episode_id,
            transit.direction,
            1,
            kind,
            price,
            2.0,
            0.0,
            related,
        );
        if completion != CompletionStatus::RightCensored
            && let Some(episode) = self.episode.as_mut()
        {
            if resolution == AuctionResolution::TransitToNextNode {
                episode.next_node_id = DESTINATION_NODE;
            }
            episode.resolution = resolution;
            episode.ended_at = self.market_time;
        }
        for value in [
            transit.id,
            u64::from(resolution.code()),
            u64::from(completion.code()),
            u64::from(censor.code()),
        ] {
            self.terminal_hash = hash_mix(self.terminal_hash, value);
        }
        self.completed_transits.push(transit);
    }

    fn close_expired_episode(&mut self) {
        let should_close = self.episode.as_ref().is_some_and(|episode| {
            episode.last_attempt_end_bar > 0
                && self.attempt.is_none()
                && self.transit.is_none()
                && self.bar_sequence - episode.last_attempt_end_bar > 2
        });
        if !should_close {
            return;
        }
        let mut episode = self.episode.take().expect("checked episode");
        episode.completion = CompletionStatus::Resolved;
        episode.censor = CensorReason::None;
        if episode.ended_at == 0 {
            episode.ended_at = self.market_time;
        }
        self.terminal_hash = hash_mix(self.terminal_hash, episode.id);
        self.terminal_hash = hash_mix(self.terminal_hash, u64::from(episode.resolution.code()));
        self.completed_episodes.push(episode);
    }

    fn finalize(&mut self, reason: CensorReason, price: f64) {
        if let Some(transit) = self.transit.take() {
            self.complete_transit(
                transit,
                price,
                AuctionResolution::None,
                CompletionStatus::RightCensored,
                reason,
            );
        }
        if let Some(mut attempt) = self.attempt.take() {
            let distance = boundary_gap(price) / ATR;
            let penetration = attempt.max_penetration_atr;
            self.emit_attempt(
                &mut attempt,
                EventType::Censor,
                price,
                distance,
                penetration,
                0,
            );
            self.censor_attempt(attempt, reason);
        }
        if let Some(mut episode) = self.episode.take() {
            episode.completion = CompletionStatus::RightCensored;
            episode.censor = reason;
            episode.ended_at = self.market_time;
            self.terminal_hash = hash_mix(self.terminal_hash, episode.id);
            self.terminal_hash = hash_mix(self.terminal_hash, u64::from(reason.code()));
            self.completed_episodes.push(episode);
        }
    }

    fn ledger(self) -> ExactLedger {
        ExactLedger {
            events: self.events,
            attempts: self.completed_attempts,
            episodes: self.completed_episodes,
            transits: self.completed_transits,
            terminal_hash: self.terminal_hash,
            event_sequence: self.event_sequence,
        }
    }
}

fn price_side(price: f64) -> i8 {
    if price < LOWER {
        -1
    } else if price > UPPER {
        1
    } else {
        0
    }
}

fn boundary_gap(price: f64) -> f64 {
    if price < LOWER {
        LOWER - price
    } else if price > UPPER {
        price - UPPER
    } else {
        0.0
    }
}

fn touches(previous: f64, current: f64, lower: f64, upper: f64) -> bool {
    (lower..=upper).contains(&current)
        || previous.min(current) <= upper && previous.max(current) >= lower
}

fn hash_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(1_099_511_628_211)
}

fn attempt_id(node_id: u64, started_at: u64, direction: i8, ordinal: u64) -> u64 {
    let hash = hash_mix(HASH_SEED, node_id);
    let hash = hash_mix(hash, started_at);
    let hash = hash_mix(hash, (direction + 2) as u64);
    hash_mix(hash, ordinal).max(1)
}

fn episode_id(node_id: u64, started_at: u64, ordinal: u64) -> u64 {
    let hash = hash_mix(1_099_511_628_211, node_id);
    let hash = hash_mix(hash, started_at);
    hash_mix(hash, ordinal).max(1)
}

fn event_id(attempt_id: u64, count: u64, kind: EventType, market_time: u64) -> u64 {
    let hash = hash_mix(attempt_id, count);
    let hash = hash_mix(hash, u64::from(kind.code()));
    hash_mix(hash, market_time).max(1)
}

fn hash_text(value: &str) -> u64 {
    value
        .encode_utf16()
        .fold(HASH_SEED, |hash, unit| hash_mix(hash, u64::from(unit)))
}

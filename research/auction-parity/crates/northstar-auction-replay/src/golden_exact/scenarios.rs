use std::fmt::Write;

use northstar_auction_contract::CensorReason;
use northstar_parity_fixtures::{Direction, GOLDEN_SCENARIOS};

use crate::grammar::GrammarDirection;

use super::*;

impl ExactLedger {
    fn text(&self) -> String {
        let mut out = String::with_capacity(4096);
        out.push_str("E[");
        for (index, row) in self.events.iter().enumerate() {
            if index > 0 {
                out.push(';');
            }
            write!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{:.4},{:.8},{:.8}",
                row.sequence,
                row.id,
                row.attempt_id,
                row.episode_id,
                SOURCE_NODE,
                row.related_node_id,
                row.kind.code(),
                row.direction,
                row.market_time,
                row.bar_time,
                row.price,
                row.distance_atr,
                row.penetration_atr
            )
            .expect("writing to String cannot fail");
        }
        out.push_str("]A[");
        for (index, row) in self.attempts.iter().enumerate() {
            if index > 0 {
                out.push(';');
            }
            write!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{},{:.8},{:.8},{:.8},{:.8}",
                row.id,
                row.episode_id,
                SOURCE_NODE,
                row.direction,
                row.resolution.code(),
                row.censor.code(),
                row.started_at,
                row.contact_at,
                row.break_at,
                row.accepted_at,
                row.resolved_at,
                row.max_penetration_atr,
                row.rejection_atr,
                row.max_above_atr,
                row.max_below_atr
            )
            .expect("writing to String cannot fail");
        }
        out.push_str("]P[");
        for (index, row) in self.episodes.iter().enumerate() {
            if index > 0 {
                out.push(';');
            }
            write!(
                out,
                "{},{},{},{},{},{},{},{},{},{}",
                row.id,
                SOURCE_NODE,
                row.resolution.code(),
                row.completion.code(),
                row.censor.code(),
                row.attempts,
                row.retests,
                row.next_node_id,
                row.started_at,
                row.ended_at
            )
            .expect("writing to String cannot fail");
        }
        out.push_str("]T[");
        for (index, row) in self.transits.iter().enumerate() {
            if index > 0 {
                out.push(';');
            }
            write!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{:.8},{:.8}",
                row.id,
                row.attempt_id,
                row.episode_id,
                SOURCE_NODE,
                DESTINATION_NODE,
                row.direction,
                row.resolution.code(),
                row.censor.code(),
                row.started_at,
                row.ended_at,
                row.max_adverse_atr,
                row.path_efficiency
            )
            .expect("writing to String cannot fail");
        }
        write!(out, "]H[{},{}]", self.terminal_hash, self.event_sequence)
            .expect("writing to String cannot fail");
        out
    }
}

pub(crate) fn verify_exact_golden() -> Result<(usize, usize), String> {
    let mut ledger_assertions = 0;
    let mut terminal_assertions = 0;
    for fixture in GOLDEN_SCENARIOS {
        for (direction, fixture_direction) in [
            (GrammarDirection::FromBelow, Direction::FromBelow),
            (GrammarDirection::FromAbove, Direction::FromAbove),
        ] {
            let ledger = run_exact(fixture.name, direction);
            let certificate = match fixture_direction {
                Direction::FromBelow => fixture.from_below,
                Direction::FromAbove => fixture.from_above,
            };
            let actual_ledger = hash_text(&ledger.text());
            if actual_ledger != certificate.ledger_hash {
                return Err(format!(
                    "{} {fixture_direction:?} ledger hash: expected {}, got {}\n{}",
                    fixture.name,
                    certificate.ledger_hash,
                    actual_ledger,
                    ledger.text()
                ));
            }
            ledger_assertions += 1;
            if ledger.terminal_hash != certificate.terminal_hash {
                return Err(format!(
                    "{} {fixture_direction:?} terminal hash: expected {}, got {}",
                    fixture.name, certificate.terminal_hash, ledger.terminal_hash
                ));
            }
            terminal_assertions += 1;
        }
    }
    Ok((ledger_assertions, terminal_assertions))
}

fn run_exact(name: &str, direction: GrammarDirection) -> ExactLedger {
    let mut engine = ExactEngine::new();
    let start = select(direction, 95.0, 107.0);
    let contact = select(direction, 100.0, 102.0);
    let break_price = select(direction, 103.0 + EPS, 99.0 - EPS);
    let far_idle = select(direction, 88.0, 114.0);
    match name {
        "APPROACH_NO_CONTACT" | "TEST_END_CENSOR" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            finish(&mut engine, CensorReason::TestEnd, 30, start);
        }
        "SHUTDOWN_CENSOR" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            finish(&mut engine, CensorReason::Shutdown, 30, start);
        }
        "DATA_GAP_CENSOR" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            finish(&mut engine, CensorReason::DataGap, 30, start);
        }
        "CONTACT_REJECTION" | "PENETRATION_REJECTION" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            tick(&mut engine, contact, 10, None, contact, true);
            if name == "PENETRATION_REJECTION" {
                tick(&mut engine, 101.0, 15, None, 101.0, true);
            }
            let reject = select(direction, 98.0, 104.0);
            tick(&mut engine, reject, 20, None, reject, true);
            close_episode(&mut engine, far_idle, 60, 1);
            finish(&mut engine, CensorReason::TestEnd, 200, far_idle);
        }
        "BREAK_NO_ACCEPTANCE" => {
            start_break(&mut engine, start, contact, break_price);
            finish(&mut engine, CensorReason::TestEnd, 30, break_price);
        }
        "BREAK_RECLAIM" => {
            start_break(&mut engine, start, contact, break_price);
            let reclaim = select(direction, 99.5, 102.5);
            tick(&mut engine, reclaim, 60, Some(1), reclaim, true);
            close_episode(&mut engine, far_idle, 120, 2);
            finish(&mut engine, CensorReason::TestEnd, 300, far_idle);
        }
        "TIMEOUT" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            for bar in 1..=4 {
                tick(&mut engine, start, bar * 60, Some(bar), start, true);
            }
            close_episode(&mut engine, far_idle, 300, 5);
            finish(&mut engine, CensorReason::TestEnd, 500, far_idle);
        }
        "NODE_RETIREMENT" => {
            tick(&mut engine, start, 0, Some(0), start, true);
            tick(&mut engine, start, 20, None, start, false);
            close_episode(&mut engine, far_idle, 60, 1);
            finish(&mut engine, CensorReason::TestEnd, 220, far_idle);
        }
        _ => {
            accept_and_depart(&mut engine, direction, start, contact, break_price);
            match name {
                "BREAK_ACCEPTANCE" => {
                    let departure = select(direction, 104.5, 97.5);
                    finish(&mut engine, CensorReason::TestEnd, 140, departure);
                }
                "ACCEPTED_RETEST" | "RETEST_HOLD" | "RETEST_FAILURE" => {
                    let retest_start = select(direction, 107.0, 95.0);
                    let retest_contact = select(direction, 102.0, 100.0);
                    tick(&mut engine, retest_start, 140, None, retest_start, true);
                    tick(&mut engine, retest_contact, 150, None, retest_contact, true);
                    if name == "RETEST_HOLD" {
                        let hold = select(direction, 104.0, 98.0);
                        tick(&mut engine, hold, 160, None, hold, true);
                        close_episode(&mut engine, far_idle, 180, 3);
                        finish(&mut engine, CensorReason::TestEnd, 360, far_idle);
                    } else if name == "RETEST_FAILURE" {
                        let failure = select(direction, 98.9999, 103.0001);
                        tick(&mut engine, failure, 160, None, failure, true);
                        close_episode(&mut engine, far_idle, 180, 3);
                        finish(&mut engine, CensorReason::TestEnd, 360, far_idle);
                    } else {
                        finish(&mut engine, CensorReason::TestEnd, 160, retest_contact);
                    }
                }
                "RETURN_SOURCE" => {
                    let source = select(direction, 102.0, 100.0);
                    tick(&mut engine, source, 150, None, source, true);
                    close_episode(&mut engine, far_idle, 180, 3);
                    finish(&mut engine, CensorReason::TestEnd, 360, far_idle);
                }
                "TRANSIT_ADJACENT" => {
                    let destination = select(direction, 120.0, 82.0);
                    tick(&mut engine, destination, 150, None, destination, true);
                    close_episode(&mut engine, far_idle, 180, 3);
                    finish(&mut engine, CensorReason::TestEnd, 360, far_idle);
                }
                _ => panic!("unknown golden scenario {name}"),
            }
        }
    }
    engine.ledger()
}

fn start_break(engine: &mut ExactEngine, start: f64, contact: f64, break_price: f64) {
    tick(engine, start, 0, Some(0), start, true);
    tick(engine, contact, 10, None, contact, true);
    tick(engine, break_price, 20, None, break_price, true);
}

fn accept_and_depart(
    engine: &mut ExactEngine,
    direction: GrammarDirection,
    start: f64,
    contact: f64,
    break_price: f64,
) {
    start_break(engine, start, contact, break_price);
    tick(engine, break_price, 60, Some(1), break_price, true);
    tick(engine, break_price, 120, Some(2), break_price, true);
    let departure = select(direction, 104.5, 97.5);
    tick(engine, departure, 130, None, departure, true);
}

fn close_episode(engine: &mut ExactEngine, price: f64, first_offset: i32, first_bar: i32) {
    for index in 0..3 {
        tick(
            engine,
            price,
            first_offset + index * 60,
            Some(first_bar + index),
            price,
            false,
        );
    }
}

fn tick(
    engine: &mut ExactEngine,
    price: f64,
    offset: i32,
    bar_slot: Option<i32>,
    closed_price: f64,
    source_present: bool,
) {
    engine.market_time = BASE_TIME + u64::try_from(offset).expect("positive offset");
    engine.observe(price, bar_slot, closed_price, source_present);
}

fn finish(engine: &mut ExactEngine, reason: CensorReason, offset: i32, price: f64) {
    engine.market_time = BASE_TIME + u64::try_from(offset).expect("positive offset");
    engine.finalize(reason, price);
}

fn select(direction: GrammarDirection, below: f64, above: f64) -> f64 {
    match direction {
        GrammarDirection::FromBelow => below,
        GrammarDirection::FromAbove => above,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ledgers_and_terminal_accumulators_match_mql5() {
        assert_eq!(verify_exact_golden().unwrap(), (32, 32));
    }
}

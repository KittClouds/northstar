use super::*;

pub(super) fn verify_boundaries() -> Result<usize, String> {
    let mut assertions = 0;
    for direction in [GrammarDirection::FromBelow, GrammarDirection::FromAbove] {
        let start = price(direction, 95.0, 107.0);
        let sign = price(direction, -1.0, 1.0);
        for (candidate, expected, label) in [
            (start + sign * EPS, false, "approach outside"),
            (start, true, "approach exact"),
            (start - sign * EPS, true, "approach inside"),
        ] {
            let mut engine = GrammarEngine::new(direction);
            engine.observe(candidate, Some(0), candidate, true);
            check(engine.active_attempts() == usize::from(expected), label)?;
            assertions += 1;
        }

        for (adjustment, expected, label) in [
            (-EPS, false, "rejection below"),
            (0.0, true, "rejection exact"),
            (EPS, true, "rejection above"),
        ] {
            let mut engine = contacted(direction);
            let reject = price(direction, 98.0 - adjustment, 104.0 + adjustment);
            engine.observe(reject, None, reject, true);
            check(engine.has(EventType::Rejection) == expected, label)?;
            assertions += 1;
        }

        for (adjustment, expected, label) in [
            (-EPS, false, "break inside"),
            (0.0, false, "break exact strict"),
            (EPS, true, "break outside"),
        ] {
            let mut engine = contacted(direction);
            let frontier = price(direction, 103.0 + adjustment, 99.0 - adjustment);
            engine.observe(frontier, None, frontier, true);
            check(engine.has(EventType::Break) == expected, label)?;
            assertions += 1;
        }

        for (bucket, expected, label) in [
            (-1.0, false, "reclaim below"),
            (0.0, true, "reclaim exact"),
            (1.0, true, "reclaim above"),
        ] {
            let mut engine = broken(direction);
            let adjustment = bucket * EPS;
            let reclaim = price(direction, 99.5 - adjustment, 102.5 + adjustment);
            engine.observe(reclaim, Some(1), reclaim, true);
            check(engine.has(EventType::Reclaim) == expected, label)?;
            assertions += 1;
        }

        for (bucket, expected, label) in [
            (-1.0, false, "departure below"),
            (0.0, true, "departure exact"),
            (1.0, true, "departure above"),
        ] {
            let mut engine = accepted(direction);
            let adjustment = bucket * EPS;
            let departure = price(direction, 104.5 + adjustment, 97.5 - adjustment);
            engine.observe(departure, None, departure, true);
            check(engine.has(EventType::Departure) == expected, label)?;
            assertions += 1;
        }

        let mut engine = broken(direction);
        let break_price = price(direction, 103.0 + EPS, 99.0 - EPS);
        engine.observe(break_price, Some(1), break_price, true);
        check(
            engine.has(EventType::ProvisionalAcceptance) && !engine.has(EventType::Acceptance),
            "acceptance N-1 provisional",
        )?;
        assertions += 1;
        let count = engine.events.len();
        engine.observe(break_price, Some(1), break_price, true);
        check(engine.events.len() == count, "duplicate provisional")?;
        assertions += 1;
        engine.observe(break_price, Some(2), break_price, true);
        check(engine.has(EventType::Acceptance), "acceptance N")?;
        assertions += 1;
        let count = engine.events.len();
        engine.observe(break_price, Some(2), break_price, true);
        check(engine.events.len() == count, "duplicate acceptance")?;
        assertions += 1;
        engine.observe(break_price, Some(3), break_price, true);
        check(engine.events.len() == count, "acceptance N+1")?;
        assertions += 1;

        for (bucket, expected, label) in [
            (-1.0, false, "acceptance distance below"),
            (0.0, true, "acceptance distance exact"),
            (1.0, true, "acceptance distance above"),
        ] {
            let mut engine = broken(direction);
            let exact = price(direction, 103.0, 99.0);
            let closed = exact + price(direction, 1.0, -1.0) * bucket * EPS;
            let current = price(direction, 103.0 + EPS, 99.0 - EPS);
            engine.observe(current, Some(1), closed, true);
            check(
                engine.has(EventType::ProvisionalAcceptance) == expected,
                label,
            )?;
            assertions += 1;
        }

        let tracker = EpisodeGapTracker {
            current: 1,
            last_end_bar: 10,
        };
        check(tracker.open(12) == 1, "episode exact gap reuses")?;
        assertions += 1;
        check(tracker.open(13) == 2, "episode gap N+1 splits")?;
        assertions += 1;

        let mut engine = GrammarEngine::new(direction);
        engine.observe(start, Some(0), start, true);
        let count = engine.events.len();
        engine.observe(start, Some(0), start, true);
        check(engine.events.len() == count, "duplicate approach")?;
        assertions += 1;
        let contact = price(direction, 100.0, 102.0);
        engine.observe(contact, None, contact, true);
        let count = engine.events.len();
        engine.observe(contact, None, contact, true);
        check(engine.events.len() == count, "duplicate contact")?;
        assertions += 1;
        let frontier = price(direction, 103.0 + EPS, 99.0 - EPS);
        engine.observe(frontier, None, frontier, true);
        let count = engine.events.len();
        engine.observe(frontier, None, frontier, true);
        check(engine.events.len() == count, "duplicate break")?;
        assertions += 1;
    }
    Ok(assertions)
}

#[derive(Clone, Copy)]
struct EpisodeGapTracker {
    current: u32,
    last_end_bar: i32,
}

impl EpisodeGapTracker {
    fn open(self, bar: i32) -> u32 {
        if bar - self.last_end_bar <= 2 {
            self.current
        } else {
            self.current + 1
        }
    }
}

fn contacted(direction: GrammarDirection) -> GrammarEngine {
    let mut engine = GrammarEngine::new(direction);
    let start = price(direction, 95.0, 107.0);
    let contact = price(direction, 100.0, 102.0);
    engine.observe(start, Some(0), start, true);
    engine.observe(contact, None, contact, true);
    engine
}

fn broken(direction: GrammarDirection) -> GrammarEngine {
    let mut engine = contacted(direction);
    let frontier = price(direction, 103.0 + EPS, 99.0 - EPS);
    engine.observe(frontier, None, frontier, true);
    engine
}

fn accepted(direction: GrammarDirection) -> GrammarEngine {
    let mut engine = broken(direction);
    let frontier = price(direction, 103.0 + EPS, 99.0 - EPS);
    engine.observe(frontier, Some(1), frontier, true);
    engine.observe(frontier, Some(2), frontier, true);
    engine
}

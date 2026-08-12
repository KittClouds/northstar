use crate::data_plane::calendar::{SessionCalendar, SessionSegment};
use crate::data_plane::event::{CanonicalEvent, EventFlags, EventKind};
use crate::data_plane::ids::{DerivationVersion, InstrumentId, JournalSequence};
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub const MINUTE_NS: i64 = 60_000_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(u16)]
pub enum Timeframe {
    M4 = 1,
    M20 = 2,
    H2 = 3,
    H4 = 4,
}

impl Timeframe {
    pub const ALL: [Self; 4] = [Self::M4, Self::M20, Self::H2, Self::H4];

    #[inline]
    pub const fn duration_ns(self) -> i64 {
        match self {
            Self::M4 => 4 * MINUTE_NS,
            Self::M20 => 20 * MINUTE_NS,
            Self::H2 => 120 * MINUTE_NS,
            Self::H4 => 240 * MINUTE_NS,
        }
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct CanonicalBar {
    pub ts_open_ns: i64,
    pub ts_close_ns: i64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub instrument_id: u32,
    pub observation_count: u32,
    pub revision: u32,
    pub derivation_version: u32,
    pub timeframe: u16,
    pub flags: u16,
    pub reserved: u32,
}

impl CanonicalBar {
    #[inline]
    pub const fn is_correction(&self) -> bool {
        self.flags as u32 & EventFlags::CORRECTION != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BarKey {
    instrument: InstrumentId,
    timeframe: Timeframe,
    open_ns: i64,
}

#[derive(Clone, Copy, Debug)]
struct BarState {
    key: BarKey,
    close_ns: i64,
    session_close_ns: i64,
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    first_event_ns: i64,
    last_event_ns: i64,
    first_sequence: u64,
    last_sequence: u64,
    observations: u32,
    revision: u32,
    emitted_revision: Option<u32>,
}

impl BarState {
    fn new(key: BarKey, close_ns: i64, session_close_ns: i64, event: &CanonicalEvent) -> Self {
        let price = event.values[0];
        Self {
            key,
            close_ns,
            session_close_ns,
            open: price,
            high: price,
            low: price,
            close: price,
            first_event_ns: event.header.ts_event_ns,
            last_event_ns: event.header.ts_event_ns,
            first_sequence: event.header.journal_sequence,
            last_sequence: event.header.journal_sequence,
            observations: 1,
            revision: 0,
            emitted_revision: None,
        }
    }

    fn update(&mut self, event: &CanonicalEvent) {
        let price = event.values[0];
        let timestamp = event.header.ts_event_ns;
        let sequence = event.header.journal_sequence;
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        if (timestamp, sequence) < (self.first_event_ns, self.first_sequence) {
            self.first_event_ns = timestamp;
            self.first_sequence = sequence;
            self.open = price;
        }
        if (timestamp, sequence) > (self.last_event_ns, self.last_sequence) {
            self.last_event_ns = timestamp;
            self.last_sequence = sequence;
            self.close = price;
        }
        self.observations = self.observations.saturating_add(1);
        if self.emitted_revision.is_some() {
            self.revision = self.revision.saturating_add(1);
        }
    }

    fn snapshot(&self, derivation_version: DerivationVersion) -> CanonicalBar {
        let mut flags = EventFlags::FINAL;
        if self.revision > 0 {
            flags |= EventFlags::CORRECTION;
        }
        if self.close_ns == self.session_close_ns
            && self.close_ns - self.key.open_ns < self.key.timeframe.duration_ns()
        {
            flags |= EventFlags::PARTIAL_SESSION_END;
        }
        CanonicalBar {
            ts_open_ns: self.key.open_ns,
            ts_close_ns: self.close_ns,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            first_sequence: self.first_sequence,
            last_sequence: self.last_sequence,
            instrument_id: self.key.instrument.get(),
            observation_count: self.observations,
            revision: self.revision,
            derivation_version: derivation_version.get(),
            timeframe: self.key.timeframe as u16,
            flags: flags as u16,
            reserved: 0,
        }
    }
}

pub struct BarAggregator {
    calendar: Arc<SessionCalendar>,
    derivation_version: DerivationVersion,
    states: HashMap<BarKey, BarState>,
}

impl BarAggregator {
    pub fn new(
        calendar: Arc<SessionCalendar>,
        derivation_version: DerivationVersion,
    ) -> Result<Self, BarError> {
        if derivation_version == DerivationVersion::UNKNOWN {
            return Err(BarError::MissingDerivationVersion);
        }
        Ok(Self {
            calendar,
            derivation_version,
            states: HashMap::new(),
        })
    }

    pub fn ingest(&mut self, event: &CanonicalEvent) -> Result<(), BarError> {
        if event.kind().map_err(|_| BarError::WrongEventKind)? != EventKind::IndexValue {
            return Err(BarError::WrongEventKind);
        }
        if event.header.journal_sequence == JournalSequence::UNKNOWN.get() {
            return Err(BarError::UncommittedEvent);
        }
        let instrument = event.instrument_id();
        let Some(session) = self.calendar.session_at(
            instrument,
            event.header.ts_event_ns,
            SessionSegment::MainReference,
        ) else {
            return Err(BarError::OutsideReferenceSession {
                instrument,
                timestamp_ns: event.header.ts_event_ns,
            });
        };

        for timeframe in Timeframe::ALL {
            let duration = timeframe.duration_ns();
            let elapsed = event.header.ts_event_ns - session.open_ns;
            let open_ns = session.open_ns + (elapsed / duration) * duration;
            let close_ns = (open_ns + duration).min(session.close_ns);
            let key = BarKey {
                instrument,
                timeframe,
                open_ns,
            };
            match self.states.get_mut(&key) {
                Some(state) => state.update(event),
                None => {
                    self.states
                        .insert(key, BarState::new(key, close_ns, session.close_ns, event));
                }
            }
        }
        Ok(())
    }

    /// Emits new and revised bars in a deterministic order. No empty interval
    /// is synthesized.
    pub fn advance_watermark(&mut self, watermark_ns: i64, output: &mut Vec<CanonicalBar>) {
        let mut keys: Vec<_> = self
            .states
            .iter()
            .filter_map(|(key, state)| {
                let changed = state.emitted_revision != Some(state.revision);
                (state.close_ns <= watermark_ns && changed).then_some(*key)
            })
            .collect();
        keys.sort_unstable_by_key(|key| (key.open_ns, key.instrument, key.timeframe));
        output.reserve(keys.len());
        for key in keys {
            let state = self.states.get_mut(&key).expect("selected state exists");
            output.push(state.snapshot(self.derivation_version));
            state.emitted_revision = Some(state.revision);
        }
    }

    pub fn tracked_bars(&self) -> usize {
        self.states.len()
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum BarError {
    #[error("bar derivation version must be non-zero")]
    MissingDerivationVersion,
    #[error("bar aggregator accepts only IndexValue events")]
    WrongEventKind,
    #[error("canonical event must be durably committed before derivation")]
    UncommittedEvent,
    #[error("event for {instrument:?} at {timestamp_ns} is outside the reference session")]
    OutsideReferenceSession {
        instrument: InstrumentId,
        timestamp_ns: i64,
    },
}

const _: () = assert!(std::mem::size_of::<CanonicalBar>() == 88);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::calendar::Session;
    use crate::data_plane::event::TimeQuality;
    use crate::data_plane::ids::{
        BatchId, CalendarId, CatalogVersion, ReceiptId, SourceId, StreamId,
    };
    use crate::data_plane::journal::JournalWriter;

    fn calendar(close_ns: i64) -> Arc<SessionCalendar> {
        Arc::new(
            SessionCalendar::new(
                CalendarId(1),
                CatalogVersion(1),
                vec![Session {
                    instrument: InstrumentId(1),
                    open_ns: 0,
                    close_ns,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                }],
            )
            .unwrap(),
        )
    }

    fn committed_events(values: &[(i64, i64)]) -> Vec<CanonicalEvent> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.nsj");
        let mut writer = JournalWriter::create(&path, 0).unwrap();
        let mut batch = crate::data_plane::event::CanonicalBatch::new(BatchId(1));
        for (id, (timestamp, price)) in values.iter().copied().enumerate() {
            batch.push(
                CanonicalEvent::index_value(
                    SourceId(1),
                    StreamId(1),
                    InstrumentId(1),
                    ReceiptId(1),
                    id as u64 + 1,
                    timestamp,
                    timestamp + 1,
                    price,
                    TimeQuality::ObservedLive,
                )
                .unwrap(),
            );
        }
        writer.append_batch(&mut batch).unwrap();
        batch.events.into_vec()
    }

    #[test]
    fn session_anchored_m4_bars_are_deterministic() {
        let events = committed_events(&[
            (30_000_000_000, 100),
            (3 * MINUTE_NS, 104),
            (MINUTE_NS, 98),
            (4 * MINUTE_NS + 1, 110),
        ]);
        let mut aggregator =
            BarAggregator::new(calendar(10 * MINUTE_NS), DerivationVersion(1)).unwrap();
        for event in &events {
            aggregator.ingest(event).unwrap();
        }
        let mut bars = Vec::new();
        aggregator.advance_watermark(4 * MINUTE_NS, &mut bars);
        let m4 = bars
            .iter()
            .find(|bar| bar.timeframe == Timeframe::M4 as u16)
            .unwrap();
        assert_eq!((m4.open, m4.high, m4.low, m4.close), (100, 104, 98, 104));
        assert_eq!(m4.observation_count, 3);
    }

    #[test]
    fn late_values_emit_revisions_without_fabricating_gaps() {
        let mut aggregator =
            BarAggregator::new(calendar(12 * MINUTE_NS), DerivationVersion(1)).unwrap();
        let initial = committed_events(&[(MINUTE_NS, 100)]);
        aggregator.ingest(&initial[0]).unwrap();
        let mut bars = Vec::new();
        aggregator.advance_watermark(4 * MINUTE_NS, &mut bars);
        let first_count = bars.len();

        let late = committed_events(&[(2 * MINUTE_NS, 105)]);
        aggregator.ingest(&late[0]).unwrap();
        aggregator.advance_watermark(4 * MINUTE_NS, &mut bars);
        assert!(bars.len() > first_count);
        assert!(bars[first_count..].iter().any(CanonicalBar::is_correction));
        assert_eq!(aggregator.tracked_bars(), 4);
    }
}

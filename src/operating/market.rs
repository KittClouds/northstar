use super::snapshot::{
    Availability, BarSeriesSnapshot, DeskSnapshot, OperatingMode, ValueMeta, BAR_SERIES_BLOCK_LEN,
};
use crate::data_plane::bars::{BarAggregator, BarError, CanonicalBar, Timeframe};
use crate::data_plane::calendar::SessionCalendar;
#[cfg(test)]
use crate::data_plane::event::CanonicalEvent;
use crate::data_plane::event::EventKind;
use crate::data_plane::ids::{DerivationVersion, InstrumentId, JournalSequence};
#[cfg(test)]
use crate::data_plane::ids::{SourceId, StreamId};
use crate::data_plane::replay::EventBatchRef;
use memchr::memchr;
use std::array;
use std::sync::Arc;
use thiserror::Error;

#[path = "market_event_projection.rs"]
mod event_projection;
use event_projection::{
    availability_for, availability_for_event, decode_timeframe, external_bar, timeframe_slot,
    value_meta,
};
#[path = "market_bridge.rs"]
mod bridge;
pub use bridge::MarketSnapshotBridge;

const TIMEFRAME_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceRole {
    ExactBenchmark,
    ContextProxy,
}

#[derive(Clone, Debug)]
pub struct MarketInstrument {
    pub id: InstrumentId,
    pub display_name: Arc<str>,
    pub reference_symbol: Arc<str>,
    pub reference_role: ReferenceRole,
    /// Fixed-point multiplier used by canonical values, for example 10_000.
    pub price_scale: i64,
}

impl MarketInstrument {
    pub fn new(
        id: InstrumentId,
        display_name: impl Into<Arc<str>>,
        reference_symbol: impl Into<Arc<str>>,
        price_scale: i64,
    ) -> Result<Self, MarketProjectorError> {
        Self::new_with_role(
            id,
            display_name,
            reference_symbol,
            price_scale,
            ReferenceRole::ExactBenchmark,
        )
    }

    pub fn new_with_role(
        id: InstrumentId,
        display_name: impl Into<Arc<str>>,
        reference_symbol: impl Into<Arc<str>>,
        price_scale: i64,
        reference_role: ReferenceRole,
    ) -> Result<Self, MarketProjectorError> {
        let display_name = display_name.into();
        let reference_symbol = reference_symbol.into();
        if id == InstrumentId::UNKNOWN || display_name.is_empty() {
            return Err(MarketProjectorError::MissingInstrumentIdentity);
        }
        if price_scale <= 0 {
            return Err(MarketProjectorError::InvalidPriceScale(price_scale));
        }
        if memchr(b':', reference_symbol.as_bytes()).is_none() {
            return Err(MarketProjectorError::InvalidReferenceSymbol(
                reference_symbol,
            ));
        }
        Ok(Self {
            id,
            display_name,
            reference_symbol,
            reference_role,
            price_scale,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ReferenceValueSnapshot {
    pub value_scaled: i64,
    pub price_scale: i64,
    pub meta: ValueMeta,
}

impl ReferenceValueSnapshot {
    fn awaiting(price_scale: i64) -> Self {
        Self {
            value_scaled: 0,
            price_scale,
            meta: ValueMeta::unavailable(Availability::AwaitingFirstReceipt),
        }
    }

    #[inline]
    pub fn value_f64(self) -> Option<f64> {
        self.meta
            .availability
            .has_value()
            .then_some(self.value_scaled as f64 / self.price_scale as f64)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VenueQuoteSnapshot {
    pub bid_scaled: i64,
    pub ask_scaled: i64,
    pub midpoint_scaled: i64,
    pub price_scale: i64,
    pub meta: ValueMeta,
}

impl VenueQuoteSnapshot {
    fn not_configured(price_scale: i64) -> Self {
        Self {
            bid_scaled: 0,
            ask_scaled: 0,
            midpoint_scaled: 0,
            price_scale,
            meta: ValueMeta::unavailable(Availability::NotConfigured),
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexMarketSnapshot {
    pub instrument: Arc<MarketInstrument>,
    pub reference: ReferenceValueSnapshot,
    pub venue: VenueQuoteSnapshot,
    pub series: [Arc<BarSeriesSnapshot>; TIMEFRAME_COUNT],
}

impl IndexMarketSnapshot {
    #[inline]
    pub fn series(&self, timeframe: Timeframe) -> &BarSeriesSnapshot {
        &self.series[timeframe_slot(timeframe)]
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarketProjectorHealth {
    pub batches: u64,
    pub events_seen: u64,
    pub reference_values: u64,
    pub venue_quotes: u64,
    pub external_bars: u64,
    pub ignored_events: u64,
    pub unknown_instruments: u64,
    pub invalid_events: u64,
    pub bar_rejections: u64,
    pub last_sequence: JournalSequence,
}

struct BarTimeline {
    timeframe: Timeframe,
    blocks: Vec<Arc<[CanonicalBar]>>,
    tail: Vec<CanonicalBar>,
    len: usize,
    revision: u64,
    cached: Arc<BarSeriesSnapshot>,
}

impl BarTimeline {
    fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            blocks: Vec::new(),
            tail: Vec::with_capacity(BAR_SERIES_BLOCK_LEN),
            len: 0,
            revision: 0,
            cached: Arc::new(BarSeriesSnapshot::empty(timeframe)),
        }
    }

    fn upsert(&mut self, bar: CanonicalBar) -> bool {
        let last_open = self
            .tail
            .last()
            .or_else(|| self.blocks.last().and_then(|block| block.last()))
            .map(|last| last.ts_open_ns);
        match last_open {
            None => self.append(bar),
            Some(last) if bar.ts_open_ns > last => self.append(bar),
            Some(last) if bar.ts_open_ns == last => self.replace_existing(bar),
            Some(_) => self.upsert_historical(bar),
        }
    }

    fn append(&mut self, bar: CanonicalBar) -> bool {
        self.tail.push(bar);
        self.len += 1;
        if self.tail.len() == BAR_SERIES_BLOCK_LEN {
            let sealed =
                std::mem::replace(&mut self.tail, Vec::with_capacity(BAR_SERIES_BLOCK_LEN));
            self.blocks.push(sealed.into());
        }
        self.refresh();
        true
    }

    fn replace_existing(&mut self, bar: CanonicalBar) -> bool {
        if let Some(last) = self.tail.last_mut() {
            if last.ts_open_ns == bar.ts_open_ns {
                if same_bar(last, &bar) {
                    return false;
                }
                *last = bar;
                self.refresh();
                return true;
            }
        }
        self.upsert_historical(bar)
    }

    #[cold]
    fn upsert_historical(&mut self, bar: CanonicalBar) -> bool {
        let mut values = Vec::with_capacity(self.len.saturating_add(1));
        for block in &self.blocks {
            values.extend_from_slice(block);
        }
        values.extend_from_slice(&self.tail);

        match values.binary_search_by_key(&bar.ts_open_ns, |value| value.ts_open_ns) {
            Ok(index) => {
                if same_bar(&values[index], &bar) {
                    return false;
                }
                values[index] = bar;
            }
            Err(index) => values.insert(index, bar),
        }

        self.blocks.clear();
        self.tail.clear();
        let sealed_len = values.len() / BAR_SERIES_BLOCK_LEN * BAR_SERIES_BLOCK_LEN;
        for chunk in values[..sealed_len].chunks_exact(BAR_SERIES_BLOCK_LEN) {
            self.blocks.push(Arc::from(chunk));
        }
        self.tail.extend_from_slice(&values[sealed_len..]);
        self.len = values.len();
        self.refresh();
        true
    }

    fn refresh(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.cached = Arc::new(BarSeriesSnapshot {
            timeframe: self.timeframe,
            blocks: Arc::from(self.blocks.clone()),
            tail: Arc::from(self.tail.clone()),
            len: self.len,
            revision: self.revision,
        });
    }
}

fn same_bar(left: &CanonicalBar, right: &CanonicalBar) -> bool {
    bytemuck::bytes_of(left) == bytemuck::bytes_of(right)
}

struct IndexState {
    instrument: Arc<MarketInstrument>,
    reference: ReferenceValueSnapshot,
    venue: VenueQuoteSnapshot,
    timelines: [BarTimeline; TIMEFRAME_COUNT],
    cached: Arc<IndexMarketSnapshot>,
}

impl IndexState {
    fn new(instrument: Arc<MarketInstrument>) -> Self {
        let reference = ReferenceValueSnapshot::awaiting(instrument.price_scale);
        let venue = VenueQuoteSnapshot::not_configured(instrument.price_scale);
        let timelines = array::from_fn(|slot| BarTimeline::new(Timeframe::ALL[slot]));
        let cached = Arc::new(IndexMarketSnapshot {
            instrument: Arc::clone(&instrument),
            reference,
            venue,
            series: array::from_fn(|slot| Arc::clone(&timelines[slot].cached)),
        });
        Self {
            instrument,
            reference,
            venue,
            timelines,
            cached,
        }
    }

    fn refresh(&mut self) {
        self.cached = Arc::new(IndexMarketSnapshot {
            instrument: Arc::clone(&self.instrument),
            reference: self.reference,
            venue: self.venue,
            series: array::from_fn(|slot| Arc::clone(&self.timelines[slot].cached)),
        });
    }
}

pub struct MarketProjector {
    mode: OperatingMode,
    aggregator: BarAggregator,
    states: Box<[IndexState]>,
    generation: u64,
    published_ns: i64,
    health: MarketProjectorHealth,
    emitted_bars: Vec<CanonicalBar>,
}

impl MarketProjector {
    pub fn new(
        mode: OperatingMode,
        calendar: Arc<SessionCalendar>,
        derivation_version: DerivationVersion,
        mut instruments: Vec<MarketInstrument>,
    ) -> Result<Self, MarketProjectorError> {
        if instruments.is_empty() {
            return Err(MarketProjectorError::EmptyInstrumentSet);
        }
        if instruments.len() > 64 {
            return Err(MarketProjectorError::TooManyInstruments(instruments.len()));
        }
        instruments.sort_unstable_by_key(|instrument| instrument.id);
        for pair in instruments.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(MarketProjectorError::DuplicateInstrument(pair[0].id));
            }
        }
        let states = instruments
            .into_iter()
            .map(|instrument| IndexState::new(Arc::new(instrument)))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            mode,
            aggregator: BarAggregator::new(calendar, derivation_version)?,
            states,
            generation: 0,
            published_ns: 0,
            health: MarketProjectorHealth::default(),
            emitted_bars: Vec::with_capacity(32),
        })
    }

    pub fn apply_batch(&mut self, batch: EventBatchRef<'_>) -> Option<Arc<DeskSnapshot>> {
        self.health.batches = self.health.batches.saturating_add(1);
        let mut changed = 0u64;
        let mut watermark_ns = i64::MIN;
        let mut last_sequence = self.health.last_sequence;
        let mut batch_availability = None;

        for event in batch.events {
            self.health.events_seen = self.health.events_seen.saturating_add(1);
            last_sequence = last_sequence.max(event.journal_sequence());
            let kind = match event.kind() {
                Ok(kind) => kind,
                Err(_) => {
                    self.health.invalid_events = self.health.invalid_events.saturating_add(1);
                    continue;
                }
            };
            if !matches!(
                kind,
                EventKind::IndexValue | EventKind::VenueQuote | EventKind::ExternalBar
            ) {
                self.health.ignored_events = self.health.ignored_events.saturating_add(1);
                continue;
            }
            let Some(slot) = self.slot(event.instrument_id()) else {
                self.health.unknown_instruments = self.health.unknown_instruments.saturating_add(1);
                continue;
            };
            let availability = availability_for_event(self.mode, event);
            batch_availability = Some(match (batch_availability, availability) {
                (Some(Availability::Live), _) | (_, Availability::Live) => Availability::Live,
                (_, value) => value,
            });
            let meta = value_meta(event, availability);
            match kind {
                EventKind::IndexValue => {
                    self.states[slot].reference = ReferenceValueSnapshot {
                        value_scaled: event.values[0],
                        price_scale: self.states[slot].instrument.price_scale,
                        meta,
                    };
                    self.health.reference_values = self.health.reference_values.saturating_add(1);
                    watermark_ns = watermark_ns.max(event.header.ts_event_ns);
                    if self.aggregator.ingest(event).is_err() {
                        self.health.bar_rejections = self.health.bar_rejections.saturating_add(1);
                    }
                }
                EventKind::VenueQuote => {
                    self.states[slot].venue = VenueQuoteSnapshot {
                        bid_scaled: event.values[0],
                        ask_scaled: event.values[1],
                        midpoint_scaled: event.values[2],
                        price_scale: self.states[slot].instrument.price_scale,
                        meta,
                    };
                    self.health.venue_quotes = self.health.venue_quotes.saturating_add(1);
                }
                EventKind::ExternalBar => {
                    let Ok(timeframe_code) = u16::try_from(event.values[5]) else {
                        self.health.invalid_events = self.health.invalid_events.saturating_add(1);
                        continue;
                    };
                    let Some(timeframe) = decode_timeframe(timeframe_code) else {
                        self.health.invalid_events = self.health.invalid_events.saturating_add(1);
                        continue;
                    };
                    let bar = external_bar(event, timeframe);
                    if self.states[slot].timelines[timeframe_slot(timeframe)].upsert(bar) {
                        changed |= 1 << slot;
                    }
                    let reference = &mut self.states[slot].reference;
                    if !reference.meta.availability.has_value()
                        || event.header.ts_event_ns >= reference.meta.ts_event_ns
                    {
                        *reference = ReferenceValueSnapshot {
                            value_scaled: event.values[3],
                            price_scale: self.states[slot].instrument.price_scale,
                            meta,
                        };
                        changed |= 1 << slot;
                    }
                    self.health.external_bars = self.health.external_bars.saturating_add(1);
                }
                _ => unreachable!(),
            }
            self.published_ns = self.published_ns.max(event.header.ts_received_ns);
            changed |= 1 << slot;
        }

        if watermark_ns != i64::MIN {
            self.emitted_bars.clear();
            self.aggregator
                .advance_watermark(watermark_ns, &mut self.emitted_bars);
            for bar in self.emitted_bars.iter().copied() {
                let Some(slot) = self.slot(InstrumentId(bar.instrument_id)) else {
                    continue;
                };
                let Some(timeframe) = decode_timeframe(bar.timeframe) else {
                    self.health.invalid_events = self.health.invalid_events.saturating_add(1);
                    continue;
                };
                if self.states[slot].timelines[timeframe_slot(timeframe)].upsert(bar) {
                    changed |= 1 << slot;
                }
            }
        }

        self.health.last_sequence = last_sequence;
        if changed == 0 {
            return None;
        }
        for (slot, state) in self.states.iter_mut().enumerate() {
            if changed & (1 << slot) != 0 {
                state.refresh();
            }
        }
        self.generation = self.generation.saturating_add(1);
        Some(Arc::new(DeskSnapshot {
            generation: self.generation,
            published_ns: self.published_ns,
            last_sequence,
            availability: batch_availability.unwrap_or_else(|| availability_for(self.mode)),
            instruments: self
                .states
                .iter()
                .map(|state| Arc::clone(&state.cached))
                .collect::<Vec<_>>()
                .into(),
        }))
    }

    #[inline]
    pub const fn health(&self) -> MarketProjectorHealth {
        self.health
    }

    /// Cheap immutable view for startup, transport-failure, and restart
    /// publication. Instrument arcs are reused; no chart history is copied.
    pub fn current_snapshot(&self, availability: Availability) -> Arc<DeskSnapshot> {
        Arc::new(DeskSnapshot {
            generation: self.generation,
            published_ns: self.published_ns,
            last_sequence: self.health.last_sequence,
            availability,
            instruments: self
                .states
                .iter()
                .map(|state| Arc::clone(&state.cached))
                .collect::<Vec<_>>()
                .into(),
        })
    }

    fn slot(&self, instrument: InstrumentId) -> Option<usize> {
        self.states
            .binary_search_by_key(&instrument, |state| state.instrument.id)
            .ok()
    }
}

#[derive(Debug, Error)]
pub enum MarketProjectorError {
    #[error("market projector requires at least one instrument")]
    EmptyInstrumentSet,
    #[error("market projector supports at most 64 dense instruments, got {0}")]
    TooManyInstruments(usize),
    #[error("instrument identity and display name must be present")]
    MissingInstrumentIdentity,
    #[error("price scale must be positive, got {0}")]
    InvalidPriceScale(i64),
    #[error("reference symbol must be namespaced: {0}")]
    InvalidReferenceSymbol(Arc<str>),
    #[error("duplicate instrument {0:?}")]
    DuplicateInstrument(InstrumentId),
    #[error(transparent)]
    Bar(#[from] BarError),
}

#[cfg(test)]
mod tests {
    use super::super::snapshot::{OfficeSnapshot, OfficeSnapshotPort, SnapshotStore};
    use super::*;
    use crate::data_plane::calendar::{Session, SessionSegment};
    use crate::data_plane::event::{CanonicalBatch, TimeQuality};
    use crate::data_plane::ids::{instruments, BatchId, CalendarId, CatalogVersion, ReceiptId};
    use crate::data_plane::journal::{JournalWriter, MappedJournal};
    use crate::data_plane::replay::{LivePublisher, ReplayConfig, ReplayEngine};

    const MINUTE: i64 = 60_000_000_000;

    fn calendar() -> Arc<SessionCalendar> {
        Arc::new(
            SessionCalendar::new(
                CalendarId(1),
                CatalogVersion(1),
                vec![Session {
                    instrument: instruments::US100,
                    open_ns: 0,
                    close_ns: 30 * MINUTE,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                }],
            )
            .unwrap(),
        )
    }

    fn instrument() -> MarketInstrument {
        MarketInstrument::new(instruments::US100, "NASDAQ 100", "I:NDX", 10_000).unwrap()
    }

    fn event(id: u64, minute: i64, value: i64) -> CanonicalEvent {
        let timestamp = minute * MINUTE;
        CanonicalEvent::index_value(
            SourceId(1),
            StreamId(1),
            instruments::US100,
            ReceiptId(1),
            id,
            timestamp,
            timestamp + 1,
            value,
            TimeQuality::ObservedLive,
        )
        .unwrap()
    }

    fn projector(mode: OperatingMode) -> MarketProjector {
        MarketProjector::new(mode, calendar(), DerivationVersion(1), vec![instrument()]).unwrap()
    }

    #[test]
    fn constructor_rejects_ambiguous_provider_symbols() {
        assert!(matches!(
            MarketInstrument::new(instruments::US100, "NASDAQ 100", "NDX", 10_000),
            Err(MarketProjectorError::InvalidReferenceSymbol(_))
        ));
    }

    #[test]
    fn chunked_series_bounds_hot_append_copying() {
        let mut timeline = BarTimeline::new(Timeframe::M4);
        for index in 0..300 {
            let bar = CanonicalBar {
                ts_open_ns: index,
                ts_close_ns: index + 1,
                open: index,
                high: index,
                low: index,
                close: index,
                first_sequence: index as u64 + 1,
                last_sequence: index as u64 + 1,
                instrument_id: 1,
                observation_count: 1,
                revision: 0,
                derivation_version: 1,
                timeframe: Timeframe::M4 as u16,
                flags: 0,
                reserved: 0,
            };
            assert!(timeline.upsert(bar));
        }
        assert_eq!(timeline.cached.blocks.len(), 1);
        assert_eq!(timeline.cached.blocks[0].len(), BAR_SERIES_BLOCK_LEN);
        assert_eq!(timeline.cached.tail.len(), 44);
        assert_eq!(timeline.cached.len, 300);
        assert_eq!(timeline.cached.get(255).unwrap().open, 255);
        assert_eq!(timeline.cached.get(256).unwrap().open, 256);
        assert_eq!(timeline.cached.last().unwrap().open, 299);
        assert!(timeline.cached.get(300).is_none());
    }

    #[test]
    fn historical_insert_is_cold_but_ordered_and_lossless() {
        let mut timeline = BarTimeline::new(Timeframe::M4);
        for open in [10, 30, 20] {
            let bar = CanonicalBar {
                ts_open_ns: open,
                ts_close_ns: open + 1,
                open,
                high: open,
                low: open,
                close: open,
                first_sequence: open as u64,
                last_sequence: open as u64,
                instrument_id: 1,
                observation_count: 1,
                revision: 0,
                derivation_version: 1,
                timeframe: Timeframe::M4 as u16,
                flags: 0,
                reserved: 0,
            };
            assert!(timeline.upsert(bar));
        }
        assert_eq!(
            timeline
                .cached
                .iter()
                .map(|bar| bar.ts_open_ns)
                .collect::<Vec<_>>(),
            [10, 20, 30]
        );
    }

    #[test]
    fn live_and_replay_publish_identical_market_values_and_bars() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("market.nsj");
        let writer = JournalWriter::create(&path, 0).unwrap();
        let (live_store, live_snapshot_publisher) =
            SnapshotStore::new(OfficeSnapshot::empty(OperatingMode::LiveData));
        let live_bridge =
            MarketSnapshotBridge::new(projector(OperatingMode::LiveData), live_snapshot_publisher);
        let mut live = LivePublisher::new(writer, live_bridge);
        let mut batch = CanonicalBatch::new(BatchId(1));
        batch.push(event(1, 1, 200_000_000));
        batch.push(event(2, 5, 200_100_000));
        live.commit_and_publish(&mut batch).unwrap();
        let (writer, _) = live.into_parts();
        drop(writer);

        let live_snapshot = live_store.current();
        let mapped = MappedJournal::open(&path).unwrap();
        let (replay_store, replay_snapshot_publisher) =
            SnapshotStore::new(OfficeSnapshot::empty(OperatingMode::OfflineReplay));
        let mut replay_bridge = MarketSnapshotBridge::new(
            projector(OperatingMode::OfflineReplay),
            replay_snapshot_publisher,
        );
        ReplayEngine::new(&mapped).run(ReplayConfig::default(), &mut replay_bridge);
        let replay_snapshot = replay_store.current();

        let live_index = &live_snapshot.desk.instruments[0];
        let replay_index = &replay_snapshot.desk.instruments[0];
        assert_eq!(
            live_index.reference.value_scaled,
            replay_index.reference.value_scaled
        );
        assert_eq!(live_index.reference.value_f64(), Some(20_010.0));
        let live_m4 = live_index.series(Timeframe::M4);
        let replay_m4 = replay_index.series(Timeframe::M4);
        assert_eq!(live_m4.len, 1);
        assert_eq!(live_m4.len, replay_m4.len);
        assert_eq!(
            bytemuck::cast_slice::<CanonicalBar, u8>(&live_m4.iter().copied().collect::<Vec<_>>()),
            bytemuck::cast_slice::<CanonicalBar, u8>(
                &replay_m4.iter().copied().collect::<Vec<_>>()
            )
        );
        assert_eq!(live_index.reference.meta.availability, Availability::Live);
        assert_eq!(
            replay_index.reference.meta.availability,
            Availability::Replaying
        );
    }

    #[test]
    fn intrabar_value_updates_reuse_unchanged_series_arcs() {
        let mut projector = projector(OperatingMode::LiveData);
        let mut first = event(1, 1, 200_000_000);
        first.header.journal_sequence = 1;
        let first_events = [first];
        let first_snapshot = projector
            .apply_batch(EventBatchRef {
                id: BatchId(1),
                first_sequence: JournalSequence(1),
                events: &first_events,
            })
            .unwrap();

        let mut second = event(2, 2, 200_050_000);
        second.header.journal_sequence = 2;
        let second_events = [second];
        let second_snapshot = projector
            .apply_batch(EventBatchRef {
                id: BatchId(2),
                first_sequence: JournalSequence(2),
                events: &second_events,
            })
            .unwrap();

        let first_index = &first_snapshot.instruments[0];
        let second_index = &second_snapshot.instruments[0];
        assert!(!Arc::ptr_eq(first_index, second_index));
        for slot in 0..TIMEFRAME_COUNT {
            assert!(Arc::ptr_eq(
                &first_index.series[slot],
                &second_index.series[slot]
            ));
        }
    }

    #[test]
    fn external_aggregate_bar_populates_chart_without_claiming_live() {
        let mut projector = projector(OperatingMode::LiveData);
        let mut bar = CanonicalEvent::external_bar(
            SourceId(10),
            StreamId(2),
            instruments::US100,
            ReceiptId(1),
            99,
            0,
            4 * MINUTE,
            5 * MINUTE,
            200_000_000,
            200_200_000,
            199_900_000,
            200_100_000,
            Timeframe::M4 as u16,
            TimeQuality::EstimatedHistorical,
        )
        .unwrap();
        bar.header.journal_sequence = 1;
        let events = [bar];
        let snapshot = projector
            .apply_batch(EventBatchRef {
                id: BatchId(1),
                first_sequence: JournalSequence(1),
                events: &events,
            })
            .unwrap();
        let index = &snapshot.instruments[0];
        assert_eq!(snapshot.availability, Availability::Stale);
        assert_eq!(index.reference.value_f64(), Some(20_010.0));
        assert_eq!(index.reference.meta.availability, Availability::Stale);
        assert_eq!(index.series(Timeframe::M4).len, 1);
        assert_eq!(
            index.series(Timeframe::M4).last().unwrap().high,
            200_200_000
        );
        assert_eq!(projector.health().external_bars, 1);
    }
}

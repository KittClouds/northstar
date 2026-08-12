use super::producers::ProducerError;
use super::{
    locate_market, project_chart_structure, ChartStructureSnapshot, LevelBook, LevelBookError,
    NodeBook, NodeMergeSpec, OpeningRangeProducer, OpeningRangeSpec, PriceSpace,
    RangeProjectionProducer, RangeProjectionSpec, SessionStructureProducer,
    StructuralGraphSnapshot, StructuralMarketSnapshot, StructuralMutation, StructuralQuality,
};
use crate::data_plane::bars::{CanonicalBar, Timeframe};
use crate::data_plane::calendar::{SessionCalendar, SessionSegment};
use crate::data_plane::ids::{DerivationVersion, InstrumentId, JournalSequence};
use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;

const ATR_PERIOD: usize = 14;

#[derive(Clone, Debug)]
pub struct StructuralUpdate {
    pub snapshot: Arc<StructuralMarketSnapshot>,
    pub chart: Arc<ChartStructureSnapshot>,
    pub mutations: Arc<[StructuralMutation]>,
    pub geometry_changed: bool,
}

struct AtrTracker {
    ranges: VecDeque<i64>,
    sum: i128,
    previous_close: Option<i64>,
}

impl AtrTracker {
    fn new() -> Self {
        Self {
            ranges: VecDeque::with_capacity(ATR_PERIOD + 1),
            sum: 0,
            previous_close: None,
        }
    }

    fn push(&mut self, bar: &CanonicalBar) -> i64 {
        let true_range = self.previous_close.map_or(bar.high - bar.low, |close| {
            (bar.high - bar.low)
                .max((bar.high - close).abs())
                .max((bar.low - close).abs())
        });
        self.previous_close = Some(bar.close);
        self.ranges.push_back(true_range);
        self.sum += true_range as i128;
        if self.ranges.len() > ATR_PERIOD {
            if let Some(expired) = self.ranges.pop_front() {
                self.sum -= expired as i128;
            }
        }
        (self.sum / self.ranges.len().max(1) as i128) as i64
    }
}

#[derive(Clone, Copy)]
struct BarContinuity {
    bar_close_ns: i64,
    session_open_ns: i64,
    session_close_ns: i64,
}

pub struct StructuralEngine {
    instrument: InstrumentId,
    price_space: PriceSpace,
    derivation_version: DerivationVersion,
    calendar: Arc<SessionCalendar>,
    session: SessionStructureProducer,
    opening_range: OpeningRangeProducer,
    range_projection: RangeProjectionProducer,
    level_book: LevelBook,
    node_book: NodeBook,
    merge_spec: NodeMergeSpec,
    tick_size: i64,
    noise_floor: i64,
    atr: AtrTracker,
    atr_value: i64,
    mutations: Vec<StructuralMutation>,
    chart: Arc<ChartStructureSnapshot>,
    graph: Arc<StructuralGraphSnapshot>,
    continuity: Option<BarContinuity>,
    gaps_present: bool,
}

impl StructuralEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument: InstrumentId,
        price_space: PriceSpace,
        calendar: Arc<SessionCalendar>,
        derivation_version: DerivationVersion,
        tick_size: i64,
        noise_floor: i64,
        opening_range_spec: OpeningRangeSpec,
        range_projection_spec: RangeProjectionSpec,
        merge_spec: NodeMergeSpec,
    ) -> Result<Self, StructuralEngineError> {
        if instrument == InstrumentId::UNKNOWN
            || instrument.get() > u16::MAX as u32
            || derivation_version == DerivationVersion::UNKNOWN
            || tick_size <= 0
            || noise_floor < 0
        {
            return Err(StructuralEngineError::InvalidConfiguration);
        }
        let session = SessionStructureProducer::new(
            instrument,
            price_space,
            Arc::clone(&calendar),
            derivation_version,
        );
        let opening_range = OpeningRangeProducer::new(
            instrument,
            price_space,
            Arc::clone(&calendar),
            derivation_version,
            opening_range_spec,
        )?;
        let range_projection = RangeProjectionProducer::new(
            instrument,
            price_space,
            calendar.id,
            derivation_version,
            range_projection_spec,
        )?;
        let graph = Arc::new(StructuralGraphSnapshot {
            generation: 0,
            nodes: Arc::from([]),
            corridors: Arc::from([]),
            genealogy: Arc::from([]),
        });
        let chart = Arc::new(project_chart_structure(
            instrument,
            0,
            &[],
            Arc::clone(&graph),
        ));
        Ok(Self {
            instrument,
            price_space,
            derivation_version,
            calendar,
            session,
            opening_range,
            range_projection,
            level_book: LevelBook::new(),
            node_book: NodeBook::new(),
            merge_spec,
            tick_size,
            noise_floor,
            atr: AtrTracker::new(),
            atr_value: 0,
            mutations: Vec::with_capacity(32),
            chart,
            graph,
            continuity: None,
            gaps_present: false,
        })
    }

    pub fn on_bar(
        &mut self,
        bar: &CanonicalBar,
    ) -> Result<StructuralUpdate, StructuralEngineError> {
        self.validate_bar(bar)?;
        let session = self
            .calendar
            .session_at(
                self.instrument,
                bar.ts_open_ns,
                SessionSegment::MainReference,
            )
            .ok_or(ProducerError::OutsideSession {
                bar_open_ns: bar.ts_open_ns,
            })?;
        self.mutations.clear();
        self.session.on_bar(bar, &mut self.mutations)?;
        self.opening_range.on_bar(bar, &mut self.mutations)?;
        self.range_projection.on_bar(bar, &mut self.mutations)?;

        self.update_continuity(bar, session.open_ns, session.close_ns);
        self.atr_value = self.atr.push(bar);
        let geometry_changed = !self.mutations.is_empty();
        if geometry_changed {
            let generation = self.level_book.generation().saturating_add(1);
            self.level_book.apply_batch(generation, &self.mutations)?;
            let levels = self.level_book.active_snapshot();
            self.graph = self.node_book.rebuild(
                &levels,
                self.instrument,
                self.price_space,
                bar.ts_close_ns,
                self.atr_value,
                self.noise_floor,
                self.tick_size,
                self.merge_spec,
            );
            self.chart = Arc::new(project_chart_structure(
                self.instrument,
                generation,
                &levels,
                Arc::clone(&self.graph),
            ));
        }
        let levels = self.level_book.active_snapshot();
        let snapshot = self.snapshot(
            bar.close,
            bar.ts_close_ns,
            JournalSequence(bar.last_sequence),
            levels,
        );
        Ok(StructuralUpdate {
            snapshot: Arc::new(snapshot),
            chart: Arc::clone(&self.chart),
            mutations: self.mutations.clone().into(),
            geometry_changed,
        })
    }

    /// Fast path for live quote/index updates. It performs only a graph lookup
    /// and shares every immutable structural allocation.
    pub fn locate_price(
        &mut self,
        price: i64,
        as_of_ns: i64,
        sequence: JournalSequence,
    ) -> Arc<StructuralMarketSnapshot> {
        let levels = self.level_book.active_snapshot();
        Arc::new(self.snapshot(price, as_of_ns, sequence, levels))
    }

    pub fn chart_snapshot(&self) -> Arc<ChartStructureSnapshot> {
        Arc::clone(&self.chart)
    }

    pub fn graph_snapshot(&self) -> Arc<StructuralGraphSnapshot> {
        Arc::clone(&self.graph)
    }

    fn snapshot(
        &self,
        price: i64,
        as_of_ns: i64,
        sequence: JournalSequence,
        levels: Arc<[super::StructuralObject]>,
    ) -> StructuralMarketSnapshot {
        let mut quality = StructuralQuality::PRICE_ONLY;
        quality.gaps_present = self.gaps_present;
        StructuralMarketSnapshot {
            instrument_id: self.instrument,
            price_space: self.price_space,
            as_of_ns,
            journal_sequence: sequence,
            derivation_generation: self.level_book.generation(),
            reference_price: price,
            levels,
            graph: Arc::clone(&self.graph),
            location: locate_market(&self.graph, price, self.atr_value),
            quality,
        }
    }

    fn validate_bar(&self, bar: &CanonicalBar) -> Result<(), StructuralEngineError> {
        if bar.instrument_id != self.instrument.get() {
            return Err(StructuralEngineError::WrongInstrument);
        }
        if bar.timeframe != Timeframe::M4 as u16 {
            return Err(StructuralEngineError::WrongTimeframe);
        }
        if bar.derivation_version != self.derivation_version.get() {
            return Err(StructuralEngineError::WrongDerivationVersion);
        }
        if bar.revision > 0 {
            return Err(StructuralEngineError::RevisionRequiresRederivation);
        }
        Ok(())
    }

    fn update_continuity(
        &mut self,
        bar: &CanonicalBar,
        session_open_ns: i64,
        session_close_ns: i64,
    ) {
        match self.continuity {
            None => {
                self.gaps_present |= bar.ts_open_ns != session_open_ns;
            }
            Some(previous) if previous.session_open_ns == session_open_ns => {
                self.gaps_present |= bar.ts_open_ns > previous.bar_close_ns;
            }
            Some(previous) => {
                self.gaps_present |= previous.bar_close_ns != previous.session_close_ns
                    || bar.ts_open_ns != session_open_ns;
            }
        }
        self.continuity = Some(BarContinuity {
            bar_close_ns: bar.ts_close_ns,
            session_open_ns,
            session_close_ns,
        });
    }
}

#[derive(Debug, Error)]
pub enum StructuralEngineError {
    #[error("structural engine configuration has zero identity or invalid price tolerance")]
    InvalidConfiguration,
    #[error("structural engine received another instrument")]
    WrongInstrument,
    #[error("M4 is the only development timeframe in this structural cut")]
    WrongTimeframe,
    #[error("bar and structural derivation versions disagree")]
    WrongDerivationVersion,
    #[error("bar revision requires an explicit structural re-derivation generation")]
    RevisionRequiresRederivation,
    #[error(transparent)]
    Producer(#[from] ProducerError),
    #[error(transparent)]
    LevelBook(#[from] LevelBookError),
}

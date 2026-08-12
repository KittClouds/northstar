use super::macro_feed::MacroFeedSnapshot;
use super::snapshot::Availability;
use crate::data_plane::event::{
    CanonicalEvent, EventFlags, EventKind, ParticipantClass, TimeQuality,
};
use crate::data_plane::ids::{
    InstrumentId, JournalSequence, ReceiptId, ReleaseId, SeriesId, SourceId, StreamId,
};
use crate::data_plane::journal::MappedJournal;
use crate::data_plane::provenance::ReleaseStatus;
use crate::data_plane::providers::cftc::{binding_for_instrument, CftcMarketBinding, CFTC_MARKETS};
use crate::data_plane::providers::macro_catalog::{
    binding_for_series, series_bindings, MacroCategory, MacroMeasure, MacroSeriesBinding,
    MacroUnit, MACRO_SERIES_COUNT,
};
use hashbrown::HashSet;
use std::sync::Arc;
use thiserror::Error;

pub const MACRO_HISTORY_POINTS: usize = 36;
pub const POSITIONING_HISTORY_WEEKS: usize = 104;

#[derive(Clone, Copy, Debug)]
pub struct MacroPointSnapshot {
    pub value: f64,
    pub period_start_ns: i64,
    pub period_end_ns: i64,
    pub publication_ns: i64,
    pub received_ns: i64,
    pub effective_ns: i64,
    pub sequence: JournalSequence,
    pub receipt_id: ReceiptId,
    pub vintage_id: u64,
    pub provisional: bool,
    pub corrected: bool,
    pub time_quality: TimeQuality,
}

#[derive(Clone, Debug)]
pub struct MacroSeriesSnapshot {
    pub series_id: SeriesId,
    pub provider_code: &'static str,
    pub label: &'static str,
    pub category: MacroCategory,
    pub unit: MacroUnit,
    pub measure: MacroMeasure,
    pub source_url: &'static str,
    pub source: SourceId,
    pub stream: StreamId,
    pub latest: Option<MacroPointSnapshot>,
    pub previous: Option<MacroPointSnapshot>,
    pub history: Arc<[MacroPointSnapshot]>,
}

impl MacroSeriesSnapshot {
    #[inline]
    pub fn delta(&self) -> Option<f64> {
        Some(self.latest?.value - self.previous?.value)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PositioningPointSnapshot {
    pub report_date_ns: i64,
    pub received_ns: i64,
    pub effective_ns: i64,
    pub sequence: JournalSequence,
    pub receipt_id: ReceiptId,
    pub participant: ParticipantClass,
    pub long: i64,
    pub short: i64,
    pub spread: i64,
    pub open_interest: i64,
    pub corrected: bool,
    pub time_quality: TimeQuality,
}

impl PositioningPointSnapshot {
    #[inline]
    pub const fn net(self) -> i64 {
        self.long - self.short
    }
}

#[derive(Clone, Debug)]
pub struct PositioningParticipantSnapshot {
    pub participant: ParticipantClass,
    pub latest: Option<PositioningPointSnapshot>,
    pub previous: Option<PositioningPointSnapshot>,
    pub net_percentile: Option<f32>,
    pub history: Arc<[PositioningPointSnapshot]>,
}

impl PositioningParticipantSnapshot {
    #[inline]
    pub fn weekly_net_change(&self) -> Option<i64> {
        Some(self.latest?.net() - self.previous?.net())
    }
}

#[derive(Clone, Debug)]
pub struct PositioningMarketSnapshot {
    pub instrument_id: InstrumentId,
    pub label: &'static str,
    pub contract_code: &'static str,
    pub contract_name: &'static str,
    pub source_url: &'static str,
    pub latest_report_ns: i64,
    pub participants: Arc<[PositioningParticipantSnapshot]>,
}

impl PositioningMarketSnapshot {
    pub fn participant(
        &self,
        participant: ParticipantClass,
    ) -> Option<&PositioningParticipantSnapshot> {
        self.participants
            .iter()
            .find(|snapshot| snapshot.participant == participant)
    }
}

#[derive(Clone, Debug)]
pub struct MacroSnapshot {
    pub generation: u64,
    pub as_of_ns: i64,
    pub last_sequence: JournalSequence,
    pub availability: Availability,
    pub health: Arc<str>,
    pub next_refresh_ns: i64,
    pub event_count: u64,
    pub release_count: usize,
    pub document_count: u64,
    pub receipt_count: usize,
    pub series: Arc<[MacroSeriesSnapshot]>,
    pub positioning: Arc<[PositioningMarketSnapshot]>,
    pub releases: Arc<[MacroReleaseSnapshot]>,
    pub feeds: Arc<[MacroFeedSnapshot]>,
}

#[derive(Clone, Copy, Debug)]
pub struct MacroReleaseSnapshot {
    pub release_id: ReleaseId,
    pub scheduled_ns: i64,
    pub actual_ns: Option<i64>,
    pub received_ns: i64,
    pub sequence: JournalSequence,
    pub receipt_id: ReceiptId,
    pub status: ReleaseStatus,
    pub schedule_version: u64,
}

impl MacroSnapshot {
    pub fn awaiting_first_receipt() -> Self {
        Self {
            generation: 0,
            as_of_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            availability: Availability::AwaitingFirstReceipt,
            health: Arc::from("Official macro journal is empty"),
            next_refresh_ns: 0,
            event_count: 0,
            release_count: 0,
            document_count: 0,
            receipt_count: 0,
            series: empty_series_snapshots(),
            positioning: empty_positioning_snapshots(),
            releases: Arc::from([]),
            feeds: Arc::from([]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MacroPoint {
    value: f64,
    period_start_ns: i64,
    period_end_ns: i64,
    publication_ns: i64,
    received_ns: i64,
    effective_ns: i64,
    sequence: JournalSequence,
    receipt_id: ReceiptId,
    vintage_id: u64,
    flags: EventFlags,
}

impl MacroPoint {
    fn snapshot(self) -> MacroPointSnapshot {
        MacroPointSnapshot {
            value: self.value,
            period_start_ns: self.period_start_ns,
            period_end_ns: self.period_end_ns,
            publication_ns: self.publication_ns,
            received_ns: self.received_ns,
            effective_ns: self.effective_ns,
            sequence: self.sequence,
            receipt_id: self.receipt_id,
            vintage_id: self.vintage_id,
            provisional: self.flags.contains(EventFlags::PROVISIONAL),
            corrected: self.flags.contains(EventFlags::CORRECTION),
            time_quality: self.flags.time_quality(),
        }
    }
}

struct SeriesState {
    binding: &'static MacroSeriesBinding,
    points: Vec<MacroPoint>,
}

#[derive(Clone, Copy, Debug)]
struct PositioningPoint {
    report_date_ns: i64,
    received_ns: i64,
    effective_ns: i64,
    sequence: JournalSequence,
    receipt_id: ReceiptId,
    participant: ParticipantClass,
    long: i64,
    short: i64,
    spread: i64,
    open_interest: i64,
    flags: EventFlags,
}

impl PositioningPoint {
    fn snapshot(self) -> PositioningPointSnapshot {
        PositioningPointSnapshot {
            report_date_ns: self.report_date_ns,
            received_ns: self.received_ns,
            effective_ns: self.effective_ns,
            sequence: self.sequence,
            receipt_id: self.receipt_id,
            participant: self.participant,
            long: self.long,
            short: self.short,
            spread: self.spread,
            open_interest: self.open_interest,
            corrected: self.flags.contains(EventFlags::CORRECTION),
            time_quality: self.flags.time_quality(),
        }
    }
}

struct PositioningState {
    binding: &'static CftcMarketBinding,
    points: Vec<PositioningPoint>,
}

#[derive(Clone, Copy, Debug)]
struct ReleasePoint {
    release_id: ReleaseId,
    scheduled_ns: i64,
    actual_ns: Option<i64>,
    received_ns: i64,
    sequence: JournalSequence,
    receipt_id: ReceiptId,
    status: ReleaseStatus,
    schedule_version: u64,
}

impl ReleasePoint {
    fn snapshot(self) -> MacroReleaseSnapshot {
        MacroReleaseSnapshot {
            release_id: self.release_id,
            scheduled_ns: self.scheduled_ns,
            actual_ns: self.actual_ns,
            received_ns: self.received_ns,
            sequence: self.sequence,
            receipt_id: self.receipt_id,
            status: self.status,
            schedule_version: self.schedule_version,
        }
    }
}

pub struct MacroProjector {
    generation: u64,
    as_of_ns: i64,
    last_sequence: JournalSequence,
    event_count: u64,
    data_event_count: u64,
    document_count: u64,
    receipts: HashSet<ReceiptId>,
    feeds: HashSet<(SourceId, StreamId)>,
    series: Vec<SeriesState>,
    positioning: Vec<PositioningState>,
    releases: Vec<ReleasePoint>,
}

impl Default for MacroProjector {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroProjector {
    pub fn new() -> Self {
        Self {
            generation: 0,
            as_of_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            event_count: 0,
            data_event_count: 0,
            document_count: 0,
            receipts: HashSet::with_capacity(MACRO_SERIES_COUNT * 2),
            feeds: HashSet::with_capacity(16),
            series: series_bindings()
                .map(|binding| SeriesState {
                    binding,
                    points: Vec::with_capacity(MACRO_HISTORY_POINTS),
                })
                .collect(),
            positioning: CFTC_MARKETS
                .iter()
                .map(|binding| PositioningState {
                    binding,
                    points: Vec::with_capacity(POSITIONING_HISTORY_WEEKS * 5),
                })
                .collect(),
            releases: Vec::with_capacity(64),
        }
    }

    pub fn from_mapped(journal: &MappedJournal) -> Result<Self, MacroProjectorError> {
        let mut projector = Self::new();
        for batch in journal.batches() {
            projector.apply_batch(batch.events)?;
        }
        Ok(projector)
    }

    pub fn apply_batch(&mut self, events: &[CanonicalEvent]) -> Result<u32, MacroProjectorError> {
        let mut applied = 0u32;
        for event in events {
            if self.apply_event(event)? {
                applied = applied.saturating_add(1);
            }
        }
        if applied != 0 {
            self.generation = self.generation.saturating_add(1);
        }
        Ok(applied)
    }

    pub fn snapshot(
        &self,
        requested_availability: Availability,
        health: impl Into<Arc<str>>,
        next_refresh_ns: i64,
    ) -> MacroSnapshot {
        let availability = if self.event_count == 0 {
            Availability::AwaitingFirstReceipt
        } else {
            requested_availability
        };
        MacroSnapshot {
            generation: self.generation,
            as_of_ns: self.as_of_ns,
            last_sequence: self.last_sequence,
            availability,
            health: health.into(),
            next_refresh_ns,
            event_count: self.event_count,
            release_count: self.releases.len(),
            document_count: self.document_count,
            receipt_count: self.receipts.len(),
            series: self.series.iter().map(series_snapshot).collect(),
            positioning: self.positioning.iter().map(positioning_snapshot).collect(),
            releases: self
                .releases
                .iter()
                .copied()
                .map(ReleasePoint::snapshot)
                .collect(),
            feeds: Arc::from([]),
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data_event_count == 0
    }

    #[inline]
    pub const fn last_received_ns(&self) -> i64 {
        self.as_of_ns
    }

    #[inline]
    pub fn has_feed(&self, source: SourceId, stream: StreamId) -> bool {
        self.feeds.contains(&(source, stream))
    }

    fn apply_event(&mut self, event: &CanonicalEvent) -> Result<bool, MacroProjectorError> {
        let kind = event.kind()?;
        if !matches!(
            kind,
            EventKind::MacroObservation
                | EventKind::PositioningObservation
                | EventKind::MacroRelease
                | EventKind::SourceDocument
        ) {
            return Ok(false);
        }
        let sequence = event.journal_sequence();
        if sequence == JournalSequence::UNKNOWN || sequence <= self.last_sequence {
            return Err(MacroProjectorError::NonMonotonicSequence {
                previous: self.last_sequence,
                actual: sequence,
            });
        }
        match kind {
            EventKind::MacroObservation => self.apply_macro(event, sequence)?,
            EventKind::PositioningObservation => self.apply_positioning(event, sequence)?,
            EventKind::MacroRelease => self.apply_release(event, sequence)?,
            EventKind::SourceDocument => {
                event.document_kind()?;
                event.document_content_hash()?;
                self.document_count = self.document_count.saturating_add(1);
            }
            _ => unreachable!(),
        }
        self.receipts.insert(event.receipt_id());
        self.feeds.insert((
            SourceId(event.header.source_id),
            StreamId(event.header.stream_id),
        ));
        self.last_sequence = sequence;
        self.as_of_ns = self.as_of_ns.max(event.header.ts_received_ns);
        self.event_count = self.event_count.saturating_add(1);
        if matches!(
            kind,
            EventKind::MacroObservation | EventKind::PositioningObservation
        ) {
            self.data_event_count = self.data_event_count.saturating_add(1);
        }
        Ok(true)
    }

    fn apply_release(
        &mut self,
        event: &CanonicalEvent,
        sequence: JournalSequence,
    ) -> Result<(), MacroProjectorError> {
        let (scheduled_ns, actual_ns) = event.release_times()?;
        let point = ReleasePoint {
            release_id: event.release_id()?,
            scheduled_ns,
            actual_ns,
            received_ns: event.header.ts_received_ns,
            sequence,
            receipt_id: event.receipt_id(),
            status: event.release_status()?,
            schedule_version: event.values[3] as u64,
        };
        self.releases.push(point);
        self.releases
            .sort_unstable_by_key(|release| (release.scheduled_ns, release.sequence));
        if self.releases.len() > 128 {
            self.releases.drain(..self.releases.len() - 128);
        }
        Ok(())
    }

    fn apply_macro(
        &mut self,
        event: &CanonicalEvent,
        sequence: JournalSequence,
    ) -> Result<(), MacroProjectorError> {
        let binding = binding_for_series(event.series_id())
            .ok_or(MacroProjectorError::UnknownSeries(event.series_id()))?;
        let state = self
            .series
            .iter_mut()
            .find(|state| state.binding.series_id == binding.series_id)
            .expect("macro binding and projector catalog diverged");
        let point = MacroPoint {
            value: event.macro_value()?,
            period_start_ns: event.values[0],
            period_end_ns: event.values[1],
            publication_ns: event.header.ts_event_ns,
            received_ns: event.header.ts_received_ns,
            effective_ns: event.header.ts_effective_ns,
            sequence,
            receipt_id: event.receipt_id(),
            vintage_id: event.values[3] as u64,
            flags: event.flags(),
        };
        if point.period_start_ns >= point.period_end_ns {
            return Err(MacroProjectorError::InvalidPeriod(binding.series_id));
        }
        state.points.push(point);
        Ok(())
    }

    fn apply_positioning(
        &mut self,
        event: &CanonicalEvent,
        sequence: JournalSequence,
    ) -> Result<(), MacroProjectorError> {
        let binding = binding_for_instrument(event.instrument_id()).ok_or(
            MacroProjectorError::UnknownPositioningMarket(event.instrument_id()),
        )?;
        let state = self
            .positioning
            .iter_mut()
            .find(|state| state.binding.instrument_id == binding.instrument_id)
            .expect("CFTC binding and projector catalog diverged");
        let (long, short, spread, open_interest) = event.positioning_counts()?;
        state.points.push(PositioningPoint {
            report_date_ns: event.header.ts_event_ns,
            received_ns: event.header.ts_received_ns,
            effective_ns: event.header.ts_effective_ns,
            sequence,
            receipt_id: event.receipt_id(),
            participant: event.participant_class()?,
            long,
            short,
            spread,
            open_interest,
            flags: event.flags(),
        });
        Ok(())
    }
}

fn series_snapshot(state: &SeriesState) -> MacroSeriesSnapshot {
    let mut visible: Vec<MacroPoint> =
        Vec::with_capacity(state.points.len().min(MACRO_HISTORY_POINTS));
    for point in &state.points {
        match visible
            .iter_mut()
            .find(|current| current.period_start_ns == point.period_start_ns)
        {
            Some(current) if point.sequence > current.sequence => *current = *point,
            Some(_) => {}
            None => visible.push(*point),
        }
    }
    visible.sort_unstable_by_key(|point| point.period_start_ns);
    let start = visible.len().saturating_sub(MACRO_HISTORY_POINTS);
    let history: Arc<[MacroPointSnapshot]> = visible[start..]
        .iter()
        .copied()
        .map(MacroPoint::snapshot)
        .collect();
    let latest = history.last().copied();
    let previous = history.len().checked_sub(2).map(|index| history[index]);
    MacroSeriesSnapshot {
        series_id: state.binding.series_id,
        provider_code: state.binding.provider_code,
        label: state.binding.label,
        category: state.binding.category,
        unit: state.binding.unit,
        measure: state.binding.measure,
        source_url: state.binding.source_url,
        source: state.binding.source,
        stream: state.binding.stream,
        latest,
        previous,
        history,
    }
}

fn empty_series_snapshots() -> Arc<[MacroSeriesSnapshot]> {
    series_bindings()
        .map(|binding| MacroSeriesSnapshot {
            series_id: binding.series_id,
            provider_code: binding.provider_code,
            label: binding.label,
            category: binding.category,
            unit: binding.unit,
            measure: binding.measure,
            source_url: binding.source_url,
            source: binding.source,
            stream: binding.stream,
            latest: None,
            previous: None,
            history: Arc::from([]),
        })
        .collect()
}

fn positioning_snapshot(state: &PositioningState) -> PositioningMarketSnapshot {
    let participants: Arc<[PositioningParticipantSnapshot]> = [
        ParticipantClass::Dealer,
        ParticipantClass::AssetManager,
        ParticipantClass::LeveragedFunds,
        ParticipantClass::OtherReportables,
        ParticipantClass::NonReportable,
    ]
    .into_iter()
    .map(|participant| participant_snapshot(&state.points, participant))
    .collect();
    let latest_report_ns = participants
        .iter()
        .filter_map(|participant| participant.latest)
        .map(|point| point.report_date_ns)
        .max()
        .unwrap_or(0);
    PositioningMarketSnapshot {
        instrument_id: state.binding.instrument_id,
        label: state.binding.label,
        contract_code: state.binding.contract_code,
        contract_name: state.binding.contract_name,
        source_url: state.binding.source_url,
        latest_report_ns,
        participants,
    }
}

fn participant_snapshot(
    points: &[PositioningPoint],
    participant: ParticipantClass,
) -> PositioningParticipantSnapshot {
    let mut visible: Vec<PositioningPoint> =
        Vec::with_capacity(points.len().min(POSITIONING_HISTORY_WEEKS));
    for point in points
        .iter()
        .filter(|point| point.participant == participant)
    {
        match visible
            .iter_mut()
            .find(|current| current.report_date_ns == point.report_date_ns)
        {
            Some(current) if point.sequence > current.sequence => *current = *point,
            Some(_) => {}
            None => visible.push(*point),
        }
    }
    visible.sort_unstable_by_key(|point| point.report_date_ns);
    let start = visible.len().saturating_sub(POSITIONING_HISTORY_WEEKS);
    let history: Arc<[PositioningPointSnapshot]> = visible[start..]
        .iter()
        .copied()
        .map(PositioningPoint::snapshot)
        .collect();
    let latest = history.last().copied();
    let previous = history.len().checked_sub(2).map(|index| history[index]);
    let net_percentile = latest.map(|latest| {
        let less_or_equal = history
            .iter()
            .filter(|point| point.net() <= latest.net())
            .count();
        less_or_equal as f32 * 100.0 / history.len().max(1) as f32
    });
    PositioningParticipantSnapshot {
        participant,
        latest,
        previous,
        net_percentile,
        history,
    }
}

fn empty_positioning_snapshots() -> Arc<[PositioningMarketSnapshot]> {
    CFTC_MARKETS
        .iter()
        .map(|binding| PositioningMarketSnapshot {
            instrument_id: binding.instrument_id,
            label: binding.label,
            contract_code: binding.contract_code,
            contract_name: binding.contract_name,
            source_url: binding.source_url,
            latest_report_ns: 0,
            participants: [
                ParticipantClass::Dealer,
                ParticipantClass::AssetManager,
                ParticipantClass::LeveragedFunds,
                ParticipantClass::OtherReportables,
                ParticipantClass::NonReportable,
            ]
            .into_iter()
            .map(|participant| PositioningParticipantSnapshot {
                participant,
                latest: None,
                previous: None,
                net_percentile: None,
                history: Arc::from([]),
            })
            .collect(),
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum MacroProjectorError {
    #[error(transparent)]
    Canonical(#[from] crate::data_plane::event::CanonicalError),
    #[error("macro sequence must increase: previous={previous:?}, actual={actual:?}")]
    NonMonotonicSequence {
        previous: JournalSequence,
        actual: JournalSequence,
    },
    #[error("macro series is not in the frozen catalog: {0:?}")]
    UnknownSeries(SeriesId),
    #[error("macro series has an invalid period: {0:?}")]
    InvalidPeriod(SeriesId),
    #[error("instrument is not in the frozen CFTC positioning catalog: {0:?}")]
    UnknownPositioningMarket(InstrumentId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::event::CanonicalEvent;
    use crate::data_plane::ids::{macro_series, ReceiptId};
    use crate::data_plane::providers::bls::{BLS_SOURCE, BLS_TIMESERIES_STREAM};
    use crate::data_plane::providers::cftc::{CFTC_SOURCE, CFTC_TFF_STREAM};

    fn event(sequence: u64, period_start: i64, value: f64, vintage: u64) -> CanonicalEvent {
        let mut event = CanonicalEvent::macro_observation(
            BLS_SOURCE,
            BLS_TIMESERIES_STREAM,
            macro_series::US_CPI_ALL_ITEMS_NSA,
            ReceiptId(sequence),
            sequence,
            period_start,
            period_start + 10,
            1_000 + sequence as i64,
            1_000 + sequence as i64,
            value,
            vintage,
            None,
            TimeQuality::ObservedLive,
        )
        .unwrap();
        event.header.journal_sequence = sequence;
        event
    }

    fn position_event(sequence: u64, report_date_ns: i64, long: i64, short: i64) -> CanonicalEvent {
        let mut event = CanonicalEvent::positioning_observation(
            CFTC_SOURCE,
            CFTC_TFF_STREAM,
            crate::data_plane::ids::instruments::US100,
            ParticipantClass::AssetManager,
            ReceiptId(100),
            sequence,
            report_date_ns,
            2_000,
            long,
            short,
            10,
            10_000,
            TimeQuality::ObservedLive,
        )
        .unwrap();
        event.header.journal_sequence = sequence;
        event
    }

    #[test]
    fn later_vintage_replaces_period_without_erasing_history() {
        let mut projector = MacroProjector::new();
        projector
            .apply_batch(&[
                event(1, 100, 2.0, 10),
                event(2, 200, 3.0, 20),
                event(3, 100, 2.1, 30),
            ])
            .unwrap();
        let snapshot = projector.snapshot(Availability::Live, "BLS current", 9_000);
        let cpi = &snapshot.series[0];
        assert_eq!(cpi.history.len(), 2);
        assert_eq!(cpi.history[0].value, 2.1);
        assert_eq!(cpi.latest.unwrap().value, 3.0);
        assert_eq!(snapshot.event_count, 3);
        assert_eq!(snapshot.receipt_count, 3);
    }

    #[test]
    fn empty_projector_never_claims_live() {
        let snapshot = MacroProjector::new().snapshot(Availability::Live, "polling", 9_000);
        assert_eq!(snapshot.availability, Availability::AwaitingFirstReceipt);
        assert!(snapshot.series.iter().all(|series| series.latest.is_none()));
    }

    #[test]
    fn positioning_snapshot_keeps_objective_net_history_and_percentile() {
        let mut projector = MacroProjector::new();
        projector
            .apply_batch(&[
                position_event(1, 100, 500, 400),
                position_event(2, 200, 700, 300),
                position_event(3, 300, 600, 400),
            ])
            .unwrap();
        let snapshot = projector.snapshot(Availability::Live, "CFTC current", 9_000);
        let asset_managers = snapshot.positioning[0]
            .participant(ParticipantClass::AssetManager)
            .unwrap();
        assert_eq!(asset_managers.history.len(), 3);
        assert_eq!(asset_managers.latest.unwrap().net(), 200);
        assert_eq!(asset_managers.weekly_net_change(), Some(-200));
        assert!((asset_managers.net_percentile.unwrap() - 66.666_664).abs() < 0.01);
    }
}

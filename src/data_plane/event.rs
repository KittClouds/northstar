use crate::data_plane::ids::{
    BatchId, InstrumentId, JournalSequence, ReceiptId, SchemaVersion, SeriesId, SourceId, StreamId,
};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

pub const NO_PROVIDER_SEQUENCE: u64 = u64::MAX;
pub const NO_VALUE: i64 = i64::MIN;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(u16)]
pub enum EventKind {
    IndexValue = 1,
    VenueQuote = 2,
    ExternalBar = 3,
    CanonicalBar = 4,
    BarRevision = 5,
    MacroObservation = 16,
    MacroRelease = 17,
    PositioningObservation = 18,
    SourceDocument = 19,
    InstrumentBinding = 32,
    InstrumentContract = 33,
    CalendarVersion = 34,
    SessionState = 35,
    ClockObservation = 48,
    DataGap = 49,
    DataQuality = 50,
    AuthorityChanged = 51,
}

impl TryFrom<u16> for EventKind {
    type Error = CanonicalError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let kind = match value {
            1 => Self::IndexValue,
            2 => Self::VenueQuote,
            3 => Self::ExternalBar,
            4 => Self::CanonicalBar,
            5 => Self::BarRevision,
            16 => Self::MacroObservation,
            17 => Self::MacroRelease,
            18 => Self::PositioningObservation,
            19 => Self::SourceDocument,
            32 => Self::InstrumentBinding,
            33 => Self::InstrumentContract,
            34 => Self::CalendarVersion,
            35 => Self::SessionState,
            48 => Self::ClockObservation,
            49 => Self::DataGap,
            50 => Self::DataQuality,
            51 => Self::AuthorityChanged,
            _ => return Err(CanonicalError::UnknownKind(value)),
        };
        Ok(kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TimeQuality {
    ObservedLive = 0,
    VerifiedPublication = 1,
    EstimatedHistorical = 2,
    Unknown = 3,
    /// Provider-authoritative value whose contract explicitly carries a delay.
    /// It remains observable truth, but must never be rendered or armed as live.
    ProviderDelayed = 4,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ParticipantClass {
    Dealer = 1,
    AssetManager = 2,
    LeveragedFunds = 3,
    OtherReportables = 4,
    NonReportable = 5,
}

impl TryFrom<u8> for ParticipantClass {
    type Error = CanonicalError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Dealer),
            2 => Ok(Self::AssetManager),
            3 => Ok(Self::LeveragedFunds),
            4 => Ok(Self::OtherReportables),
            5 => Ok(Self::NonReportable),
            _ => Err(CanonicalError::UnknownParticipantClass(value)),
        }
    }
}

impl TimeQuality {
    const SHIFT: u32 = 8;
    const MASK: u32 = 0b111 << Self::SHIFT;

    #[inline]
    const fn encode(self) -> u32 {
        (self as u32) << Self::SHIFT
    }

    #[inline]
    fn decode(flags: u32) -> Self {
        match (flags & Self::MASK) >> Self::SHIFT {
            0 => Self::ObservedLive,
            1 => Self::VerifiedPublication,
            2 => Self::EstimatedHistorical,
            3 => Self::Unknown,
            4 => Self::ProviderDelayed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EventFlags(pub u32);

impl EventFlags {
    pub const NONE: Self = Self(0);
    pub const PROVISIONAL: u32 = 1 << 0;
    pub const FINAL: u32 = 1 << 1;
    pub const CORRECTION: u32 = 1 << 2;
    pub const PARTIAL_SESSION_END: u32 = 1 << 3;
    pub const AGGREGATE_BACKFILL: u32 = 1 << 4;
    pub const VALUE_NATIVE: u32 = 1 << 5;
    pub const VENUE_TRUTH: u32 = 1 << 6;

    #[inline]
    pub const fn new(bits: u32, quality: TimeQuality) -> Self {
        Self((bits & !TimeQuality::MASK) | quality.encode())
    }

    #[inline]
    pub fn time_quality(self) -> TimeQuality {
        TimeQuality::decode(self.0)
    }

    #[inline]
    pub const fn contains(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

/// Fixed 80-byte causal envelope. `source_event_id` is stable within a source
/// stream; `journal_sequence` is assigned only after durable commit.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct EventHeader {
    pub journal_sequence: u64,
    pub receipt_id: u64,
    pub source_event_id: u64,
    pub ts_event_ns: i64,
    pub ts_received_ns: i64,
    pub ts_effective_ns: i64,
    pub provider_sequence: u64,
    pub entity_id: u32,
    pub source_id: u16,
    pub stream_id: u16,
    pub schema_version: u16,
    pub kind: u16,
    pub flags: u32,
    pub reserved: [u32; 2],
}

/// Dense canonical journal record. Payload interpretation is selected by
/// `header.kind`; hot records contain no strings, pointers, or heap ownership.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct CanonicalEvent {
    pub header: EventHeader,
    pub values: [i64; 6],
}

impl CanonicalEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: EventKind,
        source: SourceId,
        stream: StreamId,
        entity_id: u32,
        receipt: ReceiptId,
        source_event_id: u64,
        ts_event_ns: i64,
        ts_received_ns: i64,
        ts_effective_ns: i64,
        flags: EventFlags,
        values: [i64; 6],
    ) -> Result<Self, CanonicalError> {
        if ts_effective_ns < ts_received_ns
            && matches!(
                flags.time_quality(),
                TimeQuality::ObservedLive | TimeQuality::ProviderDelayed
            )
        {
            return Err(CanonicalError::LiveTimeTravel {
                received: ts_received_ns,
                effective: ts_effective_ns,
            });
        }
        if source_event_id == 0 {
            return Err(CanonicalError::MissingSourceEventId);
        }
        Ok(Self {
            header: EventHeader {
                journal_sequence: 0,
                receipt_id: receipt.get(),
                source_event_id,
                ts_event_ns,
                ts_received_ns,
                ts_effective_ns,
                provider_sequence: NO_PROVIDER_SEQUENCE,
                entity_id,
                source_id: source.get(),
                stream_id: stream.get(),
                schema_version: SchemaVersion(1).get(),
                kind: kind as u16,
                flags: flags.0,
                reserved: [0; 2],
            },
            values,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn index_value(
        source: SourceId,
        stream: StreamId,
        instrument: InstrumentId,
        receipt: ReceiptId,
        source_event_id: u64,
        ts_event_ns: i64,
        ts_received_ns: i64,
        price_scaled: i64,
        quality: TimeQuality,
    ) -> Result<Self, CanonicalError> {
        let effective = match quality {
            TimeQuality::ObservedLive | TimeQuality::ProviderDelayed => ts_received_ns,
            _ => ts_event_ns,
        };
        Self::new(
            EventKind::IndexValue,
            source,
            stream,
            instrument.get(),
            receipt,
            source_event_id,
            ts_event_ns,
            ts_received_ns,
            effective,
            EventFlags::new(EventFlags::VALUE_NATIVE, quality),
            [
                price_scaled,
                NO_VALUE,
                NO_VALUE,
                NO_VALUE,
                NO_VALUE,
                NO_VALUE,
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn venue_quote(
        source: SourceId,
        stream: StreamId,
        instrument: InstrumentId,
        receipt: ReceiptId,
        source_event_id: u64,
        ts_event_ns: i64,
        ts_received_ns: i64,
        bid_scaled: i64,
        ask_scaled: i64,
    ) -> Result<Self, CanonicalError> {
        if bid_scaled > ask_scaled {
            return Err(CanonicalError::CrossedQuote {
                bid: bid_scaled,
                ask: ask_scaled,
            });
        }
        Self::new(
            EventKind::VenueQuote,
            source,
            stream,
            instrument.get(),
            receipt,
            source_event_id,
            ts_event_ns,
            ts_received_ns,
            ts_received_ns,
            EventFlags::new(EventFlags::VENUE_TRUTH, TimeQuality::ObservedLive),
            [
                bid_scaled,
                ask_scaled,
                bid_scaled.saturating_add(ask_scaled) / 2,
                NO_VALUE,
                NO_VALUE,
                NO_VALUE,
            ],
        )
    }

    /// Provider-authored OHLC bar. The close is the event time; the provider's
    /// open time and Northstar timeframe code remain in the dense payload.
    /// External bars are immutable observations, not locally derived bars.
    #[allow(clippy::too_many_arguments)]
    pub fn external_bar(
        source: SourceId,
        stream: StreamId,
        instrument: InstrumentId,
        receipt: ReceiptId,
        source_event_id: u64,
        ts_open_ns: i64,
        ts_close_ns: i64,
        ts_received_ns: i64,
        open_scaled: i64,
        high_scaled: i64,
        low_scaled: i64,
        close_scaled: i64,
        timeframe: u16,
        quality: TimeQuality,
    ) -> Result<Self, CanonicalError> {
        if ts_open_ns < 0 || ts_close_ns <= ts_open_ns {
            return Err(CanonicalError::InvalidBarWindow);
        }
        if timeframe == 0 {
            return Err(CanonicalError::InvalidBarTimeframe);
        }
        if low_scaled > high_scaled
            || high_scaled < open_scaled.max(close_scaled)
            || low_scaled > open_scaled.min(close_scaled)
        {
            return Err(CanonicalError::InvalidOhlc);
        }
        let effective = match quality {
            TimeQuality::ObservedLive | TimeQuality::ProviderDelayed => ts_received_ns,
            _ => ts_close_ns,
        };
        Self::new(
            EventKind::ExternalBar,
            source,
            stream,
            instrument.get(),
            receipt,
            source_event_id,
            ts_close_ns,
            ts_received_ns,
            effective,
            EventFlags::new(EventFlags::FINAL | EventFlags::AGGREGATE_BACKFILL, quality),
            [
                open_scaled,
                high_scaled,
                low_scaled,
                close_scaled,
                ts_open_ns,
                i64::from(timeframe),
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn macro_observation(
        source: SourceId,
        stream: StreamId,
        series: SeriesId,
        receipt: ReceiptId,
        source_event_id: u64,
        period_start_ns: i64,
        period_end_ns: i64,
        publication_ns: i64,
        received_ns: i64,
        value: f64,
        vintage_id: u64,
        supersedes_vintage_id: Option<u64>,
        quality: TimeQuality,
    ) -> Result<Self, CanonicalError> {
        if !value.is_finite() {
            return Err(CanonicalError::NonFiniteMacroValue);
        }
        if period_start_ns >= period_end_ns || vintage_id == 0 {
            return Err(CanonicalError::InvalidMacroIdentity);
        }
        let effective = match quality {
            TimeQuality::ObservedLive => received_ns,
            _ => publication_ns,
        };
        Self::new(
            EventKind::MacroObservation,
            source,
            stream,
            series.get(),
            receipt,
            source_event_id,
            publication_ns,
            received_ns,
            effective,
            EventFlags::new(
                if supersedes_vintage_id.is_some() {
                    EventFlags::CORRECTION
                } else {
                    0
                },
                quality,
            ),
            [
                period_start_ns,
                period_end_ns,
                value.to_bits() as i64,
                vintage_id as i64,
                supersedes_vintage_id.unwrap_or(0) as i64,
                NO_VALUE,
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn positioning_observation(
        source: SourceId,
        stream: StreamId,
        instrument: InstrumentId,
        participant: ParticipantClass,
        receipt: ReceiptId,
        source_event_id: u64,
        report_date_ns: i64,
        received_ns: i64,
        long: i64,
        short: i64,
        spread: i64,
        open_interest: i64,
        quality: TimeQuality,
    ) -> Result<Self, CanonicalError> {
        if long < 0 || short < 0 || spread < 0 || open_interest <= 0 {
            return Err(CanonicalError::InvalidPositioningCounts);
        }
        let effective = match quality {
            TimeQuality::ObservedLive => received_ns,
            _ => report_date_ns,
        };
        Self::new(
            EventKind::PositioningObservation,
            source,
            stream,
            instrument.get(),
            receipt,
            source_event_id,
            report_date_ns,
            received_ns,
            effective,
            EventFlags::new(0, quality),
            [
                participant as i64,
                long,
                short,
                spread,
                open_interest,
                NO_VALUE,
            ],
        )
    }

    #[inline]
    pub fn macro_value(&self) -> Result<f64, CanonicalError> {
        if self.kind()? != EventKind::MacroObservation {
            return Err(CanonicalError::WrongPayloadKind);
        }
        Ok(f64::from_bits(self.values[2] as u64))
    }

    #[inline]
    pub const fn series_id(&self) -> SeriesId {
        SeriesId(self.header.entity_id)
    }

    pub fn participant_class(&self) -> Result<ParticipantClass, CanonicalError> {
        if self.kind()? != EventKind::PositioningObservation {
            return Err(CanonicalError::WrongPayloadKind);
        }
        u8::try_from(self.values[0])
            .map_err(|_| CanonicalError::UnknownParticipantClass(u8::MAX))?
            .try_into()
    }

    pub fn positioning_counts(&self) -> Result<(i64, i64, i64, i64), CanonicalError> {
        if self.kind()? != EventKind::PositioningObservation {
            return Err(CanonicalError::WrongPayloadKind);
        }
        Ok((
            self.values[1],
            self.values[2],
            self.values[3],
            self.values[4],
        ))
    }

    #[inline]
    pub fn kind(&self) -> Result<EventKind, CanonicalError> {
        self.header.kind.try_into()
    }

    #[inline]
    pub const fn instrument_id(&self) -> InstrumentId {
        InstrumentId(self.header.entity_id)
    }

    #[inline]
    pub const fn journal_sequence(&self) -> JournalSequence {
        JournalSequence(self.header.journal_sequence)
    }

    #[inline]
    pub const fn receipt_id(&self) -> ReceiptId {
        ReceiptId(self.header.receipt_id)
    }

    #[inline]
    pub const fn flags(&self) -> EventFlags {
        EventFlags(self.header.flags)
    }
}

#[derive(Clone, Debug)]
pub struct CanonicalBatch {
    pub id: BatchId,
    pub events: SmallVec<[CanonicalEvent; 32]>,
}

impl CanonicalBatch {
    pub fn new(id: BatchId) -> Self {
        Self {
            id,
            events: SmallVec::new(),
        }
    }

    #[inline]
    pub fn push(&mut self, event: CanonicalEvent) {
        self.events.push(event);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CanonicalError {
    #[error("observed-live effective time {effective} predates receipt {received}")]
    LiveTimeTravel { received: i64, effective: i64 },
    #[error("source event identity must be non-zero")]
    MissingSourceEventId,
    #[error("unknown canonical event kind {0}")]
    UnknownKind(u16),
    #[error("venue quote is crossed: bid {bid} exceeds ask {ask}")]
    CrossedQuote { bid: i64, ask: i64 },
    #[error("external bar window is invalid")]
    InvalidBarWindow,
    #[error("external bar timeframe is invalid")]
    InvalidBarTimeframe,
    #[error("external bar OHLC values are inconsistent")]
    InvalidOhlc,
    #[error("macro value must be finite")]
    NonFiniteMacroValue,
    #[error("macro period and vintage identity are invalid")]
    InvalidMacroIdentity,
    #[error("macro release identity or schedule is invalid")]
    InvalidMacroRelease,
    #[error("source document identity or content hash is invalid")]
    InvalidSourceDocument,
    #[error("positioning counts must be non-negative and open interest must be positive")]
    InvalidPositioningCounts,
    #[error("unknown positioning participant class {0}")]
    UnknownParticipantClass(u8),
    #[error("canonical payload accessor used with the wrong event kind")]
    WrongPayloadKind,
}

const _: () = assert!(std::mem::size_of::<EventHeader>() == 80);
const _: () = assert!(std::mem::size_of::<CanonicalEvent>() == 128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_events_cannot_backdate_effective_time() {
        let error = CanonicalEvent::new(
            EventKind::IndexValue,
            SourceId(1),
            StreamId(1),
            1,
            ReceiptId(1),
            1,
            10,
            20,
            19,
            EventFlags::new(0, TimeQuality::ObservedLive),
            [0; 6],
        )
        .unwrap_err();
        assert_eq!(
            error,
            CanonicalError::LiveTimeTravel {
                received: 20,
                effective: 19
            }
        );
    }

    #[test]
    fn event_layout_is_stable_and_dense() {
        assert_eq!(std::mem::size_of::<CanonicalEvent>(), 128);
        assert_eq!(std::mem::align_of::<CanonicalEvent>(), 8);
    }

    #[test]
    fn external_bar_keeps_provider_ohlc_and_observation_time() {
        let event = CanonicalEvent::external_bar(
            SourceId(10),
            StreamId(2),
            InstrumentId(1),
            ReceiptId(3),
            4,
            100,
            200,
            250,
            10_000,
            10_500,
            9_900,
            10_250,
            1,
            TimeQuality::EstimatedHistorical,
        )
        .unwrap();
        assert_eq!(event.kind().unwrap(), EventKind::ExternalBar);
        assert_eq!(event.header.ts_event_ns, 200);
        assert_eq!(event.header.ts_effective_ns, 200);
        assert_eq!(event.values, [10_000, 10_500, 9_900, 10_250, 100, 1]);
        assert!(event.flags().contains(EventFlags::AGGREGATE_BACKFILL));
    }

    #[test]
    fn macro_vintage_keeps_numeric_truth_and_supersession() {
        let event = CanonicalEvent::macro_observation(
            SourceId(2),
            StreamId(3),
            SeriesId(9),
            ReceiptId(4),
            77,
            10,
            20,
            30,
            31,
            2.75,
            8,
            Some(7),
            TimeQuality::ObservedLive,
        )
        .unwrap();
        assert_eq!(event.series_id(), SeriesId(9));
        assert_eq!(event.macro_value().unwrap(), 2.75);
        assert_eq!(event.values[3], 8);
        assert_eq!(event.values[4], 7);
        assert!(event.flags().contains(EventFlags::CORRECTION));
    }

    #[test]
    fn positioning_keeps_participant_and_contract_counts_compact() {
        let event = CanonicalEvent::positioning_observation(
            SourceId(21),
            StreamId(1),
            InstrumentId(1),
            ParticipantClass::AssetManager,
            ReceiptId(8),
            99,
            10,
            20,
            1_200,
            300,
            40,
            2_000,
            TimeQuality::ObservedLive,
        )
        .unwrap();
        assert_eq!(
            event.participant_class().unwrap(),
            ParticipantClass::AssetManager
        );
        assert_eq!(event.positioning_counts().unwrap(), (1_200, 300, 40, 2_000));
        assert_eq!(event.header.ts_effective_ns, 20);
    }
}

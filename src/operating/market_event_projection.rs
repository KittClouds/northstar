use super::{Availability, OperatingMode, ValueMeta};
use crate::data_plane::bars::{CanonicalBar, Timeframe};
use crate::data_plane::event::{CanonicalEvent, TimeQuality};
use crate::data_plane::ids::{DerivationVersion, SourceId, StreamId};

pub(super) fn value_meta(event: &CanonicalEvent, availability: Availability) -> ValueMeta {
    ValueMeta {
        ts_event_ns: event.header.ts_event_ns,
        ts_received_ns: event.header.ts_received_ns,
        ts_effective_ns: event.header.ts_effective_ns,
        journal_sequence: event.journal_sequence(),
        derivation_version: DerivationVersion::UNKNOWN,
        source: SourceId(event.header.source_id),
        stream: StreamId(event.header.stream_id),
        availability,
        reserved: [0; 3],
    }
}

pub(super) const fn availability_for(mode: OperatingMode) -> Availability {
    match mode {
        OperatingMode::OfflineReplay => Availability::Replaying,
        OperatingMode::LiveData | OperatingMode::Paper | OperatingMode::Live => Availability::Live,
    }
}

pub(super) fn availability_for_event(mode: OperatingMode, event: &CanonicalEvent) -> Availability {
    if matches!(mode, OperatingMode::OfflineReplay) {
        return Availability::Replaying;
    }
    match event.flags().time_quality() {
        TimeQuality::ObservedLive => Availability::Live,
        TimeQuality::ProviderDelayed => Availability::Delayed,
        TimeQuality::VerifiedPublication
        | TimeQuality::EstimatedHistorical
        | TimeQuality::Unknown => Availability::Stale,
    }
}

pub(super) fn external_bar(event: &CanonicalEvent, timeframe: Timeframe) -> CanonicalBar {
    CanonicalBar {
        ts_open_ns: event.values[4],
        ts_close_ns: event.header.ts_event_ns,
        open: event.values[0],
        high: event.values[1],
        low: event.values[2],
        close: event.values[3],
        first_sequence: event.header.journal_sequence,
        last_sequence: event.header.journal_sequence,
        instrument_id: event.header.entity_id,
        observation_count: 1,
        revision: 0,
        derivation_version: DerivationVersion::UNKNOWN.get(),
        timeframe: timeframe as u16,
        flags: (event.flags().0 & u32::from(u16::MAX)) as u16,
        reserved: 0,
    }
}

pub(super) const fn timeframe_slot(timeframe: Timeframe) -> usize {
    match timeframe {
        Timeframe::M4 => 0,
        Timeframe::M20 => 1,
        Timeframe::H2 => 2,
        Timeframe::H4 => 3,
    }
}

pub(super) const fn decode_timeframe(value: u16) -> Option<Timeframe> {
    match value {
        1 => Some(Timeframe::M4),
        2 => Some(Timeframe::M20),
        3 => Some(Timeframe::H2),
        4 => Some(Timeframe::H4),
        _ => None,
    }
}

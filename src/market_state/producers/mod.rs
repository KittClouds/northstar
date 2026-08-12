mod opening_range;
mod range_projection;
mod sessions;

pub use opening_range::{OpeningRangeProducer, OpeningRangeSpec};
pub use range_projection::{RangeProjectionProducer, RangeProjectionSpec};
pub use sessions::SessionStructureProducer;

use super::{
    LevelFamily, LevelId, LevelKind, LevelMetrics, LevelProvenance, LevelRole, LevelState,
    PriceSpace, ProducerId, StructuralLevel,
};
use crate::data_plane::bars::{CanonicalBar, Timeframe};
use crate::data_plane::ids::{CalendarId, DerivationVersion, InstrumentId, JournalSequence};
use thiserror::Error;

#[derive(Clone, Copy, Debug)]
struct IdSequence {
    producer: ProducerId,
    instrument: InstrumentId,
    next: u64,
}

impl IdSequence {
    fn new(producer: ProducerId, instrument: InstrumentId) -> Self {
        Self {
            producer,
            instrument,
            next: 1,
        }
    }

    fn take(&mut self) -> LevelId {
        let id = LevelId::from_parts(self.producer, self.instrument, self.next);
        self.next = self.next.saturating_add(1);
        id
    }
}

#[derive(Clone, Copy, Debug)]
struct LevelSeed {
    id: LevelId,
    instrument: InstrumentId,
    price_space: PriceSpace,
    kind: LevelKind,
    family: LevelFamily,
    role: LevelRole,
    price: i64,
    now_ns: i64,
    producer: ProducerId,
    calendar_id: CalendarId,
    derivation_version: DerivationVersion,
    timeframe: Timeframe,
    epoch_ns: i64,
    exclusive_group: u64,
    ordinal: u16,
    first_sequence: JournalSequence,
    last_sequence: JournalSequence,
}

fn level(seed: LevelSeed) -> StructuralLevel {
    StructuralLevel {
        id: seed.id,
        instrument_id: seed.instrument,
        price_space: seed.price_space,
        kind: seed.kind,
        family: seed.family,
        role: seed.role,
        price: seed.price,
        created_at_ns: seed.now_ns,
        effective_at_ns: seed.now_ns,
        updated_at_ns: seed.now_ns,
        state: LevelState::Developing,
        provenance: LevelProvenance {
            producer_id: seed.producer,
            first_sequence: seed.first_sequence,
            last_sequence: seed.last_sequence,
            derivation_version: seed.derivation_version,
            calendar_id: seed.calendar_id,
            timeframe: Some(seed.timeframe),
            epoch_ns: seed.epoch_ns,
            exclusive_group: seed.exclusive_group,
            ordinal: seed.ordinal,
        },
        metrics: LevelMetrics::default(),
    }
}

fn update_level(level: &mut StructuralLevel, price: i64, bar: &CanonicalBar) {
    level.price = price;
    level.updated_at_ns = bar.ts_close_ns;
    level.provenance.last_sequence = JournalSequence(bar.last_sequence);
    level.metrics.age_bars = level.metrics.age_bars.saturating_add(1);
}

fn validate_bar(bar: &CanonicalBar, instrument: InstrumentId) -> Result<(), ProducerError> {
    if bar.instrument_id != instrument.get() {
        return Err(ProducerError::WrongInstrument {
            expected: instrument,
            actual: InstrumentId(bar.instrument_id),
        });
    }
    if bar.first_sequence == 0 || bar.last_sequence < bar.first_sequence {
        return Err(ProducerError::UncommittedBar);
    }
    if bar.derivation_version == 0 {
        return Err(ProducerError::MissingDerivationVersion);
    }
    if bar.ts_open_ns >= bar.ts_close_ns
        || bar.low > bar.high
        || bar.open < bar.low
        || bar.open > bar.high
        || bar.close < bar.low
        || bar.close > bar.high
    {
        return Err(ProducerError::InvalidBar);
    }
    Ok(())
}

fn decode_timeframe(value: u16) -> Result<Timeframe, ProducerError> {
    match value {
        1 => Ok(Timeframe::M4),
        2 => Ok(Timeframe::M20),
        3 => Ok(Timeframe::H2),
        4 => Ok(Timeframe::H4),
        _ => Err(ProducerError::UnknownTimeframe(value)),
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProducerError {
    #[error("producer expected {expected:?}, got {actual:?}")]
    WrongInstrument {
        expected: InstrumentId,
        actual: InstrumentId,
    },
    #[error("canonical bar is not committed")]
    UncommittedBar,
    #[error("canonical bar has no derivation version")]
    MissingDerivationVersion,
    #[error("canonical bar has invalid time or OHLC geometry")]
    InvalidBar,
    #[error("canonical bar has unknown timeframe {0}")]
    UnknownTimeframe(u16),
    #[error("bar at {bar_open_ns} is outside the versioned reference calendar")]
    OutsideSession { bar_open_ns: i64 },
    #[error("bar stream moved backward from {previous_open_ns} to {actual_open_ns}")]
    OutOfOrder {
        previous_open_ns: i64,
        actual_open_ns: i64,
    },
    #[error("bar correction requires an explicit structural re-derivation generation")]
    RevisionRequiresRederivation,
    #[error("opening range duration must be positive")]
    InvalidOpeningRange,
    #[error("range projection requires at least two bars and ordered fractions")]
    InvalidRangeProjection,
}

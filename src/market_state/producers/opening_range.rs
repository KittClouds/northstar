use super::{
    decode_timeframe, level, update_level, validate_bar, IdSequence, LevelSeed, ProducerError,
};
use crate::data_plane::bars::CanonicalBar;
use crate::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use crate::data_plane::ids::{DerivationVersion, InstrumentId, JournalSequence};
use crate::market_state::{
    LevelFamily, LevelKind, LevelRole, PriceSpace, ProducerId, StructuralLevel, StructuralMutation,
    StructuralObject,
};
use std::sync::Arc;

pub const OPENING_RANGE_PRODUCER_ID: ProducerId = ProducerId(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpeningRangeSpec {
    pub duration_ns: i64,
}

impl OpeningRangeSpec {
    pub const M20: Self = Self {
        duration_ns: 20 * crate::data_plane::bars::MINUTE_NS,
    };
}

#[derive(Clone)]
struct ActiveRange {
    session_open_ns: i64,
    formation_end_ns: i64,
    high: StructuralLevel,
    low: StructuralLevel,
    mid: StructuralLevel,
    frozen: bool,
}

pub struct OpeningRangeProducer {
    instrument: InstrumentId,
    price_space: PriceSpace,
    calendar: Arc<SessionCalendar>,
    derivation_version: DerivationVersion,
    spec: OpeningRangeSpec,
    ids: IdSequence,
    active: Option<ActiveRange>,
    last_bar_open_ns: Option<i64>,
}

impl OpeningRangeProducer {
    pub fn new(
        instrument: InstrumentId,
        price_space: PriceSpace,
        calendar: Arc<SessionCalendar>,
        derivation_version: DerivationVersion,
        spec: OpeningRangeSpec,
    ) -> Result<Self, ProducerError> {
        if spec.duration_ns <= 0 {
            return Err(ProducerError::InvalidOpeningRange);
        }
        Ok(Self {
            instrument,
            price_space,
            calendar,
            derivation_version,
            spec,
            ids: IdSequence::new(OPENING_RANGE_PRODUCER_ID, instrument),
            active: None,
            last_bar_open_ns: None,
        })
    }

    pub fn on_bar(
        &mut self,
        bar: &CanonicalBar,
        output: &mut Vec<StructuralMutation>,
    ) -> Result<(), ProducerError> {
        validate_bar(bar, self.instrument)?;
        if bar.revision > 0 {
            return Err(ProducerError::RevisionRequiresRederivation);
        }
        if self
            .last_bar_open_ns
            .is_some_and(|previous| bar.ts_open_ns <= previous)
        {
            return Err(ProducerError::OutOfOrder {
                previous_open_ns: self.last_bar_open_ns.unwrap_or_default(),
                actual_open_ns: bar.ts_open_ns,
            });
        }
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
        self.last_bar_open_ns = Some(bar.ts_open_ns);

        if self
            .active
            .as_ref()
            .is_some_and(|active| active.session_open_ns != session.open_ns)
        {
            self.expire_active(session.open_ns, output);
        }
        if self.active.is_none() && bar.ts_open_ns < session.open_ns + self.spec.duration_ns {
            self.create(session, bar, output);
        }
        let Some(active) = self.active.as_mut() else {
            return Ok(());
        };
        if active.frozen {
            return Ok(());
        }
        if bar.ts_open_ns >= active.formation_end_ns {
            active.frozen = true;
            for id in [active.high.id, active.low.id, active.mid.id] {
                output.push(StructuralMutation::Frozen {
                    id,
                    at_ns: active.formation_end_ns,
                });
            }
            return Ok(());
        }

        let next_high = active.high.price.max(bar.high);
        let next_low = active.low.price.min(bar.low);
        if next_high != active.high.price || next_low != active.low.price {
            update_level(&mut active.high, next_high, bar);
            update_level(&mut active.low, next_low, bar);
            update_level(&mut active.mid, next_low + (next_high - next_low) / 2, bar);
            output.extend([
                StructuralMutation::Updated(StructuralObject::Level(active.high.clone())),
                StructuralMutation::Updated(StructuralObject::Level(active.low.clone())),
                StructuralMutation::Updated(StructuralObject::Level(active.mid.clone())),
            ]);
        }
        Ok(())
    }

    fn create(
        &mut self,
        session: Session,
        bar: &CanonicalBar,
        output: &mut Vec<StructuralMutation>,
    ) {
        let session_open_ns = session.open_ns;
        let group = (session_open_ns as u64).rotate_left(21) ^ self.spec.duration_ns as u64;
        let timeframe = decode_timeframe(bar.timeframe).expect("engine validates timeframe");
        let seed = |id, kind, role, price, ordinal| LevelSeed {
            id,
            instrument: self.instrument,
            price_space: self.price_space,
            kind,
            family: LevelFamily::OpeningRange,
            role,
            price,
            now_ns: bar.ts_close_ns,
            producer: OPENING_RANGE_PRODUCER_ID,
            calendar_id: self.calendar.id,
            derivation_version: self.derivation_version,
            timeframe,
            epoch_ns: session_open_ns,
            exclusive_group: group,
            ordinal,
            first_sequence: JournalSequence(bar.first_sequence),
            last_sequence: JournalSequence(bar.last_sequence),
        };
        let high = level(seed(
            self.ids.take(),
            LevelKind::OpeningRangeHigh,
            LevelRole::UpperBoundary,
            bar.high,
            0,
        ));
        let low = level(seed(
            self.ids.take(),
            LevelKind::OpeningRangeLow,
            LevelRole::LowerBoundary,
            bar.low,
            1,
        ));
        let mid = level(seed(
            self.ids.take(),
            LevelKind::OpeningRangeMid,
            LevelRole::Center,
            bar.low + (bar.high - bar.low) / 2,
            2,
        ));
        output.extend([
            StructuralMutation::Created(StructuralObject::Level(high.clone())),
            StructuralMutation::Created(StructuralObject::Level(low.clone())),
            StructuralMutation::Created(StructuralObject::Level(mid.clone())),
        ]);
        self.active = Some(ActiveRange {
            session_open_ns,
            formation_end_ns: (session_open_ns + self.spec.duration_ns).min(session.close_ns),
            high,
            low,
            mid,
            frozen: false,
        });
    }

    fn expire_active(&mut self, at_ns: i64, output: &mut Vec<StructuralMutation>) {
        if let Some(active) = self.active.take() {
            if !active.frozen {
                for id in [active.high.id, active.low.id, active.mid.id] {
                    output.push(StructuralMutation::Frozen {
                        id,
                        at_ns: active.formation_end_ns,
                    });
                }
            }
            for id in [active.high.id, active.low.id, active.mid.id] {
                output.push(StructuralMutation::Expired { id, at_ns });
            }
        }
    }
}

use super::{
    decode_timeframe, level, update_level, validate_bar, IdSequence, LevelSeed, ProducerError,
};
use crate::data_plane::bars::CanonicalBar;
use crate::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use crate::data_plane::ids::{DerivationVersion, InstrumentId, JournalSequence};
use crate::market_state::{
    LevelFamily, LevelKind, LevelMetrics, LevelProvenance, LevelRole, LevelState, PriceSpace,
    ProducerId, StructuralBand, StructuralLevel, StructuralMutation, StructuralObject,
};
use std::collections::VecDeque;
use std::sync::Arc;

pub const SESSION_PRODUCER_ID: ProducerId = ProducerId(1);
const RETAINED_DAILY_EXTREMES: usize = 5;

#[derive(Clone)]
struct ActiveSession {
    session: Session,
    high: StructuralLevel,
    low: StructuralLevel,
    upper_band: StructuralBand,
    lower_band: StructuralBand,
    extreme_high: i64,
    extreme_low: i64,
}

pub struct SessionStructureProducer {
    instrument: InstrumentId,
    price_space: PriceSpace,
    calendar: Arc<SessionCalendar>,
    derivation_version: DerivationVersion,
    ids: IdSequence,
    active: Option<ActiveSession>,
    previous_levels: Option<(crate::market_state::LevelId, crate::market_state::LevelId)>,
    completed_bands: VecDeque<(crate::market_state::LevelId, crate::market_state::LevelId)>,
    last_bar_open_ns: Option<i64>,
}

impl SessionStructureProducer {
    pub fn new(
        instrument: InstrumentId,
        price_space: PriceSpace,
        calendar: Arc<SessionCalendar>,
        derivation_version: DerivationVersion,
    ) -> Self {
        Self {
            instrument,
            price_space,
            calendar,
            derivation_version,
            ids: IdSequence::new(SESSION_PRODUCER_ID, instrument),
            active: None,
            previous_levels: None,
            completed_bands: VecDeque::with_capacity(RETAINED_DAILY_EXTREMES + 1),
            last_bar_open_ns: None,
        }
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
            .is_none_or(|active| active.session.open_ns != session.open_ns)
        {
            self.rotate_session(session, bar, output);
        } else {
            self.update_active(bar, output);
        }
        Ok(())
    }

    fn rotate_session(
        &mut self,
        session: Session,
        bar: &CanonicalBar,
        output: &mut Vec<StructuralMutation>,
    ) {
        if let Some(previous) = self.active.take() {
            for id in [previous.high.id, previous.low.id] {
                output.push(StructuralMutation::Frozen {
                    id,
                    at_ns: previous.session.close_ns,
                });
                output.push(StructuralMutation::Expired {
                    id,
                    at_ns: session.open_ns,
                });
            }
            for id in [previous.upper_band.id, previous.lower_band.id] {
                output.push(StructuralMutation::Frozen {
                    id,
                    at_ns: previous.session.close_ns,
                });
            }
            if let Some((high, low)) = self
                .previous_levels
                .replace((self.ids.take(), self.ids.take()))
            {
                output.push(StructuralMutation::Expired {
                    id: high,
                    at_ns: session.open_ns,
                });
                output.push(StructuralMutation::Expired {
                    id: low,
                    at_ns: session.open_ns,
                });
            }
            let (previous_high_id, previous_low_id) = self.previous_levels.expect("just replaced");
            let group = session_group(previous.session.open_ns, 2);
            let mut previous_high = level(self.seed(
                previous_high_id,
                LevelKind::PreviousSessionHigh,
                LevelFamily::Calendar,
                LevelRole::UpperBoundary,
                previous.high.price,
                session.open_ns,
                previous.session.open_ns,
                group,
                0,
                bar,
            ));
            previous_high.provenance.first_sequence = previous.high.provenance.first_sequence;
            previous_high.provenance.last_sequence = previous.high.provenance.last_sequence;
            let mut previous_low = level(self.seed(
                previous_low_id,
                LevelKind::PreviousSessionLow,
                LevelFamily::Calendar,
                LevelRole::LowerBoundary,
                previous.low.price,
                session.open_ns,
                previous.session.open_ns,
                group,
                1,
                bar,
            ));
            previous_low.provenance.first_sequence = previous.low.provenance.first_sequence;
            previous_low.provenance.last_sequence = previous.low.provenance.last_sequence;
            output.push(StructuralMutation::Created(StructuralObject::Level(
                previous_high,
            )));
            output.push(StructuralMutation::Created(StructuralObject::Level(
                previous_low,
            )));
            self.completed_bands
                .push_back((previous.upper_band.id, previous.lower_band.id));
            if self.completed_bands.len() > RETAINED_DAILY_EXTREMES {
                if let Some((upper, lower)) = self.completed_bands.pop_front() {
                    output.push(StructuralMutation::Expired {
                        id: upper,
                        at_ns: session.open_ns,
                    });
                    output.push(StructuralMutation::Expired {
                        id: lower,
                        at_ns: session.open_ns,
                    });
                }
            }
        }

        let active = self.create_active(session, bar);
        output.extend([
            StructuralMutation::Created(StructuralObject::Level(active.high.clone())),
            StructuralMutation::Created(StructuralObject::Level(active.low.clone())),
            StructuralMutation::Created(StructuralObject::Band(active.upper_band.clone())),
            StructuralMutation::Created(StructuralObject::Band(active.lower_band.clone())),
        ]);
        self.active = Some(active);
    }

    fn create_active(&mut self, session: Session, bar: &CanonicalBar) -> ActiveSession {
        let group = session_group(session.open_ns, 1);
        let high_id = self.ids.take();
        let low_id = self.ids.take();
        let upper_band_id = self.ids.take();
        let lower_band_id = self.ids.take();
        let high = level(self.seed(
            high_id,
            LevelKind::CurrentSessionHigh,
            LevelFamily::Session,
            LevelRole::UpperBoundary,
            bar.high,
            bar.ts_close_ns,
            session.open_ns,
            group,
            0,
            bar,
        ));
        let low = level(self.seed(
            low_id,
            LevelKind::CurrentSessionLow,
            LevelFamily::Session,
            LevelRole::LowerBoundary,
            bar.low,
            bar.ts_close_ns,
            session.open_ns,
            group,
            1,
            bar,
        ));
        let upper_band = self.band(
            upper_band_id,
            LevelKind::DailyExtremeUpperBand,
            LevelRole::UpperBoundary,
            bar.open.max(bar.close),
            bar.high,
            bar.high,
            session,
            bar,
        );
        let lower_band = self.band(
            lower_band_id,
            LevelKind::DailyExtremeLowerBand,
            LevelRole::LowerBoundary,
            bar.low,
            bar.open.min(bar.close),
            bar.low,
            session,
            bar,
        );
        ActiveSession {
            session,
            high,
            low,
            upper_band,
            lower_band,
            extreme_high: bar.high,
            extreme_low: bar.low,
        }
    }

    fn update_active(&mut self, bar: &CanonicalBar, output: &mut Vec<StructuralMutation>) {
        let active = self.active.as_mut().expect("active session exists");
        if bar.high > active.extreme_high {
            active.extreme_high = bar.high;
            update_level(&mut active.high, bar.high, bar);
            active.upper_band.lower = bar.open.max(bar.close);
            active.upper_band.upper = bar.high;
            active.upper_band.reference_price = bar.high;
            active.upper_band.updated_at_ns = bar.ts_close_ns;
            active.upper_band.provenance.last_sequence = JournalSequence(bar.last_sequence);
            active.upper_band.metrics.age_bars =
                active.upper_band.metrics.age_bars.saturating_add(1);
            output.push(StructuralMutation::Updated(StructuralObject::Level(
                active.high.clone(),
            )));
            output.push(StructuralMutation::Updated(StructuralObject::Band(
                active.upper_band.clone(),
            )));
        }
        if bar.low < active.extreme_low {
            active.extreme_low = bar.low;
            update_level(&mut active.low, bar.low, bar);
            active.lower_band.lower = bar.low;
            active.lower_band.upper = bar.open.min(bar.close);
            active.lower_band.reference_price = bar.low;
            active.lower_band.updated_at_ns = bar.ts_close_ns;
            active.lower_band.provenance.last_sequence = JournalSequence(bar.last_sequence);
            active.lower_band.metrics.age_bars =
                active.lower_band.metrics.age_bars.saturating_add(1);
            output.push(StructuralMutation::Updated(StructuralObject::Level(
                active.low.clone(),
            )));
            output.push(StructuralMutation::Updated(StructuralObject::Band(
                active.lower_band.clone(),
            )));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn seed(
        &self,
        id: crate::market_state::LevelId,
        kind: LevelKind,
        family: LevelFamily,
        role: LevelRole,
        price: i64,
        now_ns: i64,
        epoch_ns: i64,
        group: u64,
        ordinal: u16,
        bar: &CanonicalBar,
    ) -> LevelSeed {
        LevelSeed {
            id,
            instrument: self.instrument,
            price_space: self.price_space,
            kind,
            family,
            role,
            price,
            now_ns,
            producer: SESSION_PRODUCER_ID,
            calendar_id: self.calendar.id,
            derivation_version: self.derivation_version,
            timeframe: decode_timeframe(bar.timeframe).expect("engine validates timeframe"),
            epoch_ns,
            exclusive_group: group,
            ordinal,
            first_sequence: JournalSequence(bar.first_sequence),
            last_sequence: JournalSequence(bar.last_sequence),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn band(
        &self,
        id: crate::market_state::LevelId,
        kind: LevelKind,
        role: LevelRole,
        lower: i64,
        upper: i64,
        reference_price: i64,
        session: Session,
        bar: &CanonicalBar,
    ) -> StructuralBand {
        StructuralBand {
            id,
            instrument_id: self.instrument,
            price_space: self.price_space,
            kind,
            family: LevelFamily::Calendar,
            role,
            lower,
            upper,
            reference_price,
            created_at_ns: bar.ts_close_ns,
            effective_at_ns: bar.ts_close_ns,
            updated_at_ns: bar.ts_close_ns,
            state: LevelState::Developing,
            provenance: LevelProvenance {
                producer_id: SESSION_PRODUCER_ID,
                first_sequence: JournalSequence(bar.first_sequence),
                last_sequence: JournalSequence(bar.last_sequence),
                derivation_version: self.derivation_version,
                calendar_id: self.calendar.id,
                timeframe: Some(
                    decode_timeframe(bar.timeframe).expect("engine validates timeframe"),
                ),
                epoch_ns: session.open_ns,
                exclusive_group: 0,
                ordinal: 0,
            },
            metrics: LevelMetrics::default(),
        }
    }
}

fn session_group(open_ns: i64, discriminator: u64) -> u64 {
    (open_ns as u64).rotate_left(17) ^ discriminator
}

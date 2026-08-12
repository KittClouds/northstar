use super::{
    decode_timeframe, level, update_level, validate_bar, IdSequence, LevelSeed, ProducerError,
};
use crate::data_plane::bars::CanonicalBar;
use crate::data_plane::ids::{CalendarId, DerivationVersion, InstrumentId, JournalSequence};
use crate::market_state::{
    LevelFamily, LevelKind, LevelRole, PriceSpace, ProducerId, StructuralLevel, StructuralMutation,
    StructuralObject,
};
use std::collections::VecDeque;

pub const RANGE_PROJECTION_PRODUCER_ID: ProducerId = ProducerId(3);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeProjectionSpec {
    pub lookback_bars: usize,
    pub fractions_ppm: Box<[u32]>,
}

impl RangeProjectionSpec {
    pub fn default_m4() -> Self {
        Self {
            lookback_bars: 20,
            fractions_ppm: Box::new([
                0, 213_000, 333_000, 500_000, 666_000, 750_000, 900_000, 1_000_000,
            ]),
        }
    }

    fn validate(&self) -> bool {
        self.lookback_bars >= 2
            && self.fractions_ppm.len() >= 2
            && self.fractions_ppm.first() == Some(&0)
            && self.fractions_ppm.last() == Some(&1_000_000)
            && self.fractions_ppm.windows(2).all(|pair| pair[0] < pair[1])
    }
}

#[derive(Clone, Copy)]
struct BarRange {
    open_ns: i64,
    high: i64,
    low: i64,
    first_sequence: u64,
}

pub struct RangeProjectionProducer {
    instrument: InstrumentId,
    price_space: PriceSpace,
    calendar_id: CalendarId,
    derivation_version: DerivationVersion,
    spec: RangeProjectionSpec,
    ids: IdSequence,
    window: VecDeque<BarRange>,
    levels: Vec<StructuralLevel>,
    last_bar_open_ns: Option<i64>,
}

impl RangeProjectionProducer {
    pub fn new(
        instrument: InstrumentId,
        price_space: PriceSpace,
        calendar_id: CalendarId,
        derivation_version: DerivationVersion,
        spec: RangeProjectionSpec,
    ) -> Result<Self, ProducerError> {
        if !spec.validate() {
            return Err(ProducerError::InvalidRangeProjection);
        }
        Ok(Self {
            instrument,
            price_space,
            calendar_id,
            derivation_version,
            window: VecDeque::with_capacity(spec.lookback_bars + 1),
            levels: Vec::with_capacity(spec.fractions_ppm.len()),
            ids: IdSequence::new(RANGE_PROJECTION_PRODUCER_ID, instrument),
            spec,
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
        self.last_bar_open_ns = Some(bar.ts_open_ns);
        self.window.push_back(BarRange {
            open_ns: bar.ts_open_ns,
            high: bar.high,
            low: bar.low,
            first_sequence: bar.first_sequence,
        });
        if self.window.len() > self.spec.lookback_bars {
            self.window.pop_front();
        }
        if self.window.len() < 2 {
            return Ok(());
        }
        let low = self
            .window
            .iter()
            .map(|item| item.low)
            .min()
            .unwrap_or(bar.low);
        let high = self
            .window
            .iter()
            .map(|item| item.high)
            .max()
            .unwrap_or(bar.high);
        let width = high - low;
        if self.levels.is_empty() {
            self.create_levels(bar, low, width, output);
        } else {
            for (level, fraction) in self
                .levels
                .iter_mut()
                .zip(self.spec.fractions_ppm.iter().copied())
            {
                let price = projected_price(low, width, fraction);
                if level.price != price {
                    update_level(level, price, bar);
                    output.push(StructuralMutation::Updated(StructuralObject::Level(
                        level.clone(),
                    )));
                }
            }
        }
        Ok(())
    }

    fn create_levels(
        &mut self,
        bar: &CanonicalBar,
        low: i64,
        width: i64,
        output: &mut Vec<StructuralMutation>,
    ) {
        let epoch_ns = self
            .window
            .front()
            .map_or(bar.ts_open_ns, |item| item.open_ns);
        let first_sequence = self
            .window
            .front()
            .map_or(bar.first_sequence, |item| item.first_sequence);
        let group = (epoch_ns as u64).rotate_left(29) ^ self.spec.lookback_bars as u64;
        let timeframe = decode_timeframe(bar.timeframe).expect("engine validates timeframe");
        for (ordinal, fraction) in self.spec.fractions_ppm.iter().copied().enumerate() {
            let price = projected_price(low, width, fraction);
            let role = match ordinal {
                0 => LevelRole::LowerBoundary,
                index if index + 1 == self.spec.fractions_ppm.len() => LevelRole::UpperBoundary,
                index if index * 2 == self.spec.fractions_ppm.len() - 1 => LevelRole::Center,
                _ => LevelRole::NeutralReference,
            };
            let item = level(LevelSeed {
                id: self.ids.take(),
                instrument: self.instrument,
                price_space: self.price_space,
                kind: LevelKind::RangeProjection,
                family: LevelFamily::RangeProjection,
                role,
                price,
                now_ns: bar.ts_close_ns,
                producer: RANGE_PROJECTION_PRODUCER_ID,
                calendar_id: self.calendar_id,
                derivation_version: self.derivation_version,
                timeframe,
                epoch_ns,
                exclusive_group: group,
                ordinal: ordinal as u16,
                first_sequence: JournalSequence(first_sequence),
                last_sequence: JournalSequence(bar.last_sequence),
            });
            output.push(StructuralMutation::Created(StructuralObject::Level(
                item.clone(),
            )));
            self.levels.push(item);
        }
    }
}

#[inline]
fn projected_price(low: i64, width: i64, fraction_ppm: u32) -> i64 {
    low + ((width as i128 * fraction_ppm as i128) / 1_000_000) as i64
}

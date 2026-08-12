use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{macro_series, SourceId, StreamId};
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;

pub const CENSUS_SOURCE: SourceId = SourceId(25);
pub const CENSUS_MARTS_STREAM: StreamId = StreamId(1);
pub const CENSUS_MARTS_API_ROOT: &str = "https://api.census.gov/data/timeseries/eits/marts";

const SALES_TYPE: &str = "SM";
const TOTAL_RETAIL_FOOD: &str = "44X72";
const SEASONALLY_ADJUSTED: &str = "yes";

pub const CENSUS_SERIES: [MacroSeriesBinding; 1] = [MacroSeriesBinding {
    provider_code: "SM:44X72:yes",
    series_id: macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA,
    label: "Advance retail and food services sales",
    category: MacroCategory::Growth,
    unit: MacroUnit::MillionsOfDollars,
    measure: MacroMeasure::SeasonallyAdjusted,
    source_url: "https://www.census.gov/retail/sales.html",
    source: CENSUS_SOURCE,
    stream: CENSUS_MARTS_STREAM,
}];

#[derive(Clone, Copy, Debug)]
pub struct CensusMartsDecoder {
    source: SourceId,
    stream: StreamId,
}

impl CensusMartsDecoder {
    pub const fn official() -> Self {
        Self {
            source: CENSUS_SOURCE,
            stream: CENSUS_MARTS_STREAM,
        }
    }
}

impl CanonicalDecoder for CensusMartsDecoder {
    fn source_id(&self) -> SourceId {
        self.source
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        if receipt.source != self.source {
            return Err(DecodeError::WrongSource {
                expected: self.source,
                actual: receipt.source,
            });
        }
        if receipt.stream != self.stream {
            return Err(DecodeError::UnsupportedSchema(format!(
                "unexpected Census stream {}",
                receipt.stream.get()
            )));
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Census returned HTTP {}",
                receipt.status_code
            )));
        }

        let rows: Vec<Vec<&str>> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        let header = rows
            .first()
            .ok_or_else(|| DecodeError::UnsupportedSchema("Census response is empty".into()))?;
        let columns = Columns::from_header(header)?;
        let provider_records = u32::try_from(rows.len().saturating_sub(1))
            .map_err(|_| DecodeError::UnsupportedSchema("too many Census rows".into()))?;
        let mut seen_periods = HashSet::with_capacity(rows.len().saturating_sub(1));
        let before = output.events.len();
        for row in rows.iter().skip(1) {
            columns.validate_width(row)?;
            if row[columns.data_type] != SALES_TYPE
                || row[columns.category] != TOTAL_RETAIL_FOOD
                || row[columns.adjustment] != SEASONALLY_ADJUSTED
            {
                return Err(DecodeError::UnknownIdentity(format!(
                    "{}:{}:{}",
                    row[columns.data_type], row[columns.category], row[columns.adjustment]
                )));
            }
            let period = row[columns.time];
            let (period_start_ns, period_end_ns) = month_window(period)?;
            if !seen_periods.insert(period_start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate Census period {period}"
                )));
            }
            let value = parse_value(row[columns.value])?;
            let vintage = vintage_id(period, row[columns.slot], value);
            let source_event = source_event_id(period_start_ns, vintage);
            let mut event = CanonicalEvent::macro_observation(
                self.source,
                self.stream,
                macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA,
                receipt.id,
                source_event,
                period_start_ns,
                period_end_ns,
                receipt.ts_received_ns,
                receipt.ts_received_ns,
                value,
                vintage,
                None,
                TimeQuality::ObservedLive,
            )?;
            event.header.flags |= EventFlags::FINAL;
            output.push(event);
        }
        output.events[before..].sort_unstable_by_key(|event| event.values[0]);
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many Census events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(
                "Census response contained no MARTS observations".into(),
            ));
        }
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

#[derive(Clone, Copy)]
struct Columns {
    value: usize,
    data_type: usize,
    slot: usize,
    category: usize,
    adjustment: usize,
    time: usize,
    width: usize,
}

impl Columns {
    fn from_header(header: &[&str]) -> Result<Self, DecodeError> {
        let find = |name: &str| {
            header
                .iter()
                .position(|column| *column == name)
                .ok_or_else(|| {
                    DecodeError::UnsupportedSchema(format!("Census response omitted column {name}"))
                })
        };
        Ok(Self {
            value: find("cell_value")?,
            data_type: find("data_type_code")?,
            slot: find("time_slot_id")?,
            category: find("category_code")?,
            adjustment: find("seasonally_adj")?,
            time: find("time")?,
            width: header.len(),
        })
    }

    fn validate_width(self, row: &[&str]) -> Result<(), DecodeError> {
        if row.len() != self.width {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Census row width changed: expected {}, received {}",
                self.width,
                row.len()
            )));
        }
        Ok(())
    }
}

fn parse_value(text: &str) -> Result<f64, DecodeError> {
    let value: f64 = text
        .parse()
        .map_err(|_| DecodeError::InvalidNumber(text.into()))?;
    if !value.is_finite() {
        return Err(DecodeError::InvalidNumber(text.into()));
    }
    Ok(value)
}

fn month_window(text: &str) -> Result<(i64, i64), DecodeError> {
    if text.len() != 7 || text.as_bytes()[4] != b'-' || !text.is_ascii() {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid Census month {text:?}"
        )));
    }
    let year: i64 = text[..4]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid Census month {text:?}")))?;
    let month: i64 = text[5..]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid Census month {text:?}")))?;
    if !(1..=12).contains(&month) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid Census month {text:?}"
        )));
    }
    let start = month_start_ns(year, month)?;
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    Ok((start, month_start_ns(next_year, next_month)?))
}

fn month_start_ns(year: i64, month: i64) -> Result<i64, DecodeError> {
    days_from_civil(year, month, 1)
        .checked_mul(86_400_000_000_000)
        .ok_or(DecodeError::TimestampOverflow)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn vintage_id(period: &str, slot: &str, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-census-marts-vintage-v1",
        period.as_bytes(),
        slot.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(period_start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-census-marts-source-event-v1",
        &macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA
            .get()
            .to_le_bytes(),
        &period_start_ns.to_le_bytes(),
        &vintage.to_le_bytes(),
    ])
}

fn hash_parts(parts: &[&[u8]]) -> u64 {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    u64::from_le_bytes(bytes).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/census/marts_retail_sales.json");

    #[test]
    fn official_matrix_shape_freezes_total_seasonally_adjusted_monthly_sales() {
        let receipt = RawReceiptRef {
            id: ReceiptId(51),
            source: CENSUS_SOURCE,
            stream: CENSUS_MARTS_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: b"GET /data/timeseries/eits/marts binding=SM:44X72:yes",
            payload: FIXTURE,
            payload_hash: *blake3::hash(FIXTURE).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = CensusMartsDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 2);
        assert_eq!(stats.canonical_events, 2);
        assert_eq!(
            batch.events[0].series_id(),
            macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA
        );
        assert_eq!(batch.events[0].macro_value().unwrap(), 757_000.0);
        assert_eq!(batch.events[1].macro_value().unwrap(), 763_700.0);
        assert_eq!(batch.events[0].values[1], batch.events[1].values[0]);
    }
}

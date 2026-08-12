use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{macro_series, SeriesId, SourceId, StreamId};
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;
use serde::Deserialize;

pub const FRED_SOURCE: SourceId = SourceId(23);
pub const FRED_OBSERVATIONS_STREAM: StreamId = StreamId(1);
pub const FRED_API_ROOT: &str = "https://api.stlouisfed.org/fred/series/observations";

pub const FRED_SERIES: [MacroSeriesBinding; 3] = [
    MacroSeriesBinding {
        provider_code: "DFF",
        series_id: macro_series::US_EFFECTIVE_FED_FUNDS_RATE,
        label: "Effective federal funds rate",
        category: MacroCategory::Rates,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::NotSeasonallyAdjusted,
        source_url: "https://fred.stlouisfed.org/series/DFF",
        source: FRED_SOURCE,
        stream: FRED_OBSERVATIONS_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "DGS2",
        series_id: macro_series::US_TREASURY_2Y,
        label: "U.S. Treasury 2-year",
        category: MacroCategory::Rates,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::NotSeasonallyAdjusted,
        source_url: "https://fred.stlouisfed.org/series/DGS2",
        source: FRED_SOURCE,
        stream: FRED_OBSERVATIONS_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "DGS10",
        series_id: macro_series::US_TREASURY_10Y,
        label: "U.S. Treasury 10-year",
        category: MacroCategory::Rates,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::NotSeasonallyAdjusted,
        source_url: "https://fred.stlouisfed.org/series/DGS10",
        source: FRED_SOURCE,
        stream: FRED_OBSERVATIONS_STREAM,
    },
];

pub fn binding_for_code(code: &str) -> Option<&'static MacroSeriesBinding> {
    FRED_SERIES
        .iter()
        .find(|binding| binding.provider_code == code)
}

#[derive(Clone, Copy, Debug)]
pub struct FredSeriesDecoder {
    source: SourceId,
    stream: StreamId,
}

impl FredSeriesDecoder {
    pub const fn official() -> Self {
        Self {
            source: FRED_SOURCE,
            stream: FRED_OBSERVATIONS_STREAM,
        }
    }
}

#[derive(Debug, Deserialize)]
struct WireEnvelope<'a> {
    #[serde(borrow)]
    units: &'a str,
    output_type: u8,
    #[serde(borrow)]
    observations: Vec<WireObservation<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireObservation<'a> {
    #[serde(borrow)]
    realtime_start: &'a str,
    #[serde(borrow)]
    realtime_end: &'a str,
    #[serde(borrow)]
    date: &'a str,
    #[serde(borrow)]
    value: &'a str,
}

impl CanonicalDecoder for FredSeriesDecoder {
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
                "unexpected FRED stream {}",
                receipt.stream.get()
            )));
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "FRED returned HTTP {}",
                receipt.status_code
            )));
        }
        let provider_code = provider_code(receipt.metadata)?;
        let binding = binding_for_code(provider_code)
            .ok_or_else(|| DecodeError::UnknownIdentity(provider_code.into()))?;
        let envelope: WireEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if envelope.units != "lin" || envelope.output_type != 1 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "FRED contract changed: units={:?}, output_type={}",
                envelope.units, envelope.output_type
            )));
        }
        let provider_records = u32::try_from(envelope.observations.len())
            .map_err(|_| DecodeError::UnsupportedSchema("too many FRED observations".into()))?;
        let mut seen_dates = HashSet::with_capacity(envelope.observations.len());
        let before = output.events.len();
        for observation in envelope.observations {
            let period_start_ns = date_start_ns(observation.date)?;
            if !seen_dates.insert(period_start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate FRED date {}",
                    observation.date
                )));
            }
            if observation.value == "." {
                continue;
            }
            let value: f64 = observation
                .value
                .parse()
                .map_err(|_| DecodeError::InvalidNumber(observation.value.into()))?;
            if !value.is_finite() {
                return Err(DecodeError::InvalidNumber(observation.value.into()));
            }
            let period_end_ns = period_start_ns
                .checked_add(86_400_000_000_000)
                .ok_or(DecodeError::TimestampOverflow)?;
            let vintage_id = vintage_id(binding, &observation, value);
            let source_event_id = source_event_id(binding.series_id, period_start_ns, vintage_id);
            let mut event = CanonicalEvent::macro_observation(
                self.source,
                self.stream,
                binding.series_id,
                receipt.id,
                source_event_id,
                period_start_ns,
                period_end_ns,
                receipt.ts_received_ns,
                receipt.ts_received_ns,
                value,
                vintage_id,
                None,
                TimeQuality::ObservedLive,
            )?;
            event.header.flags |= EventFlags::FINAL;
            output.push(event);
        }
        output.events[before..].sort_unstable_by_key(|event| event.values[0]);
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many FRED events".into()))?;
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn provider_code(metadata: &[u8]) -> Result<&str, DecodeError> {
    let metadata =
        std::str::from_utf8(metadata).map_err(|error| DecodeError::Malformed(error.to_string()))?;
    metadata
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("series_id="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("FRED metadata omitted series_id".into()))
}

fn date_start_ns(text: &str) -> Result<i64, DecodeError> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid FRED date {text:?}"
        )));
    }
    let year = i64::from(parse_digits(&bytes[0..4], text)?);
    let month = i64::from(parse_digits(&bytes[5..7], text)?);
    let day = i64::from(parse_digits(&bytes[8..10], text)?);
    if !(1900..=2300).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=i64::from(days_in_month(year as u32, month as u32))).contains(&day)
    {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid FRED date {text:?}"
        )));
    }
    days_from_civil(year, month, day)
        .checked_mul(86_400_000_000_000)
        .ok_or(DecodeError::TimestampOverflow)
}

fn parse_digits(bytes: &[u8], original: &str) -> Result<u32, DecodeError> {
    bytes.iter().try_fold(0u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + u32::from(byte - b'0'))
            .ok_or_else(|| {
                DecodeError::UnsupportedSchema(format!("invalid FRED date {original:?}"))
            })
    })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
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

fn vintage_id(binding: &MacroSeriesBinding, observation: &WireObservation<'_>, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-fred-vintage-v1",
        binding.provider_code.as_bytes(),
        observation.date.as_bytes(),
        observation.realtime_start.as_bytes(),
        observation.realtime_end.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, period_start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-fred-source-event-v1",
        &series.get().to_le_bytes(),
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

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/fred/dgs10.json");

    #[test]
    fn official_observation_shape_preserves_dates_missing_values_and_vintage() {
        let receipt = RawReceiptRef {
            id: ReceiptId(31),
            source: FRED_SOURCE,
            stream: FRED_OBSERVATIONS_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: b"GET /fred/series/observations series_id=DGS10",
            payload: FIXTURE,
            payload_hash: *blake3::hash(FIXTURE).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = FredSeriesDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 3);
        assert_eq!(stats.canonical_events, 2);
        assert_eq!(batch.events[0].series_id(), macro_series::US_TREASURY_10Y);
        assert_eq!(batch.events[0].macro_value().unwrap(), 4.21);
        assert_eq!(batch.events[1].macro_value().unwrap(), 4.18);
        assert_eq!(batch.events[0].header.ts_effective_ns, 200);
    }
}

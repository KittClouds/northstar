use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{macro_series, SeriesId, SourceId, StreamId};
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use serde::Deserialize;
use smallvec::SmallVec;

pub const BLS_SOURCE: SourceId = SourceId(20);
pub const BLS_TIMESERIES_STREAM: StreamId = StreamId(1);
pub const BLS_API_ROOT: &str = "https://api.bls.gov/publicAPI/v1/timeseries/data";

pub const BLS_SERIES: [MacroSeriesBinding; 4] = [
    MacroSeriesBinding {
        provider_code: "CUUR0000SA0",
        series_id: macro_series::US_CPI_ALL_ITEMS_NSA,
        label: "CPI / all items",
        category: MacroCategory::Inflation,
        unit: MacroUnit::Index,
        measure: MacroMeasure::NotSeasonallyAdjusted,
        source_url: "https://data.bls.gov/timeseries/CUUR0000SA0",
        source: BLS_SOURCE,
        stream: BLS_TIMESERIES_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "LNS14000000",
        series_id: macro_series::US_UNEMPLOYMENT_RATE_SA,
        label: "Unemployment rate",
        category: MacroCategory::Labor,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::SeasonallyAdjusted,
        source_url: "https://data.bls.gov/timeseries/LNS14000000",
        source: BLS_SOURCE,
        stream: BLS_TIMESERIES_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "CES0000000001",
        series_id: macro_series::US_TOTAL_NONFARM_PAYROLLS_SA,
        label: "Total nonfarm payrolls",
        category: MacroCategory::Labor,
        unit: MacroUnit::Thousands,
        measure: MacroMeasure::SeasonallyAdjusted,
        source_url: "https://data.bls.gov/timeseries/CES0000000001",
        source: BLS_SOURCE,
        stream: BLS_TIMESERIES_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "CES0500000003",
        series_id: macro_series::US_AVERAGE_HOURLY_EARNINGS_SA,
        label: "Average hourly earnings",
        category: MacroCategory::Labor,
        unit: MacroUnit::DollarsPerHour,
        measure: MacroMeasure::SeasonallyAdjusted,
        source_url: "https://data.bls.gov/timeseries/CES0500000003",
        source: BLS_SOURCE,
        stream: BLS_TIMESERIES_STREAM,
    },
];

pub fn binding_for_code(code: &str) -> Option<&'static MacroSeriesBinding> {
    BLS_SERIES
        .iter()
        .find(|binding| binding.provider_code == code)
}

#[derive(Clone, Copy, Debug)]
pub struct BlsSeriesDecoder {
    source: SourceId,
    stream: StreamId,
}

impl BlsSeriesDecoder {
    pub const fn official() -> Self {
        Self {
            source: BLS_SOURCE,
            stream: BLS_TIMESERIES_STREAM,
        }
    }

    pub const fn new(source: SourceId, stream: StreamId) -> Self {
        Self { source, stream }
    }
}

#[derive(Debug, Deserialize)]
struct WireEnvelope<'a> {
    #[serde(borrow)]
    status: &'a str,
    #[serde(default, borrow)]
    message: Vec<&'a str>,
    #[serde(rename = "Results", borrow)]
    results: Option<WireResults<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireResults<'a> {
    #[serde(borrow)]
    series: Vec<WireSeries<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireSeries<'a> {
    #[serde(rename = "seriesID", borrow)]
    series_id: &'a str,
    #[serde(borrow)]
    data: Vec<WireObservation<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireObservation<'a> {
    #[serde(borrow)]
    year: &'a str,
    #[serde(borrow)]
    period: &'a str,
    #[serde(borrow)]
    value: &'a str,
    #[serde(default, borrow)]
    footnotes: Vec<WireFootnote<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireFootnote<'a> {
    #[serde(default, borrow)]
    code: Option<&'a str>,
    #[serde(default, borrow)]
    text: Option<&'a str>,
}

impl CanonicalDecoder for BlsSeriesDecoder {
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
                "unexpected BLS stream {}",
                receipt.stream.get()
            )));
        }
        let envelope: WireEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if envelope.status != "REQUEST_SUCCEEDED" {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BLS request failed: {}",
                envelope.message.join(" | ")
            )));
        }
        let results = envelope
            .results
            .ok_or_else(|| DecodeError::UnsupportedSchema("BLS response omitted Results".into()))?;
        if results.series.len() != 1 {
            return Err(DecodeError::UnsupportedSchema(
                "one BLS receipt must contain exactly one bound series".into(),
            ));
        }
        let series = &results.series[0];
        let binding = binding_for_code(series.series_id)
            .ok_or_else(|| DecodeError::UnknownIdentity(series.series_id.into()))?;
        let provider_records = u32::try_from(series.data.len())
            .map_err(|_| DecodeError::UnsupportedSchema("too many BLS observations".into()))?;
        let mut seen_periods: SmallVec<[(i32, u8); 32]> = SmallVec::new();
        let before = output.events.len();
        for observation in series.data.iter().rev() {
            let year = parse_year(observation.year)?;
            let Some(month) = parse_month(observation.period)? else {
                continue;
            };
            if seen_periods.contains(&(year, month)) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate BLS period {}-{:02}",
                    year, month
                )));
            }
            seen_periods.push((year, month));
            let Some(value) = parse_value(observation.value)? else {
                continue;
            };
            let period_start_ns = month_start_ns(year, month)?;
            let (next_year, next_month) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
            let period_end_ns = month_start_ns(next_year, next_month)?;
            let vintage_id = vintage_id(binding, observation, value);
            let source_event_id = source_event_id(binding.series_id, year, month, vintage_id);
            let preliminary = observation
                .footnotes
                .iter()
                .any(|footnote| footnote.code == Some("P"));
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
            event.header.flags |= if preliminary {
                EventFlags::PROVISIONAL
            } else {
                EventFlags::FINAL
            };
            output.push(event);
        }
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many BLS events".into()))?;
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn parse_year(text: &str) -> Result<i32, DecodeError> {
    let year: i32 = text
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid BLS year {text:?}")))?;
    if !(1900..=2300).contains(&year) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "BLS year is outside supported range: {year}"
        )));
    }
    Ok(year)
}

fn parse_month(period: &str) -> Result<Option<u8>, DecodeError> {
    let Some(month) = period.strip_prefix('M') else {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid BLS period {period:?}"
        )));
    };
    let month: u8 = month
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid BLS period {period:?}")))?;
    match month {
        1..=12 => Ok(Some(month)),
        13 => Ok(None),
        _ => Err(DecodeError::UnsupportedSchema(format!(
            "unsupported BLS month {month}"
        ))),
    }
}

fn parse_value(text: &str) -> Result<Option<f64>, DecodeError> {
    if text == "-" {
        return Ok(None);
    }
    let value: f64 = text
        .parse()
        .map_err(|_| DecodeError::InvalidNumber(text.into()))?;
    if !value.is_finite() {
        return Err(DecodeError::InvalidNumber(text.into()));
    }
    Ok(Some(value))
}

fn vintage_id(binding: &MacroSeriesBinding, observation: &WireObservation<'_>, value: f64) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-bls-vintage-v1");
    hasher.update(binding.provider_code.as_bytes());
    hasher.update(observation.year.as_bytes());
    hasher.update(observation.period.as_bytes());
    hasher.update(&value.to_bits().to_le_bytes());
    for footnote in &observation.footnotes {
        hasher.update(footnote.code.unwrap_or_default().as_bytes());
        hasher.update(footnote.text.unwrap_or_default().as_bytes());
    }
    nonzero_hash64(hasher.finalize().as_bytes())
}

fn source_event_id(series: SeriesId, year: i32, month: u8, vintage: u64) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-bls-source-event-v1");
    hasher.update(&series.get().to_le_bytes());
    hasher.update(&year.to_le_bytes());
    hasher.update(&[month]);
    hasher.update(&vintage.to_le_bytes());
    nonzero_hash64(hasher.finalize().as_bytes())
}

fn nonzero_hash64(hash: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash[..8]);
    u64::from_le_bytes(bytes).max(1)
}

fn month_start_ns(year: i32, month: u8) -> Result<i64, DecodeError> {
    let days = days_from_civil(year as i64, month as i64, 1);
    days.checked_mul(86_400)
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .ok_or(DecodeError::TimestampOverflow)
}

/// Howard Hinnant's civil-date transform, expressed with Euclidean division
/// so pre-1970 dates remain deterministic.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/bls/cpi_v1.json");

    #[test]
    fn official_shape_preserves_numeric_periods_and_missing_observations() {
        let decoder = BlsSeriesDecoder::official();
        let receipt = RawReceiptRef {
            id: ReceiptId(7),
            source: BLS_SOURCE,
            stream: BLS_TIMESERIES_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 1_000,
            ts_received_ns: 2_000,
            metadata: b"series=CUUR0000SA0",
            payload: FIXTURE,
            payload_hash: *blake3::hash(FIXTURE).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = decoder.decode(receipt, &mut batch).unwrap();
        assert_eq!(stats.provider_records, 4);
        assert_eq!(stats.canonical_events, 2);
        assert_eq!(
            batch.events[0].series_id(),
            macro_series::US_CPI_ALL_ITEMS_NSA
        );
        assert_eq!(batch.events[0].macro_value().unwrap(), 335.123);
        assert_eq!(batch.events[1].macro_value().unwrap(), 333.952);
        assert!(batch.events[1].flags().contains(EventFlags::PROVISIONAL));
        assert_eq!(batch.events[1].header.ts_effective_ns, 2_000);
    }

    #[test]
    fn civil_month_boundaries_are_exact() {
        assert_eq!(month_start_ns(1970, 1).unwrap(), 0);
        assert_eq!(
            month_start_ns(2026, 7).unwrap() - month_start_ns(2026, 6).unwrap(),
            30 * 86_400 * 1_000_000_000
        );
    }
}

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

pub const BEA_SOURCE: SourceId = SourceId(24);
pub const BEA_NIPA_STREAM: StreamId = StreamId(1);
pub const BEA_API_ROOT: &str = "https://apps.bea.gov/api/data";

pub const BEA_SERIES: [MacroSeriesBinding; 2] = [
    MacroSeriesBinding {
        provider_code: "T10101:1:Q",
        series_id: macro_series::US_REAL_GDP_QOQ_ANNUALIZED,
        label: "Real GDP growth",
        category: MacroCategory::Growth,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::QuarterlyAnnualized,
        source_url: "https://apps.bea.gov/iTable/?reqid=19&step=3&isuri=1&nipa_table_list=5",
        source: BEA_SOURCE,
        stream: BEA_NIPA_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "T20804:6:M",
        series_id: macro_series::US_CORE_PCE_PRICE_INDEX_SA,
        label: "Core PCE price index",
        category: MacroCategory::Inflation,
        unit: MacroUnit::Index,
        measure: MacroMeasure::SeasonallyAdjusted,
        source_url: "https://apps.bea.gov/iTable/?reqid=19&step=3&isuri=1&nipa_table_list=84",
        source: BEA_SOURCE,
        stream: BEA_NIPA_STREAM,
    },
];

pub fn binding_for_code(code: &str) -> Option<&'static MacroSeriesBinding> {
    BEA_SERIES
        .iter()
        .find(|binding| binding.provider_code == code)
}

#[derive(Clone, Copy, Debug)]
pub struct BeaNipaDecoder {
    source: SourceId,
    stream: StreamId,
}

impl BeaNipaDecoder {
    pub const fn official() -> Self {
        Self {
            source: BEA_SOURCE,
            stream: BEA_NIPA_STREAM,
        }
    }
}

#[derive(Debug, Deserialize)]
struct WireEnvelope<'a> {
    #[serde(rename = "BEAAPI", borrow)]
    api: WireApi<'a>,
}

#[derive(Debug, Deserialize)]
struct WireApi<'a> {
    #[serde(rename = "Results", borrow)]
    results: WireResults<'a>,
}

#[derive(Debug, Deserialize)]
struct WireResults<'a> {
    #[serde(rename = "Data", default, borrow)]
    data: Vec<WireObservation<'a>>,
    #[serde(rename = "Error", default, borrow)]
    error: Option<WireError<'a>>,
}

#[derive(Debug, Deserialize)]
struct WireError<'a> {
    #[serde(rename = "APIErrorCode", borrow)]
    code: &'a str,
    #[serde(rename = "APIErrorDescription", borrow)]
    description: &'a str,
}

#[derive(Debug, Deserialize)]
struct WireObservation<'a> {
    #[serde(rename = "TableName", borrow)]
    table_name: &'a str,
    #[serde(rename = "SeriesCode", default, borrow)]
    series_code: &'a str,
    #[serde(rename = "LineNumber", borrow)]
    line_number: &'a str,
    #[serde(rename = "TimePeriod", borrow)]
    time_period: &'a str,
    #[serde(rename = "CL_UNIT", borrow)]
    unit: &'a str,
    #[serde(rename = "UNIT_MULT", borrow)]
    unit_multiplier: &'a str,
    #[serde(rename = "DataValue", borrow)]
    value: &'a str,
    #[serde(rename = "NoteRef", default, borrow)]
    note_ref: &'a str,
}

impl CanonicalDecoder for BeaNipaDecoder {
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
                "unexpected BEA stream {}",
                receipt.stream.get()
            )));
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BEA returned HTTP {}",
                receipt.status_code
            )));
        }
        let provider_code = provider_code(receipt.metadata)?;
        let binding = binding_for_code(provider_code)
            .ok_or_else(|| DecodeError::UnknownIdentity(provider_code.into()))?;
        let (expected_table, expected_line, expected_frequency) = binding_parts(provider_code)?;
        let envelope: WireEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if let Some(error) = envelope.api.results.error {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BEA API error {}: {}",
                error.code, error.description
            )));
        }
        let provider_records = u32::try_from(envelope.api.results.data.len())
            .map_err(|_| DecodeError::UnsupportedSchema("too many BEA observations".into()))?;
        let mut seen_periods = HashSet::with_capacity(envelope.api.results.data.len());
        let before = output.events.len();
        for observation in envelope.api.results.data {
            if observation.table_name != expected_table || observation.line_number != expected_line
            {
                continue;
            }
            validate_unit(binding, &observation)?;
            let (period_start_ns, period_end_ns) =
                period_window(observation.time_period, expected_frequency)?;
            if !seen_periods.insert(period_start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate BEA period {}",
                    observation.time_period
                )));
            }
            let value = parse_value(observation.value)?;
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
            .map_err(|_| DecodeError::UnsupportedSchema("too many BEA events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BEA response contained no bound rows for {provider_code}"
            )));
        }
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
        .find_map(|field| field.strip_prefix("binding="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("BEA metadata omitted binding".into()))
}

fn binding_parts(code: &str) -> Result<(&str, &str, &str), DecodeError> {
    let mut parts = code.split(':');
    let table = parts.next();
    let line = parts.next();
    let frequency = parts.next();
    if parts.next().is_some() || table.is_none() || line.is_none() || frequency.is_none() {
        return Err(DecodeError::UnknownIdentity(code.into()));
    }
    Ok((table.unwrap(), line.unwrap(), frequency.unwrap()))
}

fn validate_unit(
    binding: &MacroSeriesBinding,
    observation: &WireObservation<'_>,
) -> Result<(), DecodeError> {
    let expected = match binding.unit {
        MacroUnit::Percent => "Percent",
        MacroUnit::Index => "Index",
        _ => {
            return Err(DecodeError::UnsupportedSchema(
                "unsupported BEA unit binding".into(),
            ))
        }
    };
    if observation.unit != expected || observation.unit_multiplier != "0" {
        return Err(DecodeError::UnsupportedSchema(format!(
            "BEA unit contract changed: unit={:?}, multiplier={:?}",
            observation.unit, observation.unit_multiplier
        )));
    }
    Ok(())
}

fn parse_value(text: &str) -> Result<f64, DecodeError> {
    let normalized;
    let text = if text.as_bytes().contains(&b',') {
        normalized = text.replace(',', "");
        normalized.as_str()
    } else {
        text
    };
    let value: f64 = text
        .parse()
        .map_err(|_| DecodeError::InvalidNumber(text.into()))?;
    if !value.is_finite() {
        return Err(DecodeError::InvalidNumber(text.into()));
    }
    Ok(value)
}

fn period_window(text: &str, frequency: &str) -> Result<(i64, i64), DecodeError> {
    if !text.is_ascii() || text.len() < 6 {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid BEA period {text:?}"
        )));
    }
    let year: i64 = text[..4]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid BEA period {text:?}")))?;
    let (month, span_months) = match frequency {
        "Q" if text.len() == 6 && text.as_bytes()[4] == b'Q' => {
            let digit = text.as_bytes()[5];
            if !digit.is_ascii_digit() {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid BEA period {text:?}"
                )));
            }
            let quarter = digit - b'0';
            if !(1..=4).contains(&quarter) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid BEA period {text:?}"
                )));
            }
            (i64::from((quarter - 1) * 3 + 1), 3)
        }
        "M" if text.len() == 7 && text.as_bytes()[4] == b'M' => {
            let month: i64 = text[5..7].parse().map_err(|_| {
                DecodeError::UnsupportedSchema(format!("invalid BEA period {text:?}"))
            })?;
            if !(1..=12).contains(&month) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid BEA period {text:?}"
                )));
            }
            (month, 1)
        }
        _ => {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BEA frequency mismatch {text:?}"
            )))
        }
    };
    let start = month_start_ns(year, month)?;
    let next_month = month + span_months;
    let (end_year, end_month) = if next_month > 12 {
        (year + 1, next_month - 12)
    } else {
        (year, next_month)
    };
    Ok((start, month_start_ns(end_year, end_month)?))
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

fn vintage_id(binding: &MacroSeriesBinding, observation: &WireObservation<'_>, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-bea-vintage-v1",
        binding.provider_code.as_bytes(),
        observation.series_code.as_bytes(),
        observation.time_period.as_bytes(),
        observation.note_ref.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, period_start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-bea-source-event-v1",
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

    const GDP_FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/bea/gdp.json");
    const CORE_PCE_FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/bea/core_pce.json");

    fn decode_fixture(binding: &'static str, payload: &'static [u8]) -> CanonicalBatch {
        let metadata = format!("GET /api/data binding={binding}");
        let receipt = RawReceiptRef {
            id: ReceiptId(41),
            source: BEA_SOURCE,
            stream: BEA_NIPA_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: metadata.as_bytes(),
            payload,
            payload_hash: *blake3::hash(payload).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = BeaNipaDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 3);
        assert_eq!(stats.canonical_events, 2);
        batch
    }

    #[test]
    fn documented_nipa_shape_selects_only_frozen_line_and_preserves_revision_identity() {
        let batch = decode_fixture("T10101:1:Q", GDP_FIXTURE);
        assert_eq!(
            batch.events[0].series_id(),
            macro_series::US_REAL_GDP_QOQ_ANNUALIZED
        );
        assert_eq!(batch.events[0].macro_value().unwrap(), 3.8);
        assert_eq!(batch.events[1].macro_value().unwrap(), -0.5);
    }

    #[test]
    fn core_pce_contract_selects_only_line_six_and_monthly_periods() {
        let batch = decode_fixture("T20804:6:M", CORE_PCE_FIXTURE);
        assert_eq!(
            batch.events[0].series_id(),
            macro_series::US_CORE_PCE_PRICE_INDEX_SA
        );
        assert_eq!(batch.events[0].macro_value().unwrap(), 124.317);
        assert_eq!(batch.events[1].macro_value().unwrap(), 124.529);
        assert_eq!(batch.events[0].values[1], batch.events[1].values[0]);
    }
}

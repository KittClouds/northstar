use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{documents, macro_series, DocumentId, SeriesId, SourceId, StreamId};
use crate::data_plane::provenance::DocumentKind;
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;
use serde::Deserialize;

pub const BOJ_SOURCE: SourceId = SourceId(29);
pub const BOJ_TIMESERIES_STREAM: StreamId = StreamId(1);
pub const BOJ_API_ROOT: &str = "https://www.stat-search.boj.or.jp/api/v1/getDataCode";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BojFrequency {
    Daily,
    Quarterly,
}

#[derive(Clone, Copy, Debug)]
pub struct BojApiBinding {
    pub catalog: MacroSeriesBinding,
    pub database: &'static str,
    pub provider_name: &'static str,
    pub provider_unit: &'static str,
    pub provider_frequency: &'static str,
    pub frequency: BojFrequency,
    pub document: DocumentId,
}

pub const BOJ_BINDINGS: [BojApiBinding; 2] = [
    BojApiBinding {
        catalog: MacroSeriesBinding {
            provider_code: "STRDCLUCON",
            series_id: macro_series::JP_OVERNIGHT_CALL_RATE,
            label: "Japan uncollateralized overnight call rate, average",
            category: MacroCategory::Rates,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::MarketRate,
            source_url: "https://www.stat-search.boj.or.jp/info/api_manual_en.pdf",
            source: BOJ_SOURCE,
            stream: BOJ_TIMESERIES_STREAM,
        },
        database: "FM01",
        provider_name: "Call Rate, Uncollateralized Overnight, Average (Daily)",
        provider_unit: "percent per annum",
        provider_frequency: "DAILY",
        frequency: BojFrequency::Daily,
        document: documents::BOJ_CALL_RATE_TIMESERIES,
    },
    BojApiBinding {
        catalog: MacroSeriesBinding {
            provider_code: "TK99F1000601GCQ01000",
            series_id: macro_series::JP_TANKAN_LARGE_MANUFACTURING_CONDITIONS,
            label: "Tankan business conditions, large manufacturers, actual",
            category: MacroCategory::Growth,
            unit: MacroUnit::PercentagePoints,
            measure: MacroMeasure::DiffusionIndex,
            source_url: "https://www.stat-search.boj.or.jp/ssi/mtshtml/co_q_1_en.html",
            source: BOJ_SOURCE,
            stream: BOJ_TIMESERIES_STREAM,
        },
        database: "CO",
        provider_name: "D.I./Business Conditions/Large Enterprises/Manufacturing/Actual result",
        provider_unit: "% points",
        provider_frequency: "QUARTERLY",
        frequency: BojFrequency::Quarterly,
        document: documents::BOJ_TANKAN_TIMESERIES,
    },
];

pub const BOJ_SERIES: [MacroSeriesBinding; 2] = [BOJ_BINDINGS[0].catalog, BOJ_BINDINGS[1].catalog];

pub fn binding_for_code(provider_code: &str) -> Option<&'static BojApiBinding> {
    BOJ_BINDINGS
        .iter()
        .find(|binding| binding.catalog.provider_code == provider_code)
}

#[derive(Clone, Copy, Debug)]
pub struct BojTimeseriesDecoder;

impl BojTimeseriesDecoder {
    pub const fn official() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Response<'a> {
    #[serde(rename = "STATUS")]
    status: u16,
    #[serde(rename = "MESSAGE")]
    message: &'a str,
    #[serde(rename = "DATE")]
    response_date: &'a str,
    #[serde(rename = "PARAMETER")]
    parameter: Parameter<'a>,
    #[serde(rename = "NEXTPOSITION")]
    next_position: Option<serde_json::Value>,
    #[serde(rename = "RESULTSET", borrow)]
    result_set: Vec<ResultSeries<'a>>,
}

#[derive(Deserialize)]
struct Parameter<'a> {
    #[serde(rename = "FORMAT")]
    format: &'a str,
    #[serde(rename = "LANG")]
    language: &'a str,
    #[serde(rename = "DB")]
    database: &'a str,
}

#[derive(Deserialize)]
struct ResultSeries<'a> {
    #[serde(rename = "SERIES_CODE")]
    code: &'a str,
    #[serde(rename = "NAME_OF_TIME_SERIES")]
    name: &'a str,
    #[serde(rename = "UNIT")]
    unit: &'a str,
    #[serde(rename = "FREQUENCY")]
    frequency: &'a str,
    #[serde(rename = "LAST_UPDATE")]
    last_update: i64,
    #[serde(rename = "VALUES")]
    values: Values,
}

#[derive(Deserialize)]
struct Values {
    #[serde(rename = "SURVEY_DATES")]
    dates: Vec<i64>,
    #[serde(rename = "VALUES")]
    values: Vec<Option<f64>>,
}

impl CanonicalDecoder for BojTimeseriesDecoder {
    fn source_id(&self) -> SourceId {
        BOJ_SOURCE
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        validate_receipt(receipt)?;
        let code = metadata_binding(receipt.metadata)?;
        let binding =
            binding_for_code(code).ok_or_else(|| DecodeError::UnknownIdentity(code.into()))?;
        let response: Response<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        validate_response(&response, binding)?;
        let series = &response.result_set[0];
        if series.values.dates.len() != series.values.values.len() {
            return Err(DecodeError::UnsupportedSchema(
                "BOJ date/value arrays differ in length".into(),
            ));
        }
        let before = output.events.len();
        let mut seen = HashSet::with_capacity(series.values.dates.len());
        for (&period, value) in series.values.dates.iter().zip(&series.values.values) {
            let Some(value) = *value else {
                continue;
            };
            if !value.is_finite() {
                return Err(DecodeError::InvalidNumber(value.to_string()));
            }
            let (start_ns, end_ns) = period_window(period, binding.frequency)?;
            if !seen.insert(start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate BOJ period {period}"
                )));
            }
            let vintage = vintage_id(binding, series.last_update, period, value);
            let source_event = source_event_id(binding.catalog.series_id, start_ns, vintage);
            let mut event = CanonicalEvent::macro_observation(
                BOJ_SOURCE,
                BOJ_TIMESERIES_STREAM,
                binding.catalog.series_id,
                receipt.id,
                source_event,
                start_ns,
                end_ns,
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
        let document_event = hash_parts(&[
            b"northstar-boj-document-v1",
            &binding.document.get().to_le_bytes(),
            receipt.payload_hash.as_slice(),
        ]);
        output.push(CanonicalEvent::source_document(
            BOJ_SOURCE,
            BOJ_TIMESERIES_STREAM,
            binding.document,
            receipt.id,
            document_event,
            receipt.ts_received_ns,
            receipt.ts_received_ns,
            receipt.payload_hash,
            DocumentKind::DatasetSnapshot,
            None,
        )?);
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many BOJ events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(
                "BOJ response contained no published observations".into(),
            ));
        }
        Ok(DecodeStats {
            provider_records: u32::try_from(series.values.dates.len())
                .map_err(|_| DecodeError::UnsupportedSchema("too many BOJ rows".into()))?,
            canonical_events,
        })
    }
}

fn validate_receipt(receipt: RawReceiptRef<'_>) -> Result<(), DecodeError> {
    if receipt.source != BOJ_SOURCE {
        return Err(DecodeError::WrongSource {
            expected: BOJ_SOURCE,
            actual: receipt.source,
        });
    }
    if receipt.stream != BOJ_TIMESERIES_STREAM || receipt.status_code != 200 {
        return Err(DecodeError::UnsupportedSchema(format!(
            "unexpected BOJ stream/status {}/{}",
            receipt.stream.get(),
            receipt.status_code
        )));
    }
    Ok(())
}

fn validate_response(response: &Response<'_>, binding: &BojApiBinding) -> Result<(), DecodeError> {
    if response.status != 200
        || response.message != "Successfully completed"
        || response.response_date.is_empty()
        || response.parameter.format != "JSON"
        || response.parameter.language != "EN"
        || response.parameter.database != binding.database
        || response.next_position.is_some()
        || response.result_set.len() != 1
    {
        return Err(DecodeError::UnsupportedSchema(
            "BOJ response envelope changed or is paginated".into(),
        ));
    }
    let series = &response.result_set[0];
    if series.code != binding.catalog.provider_code
        || series.name != binding.provider_name
        || series.unit != binding.provider_unit
        || series.frequency != binding.provider_frequency
    {
        return Err(DecodeError::UnknownIdentity(series.code.into()));
    }
    Ok(())
}

fn metadata_binding(metadata: &[u8]) -> Result<&str, DecodeError> {
    std::str::from_utf8(metadata)
        .map_err(|error| DecodeError::Malformed(error.to_string()))?
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("binding="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("BOJ metadata omitted binding".into()))
}

fn period_window(period: i64, frequency: BojFrequency) -> Result<(i64, i64), DecodeError> {
    match frequency {
        BojFrequency::Daily => {
            let year = period / 10_000;
            let month = period / 100 % 100;
            let day = period % 100;
            validate_ymd(year, month, day, period)?;
            let start = civil_ns(year, month, day);
            Ok((start, start.saturating_add(86_400_000_000_000)))
        }
        BojFrequency::Quarterly => {
            let year = period / 100;
            let quarter = period % 100;
            if !(1900..=2300).contains(&year) || !(1..=4).contains(&quarter) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid BOJ quarter {period}"
                )));
            }
            let month = (quarter - 1) * 3 + 1;
            Ok((civil_ns(year, month, 1), civil_ns(year, month + 3, 1)))
        }
    }
}

fn validate_ymd(year: i64, month: i64, day: i64, raw: i64) -> Result<(), DecodeError> {
    if !(1900..=2300).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid BOJ date {raw}"
        )));
    }
    Ok(())
}

fn civil_ns(year: i64, month: i64, day: i64) -> i64 {
    let normalized_year = year - i64::from(month <= 2);
    let era = normalized_year.div_euclid(400);
    let year_of_era = normalized_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era - 719_468) * 86_400_000_000_000
}

fn vintage_id(binding: &BojApiBinding, update: i64, period: i64, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-boj-vintage-v1",
        binding.database.as_bytes(),
        binding.catalog.provider_code.as_bytes(),
        &update.to_le_bytes(),
        &period.to_le_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-boj-source-event-v1",
        &series.get().to_le_bytes(),
        &start_ns.to_le_bytes(),
        &vintage.to_le_bytes(),
    ])
}

fn hash_parts(parts: &[&[u8]]) -> u64 {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    u64::from_le_bytes(bytes).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    fn decode(code: &'static str, fixture: &'static [u8]) -> CanonicalBatch {
        let metadata: &'static [u8] = format!("GET /api/v1/getDataCode binding={code}")
            .leak()
            .as_bytes();
        let receipt = RawReceiptRef {
            id: ReceiptId(91),
            source: BOJ_SOURCE,
            stream: BOJ_TIMESERIES_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata,
            payload: fixture,
            payload_hash: *blake3::hash(fixture).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        BojTimeseriesDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        batch
    }

    #[test]
    fn freezes_daily_call_rate_and_skips_unpublished_days() {
        let batch = decode(
            "STRDCLUCON",
            include_bytes!("../../../tests/fixtures/boj/call_rate.json"),
        );
        assert_eq!(batch.events.len(), 4);
        assert_eq!(
            batch.events[2].series_id(),
            macro_series::JP_OVERNIGHT_CALL_RATE
        );
        assert_eq!(batch.events[2].macro_value().unwrap(), 0.98);
    }

    #[test]
    fn freezes_quarterly_tankan_identity() {
        let batch = decode(
            "TK99F1000601GCQ01000",
            include_bytes!("../../../tests/fixtures/boj/tankan.json"),
        );
        assert_eq!(batch.events.len(), 4);
        assert_eq!(
            batch.events[2].series_id(),
            macro_series::JP_TANKAN_LARGE_MANUFACTURING_CONDITIONS
        );
        assert_eq!(batch.events[2].macro_value().unwrap(), 22.0);
    }
}

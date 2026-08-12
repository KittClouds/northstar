use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{documents, macro_series, SeriesId, SourceId, StreamId};
use crate::data_plane::provenance::DocumentKind;
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;

pub const BOE_SOURCE: SourceId = SourceId(28);
pub const BOE_RATES_STREAM: StreamId = StreamId(1);
pub const BOE_DATABASE_EXPORT: &str =
    "https://www.bankofengland.co.uk/boeapps/database/_iadb-fromshowcolumns.asp";

pub const BOE_SERIES: [MacroSeriesBinding; 1] = [MacroSeriesBinding {
    provider_code: "IUDBEDR",
    series_id: macro_series::BOE_BANK_RATE,
    label: "Bank of England Official Bank Rate",
    category: MacroCategory::Rates,
    unit: MacroUnit::Percent,
    measure: MacroMeasure::PolicyRate,
    source_url: "https://www.bankofengland.co.uk/boeapps/database/Bank-Rate.asp",
    source: BOE_SOURCE,
    stream: BOE_RATES_STREAM,
}];

pub fn binding_for_code(provider_code: &str) -> Option<&'static MacroSeriesBinding> {
    BOE_SERIES
        .iter()
        .find(|binding| binding.provider_code == provider_code)
}

#[derive(Clone, Copy, Debug)]
pub struct BoeRateDecoder;

impl BoeRateDecoder {
    pub const fn official() -> Self {
        Self
    }
}

impl CanonicalDecoder for BoeRateDecoder {
    fn source_id(&self) -> SourceId {
        BOE_SOURCE
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
        let mut lines = receipt.payload.split(|byte| *byte == b'\n');
        let header = trim_cr(lines.next().unwrap_or_default());
        let expected_header = format!("DATE,{}", binding.provider_code);
        if header != expected_header.as_bytes() {
            return Err(DecodeError::UnsupportedSchema(
                "Bank of England CSV header changed".into(),
            ));
        }
        let before = output.events.len();
        let mut provider_records = 0u32;
        let mut seen = HashSet::new();
        for raw_line in lines {
            let line = trim_cr(raw_line);
            if line.is_empty() {
                continue;
            }
            provider_records = provider_records
                .checked_add(1)
                .ok_or_else(|| DecodeError::UnsupportedSchema("too many BoE rows".into()))?;
            let comma = memchr::memchr(b',', line)
                .ok_or_else(|| DecodeError::UnsupportedSchema("BoE row omitted comma".into()))?;
            if memchr::memchr(b',', &line[comma + 1..]).is_some() {
                return Err(DecodeError::UnsupportedSchema(
                    "BoE row gained columns".into(),
                ));
            }
            let date = utf8(&line[..comma])?;
            let (start_ns, end_ns) = day_window(date)?;
            if !seen.insert(start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate BoE date {date}"
                )));
            }
            let value_text = utf8(&line[comma + 1..])?;
            let value: f64 = value_text
                .parse()
                .map_err(|_| DecodeError::InvalidNumber(value_text.into()))?;
            if !value.is_finite() {
                return Err(DecodeError::InvalidNumber(value_text.into()));
            }
            let vintage = vintage_id(binding, date, value);
            let source_event = source_event_id(binding.series_id, start_ns, vintage);
            let mut event = CanonicalEvent::macro_observation(
                BOE_SOURCE,
                BOE_RATES_STREAM,
                binding.series_id,
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
            b"northstar-boe-document-v1",
            receipt.payload_hash.as_slice(),
        ]);
        output.push(CanonicalEvent::source_document(
            BOE_SOURCE,
            BOE_RATES_STREAM,
            documents::BOE_BANK_RATE_TIMESERIES,
            receipt.id,
            document_event,
            receipt.ts_received_ns,
            receipt.ts_received_ns,
            receipt.payload_hash,
            DocumentKind::DatasetSnapshot,
            None,
        )?);
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many BoE events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(
                "BoE response contained no observations".into(),
            ));
        }
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn validate_receipt(receipt: RawReceiptRef<'_>) -> Result<(), DecodeError> {
    if receipt.source != BOE_SOURCE {
        return Err(DecodeError::WrongSource {
            expected: BOE_SOURCE,
            actual: receipt.source,
        });
    }
    if receipt.stream != BOE_RATES_STREAM || receipt.status_code != 200 {
        return Err(DecodeError::UnsupportedSchema(format!(
            "unexpected BoE stream/status {}/{}",
            receipt.stream.get(),
            receipt.status_code
        )));
    }
    Ok(())
}

fn metadata_binding(metadata: &[u8]) -> Result<&str, DecodeError> {
    utf8(metadata)?
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("binding="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("BoE metadata omitted binding".into()))
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn utf8(bytes: &[u8]) -> Result<&str, DecodeError> {
    std::str::from_utf8(bytes).map_err(|error| DecodeError::Malformed(error.to_string()))
}

fn day_window(text: &str) -> Result<(i64, i64), DecodeError> {
    if text.len() != 11 || text.as_bytes()[2] != b' ' || text.as_bytes()[6] != b' ' {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid BoE date {text:?}"
        )));
    }
    let day: i64 = text[..2]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid BoE date {text:?}")))?;
    let month = month_number(&text[3..6])
        .ok_or_else(|| DecodeError::UnsupportedSchema(format!("invalid BoE date {text:?}")))?;
    let year: i64 = text[7..]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid BoE date {text:?}")))?;
    if !(1900..=2300).contains(&year) || !(1..=31).contains(&day) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "BoE date is outside supported bounds: {text}"
        )));
    }
    let start = civil_ns(year, month, day);
    Ok((start, start.saturating_add(86_400_000_000_000)))
}

fn month_number(text: &str) -> Option<i64> {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|month| *month == text)
    .and_then(|index| i64::try_from(index + 1).ok())
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

fn vintage_id(binding: &MacroSeriesBinding, date: &str, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-boe-vintage-v1",
        binding.provider_code.as_bytes(),
        date.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-boe-source-event-v1",
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

    #[test]
    fn freezes_bank_rate_identity_and_daily_window() {
        let fixture = include_bytes!("../../../tests/fixtures/boe/bank_rate.csv");
        let receipt = RawReceiptRef {
            id: ReceiptId(81),
            source: BOE_SOURCE,
            stream: BOE_RATES_STREAM,
            status_code: 200,
            content_type: 2,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: b"GET /boeapps/database binding=IUDBEDR",
            payload: fixture,
            payload_hash: *blake3::hash(fixture).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = BoeRateDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.canonical_events, 4);
        assert_eq!(batch.events[2].series_id(), macro_series::BOE_BANK_RATE);
        assert_eq!(batch.events[2].macro_value().unwrap(), 3.75);
        assert_eq!(
            batch.events[2].values[1] - batch.events[2].values[0],
            86_400_000_000_000
        );
    }
}

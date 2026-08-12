use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{macro_series, SeriesId, SourceId, StreamId};
use crate::data_plane::providers::macro_catalog::{
    MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit,
};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;

pub const ECB_SOURCE: SourceId = SourceId(26);
pub const ECB_POLICY_RATES_STREAM: StreamId = StreamId(1);
pub const ECB_API_ROOT: &str = "https://data-api.ecb.europa.eu/service/data/FM";

pub const ECB_SERIES: [MacroSeriesBinding; 2] = [
    MacroSeriesBinding {
        provider_code: "D.U2.EUR.4F.KR.DFR.LEV",
        series_id: macro_series::ECB_DEPOSIT_FACILITY_RATE,
        label: "ECB deposit facility rate",
        category: MacroCategory::Rates,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::PolicyRate,
        source_url: "https://data.ecb.europa.eu/data/datasets/FM/FM.D.U2.EUR.4F.KR.DFR.LEV",
        source: ECB_SOURCE,
        stream: ECB_POLICY_RATES_STREAM,
    },
    MacroSeriesBinding {
        provider_code: "D.U2.EUR.4F.KR.MRR_RT.LEV",
        series_id: macro_series::ECB_MAIN_REFINANCING_RATE,
        label: "ECB main refinancing rate",
        category: MacroCategory::Rates,
        unit: MacroUnit::Percent,
        measure: MacroMeasure::PolicyRate,
        source_url: "https://data.ecb.europa.eu/data/datasets/FM/FM.D.U2.EUR.4F.KR.MRR_RT.LEV",
        source: ECB_SOURCE,
        stream: ECB_POLICY_RATES_STREAM,
    },
];

pub fn binding_for_code(code: &str) -> Option<&'static MacroSeriesBinding> {
    ECB_SERIES
        .iter()
        .find(|binding| binding.provider_code == code)
}

#[derive(Clone, Copy, Debug)]
pub struct EcbPolicyRateDecoder {
    source: SourceId,
    stream: StreamId,
}

impl EcbPolicyRateDecoder {
    pub const fn official() -> Self {
        Self {
            source: ECB_SOURCE,
            stream: ECB_POLICY_RATES_STREAM,
        }
    }
}

impl CanonicalDecoder for EcbPolicyRateDecoder {
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
                "unexpected ECB stream {}",
                receipt.stream.get()
            )));
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "ECB returned HTTP {}",
                receipt.status_code
            )));
        }
        let code = metadata_binding(receipt.metadata)?;
        let binding =
            binding_for_code(code).ok_or_else(|| DecodeError::UnknownIdentity(code.into()))?;
        let expected_key = format!("FM.{code}");
        let expected_provider_id = code
            .split('.')
            .nth(5)
            .ok_or_else(|| DecodeError::UnknownIdentity(code.into()))?;
        let mut lines = receipt.payload.split(|byte| *byte == b'\n');
        let header = trim_cr(lines.next().unwrap_or_default());
        if header != b"KEY,FREQ,REF_AREA,CURRENCY,PROVIDER_FM,INSTRUMENT_FM,PROVIDER_FM_ID,DATA_TYPE_FM,TIME_PERIOD,OBS_VALUE"
        {
            return Err(DecodeError::UnsupportedSchema(
                "ECB data-only CSV header changed".into(),
            ));
        }
        let before = output.events.len();
        let mut seen_days = HashSet::new();
        let mut provider_records = 0u32;
        for raw_line in lines {
            let line = trim_cr(raw_line);
            if line.is_empty() {
                continue;
            }
            provider_records = provider_records
                .checked_add(1)
                .ok_or_else(|| DecodeError::UnsupportedSchema("too many ECB rows".into()))?;
            let fields = split_ten(line)?;
            validate_identity(&fields, &expected_key, expected_provider_id)?;
            let period = utf8(fields[8])?;
            let (start_ns, end_ns) = day_window(period)?;
            if !seen_days.insert(start_ns) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate ECB period {period}"
                )));
            }
            let value_text = utf8(fields[9])?;
            let value: f64 = value_text
                .parse()
                .map_err(|_| DecodeError::InvalidNumber(value_text.into()))?;
            if !value.is_finite() {
                return Err(DecodeError::InvalidNumber(value_text.into()));
            }
            let vintage = vintage_id(binding, period, value);
            let source_event = source_event_id(binding.series_id, start_ns, vintage);
            let mut event = CanonicalEvent::macro_observation(
                self.source,
                self.stream,
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
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many ECB events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(
                "ECB response contained no observations".into(),
            ));
        }
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn metadata_binding(metadata: &[u8]) -> Result<&str, DecodeError> {
    let metadata = utf8(metadata)?;
    metadata
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("binding="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("ECB metadata omitted binding".into()))
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn split_ten(line: &[u8]) -> Result<[&[u8]; 10], DecodeError> {
    let mut fields = [b"".as_slice(); 10];
    let mut start = 0usize;
    let mut count = 0usize;
    for end in memchr::memchr_iter(b',', line) {
        if count >= 9 {
            return Err(DecodeError::UnsupportedSchema(
                "ECB data-only row gained columns".into(),
            ));
        }
        fields[count] = &line[start..end];
        count += 1;
        start = end + 1;
    }
    if count != 9 {
        return Err(DecodeError::UnsupportedSchema(format!(
            "ECB row has {} columns",
            count + 1
        )));
    }
    fields[9] = &line[start..];
    Ok(fields)
}

fn validate_identity(
    fields: &[&[u8]; 10],
    expected_key: &str,
    expected_provider_id: &str,
) -> Result<(), DecodeError> {
    let expected = [
        expected_key,
        "D",
        "U2",
        "EUR",
        "4F",
        "KR",
        expected_provider_id,
        "LEV",
    ];
    for (actual, expected) in fields.iter().zip(expected).take(8) {
        if *actual != expected.as_bytes() {
            return Err(DecodeError::UnknownIdentity(
                String::from_utf8_lossy(actual).into_owned(),
            ));
        }
    }
    Ok(())
}

fn utf8(bytes: &[u8]) -> Result<&str, DecodeError> {
    std::str::from_utf8(bytes).map_err(|error| DecodeError::Malformed(error.to_string()))
}

fn day_window(text: &str) -> Result<(i64, i64), DecodeError> {
    if text.len() != 10 || text.as_bytes()[4] != b'-' || text.as_bytes()[7] != b'-' {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid ECB date {text:?}"
        )));
    }
    let year: i64 = text[..4]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid ECB date {text:?}")))?;
    let month: i64 = text[5..7]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid ECB date {text:?}")))?;
    let day: i64 = text[8..]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid ECB date {text:?}")))?;
    if !(1999..=2300).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
    {
        return Err(DecodeError::UnsupportedSchema(format!(
            "ECB date is outside supported bounds: {text}"
        )));
    }
    let start = days_from_civil(year, month, day)
        .checked_mul(86_400_000_000_000)
        .ok_or(DecodeError::TimestampOverflow)?;
    Ok((start, start.saturating_add(86_400_000_000_000)))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year.unsigned_abs().is_multiple_of(400)
            || (year.unsigned_abs().is_multiple_of(4)
                && !year.unsigned_abs().is_multiple_of(100)) =>
        {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
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

fn vintage_id(binding: &MacroSeriesBinding, period: &str, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-ecb-policy-vintage-v1",
        binding.provider_code.as_bytes(),
        period.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-ecb-policy-source-event-v1",
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
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    u64::from_le_bytes(bytes).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/ecb/deposit_rate.csv");

    #[test]
    fn data_only_csv_freezes_policy_identity_and_daily_windows() {
        let receipt = RawReceiptRef {
            id: ReceiptId(61),
            source: ECB_SOURCE,
            stream: ECB_POLICY_RATES_STREAM,
            status_code: 200,
            content_type: 2,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: b"GET /service/data/FM binding=D.U2.EUR.4F.KR.DFR.LEV",
            payload: FIXTURE,
            payload_hash: *blake3::hash(FIXTURE).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = EcbPolicyRateDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 3);
        assert_eq!(stats.canonical_events, 3);
        assert_eq!(
            batch.events[2].series_id(),
            macro_series::ECB_DEPOSIT_FACILITY_RATE
        );
        assert_eq!(batch.events[2].macro_value().unwrap(), 2.25);
        assert_eq!(
            batch.events[2].values[1] - batch.events[2].values[0],
            86_400_000_000_000
        );
    }
}

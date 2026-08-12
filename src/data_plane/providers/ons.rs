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

pub const ONS_SOURCE: SourceId = SourceId(27);
pub const ONS_TIMESERIES_STREAM: StreamId = StreamId(1);
pub const ONS_DOWNLOAD_ROOT: &str = "https://www.ons.gov.uk/generator";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Frequency {
    Monthly,
    Quarterly,
}

#[derive(Clone, Copy, Debug)]
struct OnsBinding {
    catalog: MacroSeriesBinding,
    dataset: &'static str,
    path: &'static str,
    expected_unit: &'static str,
    frequency: Frequency,
    document: DocumentId,
}

const ONS_BINDINGS: [OnsBinding; 4] = [
    OnsBinding {
        catalog: MacroSeriesBinding {
            provider_code: "D7G7",
            series_id: macro_series::UK_CPI_ALL_ITEMS_YOY,
            label: "UK CPI all-items annual rate",
            category: MacroCategory::Inflation,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::AnnualRate,
            source_url: "https://www.ons.gov.uk/economy/inflationandpriceindices/timeseries/d7g7/mm23",
            source: ONS_SOURCE,
            stream: ONS_TIMESERIES_STREAM,
        },
        dataset: "MM23",
        path: "/economy/inflationandpriceindices/timeseries/d7g7/mm23",
        expected_unit: "%",
        frequency: Frequency::Monthly,
        document: documents::ONS_CPI_TIMESERIES,
    },
    OnsBinding {
        catalog: MacroSeriesBinding {
            provider_code: "IHYQ",
            series_id: macro_series::UK_REAL_GDP_QOQ,
            label: "UK real GDP quarter-on-quarter growth",
            category: MacroCategory::Growth,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::QuarterOverQuarter,
            source_url: "https://www.ons.gov.uk/economy/grossdomesticproductgdp/timeseries/ihyq/qna",
            source: ONS_SOURCE,
            stream: ONS_TIMESERIES_STREAM,
        },
        dataset: "QNA",
        path: "/economy/grossdomesticproductgdp/timeseries/ihyq/qna",
        expected_unit: "%",
        frequency: Frequency::Quarterly,
        document: documents::ONS_GDP_TIMESERIES,
    },
    OnsBinding {
        catalog: MacroSeriesBinding {
            provider_code: "MGSX",
            series_id: macro_series::UK_UNEMPLOYMENT_RATE_SA,
            label: "UK unemployment rate, age 16+, seasonally adjusted",
            category: MacroCategory::Labor,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::SeasonallyAdjusted,
            source_url: "https://www.ons.gov.uk/employmentandlabourmarket/peoplenotinwork/unemployment/timeseries/mgsx/lms",
            source: ONS_SOURCE,
            stream: ONS_TIMESERIES_STREAM,
        },
        dataset: "LMS",
        path: "/employmentandlabourmarket/peoplenotinwork/unemployment/timeseries/mgsx/lms",
        expected_unit: "%",
        frequency: Frequency::Monthly,
        document: documents::ONS_UNEMPLOYMENT_TIMESERIES,
    },
    OnsBinding {
        catalog: MacroSeriesBinding {
            provider_code: "J5EK",
            series_id: macro_series::UK_RETAIL_VOLUME_SA,
            label: "Great Britain retail volume, all retailers including fuel",
            category: MacroCategory::Growth,
            unit: MacroUnit::Index,
            measure: MacroMeasure::SeasonallyAdjusted,
            source_url: "https://www.ons.gov.uk/businessindustryandtrade/retailindustry/timeseries/j5ek/drsi",
            source: ONS_SOURCE,
            stream: ONS_TIMESERIES_STREAM,
        },
        dataset: "DRSI",
        path: "/businessindustryandtrade/retailindustry/timeseries/j5ek/drsi",
        expected_unit: "Index, base year = 100",
        frequency: Frequency::Monthly,
        document: documents::ONS_RETAIL_TIMESERIES,
    },
];

pub const ONS_SERIES: [MacroSeriesBinding; 4] = [
    ONS_BINDINGS[0].catalog,
    ONS_BINDINGS[1].catalog,
    ONS_BINDINGS[2].catalog,
    ONS_BINDINGS[3].catalog,
];

pub fn download_path(provider_code: &str) -> Option<&'static str> {
    ONS_BINDINGS
        .iter()
        .find(|binding| binding.catalog.provider_code == provider_code)
        .map(|binding| binding.path)
}

pub fn binding_for_code(provider_code: &str) -> Option<&'static MacroSeriesBinding> {
    ONS_BINDINGS
        .iter()
        .find(|binding| binding.catalog.provider_code == provider_code)
        .map(|binding| &binding.catalog)
}

#[derive(Clone, Copy, Debug)]
pub struct OnsTimeseriesDecoder;

impl OnsTimeseriesDecoder {
    pub const fn official() -> Self {
        Self
    }
}

impl CanonicalDecoder for OnsTimeseriesDecoder {
    fn source_id(&self) -> SourceId {
        ONS_SOURCE
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        validate_receipt(receipt)?;
        let code = metadata_binding(receipt.metadata)?;
        let binding = ONS_BINDINGS
            .iter()
            .find(|binding| binding.catalog.provider_code == code)
            .ok_or_else(|| DecodeError::UnknownIdentity(code.into()))?;
        let before = output.events.len();
        let mut provider_records = 0u32;
        let mut release = None;
        let mut cdid = None;
        let mut dataset = None;
        let mut unit = None;
        let mut seen = HashSet::new();
        for raw_line in receipt.payload.split(|byte| *byte == b'\n') {
            let line = trim_cr(raw_line);
            if line.is_empty() {
                continue;
            }
            provider_records = provider_records
                .checked_add(1)
                .ok_or_else(|| DecodeError::UnsupportedSchema("too many ONS rows".into()))?;
            let Some((left, right)) = quoted_pair(line) else {
                return Err(DecodeError::UnsupportedSchema(
                    "ONS CSV is no longer a quoted two-column document".into(),
                ));
            };
            match left {
                b"CDID" => cdid = Some(utf8(right)?),
                b"Source dataset ID" => dataset = Some(utf8(right)?),
                b"Unit" => unit = Some(utf8(right)?),
                b"Release date" => release = Some(utf8(right)?),
                _ => {
                    let period = utf8(left)?;
                    let Some((start_ns, end_ns)) = period_window(period, binding.frequency)? else {
                        continue;
                    };
                    let value_text = utf8(right)?;
                    let value: f64 = value_text
                        .parse()
                        .map_err(|_| DecodeError::InvalidNumber(value_text.into()))?;
                    if !value.is_finite() || !seen.insert(start_ns) {
                        return Err(DecodeError::UnsupportedSchema(format!(
                            "invalid or duplicate ONS observation {period}"
                        )));
                    }
                    let release = release.ok_or_else(|| {
                        DecodeError::UnsupportedSchema(
                            "ONS observation precedes release metadata".into(),
                        )
                    })?;
                    let publication_ns = parse_release_date(release)?;
                    let vintage = vintage_id(&binding.catalog, release, period, value);
                    let source_event =
                        source_event_id(binding.catalog.series_id, start_ns, vintage);
                    let mut event = CanonicalEvent::macro_observation(
                        ONS_SOURCE,
                        ONS_TIMESERIES_STREAM,
                        binding.catalog.series_id,
                        receipt.id,
                        source_event,
                        start_ns,
                        end_ns,
                        publication_ns,
                        receipt.ts_received_ns,
                        value,
                        vintage,
                        None,
                        TimeQuality::ObservedLive,
                    )?;
                    event.header.flags |= EventFlags::FINAL;
                    output.push(event);
                }
            }
        }
        validate_metadata(binding, cdid, dataset, unit, release)?;
        output.events[before..].sort_unstable_by_key(|event| event.values[0]);
        let publication_ns = parse_release_date(release.expect("validated ONS release"))?;
        let document_event = hash_parts(&[
            b"northstar-ons-document-v1",
            &binding.document.get().to_le_bytes(),
            receipt.payload_hash.as_slice(),
        ]);
        output.push(CanonicalEvent::source_document(
            ONS_SOURCE,
            ONS_TIMESERIES_STREAM,
            binding.document,
            receipt.id,
            document_event,
            publication_ns,
            receipt.ts_received_ns,
            receipt.payload_hash,
            DocumentKind::DatasetSnapshot,
            None,
        )?);
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many ONS events".into()))?;
        if canonical_events == 0 {
            return Err(DecodeError::UnsupportedSchema(
                "ONS response contained no matching observations".into(),
            ));
        }
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn validate_receipt(receipt: RawReceiptRef<'_>) -> Result<(), DecodeError> {
    if receipt.source != ONS_SOURCE {
        return Err(DecodeError::WrongSource {
            expected: ONS_SOURCE,
            actual: receipt.source,
        });
    }
    if receipt.stream != ONS_TIMESERIES_STREAM || receipt.status_code != 200 {
        return Err(DecodeError::UnsupportedSchema(format!(
            "unexpected ONS stream/status {}/{}",
            receipt.stream.get(),
            receipt.status_code
        )));
    }
    Ok(())
}

fn validate_metadata(
    binding: &OnsBinding,
    cdid: Option<&str>,
    dataset: Option<&str>,
    unit: Option<&str>,
    release: Option<&str>,
) -> Result<(), DecodeError> {
    if cdid != Some(binding.catalog.provider_code)
        || dataset != Some(binding.dataset)
        || unit != Some(binding.expected_unit)
        || release.is_none_or(|value| parse_release_date(value).is_err())
    {
        return Err(DecodeError::UnknownIdentity(format!(
            "ONS metadata mismatch for {}",
            binding.catalog.provider_code
        )));
    }
    Ok(())
}

fn metadata_binding(metadata: &[u8]) -> Result<&str, DecodeError> {
    utf8(metadata)?
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("binding="))
        .ok_or_else(|| DecodeError::UnsupportedSchema("ONS metadata omitted binding".into()))
}

fn quoted_pair(line: &[u8]) -> Option<(&[u8], &[u8])> {
    if line.len() < 3 || line[0] != b'"' {
        return None;
    }
    if line.ends_with(b"\",") {
        return Some((&line[1..line.len() - 2], b""));
    }
    if *line.last()? != b'"' {
        return None;
    }
    let delimiter = line.windows(3).position(|window| window == b"\",\"")?;
    Some((&line[1..delimiter], &line[delimiter + 3..line.len() - 1]))
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn utf8(bytes: &[u8]) -> Result<&str, DecodeError> {
    std::str::from_utf8(bytes).map_err(|error| DecodeError::Malformed(error.to_string()))
}

fn period_window(text: &str, frequency: Frequency) -> Result<Option<(i64, i64)>, DecodeError> {
    match frequency {
        Frequency::Monthly if text.len() == 8 && text.as_bytes()[4] == b' ' => {
            let year = parse_year(&text[..4])?;
            let month = month_number(&text[5..]).ok_or_else(|| {
                DecodeError::UnsupportedSchema(format!("invalid ONS month {text}"))
            })?;
            Ok(Some(month_window(year, month)))
        }
        Frequency::Quarterly if text.len() == 7 && &text[4..6] == " Q" => {
            let year = parse_year(&text[..4])?;
            let quarter: i64 = text[6..].parse().map_err(|_| {
                DecodeError::UnsupportedSchema(format!("invalid ONS quarter {text}"))
            })?;
            if !(1..=4).contains(&quarter) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid ONS quarter {text}"
                )));
            }
            let month = (quarter - 1) * 3 + 1;
            Ok(Some((
                civil_ns(year, month, 1),
                civil_ns(year, month + 3, 1),
            )))
        }
        _ => Ok(None),
    }
}

fn parse_year(text: &str) -> Result<i64, DecodeError> {
    let year = text
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid ONS year {text}")))?;
    if !(1900..=2300).contains(&year) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid ONS year {text}"
        )));
    }
    Ok(year)
}

fn month_number(text: &str) -> Option<i64> {
    [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ]
    .iter()
    .position(|month| *month == text)
    .and_then(|index| i64::try_from(index + 1).ok())
}

fn month_window(year: i64, month: i64) -> (i64, i64) {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (civil_ns(year, month, 1), civil_ns(next_year, next_month, 1))
}

fn parse_release_date(text: &str) -> Result<i64, DecodeError> {
    if text.len() != 10 || text.as_bytes()[2] != b'-' || text.as_bytes()[5] != b'-' {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid ONS release date {text}"
        )));
    }
    let day = text[..2]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(text.into()))?;
    let month = text[3..5]
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(text.into()))?;
    let year = parse_year(&text[6..])?;
    Ok(civil_ns(year, month, day))
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

fn vintage_id(binding: &MacroSeriesBinding, release: &str, period: &str, value: f64) -> u64 {
    hash_parts(&[
        b"northstar-ons-vintage-v1",
        binding.provider_code.as_bytes(),
        release.as_bytes(),
        period.as_bytes(),
        &value.to_bits().to_le_bytes(),
    ])
}

fn source_event_id(series: SeriesId, start_ns: i64, vintage: u64) -> u64 {
    hash_parts(&[
        b"northstar-ons-source-event-v1",
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
    use crate::data_plane::event::EventKind;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    fn decode(code: &'static str, fixture: &'static [u8]) -> CanonicalBatch {
        let receipt = RawReceiptRef {
            id: ReceiptId(71),
            source: ONS_SOURCE,
            stream: ONS_TIMESERIES_STREAM,
            status_code: 200,
            content_type: 2,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: format!("GET /generator binding={code}").leak().as_bytes(),
            payload: fixture,
            payload_hash: *blake3::hash(fixture).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        OnsTimeseriesDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        batch
    }

    #[test]
    fn freezes_monthly_ons_identity_and_ignores_annual_rows() {
        let batch = decode(
            "D7G7",
            include_bytes!("../../../tests/fixtures/ons/cpi.csv"),
        );
        assert_eq!(batch.events.len(), 4);
        assert_eq!(
            batch.events[2].series_id(),
            macro_series::UK_CPI_ALL_ITEMS_YOY
        );
        assert_eq!(batch.events[2].macro_value().unwrap(), 2.6);
    }

    #[test]
    fn freezes_quarterly_ons_window() {
        let batch = decode(
            "IHYQ",
            include_bytes!("../../../tests/fixtures/ons/gdp.csv"),
        );
        assert_eq!(batch.events.len(), 4);
        assert_eq!(batch.events[2].series_id(), macro_series::UK_REAL_GDP_QOQ);
        assert_eq!(batch.events[2].macro_value().unwrap(), 0.6);
    }

    #[test]
    fn accepts_official_empty_metadata_rows() {
        let batch = decode(
            "D7G7",
            b"\"Title\",\"CPI ANNUAL RATE 00: ALL ITEMS 2015=100\"\n\
\"CDID\",\"D7G7\"\n\
\"Source dataset ID\",\"MM23\"\n\
\"Unit\",\"%\"\n\
\"Release date\",\"22-07-2026\"\n\
\"Important notes\",\n\
\"2026 JUN\",\"2.6\"\n",
        );
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.events[0].macro_value().unwrap(), 2.6);
        assert_eq!(batch.events[1].kind().unwrap(), EventKind::SourceDocument);
    }
}

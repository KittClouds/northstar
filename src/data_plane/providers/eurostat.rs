use super::macro_catalog::{MacroCategory, MacroMeasure, MacroSeriesBinding, MacroUnit};
use crate::data_plane::event::{CanonicalEvent, EventFlags, TimeQuality};
use crate::data_plane::ids::{macro_series, SeriesId, SourceId, StreamId};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use serde::Deserialize;
use std::collections::HashMap;

pub const EUROSTAT_SOURCE: SourceId = SourceId(22);
pub const EUROSTAT_STATISTICS_STREAM: StreamId = StreamId(1);
pub const EUROSTAT_API_ROOT: &str =
    "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data";

#[derive(Clone, Copy, Debug)]
pub struct EurostatSeriesBinding {
    pub catalog: MacroSeriesBinding,
    pub dataset_code: &'static str,
    pub dimensions: &'static [(&'static str, &'static str)],
    pub expected_ids: &'static [&'static str],
}

const HICP_DIMENSIONS: &[(&str, &str)] = &[
    ("freq", "M"),
    ("unit", "RCH_A"),
    ("coicop18", "TOTAL"),
    ("geo", "DE"),
];
const HICP_IDS: &[&str] = &["freq", "unit", "coicop18", "geo", "time"];
const UNEMPLOYMENT_DIMENSIONS: &[(&str, &str)] = &[
    ("freq", "M"),
    ("s_adj", "TC"),
    ("age", "TOTAL"),
    ("unit", "PC_ACT"),
    ("sex", "T"),
    ("geo", "DE"),
];
const UNEMPLOYMENT_IDS: &[&str] = &["freq", "s_adj", "age", "unit", "sex", "geo", "time"];
const GDP_DIMENSIONS: &[(&str, &str)] = &[
    ("freq", "Q"),
    ("unit", "CLV_PCH_PRE"),
    ("s_adj", "SCA"),
    ("na_item", "B1GQ"),
    ("geo", "EA20"),
];
const GDP_IDS: &[&str] = &["freq", "unit", "s_adj", "na_item", "geo", "time"];
const RETAIL_DIMENSIONS: &[(&str, &str)] = &[
    ("freq", "M"),
    ("indic_bt", "VOL_SLS"),
    ("nace_r2", "G47"),
    ("s_adj", "SCA"),
    ("unit", "I21"),
    ("geo", "DE"),
];
const INDUSTRIAL_DIMENSIONS: &[(&str, &str)] = &[
    ("freq", "M"),
    ("indic_bt", "PRD"),
    ("nace_r2", "B-D"),
    ("s_adj", "SCA"),
    ("unit", "I21"),
    ("geo", "DE"),
];
const STS_IDS: &[&str] = &[
    "freq", "indic_bt", "nace_r2", "s_adj", "unit", "geo", "time",
];

pub const EUROSTAT_BINDINGS: [EurostatSeriesBinding; 5] = [
    EurostatSeriesBinding {
        catalog: MacroSeriesBinding {
            provider_code: "prc_hicp_minr/DE/TOTAL/RCH_A",
            series_id: macro_series::DE_HICP_ALL_ITEMS_YOY,
            label: "Germany HICP / all items",
            category: MacroCategory::Inflation,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::AnnualRate,
            source_url:
                "https://ec.europa.eu/eurostat/databrowser/view/prc_hicp_minr/default/table",
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        },
        dataset_code: "prc_hicp_minr",
        dimensions: HICP_DIMENSIONS,
        expected_ids: HICP_IDS,
    },
    EurostatSeriesBinding {
        catalog: MacroSeriesBinding {
            provider_code: "une_rt_m/DE/TOTAL/TC",
            series_id: macro_series::DE_UNEMPLOYMENT_RATE_TC,
            label: "Germany unemployment",
            category: MacroCategory::Labor,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::TrendCycle,
            source_url: "https://ec.europa.eu/eurostat/databrowser/view/une_rt_m/default/table",
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        },
        dataset_code: "une_rt_m",
        dimensions: UNEMPLOYMENT_DIMENSIONS,
        expected_ids: UNEMPLOYMENT_IDS,
    },
    EurostatSeriesBinding {
        catalog: MacroSeriesBinding {
            provider_code: "namq_10_gdp/EA20/B1GQ/CLV_PCH_PRE/SCA",
            series_id: macro_series::EA_REAL_GDP_QOQ,
            label: "Euro-area real GDP growth",
            category: MacroCategory::Growth,
            unit: MacroUnit::Percent,
            measure: MacroMeasure::QuarterOverQuarter,
            source_url: "https://ec.europa.eu/eurostat/databrowser/view/namq_10_gdp/default/table",
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        },
        dataset_code: "namq_10_gdp",
        dimensions: GDP_DIMENSIONS,
        expected_ids: GDP_IDS,
    },
    EurostatSeriesBinding {
        catalog: MacroSeriesBinding {
            provider_code: "sts_trtu_m/DE/G47/VOL_SLS/I21/SCA",
            series_id: macro_series::DE_RETAIL_VOLUME_SA,
            label: "Germany retail volume",
            category: MacroCategory::Growth,
            unit: MacroUnit::Index,
            measure: MacroMeasure::SeasonallyAdjusted,
            source_url: "https://ec.europa.eu/eurostat/databrowser/view/sts_trtu_m/default/table",
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        },
        dataset_code: "sts_trtu_m",
        dimensions: RETAIL_DIMENSIONS,
        expected_ids: STS_IDS,
    },
    EurostatSeriesBinding {
        catalog: MacroSeriesBinding {
            provider_code: "sts_inpr_m/DE/B-D/PRD/I21/SCA",
            series_id: macro_series::DE_INDUSTRIAL_PRODUCTION_SA,
            label: "Germany industrial production",
            category: MacroCategory::Growth,
            unit: MacroUnit::Index,
            measure: MacroMeasure::SeasonallyAdjusted,
            source_url: "https://ec.europa.eu/eurostat/databrowser/view/sts_inpr_m/default/table",
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        },
        dataset_code: "sts_inpr_m",
        dimensions: INDUSTRIAL_DIMENSIONS,
        expected_ids: STS_IDS,
    },
];

pub const EUROSTAT_SERIES: [MacroSeriesBinding; 5] = [
    EUROSTAT_BINDINGS[0].catalog,
    EUROSTAT_BINDINGS[1].catalog,
    EUROSTAT_BINDINGS[2].catalog,
    EUROSTAT_BINDINGS[3].catalog,
    EUROSTAT_BINDINGS[4].catalog,
];

pub fn binding_for_dataset(code: &str) -> Option<&'static EurostatSeriesBinding> {
    EUROSTAT_BINDINGS
        .iter()
        .find(|binding| binding.dataset_code == code)
}

#[derive(Clone, Copy, Debug)]
pub struct EurostatDecoder {
    source: SourceId,
    stream: StreamId,
}

impl EurostatDecoder {
    pub const fn official() -> Self {
        Self {
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
        }
    }
}

#[derive(Debug, Deserialize)]
struct WireEnvelope<'a> {
    #[serde(borrow)]
    version: &'a str,
    #[serde(rename = "class", borrow)]
    class_name: &'a str,
    #[serde(borrow)]
    source: &'a str,
    #[serde(rename = "updated", borrow)]
    _updated: &'a str,
    #[serde(borrow)]
    id: Vec<&'a str>,
    size: Vec<usize>,
    #[serde(borrow)]
    dimension: HashMap<&'a str, WireDimension<'a>>,
    #[serde(default)]
    value: HashMap<usize, f64>,
    #[serde(default, borrow)]
    status: HashMap<usize, &'a str>,
}

#[derive(Debug, Deserialize)]
struct WireDimension<'a> {
    #[serde(borrow)]
    category: WireCategory<'a>,
}

#[derive(Debug, Deserialize)]
struct WireCategory<'a> {
    #[serde(borrow)]
    index: HashMap<&'a str, usize>,
}

impl CanonicalDecoder for EurostatDecoder {
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
                "unexpected Eurostat stream {}",
                receipt.stream.get()
            )));
        }
        let dataset_code = receipt
            .metadata
            .strip_prefix(b"dataset=")
            .and_then(|value| std::str::from_utf8(value).ok())
            .ok_or_else(|| {
                DecodeError::UnsupportedSchema("missing Eurostat dataset metadata".into())
            })?;
        let binding = binding_for_dataset(dataset_code)
            .ok_or_else(|| DecodeError::UnknownIdentity(dataset_code.into()))?;
        let envelope: WireEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        validate_envelope(binding, &envelope)?;
        let periods = ordered_periods(&envelope)?;
        let provider_records = u32::try_from(periods.len())
            .map_err(|_| DecodeError::UnsupportedSchema("too many Eurostat periods".into()))?;
        let before = output.events.len();
        for (index, period) in periods.into_iter().enumerate() {
            let Some(value) = envelope.value.get(&index).copied() else {
                continue;
            };
            if !value.is_finite() {
                return Err(DecodeError::InvalidNumber(value.to_string()));
            }
            let frequency = binding
                .dimensions
                .iter()
                .find_map(|(name, value)| (*name == "freq").then_some(*value))
                .ok_or_else(|| {
                    DecodeError::UnsupportedSchema("Eurostat binding omitted frequency".into())
                })?;
            let window = parse_period(period, frequency)?;
            let status = envelope.status.get(&index).copied().unwrap_or_default();
            let vintage_id = vintage_id(binding, period, value, status);
            let source_event_id = source_event_id(
                binding.catalog.series_id,
                window.year,
                window.slot,
                vintage_id,
            );
            let mut event = CanonicalEvent::macro_observation(
                self.source,
                self.stream,
                binding.catalog.series_id,
                receipt.id,
                source_event_id,
                window.start_ns,
                window.end_ns,
                receipt.ts_received_ns,
                receipt.ts_received_ns,
                value,
                vintage_id,
                None,
                TimeQuality::ObservedLive,
            )?;
            event.header.flags |= if status.contains('e') || status.contains('p') {
                EventFlags::PROVISIONAL
            } else {
                EventFlags::FINAL
            };
            output.push(event);
        }
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many Eurostat events".into()))?;
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn validate_envelope(
    binding: &EurostatSeriesBinding,
    envelope: &WireEnvelope<'_>,
) -> Result<(), DecodeError> {
    if envelope.version != "2.0" || envelope.class_name != "dataset" || envelope.source != "ESTAT" {
        return Err(DecodeError::UnsupportedSchema(
            "Eurostat JSON-stat identity changed".into(),
        ));
    }
    if envelope.id.as_slice() != binding.expected_ids || envelope.size.len() != envelope.id.len() {
        return Err(DecodeError::UnsupportedSchema(format!(
            "Eurostat {} dimensions changed",
            binding.dataset_code
        )));
    }
    for (position, id) in envelope.id.iter().enumerate() {
        let dimension = envelope.dimension.get(id).ok_or_else(|| {
            DecodeError::UnsupportedSchema(format!("Eurostat omitted dimension {id}"))
        })?;
        if *id == "time" {
            if envelope.size[position] != dimension.category.index.len() {
                return Err(DecodeError::UnsupportedSchema(
                    "Eurostat time dimension size changed".into(),
                ));
            }
            continue;
        }
        let expected = binding
            .dimensions
            .iter()
            .find_map(|(name, value)| (*name == *id).then_some(*value))
            .ok_or_else(|| {
                DecodeError::UnsupportedSchema(format!("unbound Eurostat dimension {id}"))
            })?;
        if envelope.size[position] != 1 || dimension.category.index.get(expected) != Some(&0) {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Eurostat dimension {id} is not the frozen {expected} slice"
            )));
        }
    }
    Ok(())
}

fn ordered_periods<'a>(envelope: &'a WireEnvelope<'a>) -> Result<Vec<&'a str>, DecodeError> {
    let time = envelope
        .dimension
        .get("time")
        .ok_or_else(|| DecodeError::UnsupportedSchema("Eurostat omitted time".into()))?;
    let mut periods = vec![None; time.category.index.len()];
    for (&period, &index) in &time.category.index {
        let slot = periods.get_mut(index).ok_or_else(|| {
            DecodeError::UnsupportedSchema("Eurostat time index is out of bounds".into())
        })?;
        if slot.replace(period).is_some() {
            return Err(DecodeError::UnsupportedSchema(
                "Eurostat time index is duplicated".into(),
            ));
        }
    }
    periods
        .into_iter()
        .map(|period| {
            period.ok_or_else(|| {
                DecodeError::UnsupportedSchema("Eurostat time index has a gap".into())
            })
        })
        .collect()
}

struct PeriodWindow {
    year: i32,
    slot: u8,
    start_ns: i64,
    end_ns: i64,
}

fn parse_period(period: &str, frequency: &str) -> Result<PeriodWindow, DecodeError> {
    let (year, suffix) = period.split_once('-').ok_or_else(|| {
        DecodeError::UnsupportedSchema(format!("invalid Eurostat period {period:?}"))
    })?;
    let year: i32 = year
        .parse()
        .map_err(|_| DecodeError::UnsupportedSchema(format!("invalid Eurostat year {year:?}")))?;
    if !(1900..=2300).contains(&year) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "Eurostat period is outside supported bounds: {period}"
        )));
    }
    let (slot, start_month, span_months) = match frequency {
        "M" => {
            let month: u8 = suffix.parse().map_err(|_| {
                DecodeError::UnsupportedSchema(format!("invalid Eurostat month {suffix:?}"))
            })?;
            if !(1..=12).contains(&month) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "Eurostat month is outside supported bounds: {period}"
                )));
            }
            (month, month, 1u8)
        }
        "Q" if suffix.len() == 2 && suffix.as_bytes()[0] == b'Q' => {
            let quarter = suffix.as_bytes()[1].wrapping_sub(b'0');
            if !(1..=4).contains(&quarter) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "invalid Eurostat quarter {period:?}"
                )));
            }
            (0x80 | quarter, (quarter - 1) * 3 + 1, 3u8)
        }
        _ => {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Eurostat frequency mismatch for {period:?}"
            )))
        }
    };
    let start_ns = month_start_ns(year, start_month)?;
    let end_month = start_month + span_months;
    let (end_year, end_month) = if end_month > 12 {
        (year + 1, end_month - 12)
    } else {
        (year, end_month)
    };
    Ok(PeriodWindow {
        year,
        slot,
        start_ns,
        end_ns: month_start_ns(end_year, end_month)?,
    })
}

fn vintage_id(binding: &EurostatSeriesBinding, period: &str, value: f64, status: &str) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-eurostat-vintage-v1");
    hasher.update(binding.catalog.provider_code.as_bytes());
    hasher.update(period.as_bytes());
    hasher.update(&value.to_bits().to_le_bytes());
    hasher.update(status.as_bytes());
    nonzero_hash64(hasher.finalize().as_bytes())
}

fn source_event_id(series: SeriesId, year: i32, slot: u8, vintage: u64) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-eurostat-source-event-v1");
    hasher.update(&series.get().to_le_bytes());
    hasher.update(&year.to_le_bytes());
    hasher.update(&[slot]);
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

    const HICP: &[u8] = include_bytes!("../../../tests/fixtures/eurostat/hicp_de.json");
    const GDP: &[u8] = include_bytes!("../../../tests/fixtures/eurostat/gdp_ea20.json");
    const RETAIL: &[u8] = include_bytes!("../../../tests/fixtures/eurostat/retail_de.json");
    const INDUSTRIAL: &[u8] = include_bytes!("../../../tests/fixtures/eurostat/industrial_de.json");

    fn decode_fixture(dataset: &str, payload: &[u8]) -> CanonicalBatch {
        let metadata = format!("dataset={dataset}");
        let receipt = RawReceiptRef {
            id: ReceiptId(8),
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 1_000,
            ts_received_ns: 2_000,
            metadata: metadata.as_bytes(),
            payload,
            payload_hash: *blake3::hash(payload).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        EurostatDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        batch
    }

    #[test]
    fn official_json_stat_shape_preserves_estimates_and_periods() {
        let decoder = EurostatDecoder::official();
        let receipt = RawReceiptRef {
            id: ReceiptId(8),
            source: EUROSTAT_SOURCE,
            stream: EUROSTAT_STATISTICS_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 1_000,
            ts_received_ns: 2_000,
            metadata: b"dataset=prc_hicp_minr",
            payload: HICP,
            payload_hash: *blake3::hash(HICP).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = decoder.decode(receipt, &mut batch).unwrap();
        assert_eq!(stats.provider_records, 3);
        assert_eq!(stats.canonical_events, 3);
        assert_eq!(
            batch.events[2].series_id(),
            macro_series::DE_HICP_ALL_ITEMS_YOY
        );
        assert_eq!(batch.events[2].macro_value().unwrap(), 2.8);
        assert!(batch.events[2].flags().contains(EventFlags::PROVISIONAL));
        assert_eq!(batch.events[2].header.ts_effective_ns, 2_000);
    }

    #[test]
    fn quarterly_gdp_uses_quarter_windows_without_annualizing() {
        let batch = decode_fixture("namq_10_gdp", GDP);
        assert_eq!(batch.events.len(), 4);
        assert_eq!(batch.events[3].series_id(), macro_series::EA_REAL_GDP_QOQ);
        assert_eq!(batch.events[3].macro_value().unwrap(), 0.4);
        assert_eq!(
            batch.events[0].values[1] - batch.events[0].values[0],
            7_948_800_000_000_000
        );
    }

    #[test]
    fn current_short_term_statistics_bind_exact_activity_slices() {
        let retail = decode_fixture("sts_trtu_m", RETAIL);
        assert_eq!(retail.events.len(), 3);
        assert_eq!(
            retail.events[2].series_id(),
            macro_series::DE_RETAIL_VOLUME_SA
        );
        assert_eq!(retail.events[2].macro_value().unwrap(), 101.2);
        assert!(retail.events[2].flags().contains(EventFlags::PROVISIONAL));

        let industrial = decode_fixture("sts_inpr_m", INDUSTRIAL);
        assert_eq!(industrial.events.len(), 3);
        assert_eq!(
            industrial.events[2].series_id(),
            macro_series::DE_INDUSTRIAL_PRODUCTION_SA
        );
        assert_eq!(industrial.events[2].macro_value().unwrap(), 92.1);
    }
}

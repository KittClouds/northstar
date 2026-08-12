use crate::data_plane::event::{CanonicalEvent, ParticipantClass, TimeQuality};
use crate::data_plane::ids::{instruments, InstrumentId, SourceId, StreamId};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;
use serde::Deserialize;

pub const CFTC_SOURCE: SourceId = SourceId(21);
pub const CFTC_TFF_STREAM: StreamId = StreamId(1);
pub const CFTC_TFF_ENDPOINT: &str = "https://publicreporting.cftc.gov/resource/gpe5-46if.json";

#[derive(Clone, Copy, Debug)]
pub struct CftcMarketBinding {
    pub contract_code: &'static str,
    pub instrument_id: InstrumentId,
    pub label: &'static str,
    pub contract_name: &'static str,
    pub source_url: &'static str,
}

pub const CFTC_MARKETS: [CftcMarketBinding; 4] = [
    CftcMarketBinding {
        contract_code: "20974+",
        instrument_id: instruments::US100,
        label: "US100",
        contract_name: "NASDAQ-100 Consolidated",
        source_url: "https://publicreporting.cftc.gov/resource/gpe5-46if.json",
    },
    CftcMarketBinding {
        contract_code: "13874+",
        instrument_id: instruments::US500,
        label: "US500",
        contract_name: "S&P 500 Consolidated",
        source_url: "https://publicreporting.cftc.gov/resource/gpe5-46if.json",
    },
    CftcMarketBinding {
        contract_code: "124603",
        instrument_id: instruments::US30,
        label: "US30",
        contract_name: "DJIA x $5",
        source_url: "https://publicreporting.cftc.gov/resource/gpe5-46if.json",
    },
    CftcMarketBinding {
        contract_code: "240743",
        instrument_id: instruments::JP225,
        label: "JP225",
        contract_name: "Nikkei Stock Average / JPY",
        source_url: "https://publicreporting.cftc.gov/resource/gpe5-46if.json",
    },
];

pub fn binding_for_contract(code: &str) -> Option<&'static CftcMarketBinding> {
    CFTC_MARKETS
        .iter()
        .find(|binding| binding.contract_code == code)
}

pub fn binding_for_instrument(instrument: InstrumentId) -> Option<&'static CftcMarketBinding> {
    CFTC_MARKETS
        .iter()
        .find(|binding| binding.instrument_id == instrument)
}

#[derive(Clone, Copy, Debug)]
pub struct CftcTffDecoder {
    source: SourceId,
    stream: StreamId,
}

impl CftcTffDecoder {
    pub const fn official() -> Self {
        Self {
            source: CFTC_SOURCE,
            stream: CFTC_TFF_STREAM,
        }
    }
}

#[derive(Debug, Deserialize)]
struct WireRow<'a> {
    #[serde(borrow)]
    id: &'a str,
    #[serde(borrow)]
    report_date_as_yyyy_mm_dd: &'a str,
    #[serde(borrow)]
    cftc_contract_market_code: &'a str,
    #[serde(borrow)]
    open_interest_all: &'a str,
    #[serde(default, borrow)]
    dealer_positions_long_all: Option<&'a str>,
    #[serde(default, borrow)]
    dealer_positions_short_all: Option<&'a str>,
    #[serde(default, borrow)]
    dealer_positions_spread_all: Option<&'a str>,
    #[serde(default, borrow)]
    asset_mgr_positions_long: Option<&'a str>,
    #[serde(default, borrow)]
    asset_mgr_positions_short: Option<&'a str>,
    #[serde(default, borrow)]
    asset_mgr_positions_spread: Option<&'a str>,
    #[serde(default, borrow)]
    lev_money_positions_long: Option<&'a str>,
    #[serde(default, borrow)]
    lev_money_positions_short: Option<&'a str>,
    #[serde(default, borrow)]
    lev_money_positions_spread: Option<&'a str>,
    #[serde(default, borrow)]
    other_rept_positions_long: Option<&'a str>,
    #[serde(default, borrow)]
    other_rept_positions_short: Option<&'a str>,
    #[serde(default, borrow)]
    other_rept_positions_spread: Option<&'a str>,
    #[serde(default, borrow)]
    nonrept_positions_long_all: Option<&'a str>,
    #[serde(default, borrow)]
    nonrept_positions_short_all: Option<&'a str>,
}

impl CanonicalDecoder for CftcTffDecoder {
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
                "unexpected CFTC stream {}",
                receipt.stream.get()
            )));
        }
        let mut rows: Vec<WireRow<'_>> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if rows.is_empty() {
            return Err(DecodeError::UnsupportedSchema(
                "CFTC TFF response is empty".into(),
            ));
        }
        let provider_records = u32::try_from(rows.len())
            .map_err(|_| DecodeError::UnsupportedSchema("too many CFTC rows".into()))?;
        let mut ids = HashSet::with_capacity(rows.len());
        let mut contracts = HashSet::with_capacity(CFTC_MARKETS.len());
        for row in &rows {
            if !ids.insert(row.id) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate CFTC row id {:?}",
                    row.id
                )));
            }
            let binding = binding_for_contract(row.cftc_contract_market_code).ok_or_else(|| {
                DecodeError::UnknownIdentity(row.cftc_contract_market_code.into())
            })?;
            contracts.insert(binding.instrument_id);
        }
        for binding in CFTC_MARKETS {
            if !contracts.contains(&binding.instrument_id) {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "CFTC response omitted {}",
                    binding.contract_name
                )));
            }
        }
        rows.sort_unstable_by_key(|row| {
            (
                parse_report_date(row.report_date_as_yyyy_mm_dd).unwrap_or(i64::MAX),
                binding_for_contract(row.cftc_contract_market_code)
                    .map_or(u32::MAX, |binding| binding.instrument_id.get()),
            )
        });

        let before = output.events.len();
        for row in &rows {
            let binding = binding_for_contract(row.cftc_contract_market_code)
                .expect("CFTC contract validated before sorting");
            let report_date_ns = parse_report_date(row.report_date_as_yyyy_mm_dd)?;
            let open_interest = parse_count(row.open_interest_all)?;
            let groups = [
                (
                    ParticipantClass::Dealer,
                    optional_group(
                        row.dealer_positions_long_all,
                        row.dealer_positions_short_all,
                        row.dealer_positions_spread_all,
                    )?,
                ),
                (
                    ParticipantClass::AssetManager,
                    optional_group(
                        row.asset_mgr_positions_long,
                        row.asset_mgr_positions_short,
                        row.asset_mgr_positions_spread,
                    )?,
                ),
                (
                    ParticipantClass::LeveragedFunds,
                    optional_group(
                        row.lev_money_positions_long,
                        row.lev_money_positions_short,
                        row.lev_money_positions_spread,
                    )?,
                ),
                (
                    ParticipantClass::OtherReportables,
                    optional_group(
                        row.other_rept_positions_long,
                        row.other_rept_positions_short,
                        row.other_rept_positions_spread,
                    )?,
                ),
                (
                    ParticipantClass::NonReportable,
                    optional_nonreportable(
                        row.nonrept_positions_long_all,
                        row.nonrept_positions_short_all,
                    )?,
                ),
            ];
            for (participant, counts) in groups {
                let Some((long, short, spread)) = counts else {
                    continue;
                };
                let source_event_id =
                    source_event_id(row.id, participant, long, short, spread, open_interest);
                output.push(CanonicalEvent::positioning_observation(
                    self.source,
                    self.stream,
                    binding.instrument_id,
                    participant,
                    receipt.id,
                    source_event_id,
                    report_date_ns,
                    receipt.ts_received_ns,
                    long,
                    short,
                    spread,
                    open_interest,
                    TimeQuality::ObservedLive,
                )?);
            }
        }
        let canonical_events = u32::try_from(output.events.len() - before)
            .map_err(|_| DecodeError::UnsupportedSchema("too many CFTC events".into()))?;
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn optional_group(
    long: Option<&str>,
    short: Option<&str>,
    spread: Option<&str>,
) -> Result<Option<(i64, i64, i64)>, DecodeError> {
    match (long, short, spread) {
        (Some(long), Some(short), Some(spread)) => Ok(Some((
            parse_count(long)?,
            parse_count(short)?,
            parse_count(spread)?,
        ))),
        (None, None, None) => Ok(None),
        _ => Err(DecodeError::UnsupportedSchema(
            "CFTC participant group is partially missing".into(),
        )),
    }
}

fn optional_nonreportable(
    long: Option<&str>,
    short: Option<&str>,
) -> Result<Option<(i64, i64, i64)>, DecodeError> {
    match (long, short) {
        (Some(long), Some(short)) => Ok(Some((parse_count(long)?, parse_count(short)?, 0))),
        (None, None) => Ok(None),
        _ => Err(DecodeError::UnsupportedSchema(
            "CFTC nonreportable group is partially missing".into(),
        )),
    }
}

fn parse_count(text: &str) -> Result<i64, DecodeError> {
    let value: i64 = text
        .parse()
        .map_err(|_| DecodeError::InvalidNumber(text.into()))?;
    if value < 0 {
        return Err(DecodeError::InvalidNumber(text.into()));
    }
    Ok(value)
}

fn parse_report_date(text: &str) -> Result<i64, DecodeError> {
    let date = text
        .as_bytes()
        .get(..10)
        .ok_or_else(|| DecodeError::UnsupportedSchema(format!("invalid CFTC date {text:?}")))?;
    if date.get(4) != Some(&b'-') || date.get(7) != Some(&b'-') {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid CFTC date {text:?}"
        )));
    }
    let year = parse_digits(&date[0..4])? as i64;
    let month = parse_digits(&date[5..7])? as i64;
    let day = parse_digits(&date[8..10])? as i64;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid CFTC date {text:?}"
        )));
    }
    days_from_civil(year, month, day)
        .checked_mul(86_400_000_000_000)
        .ok_or(DecodeError::TimestampOverflow)
}

fn parse_digits(bytes: &[u8]) -> Result<u32, DecodeError> {
    bytes.iter().try_fold(0u32, |value, byte| {
        byte.checked_sub(b'0')
            .filter(|digit| *digit <= 9)
            .map(|digit| value * 10 + u32::from(digit))
            .ok_or_else(|| DecodeError::UnsupportedSchema("invalid CFTC date digit".into()))
    })
}

fn source_event_id(
    row_id: &str,
    participant: ParticipantClass,
    long: i64,
    short: i64,
    spread: i64,
    open_interest: i64,
) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-cftc-tff-event-v1");
    hasher.update(row_id.as_bytes());
    hasher.update(&[participant as u8]);
    hasher.update(&long.to_le_bytes());
    hasher.update(&short.to_le_bytes());
    hasher.update(&spread.to_le_bytes());
    hasher.update(&open_interest.to_le_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    u64::from_le_bytes(bytes).max(1)
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

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/cftc/tff_indices.json");

    #[test]
    fn official_tff_shape_emits_five_participant_records_per_market() {
        let receipt = RawReceiptRef {
            id: ReceiptId(9),
            source: CFTC_SOURCE,
            stream: CFTC_TFF_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 1_000,
            ts_received_ns: 2_000,
            metadata: b"TFF indices",
            payload: FIXTURE,
            payload_hash: *blake3::hash(FIXTURE).as_bytes(),
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = CftcTffDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 4);
        assert_eq!(stats.canonical_events, 20);
        assert_eq!(batch.events[0].instrument_id(), instruments::US100);
        assert_eq!(
            batch.events[1].participant_class().unwrap(),
            ParticipantClass::AssetManager
        );
        assert_eq!(
            batch.events[1].positioning_counts().unwrap(),
            (100_767, 35_515, 5_113, 334_748)
        );
        assert_eq!(batch.events[19].instrument_id(), instruments::JP225);
    }

    #[test]
    fn report_date_is_utc_midnight_and_deterministic() {
        assert_eq!(parse_report_date("1970-01-01T00:00:00.000").unwrap(), 0);
    }
}

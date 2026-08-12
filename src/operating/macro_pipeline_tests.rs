use super::*;
use crate::data_plane::ids::macro_series;
use crate::operating::MacroFeedState;

const CPI: &[u8] = include_bytes!("../../tests/fixtures/bls/cpi_v1.json");
const BLS_CALENDAR: &[u8] = include_bytes!("../../tests/fixtures/bls/releases.ics");
const CFTC: &[u8] = include_bytes!("../../tests/fixtures/cftc/tff_indices.json");
const EURO_HICP: &[u8] = include_bytes!("../../tests/fixtures/eurostat/hicp_de.json");
const EURO_UNEMPLOYMENT: &[u8] =
    include_bytes!("../../tests/fixtures/eurostat/unemployment_de.json");
const EURO_GDP: &[u8] = include_bytes!("../../tests/fixtures/eurostat/gdp_ea20.json");
const EURO_RETAIL: &[u8] = include_bytes!("../../tests/fixtures/eurostat/retail_de.json");
const EURO_INDUSTRIAL: &[u8] = include_bytes!("../../tests/fixtures/eurostat/industrial_de.json");
const FRED_DGS10: &[u8] = include_bytes!("../../tests/fixtures/fred/dgs10.json");
const BEA_GDP: &[u8] = include_bytes!("../../tests/fixtures/bea/gdp.json");
const BEA_CORE_PCE: &[u8] = include_bytes!("../../tests/fixtures/bea/core_pce.json");
const CENSUS_MARTS: &[u8] = include_bytes!("../../tests/fixtures/census/marts_retail_sales.json");
const ECB_DEPOSIT: &[u8] = include_bytes!("../../tests/fixtures/ecb/deposit_rate.csv");
const ONS_CPI: &[u8] = include_bytes!("../../tests/fixtures/ons/cpi.csv");
const ONS_GDP: &[u8] = include_bytes!("../../tests/fixtures/ons/gdp.csv");
const ONS_UNEMPLOYMENT: &[u8] = include_bytes!("../../tests/fixtures/ons/unemployment.csv");
const ONS_RETAIL: &[u8] = include_bytes!("../../tests/fixtures/ons/retail.csv");
const BOE_BANK_RATE: &[u8] = include_bytes!("../../tests/fixtures/boe/bank_rate.csv");
const BOJ_CALL_RATE: &[u8] = include_bytes!("../../tests/fixtures/boj/call_rate.json");
const BOJ_TANKAN: &[u8] = include_bytes!("../../tests/fixtures/boj/tankan.json");

fn fixture(code: &'static str) -> FetchedMacroReceipt {
    let payload = if code == "CUUR0000SA0" {
        CPI.to_vec()
    } else {
        String::from_utf8_lossy(CPI)
            .replace("CUUR0000SA0", code)
            .into_bytes()
    };
    FetchedMacroReceipt {
        provider_code: code,
        status_code: 200,
        ts_started_ns: 100,
        ts_received_ns: 200,
        payload,
    }
}

fn macro_value(snapshot: &MacroSnapshot, series_id: SeriesId) -> f64 {
    snapshot
        .series
        .iter()
        .find(|series| series.series_id == series_id)
        .and_then(|series| series.latest)
        .unwrap()
        .value
}

fn feed_state(
    snapshot: &MacroSnapshot,
    source: SourceId,
    stream: crate::data_plane::ids::StreamId,
) -> MacroFeedState {
    snapshot
        .feeds
        .iter()
        .find(|feed| feed.source == source && feed.stream == stream)
        .unwrap()
        .state
}

#[test]
fn feed_truth_distinguishes_configuration_waiting_and_rejection() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    plane.set_feed_configured(FRED_SOURCE, FRED_OBSERVATIONS_STREAM, false);
    let recovered = plane.recovered_snapshot(2);
    assert_eq!(
        feed_state(&recovered, FRED_SOURCE, FRED_OBSERVATIONS_STREAM),
        MacroFeedState::NotConfigured
    );
    assert_eq!(
        feed_state(&recovered, BLS_SOURCE, BLS_TIMESERIES_STREAM),
        MacroFeedState::Awaiting
    );

    let rejected = plane.failure_snapshot(
        BLS_SOURCE,
        BLS_TIMESERIES_STREAM,
        "BLS payload rejected",
        5_000,
    );
    let feed = rejected
        .feeds
        .iter()
        .find(|feed| feed.source == BLS_SOURCE && feed.stream == BLS_TIMESERIES_STREAM)
        .unwrap();
    assert_eq!(feed.state, MacroFeedState::Rejected);
    assert_eq!(&*feed.detail, "BLS payload rejected");
    assert_eq!(feed.next_retry_ns, 5_000);
}

#[test]
fn complete_refresh_is_durable_and_duplicate_safe() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = BLS_SERIES
        .iter()
        .map(|binding| fixture(binding.provider_code))
        .collect();
    let first = plane.ingest_complete_bls_refresh(refresh, 1_000).unwrap();
    assert_eq!(first.raw_receipts, 4);
    assert_eq!(first.committed_events, 8);
    let material = first.material.unwrap();
    assert_eq!(
        material.first_sequence,
        crate::data_plane::ids::JournalSequence(1)
    );
    assert_eq!(
        material.last_sequence,
        crate::data_plane::ids::JournalSequence(8)
    );
    assert_eq!(
        first.snapshot.series.len(),
        crate::data_plane::providers::macro_catalog::MACRO_SERIES_COUNT
    );

    let repeated = BLS_SERIES
        .iter()
        .map(|binding| fixture(binding.provider_code))
        .collect();
    let second = plane.ingest_complete_bls_refresh(repeated, 2_000).unwrap();
    assert_eq!(second.committed_events, 0);
    assert!(second.material.is_none());
    drop(plane);

    let mut recovered = MacroPlane::open_or_create(dir.path(), 3).unwrap();
    let snapshot = recovered.recovered_snapshot(3);
    assert_eq!(snapshot.event_count, 8);
    assert_eq!(snapshot.receipt_count, 8);
    let material = recovered.take_recovered_material();
    assert_eq!(material.len(), 1);
    assert_eq!(material[0].event_count, 8);
    assert!(recovered.take_recovered_material().is_empty());
}

#[test]
fn release_calendar_is_durable_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let receipt = || FetchedBlsCalendarReceipt {
        status_code: 200,
        ts_started_ns: 100,
        ts_received_ns: 200,
        payload: BLS_CALENDAR.to_vec(),
    };
    let first = plane.ingest_bls_calendar_refresh(receipt(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 1);
    assert_eq!(first.committed_events, 3);
    assert_eq!(first.snapshot.release_count, 2);
    assert_eq!(first.snapshot.document_count, 1);
    assert!(first.snapshot.releases[0].scheduled_ns < first.snapshot.releases[1].scheduled_ns);
    assert_eq!(
        first.material.unwrap().kind,
        crate::operating::MacroMaterialKind::Provenance
    );

    let repeated = plane.ingest_bls_calendar_refresh(receipt(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    drop(plane);

    let mut recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.release_count, 2);
    assert_eq!(snapshot.document_count, 1);
    assert_eq!(snapshot.event_count, 3);
    assert_eq!(snapshot.receipt_count, 2);
    let material = recovered.take_recovered_material();
    assert_eq!(material.len(), 1);
    assert_eq!(
        material[0].kind,
        crate::operating::MacroMaterialKind::Provenance
    );
}

#[test]
fn partial_refresh_never_reaches_storage() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let error = plane
        .ingest_complete_bls_refresh(vec![fixture("CUUR0000SA0")], 1_000)
        .unwrap_err();
    assert!(matches!(error, MacroPlaneError::IncompleteRefresh { .. }));
    assert_eq!(
        MappedReceiptStore::open(dir.path().join("macro.raw"))
            .unwrap()
            .receipts()
            .count(),
        0
    );
}

#[test]
fn cftc_refresh_joins_the_same_journal_and_replays_positioning() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let first = plane
        .ingest_complete_cftc_refresh(
            FetchedCftcReceipt {
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: CFTC.to_vec(),
            },
            1_000,
        )
        .unwrap();
    assert_eq!(first.raw_receipts, 1);
    assert_eq!(first.committed_events, 20);
    assert_eq!(first.material.unwrap().event_count, 20);
    assert_eq!(first.snapshot.positioning.len(), 4);
    assert!(first.snapshot.positioning[0].latest_report_ns > 0);

    let repeated = plane
        .ingest_complete_cftc_refresh(
            FetchedCftcReceipt {
                status_code: 200,
                ts_started_ns: 300,
                ts_received_ns: 400,
                payload: CFTC.to_vec(),
            },
            2_000,
        )
        .unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    drop(plane);

    let recovered = MacroPlane::open_or_create(dir.path(), 500).unwrap();
    let snapshot = recovered.recovered_snapshot(500);
    let asset_managers = snapshot.positioning[0]
        .participant(ParticipantClass::AssetManager)
        .unwrap();
    assert_eq!(asset_managers.latest.unwrap().net(), 65_252);
    assert_eq!(snapshot.event_count, 20);
    assert_eq!(snapshot.receipt_count, 2);
}

#[test]
fn eurostat_refresh_is_atomic_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        vec![
            FetchedEurostatReceipt {
                dataset_code: "prc_hicp_minr",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: EURO_HICP.to_vec(),
            },
            FetchedEurostatReceipt {
                dataset_code: "une_rt_m",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: EURO_UNEMPLOYMENT.to_vec(),
            },
            FetchedEurostatReceipt {
                dataset_code: "namq_10_gdp",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: EURO_GDP.to_vec(),
            },
            FetchedEurostatReceipt {
                dataset_code: "sts_trtu_m",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: EURO_RETAIL.to_vec(),
            },
            FetchedEurostatReceipt {
                dataset_code: "sts_inpr_m",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: EURO_INDUSTRIAL.to_vec(),
            },
        ]
    };
    let first = plane
        .ingest_complete_eurostat_refresh(refresh(), 1_000)
        .unwrap();
    assert_eq!(first.raw_receipts, 5);
    assert_eq!(first.committed_events, 16);
    assert_eq!(first.material.unwrap().event_count, 16);
    assert_eq!(
        first.snapshot.series.len(),
        crate::data_plane::providers::macro_catalog::MACRO_SERIES_COUNT
    );
    let mut republished = refresh();
    for receipt in &mut republished {
        receipt.payload = String::from_utf8_lossy(&receipt.payload)
            .replace("2026-07-31T11:00:00+0200", "2026-08-01T11:00:00+0200")
            .into_bytes();
    }
    let second = plane
        .ingest_complete_eurostat_refresh(republished, 2_000)
        .unwrap();
    assert_eq!(second.committed_events, 0);
    assert!(second.material.is_none());
    drop(plane);
    let recovered = MacroPlane::open_or_create(dir.path(), 500).unwrap();
    let snapshot = recovered.recovered_snapshot(500);
    assert_eq!(snapshot.event_count, 16);
    assert_eq!(snapshot.receipt_count, 10);
    assert_eq!(
        macro_value(&snapshot, macro_series::DE_HICP_ALL_ITEMS_YOY),
        2.8
    );
    assert_eq!(
        macro_value(&snapshot, macro_series::DE_UNEMPLOYMENT_RATE_TC),
        3.9
    );
    assert_eq!(macro_value(&snapshot, macro_series::EA_REAL_GDP_QOQ), 0.4);
    assert_eq!(
        macro_value(&snapshot, macro_series::DE_RETAIL_VOLUME_SA),
        101.2
    );
    assert_eq!(
        macro_value(&snapshot, macro_series::DE_INDUSTRIAL_PRODUCTION_SA),
        92.1
    );
}

#[test]
fn fred_refresh_is_key_redacted_durable_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        crate::data_plane::providers::fred::FRED_SERIES
            .iter()
            .map(|binding| FetchedFredReceipt {
                provider_code: binding.provider_code,
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: FRED_DGS10.to_vec(),
            })
            .collect()
    };
    let first = plane
        .ingest_complete_fred_refresh(refresh(), 1_000)
        .unwrap();
    assert_eq!(first.raw_receipts, 3);
    assert_eq!(first.committed_events, 6);
    assert_eq!(
        first.snapshot.series.len(),
        crate::data_plane::providers::macro_catalog::MACRO_SERIES_COUNT
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::US_TREASURY_10Y),
        4.18
    );
    let repeated = plane
        .ingest_complete_fred_refresh(refresh(), 2_000)
        .unwrap();
    assert_eq!(repeated.committed_events, 0);
    drop(plane);

    let raw = MappedReceiptStore::open(dir.path().join("macro.raw")).unwrap();
    for receipt in raw.receipts() {
        assert!(!receipt
            .metadata
            .windows(7)
            .any(|window| window == b"api_key"));
    }
    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 6);
    assert_eq!(snapshot.receipt_count, 6);
    assert_eq!(
        macro_value(&snapshot, macro_series::US_EFFECTIVE_FED_FUNDS_RATE),
        4.18
    );
}

#[test]
fn bea_refresh_is_key_redacted_atomic_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        vec![
            FetchedBeaReceipt {
                provider_code: "T10101:1:Q",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: BEA_GDP.to_vec(),
            },
            FetchedBeaReceipt {
                provider_code: "T20804:6:M",
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: BEA_CORE_PCE.to_vec(),
            },
        ]
    };
    let first = plane.ingest_complete_bea_refresh(refresh(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 2);
    assert_eq!(first.committed_events, 4);
    assert_eq!(
        macro_value(&first.snapshot, macro_series::US_REAL_GDP_QOQ_ANNUALIZED),
        -0.5
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::US_CORE_PCE_PRICE_INDEX_SA),
        124.529
    );

    let repeated = plane.ingest_complete_bea_refresh(refresh(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    drop(plane);

    let raw = MappedReceiptStore::open(dir.path().join("macro.raw")).unwrap();
    for receipt in raw.receipts() {
        assert!(!receipt
            .metadata
            .windows(6)
            .any(|window| window == b"UserID"));
        assert!(!receipt
            .metadata
            .windows(7)
            .any(|window| window == b"api_key"));
    }
    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 4);
    assert_eq!(snapshot.receipt_count, 4);
    assert_eq!(
        macro_value(&snapshot, macro_series::US_REAL_GDP_QOQ_ANNUALIZED),
        -0.5
    );
    assert_eq!(
        macro_value(&snapshot, macro_series::US_CORE_PCE_PRICE_INDEX_SA),
        124.529
    );
}

#[test]
fn census_refresh_is_key_redacted_durable_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let receipt = || FetchedCensusReceipt {
        status_code: 200,
        ts_started_ns: 100,
        ts_received_ns: 200,
        payload: CENSUS_MARTS.to_vec(),
    };
    let first = plane.ingest_census_refresh(receipt(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 1);
    assert_eq!(first.committed_events, 2);
    assert_eq!(
        macro_value(
            &first.snapshot,
            macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA
        ),
        763_700.0
    );

    let repeated = plane.ingest_census_refresh(receipt(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    drop(plane);

    let raw = MappedReceiptStore::open(dir.path().join("macro.raw")).unwrap();
    for receipt in raw.receipts() {
        assert!(!receipt.metadata.windows(4).any(|window| window == b"key="));
    }
    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 2);
    assert_eq!(snapshot.receipt_count, 2);
    assert_eq!(
        macro_value(&snapshot, macro_series::US_ADVANCE_RETAIL_FOOD_SALES_SA),
        763_700.0
    );
}

#[test]
fn ecb_refresh_is_atomic_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        crate::data_plane::providers::ecb::ECB_SERIES
            .iter()
            .map(|binding| {
                let payload = if binding.provider_code.contains("DFR") {
                    ECB_DEPOSIT.to_vec()
                } else {
                    String::from_utf8_lossy(ECB_DEPOSIT)
                        .replace("DFR", "MRR_RT")
                        .replace("2.25", "2.40")
                        .into_bytes()
                };
                FetchedEcbReceipt {
                    provider_code: binding.provider_code,
                    status_code: 200,
                    ts_started_ns: 100,
                    ts_received_ns: 200,
                    payload,
                }
            })
            .collect()
    };
    let first = plane.ingest_complete_ecb_refresh(refresh(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 2);
    assert_eq!(first.committed_events, 6);
    assert_eq!(
        macro_value(&first.snapshot, macro_series::ECB_DEPOSIT_FACILITY_RATE),
        2.25
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::ECB_MAIN_REFINANCING_RATE),
        2.40
    );

    let repeated = plane.ingest_complete_ecb_refresh(refresh(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    drop(plane);

    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 6);
    assert_eq!(snapshot.receipt_count, 4);
    assert_eq!(
        macro_value(&snapshot, macro_series::ECB_DEPOSIT_FACILITY_RATE),
        2.25
    );
}

#[test]
fn ons_refresh_is_atomic_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        crate::data_plane::providers::ons::ONS_SERIES
            .iter()
            .map(|binding| FetchedOnsReceipt {
                provider_code: binding.provider_code,
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: match binding.provider_code {
                    "D7G7" => ONS_CPI.to_vec(),
                    "IHYQ" => ONS_GDP.to_vec(),
                    "MGSX" => ONS_UNEMPLOYMENT.to_vec(),
                    "J5EK" => ONS_RETAIL.to_vec(),
                    _ => unreachable!(),
                },
            })
            .collect()
    };
    let first = plane.ingest_complete_ons_refresh(refresh(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 4);
    assert_eq!(first.committed_events, 16);
    assert_eq!(
        first.material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Observations)
    );
    assert_eq!(
        first.provenance_material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Provenance)
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::UK_CPI_ALL_ITEMS_YOY),
        2.6
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::UK_REAL_GDP_QOQ),
        0.6
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::UK_UNEMPLOYMENT_RATE_SA),
        4.9
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::UK_RETAIL_VOLUME_SA),
        104.8
    );

    let repeated = plane.ingest_complete_ons_refresh(refresh(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    assert!(repeated.provenance_material.is_none());
    drop(plane);

    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 16);
    assert_eq!(snapshot.receipt_count, 8);
    assert_eq!(snapshot.document_count, 4);
}

#[test]
fn boe_refresh_is_atomic_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        vec![FetchedBoeReceipt {
            provider_code: "IUDBEDR",
            status_code: 200,
            ts_started_ns: 100,
            ts_received_ns: 200,
            payload: BOE_BANK_RATE.to_vec(),
        }]
    };
    let first = plane.ingest_complete_boe_refresh(refresh(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 1);
    assert_eq!(first.committed_events, 4);
    assert_eq!(
        first.material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Observations)
    );
    assert_eq!(
        first.provenance_material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Provenance)
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::BOE_BANK_RATE),
        3.75
    );

    let repeated = plane.ingest_complete_boe_refresh(refresh(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    assert!(repeated.provenance_material.is_none());
    drop(plane);

    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 4);
    assert_eq!(snapshot.receipt_count, 2);
    assert_eq!(snapshot.document_count, 1);
}

#[test]
fn boj_refresh_is_atomic_duplicate_safe_and_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let mut plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let refresh = || {
        crate::data_plane::providers::boj::BOJ_SERIES
            .iter()
            .map(|binding| FetchedBojReceipt {
                provider_code: binding.provider_code,
                status_code: 200,
                ts_started_ns: 100,
                ts_received_ns: 200,
                payload: match binding.provider_code {
                    "STRDCLUCON" => BOJ_CALL_RATE.to_vec(),
                    "TK99F1000601GCQ01000" => BOJ_TANKAN.to_vec(),
                    _ => unreachable!(),
                },
            })
            .collect()
    };
    let first = plane.ingest_complete_boj_refresh(refresh(), 1_000).unwrap();
    assert_eq!(first.raw_receipts, 2);
    assert_eq!(first.committed_events, 8);
    assert_eq!(
        first.material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Observations)
    );
    assert_eq!(
        first.provenance_material.as_ref().map(|batch| batch.kind),
        Some(crate::operating::MacroMaterialKind::Provenance)
    );
    assert_eq!(
        macro_value(&first.snapshot, macro_series::JP_OVERNIGHT_CALL_RATE),
        0.98
    );
    assert_eq!(
        macro_value(
            &first.snapshot,
            macro_series::JP_TANKAN_LARGE_MANUFACTURING_CONDITIONS
        ),
        22.0
    );

    let repeated = plane.ingest_complete_boj_refresh(refresh(), 2_000).unwrap();
    assert_eq!(repeated.committed_events, 0);
    assert!(repeated.material.is_none());
    assert!(repeated.provenance_material.is_none());
    drop(plane);

    let recovered = MacroPlane::open_or_create(dir.path(), 300).unwrap();
    let snapshot = recovered.recovered_snapshot(300);
    assert_eq!(snapshot.event_count, 8);
    assert_eq!(snapshot.receipt_count, 4);
    assert_eq!(snapshot.document_count, 2);
}

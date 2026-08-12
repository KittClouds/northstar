use northstar_index_fund::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use northstar_index_fund::data_plane::ids::{
    instruments, BatchId, CalendarId, CatalogVersion, DerivationVersion, ReceiptId, StreamId,
};
use northstar_index_fund::data_plane::journal::{JournalError, JournalWriter, MappedJournal};
use northstar_index_fund::data_plane::providers::massive_catalog::{
    MassiveCatalogManifestV1, MassiveIndexBindingV1, MassiveRecency, MassiveReferenceRole,
    MASSIVE_SOURCE,
};
use northstar_index_fund::data_plane::providers::massive_rest::MASSIVE_SNAPSHOT_STREAM;
use northstar_index_fund::data_plane::providers::massive_snapshot::MassiveIndexSnapshotDecoder;
use northstar_index_fund::data_plane::receipt::{
    MappedReceiptStore, RawReceipt, ReceiptStoreWriter,
};
use northstar_index_fund::data_plane::source::LicenseClass;
use northstar_index_fund::operating::{
    projector_from_massive_catalog, Availability, MassiveMarketError, MassiveMarketPlane,
    OperatingMode,
};
use serde_json::json;
use std::sync::Arc;

fn catalog(
) -> Arc<northstar_index_fund::data_plane::providers::massive_catalog::ValidatedMassiveCatalog> {
    let definitions = [
        (instruments::US100, "US100", "I:ONE"),
        (instruments::US500, "US500", "I:TWO"),
        (instruments::US30, "US30", "I:THREE"),
        (instruments::DE40, "DE40", "I:FOUR"),
        (instruments::UK100, "UK100", "I:FIVE"),
        (instruments::JP225, "JP225", "I:SIX"),
    ];
    Arc::new(
        MassiveCatalogManifestV1 {
            schema_version: 1,
            catalog_version: CatalogVersion(4),
            source_id: MASSIVE_SOURCE,
            captured_at_ns: 50,
            license: LicenseClass::InternalRestricted,
            indices: definitions
                .into_iter()
                .map(|(instrument, symbol, ticker)| MassiveIndexBindingV1 {
                    instrument,
                    northstar_symbol: symbol.into(),
                    display_name: format!("{symbol} fixture").into(),
                    provider_ticker: ticker.into(),
                    provider_name: format!("Provider {symbol}").into(),
                    reference_role: MassiveReferenceRole::ExactBenchmark,
                    binding_basis: format!("fixture exact binding for {symbol}").into(),
                    price_scale: 100,
                    effective_from_ns: 1,
                    verified_at_ns: 40,
                    recency: MassiveRecency::Delayed,
                })
                .collect(),
        }
        .validate()
        .unwrap(),
    )
}

fn calendar() -> Arc<SessionCalendar> {
    Arc::new(
        SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(4),
            [
                instruments::US100,
                instruments::US500,
                instruments::US30,
                instruments::DE40,
                instruments::UK100,
                instruments::JP225,
            ]
            .into_iter()
            .map(|instrument| Session {
                instrument,
                open_ns: 1,
                close_ns: i64::MAX,
                segment: SessionSegment::MainReference,
                flags: 0,
            })
            .collect(),
        )
        .unwrap(),
    )
}

fn refresh(catalog: &MassiveCatalogManifestV1, first_receipt: u64) -> Vec<RawReceipt> {
    catalog
        .indices
        .iter()
        .enumerate()
        .map(|(index, binding)| RawReceipt {
            id: ReceiptId(first_receipt + index as u64),
            source: MASSIVE_SOURCE,
            stream: MASSIVE_SNAPSHOT_STREAM,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 2_000_000_000,
            ts_received_ns: 2_000_000_010,
            license: LicenseClass::InternalRestricted,
            metadata: format!(r#"{{"ticker":"{}"}}"#, binding.provider_ticker).into_bytes(),
            payload: serde_json::to_vec(&json!({
                "status": "OK",
                "results": [{
                    "ticker": binding.provider_ticker,
                    "type": "indices",
                    "timeframe": "DELAYED",
                    "last_updated": 1_000_000_000 + index as i64,
                    "value": 100.25 + index as f64
                }]
            }))
            .unwrap(),
        })
        .collect()
}

#[test]
fn six_receipts_commit_one_batch_and_one_delayed_desk_generation() {
    let dir = tempfile::tempdir().unwrap();
    let raw_path = dir.path().join("massive.raw");
    let journal_path = dir.path().join("massive.canonical");
    let catalog = catalog();
    let decoder = MassiveIndexSnapshotDecoder::new(StreamId(1), Arc::clone(&catalog)).unwrap();
    let projector = projector_from_massive_catalog(
        OperatingMode::LiveData,
        &catalog,
        calendar(),
        DerivationVersion(1),
    )
    .unwrap();
    let mut plane = MassiveMarketPlane::new(
        ReceiptStoreWriter::create(&raw_path, 1).unwrap(),
        JournalWriter::create(&journal_path, 1).unwrap(),
        decoder,
        projector,
        BatchId(1),
    )
    .unwrap();

    let first = plane
        .ingest_refresh(refresh(catalog.manifest(), 1))
        .unwrap();
    assert_eq!(first.decoded.canonical_events, 6);
    assert_eq!(first.first_sequence.0, 1);
    assert_eq!(first.last_sequence.0, 6);
    assert_eq!(first.snapshot.generation, 1);
    assert_eq!(first.snapshot.availability, Availability::Delayed);
    assert_eq!(first.snapshot.instruments.len(), 6);
    assert!(first.snapshot.instruments.iter().all(|instrument| {
        instrument.reference.meta.availability == Availability::Delayed
            && instrument.reference.value_f64().is_some()
    }));

    let duplicate = plane
        .ingest_refresh(refresh(catalog.manifest(), 7))
        .unwrap_err();
    assert!(matches!(
        duplicate,
        MassiveMarketError::Journal(JournalError::DuplicateSourceEvent { .. })
    ));
    let (raw, journal, _) = plane.into_parts();
    drop(raw);
    drop(journal);

    let mapped_raw = MappedReceiptStore::open(&raw_path).unwrap();
    assert_eq!(mapped_raw.receipts().count(), 12);
    let mapped_journal = MappedJournal::open(&journal_path).unwrap();
    assert_eq!(mapped_journal.batch_count(), 1);
    assert_eq!(mapped_journal.event_count(), 6);
}

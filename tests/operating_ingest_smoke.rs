use northstar_index_fund::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use northstar_index_fund::data_plane::catalog::{
    AuthorityDomain, AuthorityLease, CanonicalCatalog, ProviderSymbolBinding,
};
use northstar_index_fund::data_plane::ids::{
    instruments, BatchId, CalendarId, CatalogVersion, DerivationVersion, ReceiptId, SourceId,
    StreamId,
};
use northstar_index_fund::data_plane::journal::{JournalWriter, MappedJournal};
use northstar_index_fund::data_plane::providers::massive::MassiveIndexValueDecoder;
use northstar_index_fund::data_plane::receipt::{
    MappedReceiptStore, RawReceipt, ReceiptStoreWriter,
};
use northstar_index_fund::data_plane::source::LicenseClass;
use northstar_index_fund::operating::{
    DomainMask, MarketInstrument, MarketProjector, MassiveDeskPipeline, NorthstarRuntime,
    OfficeSnapshotPort, OperatingMode,
};
use std::sync::Arc;

const SOURCE: SourceId = SourceId(10);
const STREAM: StreamId = StreamId(1);
const EVENT_MS: i64 = 1_678_220_098_130;
const EVENT_NS: i64 = EVENT_MS * 1_000_000;
const MINUTE_NS: i64 = 60_000_000_000;

fn catalog() -> Arc<CanonicalCatalog> {
    Arc::new(
        CanonicalCatalog::new(
            CatalogVersion(1),
            vec![ProviderSymbolBinding {
                source: SOURCE,
                instrument: instruments::US100,
                symbol: "I:NDX".into(),
                price_scale: 10_000,
                effective_from_ns: 0,
                effective_to_ns: i64::MAX,
                catalog_version: CatalogVersion(1),
            }],
            vec![AuthorityLease {
                domain: AuthorityDomain::ReferenceValue,
                entity_id: instruments::US100.get(),
                source: SOURCE,
                effective_from_ns: 0,
                effective_to_ns: i64::MAX,
            }],
        )
        .unwrap(),
    )
}

fn calendar() -> Arc<SessionCalendar> {
    Arc::new(
        SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![Session {
                instrument: instruments::US100,
                open_ns: EVENT_NS - MINUTE_NS,
                close_ns: EVENT_NS + 10 * MINUTE_NS,
                segment: SessionSegment::MainReference,
                flags: 0,
            }],
        )
        .unwrap(),
    )
}

#[test]
fn durable_massive_receipt_reaches_the_gpui_snapshot_port() {
    let directory = tempfile::tempdir().unwrap();
    let receipt_path = directory.path().join("massive.nsl0");
    let journal_path = directory.path().join("canonical.nsj");
    let (runtime, snapshots) = NorthstarRuntime::empty(OperatingMode::LiveData);
    let subscription = runtime.subscribe();
    let decoder = MassiveIndexValueDecoder::new(SOURCE, STREAM, catalog());
    let projector = MarketProjector::new(
        OperatingMode::LiveData,
        calendar(),
        DerivationVersion(1),
        vec![MarketInstrument::new(instruments::US100, "NASDAQ 100", "I:NDX", 10_000).unwrap()],
    )
    .unwrap();
    let mut pipeline = MassiveDeskPipeline::new(
        ReceiptStoreWriter::create(&receipt_path, EVENT_NS).unwrap(),
        JournalWriter::create(&journal_path, EVENT_NS).unwrap(),
        decoder,
        projector,
        snapshots,
        BatchId(1),
    )
    .unwrap();
    let payload = br#"{"ev":"V","val":3988.5,"T":"I:NDX","t":1678220098130}"#;
    let report = pipeline
        .ingest(RawReceipt {
            id: ReceiptId(1),
            source: SOURCE,
            stream: STREAM,
            status_code: 101,
            content_type: 1,
            ts_started_ns: EVENT_NS,
            ts_received_ns: EVENT_NS + 10,
            license: LicenseClass::LicensedNonDisplay,
            metadata: br#"{"transport":"websocket","channel":"V.I:NDX"}"#.to_vec(),
            payload: payload.to_vec(),
        })
        .unwrap();

    assert_eq!(report.first_sequence.get(), 1);
    assert_eq!(report.last_sequence.get(), 1);
    let notice = subscription.try_take_notice().unwrap();
    assert!(notice.changed.contains(DomainMask::DESK));
    let snapshot = runtime.current();
    assert_eq!(snapshot.desk.instruments.len(), 1);
    assert_eq!(
        snapshot.desk.instruments[0].reference.value_f64(),
        Some(3988.5)
    );
    assert_eq!(
        snapshot.desk.instruments[0]
            .reference
            .meta
            .journal_sequence
            .get(),
        1
    );

    let (receipts, journal, _, _) = pipeline.into_parts();
    drop(receipts);
    drop(journal);
    assert_eq!(
        MappedReceiptStore::open(&receipt_path)
            .unwrap()
            .receipts()
            .count(),
        1
    );
    assert_eq!(
        MappedJournal::open(&journal_path)
            .unwrap()
            .batches()
            .next()
            .unwrap()
            .events
            .len(),
        1
    );
}

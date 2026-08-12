use northstar_index_fund::data_plane::bars::{BarAggregator, CanonicalBar, MINUTE_NS};
use northstar_index_fund::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use northstar_index_fund::data_plane::catalog::{
    AuthorityDomain, AuthorityLease, CanonicalCatalog, ProviderSymbolBinding,
};
use northstar_index_fund::data_plane::event::CanonicalBatch;
use northstar_index_fund::data_plane::export::{DatasetManifest, DatasetSegment};
use northstar_index_fund::data_plane::ids::{
    BatchId, CalendarId, CatalogVersion, DerivationVersion, InstrumentId, ReceiptId, SourceId,
    StreamId,
};
use northstar_index_fund::data_plane::journal::{JournalWriter, MappedJournal};
use northstar_index_fund::data_plane::providers::massive::MassiveIndexValueDecoder;
use northstar_index_fund::data_plane::receipt::{
    MappedReceiptStore, RawReceipt, ReceiptStoreWriter,
};
use northstar_index_fund::data_plane::replay::{
    CanonicalConsumer, EventBatchRef, LivePublisher, ReplayConfig, ReplayEngine,
};
use northstar_index_fund::data_plane::source::{
    CanonicalDecoder, ExportScope, LicenseClass, SourceCapabilities, SourceContract,
};
use std::sync::Arc;

struct BarConsumer {
    aggregator: BarAggregator,
}

impl CanonicalConsumer for BarConsumer {
    fn apply_batch(&mut self, batch: EventBatchRef<'_>) {
        for event in batch.events {
            self.aggregator.ingest(event).expect("valid index value");
        }
    }
}

fn catalog(source: SourceId, instrument: InstrumentId) -> Arc<CanonicalCatalog> {
    Arc::new(
        CanonicalCatalog::new(
            CatalogVersion(1),
            vec![ProviderSymbolBinding {
                source,
                instrument,
                symbol: "I:FIXTURE".into(),
                price_scale: 100,
                effective_from_ns: 0,
                effective_to_ns: i64::MAX,
                catalog_version: CatalogVersion(1),
            }],
            vec![AuthorityLease {
                domain: AuthorityDomain::ReferenceValue,
                entity_id: instrument.get(),
                source,
                effective_from_ns: 0,
                effective_to_ns: i64::MAX,
            }],
        )
        .unwrap(),
    )
}

fn calendar(instrument: InstrumentId) -> Arc<SessionCalendar> {
    Arc::new(
        SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![Session {
                instrument,
                open_ns: 0,
                close_ns: 10 * MINUTE_NS,
                segment: SessionSegment::MainReference,
                flags: 0,
            }],
        )
        .unwrap(),
    )
}

fn finalized_bars(mut consumer: BarConsumer) -> Vec<CanonicalBar> {
    let mut bars = Vec::new();
    consumer
        .aggregator
        .advance_watermark(10 * MINUTE_NS, &mut bars);
    bars
}

#[test]
fn raw_to_canonical_to_live_and_replay_is_identical() {
    let directory = tempfile::tempdir().unwrap();
    let receipt_path = directory.path().join("massive.nsl0");
    let journal_path = directory.path().join("canonical.nsj");
    let source = SourceId(10);
    let stream = StreamId(1);
    let instrument = InstrumentId(1);
    let payload = br#"[
        {"ev":"V","val":20100.00,"T":"I:FIXTURE","t":30000},
        {"ev":"V","val":20104.00,"T":"I:FIXTURE","t":180000},
        {"ev":"V","val":20098.00,"T":"I:FIXTURE","t":60000},
        {"ev":"V","val":20110.00,"T":"I:FIXTURE","t":240001}
    ]"#;
    let raw = RawReceipt {
        id: ReceiptId(1),
        source,
        stream,
        status_code: 101,
        content_type: 1,
        ts_started_ns: 240_001_000_000,
        ts_received_ns: 240_001_000_100,
        license: LicenseClass::LicensedNonDisplay,
        metadata: br#"{"transport":"fixture-websocket","schema":"V"}"#.to_vec(),
        payload: payload.to_vec(),
    };

    let mut receipt_writer = ReceiptStoreWriter::create(&receipt_path, 0).unwrap();
    let expected_payload_hash = receipt_writer.append(&raw).unwrap();
    drop(receipt_writer);
    let receipts = MappedReceiptStore::open(&receipt_path).unwrap();
    let receipt = receipts.receipts().next().unwrap();
    assert_eq!(receipt.payload, payload);
    assert_eq!(receipt.payload_hash, expected_payload_hash);

    let canonical_catalog = catalog(source, instrument);
    let decoder = MassiveIndexValueDecoder::new(source, stream, canonical_catalog);
    let mut batch = CanonicalBatch::new(BatchId(1));
    let decoded = decoder.decode(receipt, &mut batch).unwrap();
    assert_eq!(decoded.canonical_events, 4);

    let journal_writer = JournalWriter::create(&journal_path, 0).unwrap();
    let live_consumer = BarConsumer {
        aggregator: BarAggregator::new(calendar(instrument), DerivationVersion(1)).unwrap(),
    };
    let mut live = LivePublisher::new(journal_writer, live_consumer);
    live.commit_and_publish(&mut batch).unwrap();
    let (writer, live_consumer) = live.into_parts();
    drop(writer);
    let live_bars = finalized_bars(live_consumer);

    let mapped = MappedJournal::open(&journal_path).unwrap();
    let replay_consumer = BarConsumer {
        aggregator: BarAggregator::new(calendar(instrument), DerivationVersion(1)).unwrap(),
    };
    let mut replay_consumer = replay_consumer;
    let report = ReplayEngine::new(&mapped).run(ReplayConfig::default(), &mut replay_consumer);
    let replay_bars = finalized_bars(replay_consumer);
    assert_eq!(report.event_count, 4);
    assert_eq!(
        bytemuck::cast_slice::<CanonicalBar, u8>(&live_bars),
        bytemuck::cast_slice::<CanonicalBar, u8>(&replay_bars)
    );

    let source_contract = SourceContract {
        source_id: source,
        name: "authorized fixture".into(),
        capabilities: SourceCapabilities(SourceCapabilities::INDEX_VALUES),
        license: LicenseClass::LicensedNonDisplay,
        contract_version: 1,
    };
    let manifest = DatasetManifest {
        format: "northstar-dataset-v1".into(),
        schema_version: 1,
        catalog_version: 1,
        calendar_version: 1,
        derivation_version: 1,
        replay_quality: "observed-live-fixture".into(),
        start_effective_ns: 0,
        end_effective_ns: 10 * MINUTE_NS,
        source_contract_versions: vec![(source.get(), 1)],
        segments: vec![DatasetSegment {
            relative_path: "canonical.nsj".into(),
            layer: "L1".into(),
            bytes: std::fs::metadata(&journal_path).unwrap().len(),
            blake3: blake3::Hash::from_bytes(mapped.content_hash())
                .to_hex()
                .to_string(),
        }],
    };
    manifest
        .validate_sources(&[source_contract], ExportScope::InternalResearch)
        .unwrap();
    let manifest_path = directory.path().join("dataset.json");
    manifest.write_atomic(&manifest_path).unwrap();
    assert!(manifest_path.is_file());
}

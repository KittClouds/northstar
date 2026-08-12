use northstar_index_fund::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use northstar_index_fund::data_plane::event::{CanonicalEvent, TimeQuality};
use northstar_index_fund::data_plane::ids::{
    instruments, BatchId, CalendarId, CatalogVersion, DerivationVersion, JournalSequence,
    ReceiptId, SourceId, StreamId,
};
use northstar_index_fund::data_plane::replay::EventBatchRef;
use northstar_index_fund::ledger::{
    ActorKind, LedgerCommit, LedgerEntryId, LedgerEventHeader, LedgerEventKind, LedgerEventStatus,
    TradeCaseId,
};
use northstar_index_fund::operating::{
    LedgerProjector, MarketInstrument, MarketProjector, OperatingMode, LEDGER_VISIBLE_ROWS,
};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

#[test]
#[ignore = "manual release-mode operating snapshot performance gate"]
fn project_one_hundred_thousand_values_in_bounded_batches() {
    const EVENT_COUNT: u64 = 100_000;
    const BATCH_LEN: u64 = 128;
    const STEP_NS: i64 = 100_000_000;

    let calendar = Arc::new(
        SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![Session {
                instrument: instruments::US100,
                open_ns: 0,
                close_ns: EVENT_COUNT as i64 * STEP_NS + 1,
                segment: SessionSegment::MainReference,
                flags: 0,
            }],
        )
        .unwrap(),
    );
    let instrument =
        MarketInstrument::new(instruments::US100, "NASDAQ 100", "I:NDX", 10_000).unwrap();
    let mut projector = MarketProjector::new(
        OperatingMode::LiveData,
        calendar,
        DerivationVersion(1),
        vec![instrument],
    )
    .unwrap();
    let mut events = Vec::with_capacity(BATCH_LEN as usize);
    let mut last_snapshot = None;

    let started = Instant::now();
    for batch_start in (1..=EVENT_COUNT).step_by(BATCH_LEN as usize) {
        events.clear();
        let batch_end = (batch_start + BATCH_LEN).min(EVENT_COUNT + 1);
        for sequence in batch_start..batch_end {
            let timestamp = sequence as i64 * STEP_NS;
            let mut event = CanonicalEvent::index_value(
                SourceId(1),
                StreamId(1),
                instruments::US100,
                ReceiptId(sequence),
                sequence,
                timestamp,
                timestamp + 1,
                200_000_000 + sequence as i64,
                TimeQuality::ObservedLive,
            )
            .unwrap();
            event.header.journal_sequence = sequence;
            events.push(event);
        }
        last_snapshot = projector.apply_batch(EventBatchRef {
            id: BatchId(batch_start),
            first_sequence: JournalSequence(batch_start),
            events: &events,
        });
    }
    let elapsed = started.elapsed();
    let snapshot = black_box(last_snapshot.unwrap());
    assert_eq!(projector.health().reference_values, EVENT_COUNT);
    assert_eq!(snapshot.last_sequence, JournalSequence(EVENT_COUNT));
    assert_eq!(snapshot.instruments.len(), 1);
    assert!(
        elapsed.as_millis() < 2_000,
        "100k operating projection took {elapsed:?}"
    );
    eprintln!(
        "OPERATING_100K events={} elapsed_ms={} desk_generation={}",
        EVENT_COUNT,
        elapsed.as_millis(),
        snapshot.generation
    );
}

#[test]
#[ignore = "manual release-mode Ledger projection performance gate"]
fn project_one_hundred_thousand_ledger_entries_with_bounded_visible_state() {
    const ENTRY_COUNT: u64 = 100_000;
    let body = b"operator observation";
    let body_hash = *blake3::hash(body).as_bytes();
    let mut projector = LedgerProjector::empty();
    let started = Instant::now();
    for entry_id in 1..=ENTRY_COUNT {
        let header = LedgerEventHeader {
            entry_id,
            case_id: 1,
            parent_entry_id: 0,
            correlation_low: 1,
            correlation_high: 0,
            source_key_low: entry_id,
            source_key_high: 0,
            actor_id: 1,
            ts_event_ns: entry_id as i64,
            ts_received_ns: entry_id as i64,
            ts_recorded_ns: entry_id as i64,
            canonical_first_sequence: 0,
            canonical_last_sequence: 0,
            body_offset: 0,
            account_id: 0,
            body_hash,
            body_len: body.len() as u32,
            instrument_id: instruments::US100.get(),
            schema_version: 1,
            kind: LedgerEventKind::OperatorObservation as u16,
            flags: 0,
            status: LedgerEventStatus::Accepted as u8,
            actor_kind: ActorKind::Operator as u8,
            reserved: [0; 8],
        };
        assert!(projector.apply_commit(
            LedgerCommit {
                entry_id: LedgerEntryId(entry_id),
                case_id: TradeCaseId(1),
                body_hash,
                header,
                inserted: true,
            },
            body,
        ));
        if entry_id % 128 == 0 {
            black_box(projector.snapshot());
        }
    }
    let elapsed = started.elapsed();
    let snapshot = black_box(projector.snapshot());
    assert_eq!(snapshot.entry_count, ENTRY_COUNT);
    assert_eq!(snapshot.case_count, 1);
    assert_eq!(snapshot.rows.len(), LEDGER_VISIBLE_ROWS);
    assert!(
        elapsed.as_millis() < 2_000,
        "100k Ledger projections took {elapsed:?}"
    );
    eprintln!(
        "LEDGER_100K entries={} elapsed_ms={} visible_rows={}",
        ENTRY_COUNT,
        elapsed.as_millis(),
        snapshot.rows.len()
    );
}

use super::*;
use crate::data_plane::ids::{instruments, JournalSequence, SchemaVersion};
use crate::ledger::{ActorId, CorrelationId, LedgerFlags};

fn draft(body: &[u8], source: u128) -> LedgerEventDraft<'_> {
    LedgerEventDraft {
        case_id: TradeCaseId(1),
        parent_entry_id: LedgerEntryId::UNKNOWN,
        correlation_id: CorrelationId(7),
        source_key: SourceEventKey(source),
        actor_id: ActorId(1),
        actor_kind: ActorKind::Operator,
        kind: LedgerEventKind::OperatorNote,
        status: LedgerEventStatus::Accepted,
        flags: LedgerFlags::MATERIAL,
        instrument_id: instruments::US100,
        account_id: 0,
        ts_event_ns: 10,
        ts_received_ns: 11,
        ts_recorded_ns: 12,
        canonical_sequences: Some((JournalSequence(20), JournalSequence(22))),
        schema_version: SchemaVersion(1),
        body,
    }
}

#[test]
fn append_reopen_and_mmap_body_are_exact() {
    let dir = tempfile::tempdir().unwrap();
    let hot = dir.path().join("ledger.hot");
    let cold = dir.path().join("ledger.body");
    let mut store = LedgerStore::create(&hot, &cold, 1).unwrap();
    let commit = store.append(&draft(br#"{"note":"first"}"#, 91)).unwrap();
    assert!(commit.inserted);
    assert_eq!(commit.entry_id, LedgerEntryId(1));
    drop(store);

    let mut reopened = LedgerStore::open(&hot, &cold).unwrap();
    assert_eq!(reopened.len(), 1);
    let duplicate = reopened
        .append(&draft(br#"{"note":"ignored retry"}"#, 91))
        .unwrap();
    assert!(!duplicate.inserted);
    assert_eq!(duplicate.entry_id, LedgerEntryId(1));
    drop(reopened);

    let mapped = LedgerMappedStore::open(&hot, &cold).unwrap();
    let entries: Vec<_> = mapped.iter().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].body, br#"{"note":"first"}"#);
    assert_eq!(
        entries[0].header.kind(),
        Some(LedgerEventKind::OperatorNote)
    );
}

#[test]
fn open_truncates_incomplete_hot_and_orphan_cold_tails() {
    let dir = tempfile::tempdir().unwrap();
    let hot = dir.path().join("ledger.hot");
    let cold = dir.path().join("ledger.body");
    let mut store = LedgerStore::create(&hot, &cold, 1).unwrap();
    store.append(&draft(b"kept", 1)).unwrap();
    drop(store);

    OpenOptions::new()
        .append(true)
        .open(&hot)
        .unwrap()
        .write_all(b"partial")
        .unwrap();
    OpenOptions::new()
        .append(true)
        .open(&cold)
        .unwrap()
        .write_all(b"orphan")
        .unwrap();

    let reopened = LedgerStore::open(&hot, &cold).unwrap();
    assert_eq!(reopened.len(), 1);
    drop(reopened);
    assert_eq!(LedgerMappedStore::open(&hot, &cold).unwrap().len(), 1);
}

#[test]
fn amendments_are_append_only_and_case_scoped() {
    let dir = tempfile::tempdir().unwrap();
    let hot = dir.path().join("ledger.hot");
    let cold = dir.path().join("ledger.body");
    let mut store = LedgerStore::create(&hot, &cold, 1).unwrap();
    store.append(&draft(b"original", 1)).unwrap();

    let mut amendment = draft(b"corrected", 2);
    amendment.kind = LedgerEventKind::Amendment;
    amendment.parent_entry_id = LedgerEntryId(1);
    assert_eq!(store.append(&amendment).unwrap().entry_id, LedgerEntryId(2));

    amendment.source_key = SourceEventKey(3);
    amendment.case_id = TradeCaseId(2);
    assert!(matches!(
        store.append(&amendment),
        Err(LedgerError::ParentCaseMismatch)
    ));
}

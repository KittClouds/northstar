use super::*;
use crate::data_plane::ids::{instruments, SchemaVersion};
use crate::data_plane::providers::bls::BLS_SERIES;
use crate::ledger::{
    ActorId, ActorKind, CorrelationId, LedgerEntryId, LedgerEventKind, LedgerEventStatus,
    LedgerFlags, SourceEventKey, TradeCaseId,
};
use crate::operating::{
    AutomaticCasePolicy, AutomaticLedgerEventCommand, Availability, OperatorEntryKind,
    OperatorLedgerCommand, OperatorMutation,
};

#[test]
fn empty_production_runtime_is_honest_and_contains_no_demo_values() {
    let (runtime, _) = NorthstarRuntime::empty(OperatingMode::LiveData);
    let snapshot = runtime.current();
    assert_eq!(snapshot.mode, OperatingMode::LiveData);
    assert!(snapshot.desk.instruments.is_empty());
    assert_eq!(
        snapshot.desk.availability,
        Availability::AwaitingFirstReceipt
    );
    assert_eq!(snapshot.fund.availability, Availability::NotConfigured);
}

#[test]
fn typed_operator_plan_is_durable_and_publishes_one_ledger_generation() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    let subscription = runtime.subscribe();
    runtime
        .submit_operator_event(OperatorLedgerCommand {
            command_key: SourceEventKey(99),
            case_id: TradeCaseId::UNKNOWN,
            correlation_id: CorrelationId(99),
            instrument_id: instruments::US100,
            submitted_ns: 10,
            entry_kind: OperatorEntryKind::Plan,
            mutation: OperatorMutation::Append,
            body: Arc::from("Opening plan: wait for the cash session."),
        })
        .unwrap();
    let (_cancel_tx, cancel_rx) = bounded(1);
    let notice = subscription.wait_or_cancel(&cancel_rx).unwrap();
    assert_eq!(notice.changed, super::super::DomainMask::LEDGER);
    let snapshot = runtime.current();
    assert_eq!(snapshot.ledger.entry_count, 1);
    assert_eq!(snapshot.ledger.case_count, 1);
    assert_eq!(snapshot.ledger.rows[0].kind, LedgerEventKind::OperatorPlan);
    assert_eq!(
        &*snapshot.ledger.rows[0].preview,
        "Opening plan: wait for the cash session."
    );
    drop(runtime);
    let mapped = LedgerMappedStore::open(
        dir.path().join("ledger.hot"),
        dir.path().join("ledger.body"),
    )
    .unwrap();
    assert_eq!(mapped.len(), 1);
}

#[test]
fn typed_operator_lifecycle_and_mutations_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    let subscription = runtime.subscribe();
    let submit = |key, case_id, entry_kind, mutation, body: &'static str| {
        runtime
            .submit_operator_event(OperatorLedgerCommand {
                command_key: SourceEventKey(key),
                case_id,
                correlation_id: CorrelationId(700),
                instrument_id: instruments::US100,
                submitted_ns: key as i64,
                entry_kind,
                mutation,
                body: Arc::from(body),
            })
            .unwrap();
        let (_cancel_tx, cancel_rx) = bounded(1);
        assert_eq!(
            subscription.wait_or_cancel(&cancel_rx).unwrap().changed,
            super::super::DomainMask::LEDGER
        );
    };

    submit(
        101,
        TradeCaseId::UNKNOWN,
        OperatorEntryKind::Plan,
        OperatorMutation::Append,
        "Opening plan",
    );
    let case_id = runtime.current().ledger.rows[0].case_id;
    submit(
        102,
        case_id,
        OperatorEntryKind::Observation,
        OperatorMutation::Append,
        "Cash-session observation",
    );
    submit(
        103,
        case_id,
        OperatorEntryKind::Plan,
        OperatorMutation::Amend(LedgerEntryId(1)),
        "Opening plan, clarified",
    );
    submit(
        104,
        case_id,
        OperatorEntryKind::Observation,
        OperatorMutation::Redact(LedgerEntryId(2)),
        "Entered against the wrong session",
    );
    drop(runtime);

    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    let ledger = &runtime.current().ledger;
    assert_eq!(ledger.entry_count, 4);
    assert_eq!(ledger.case_count, 1);
    assert_eq!(ledger.operator_count, 4);
    assert_eq!(ledger.rows[0].kind, LedgerEventKind::Redaction);
    assert_eq!(ledger.rows[0].parent_entry_id, LedgerEntryId(2));
    assert_eq!(ledger.rows[1].kind, LedgerEventKind::Amendment);
    assert_eq!(ledger.rows[1].parent_entry_id, LedgerEntryId(1));
    assert_eq!(ledger.rows[2].kind, LedgerEventKind::OperatorObservation);
    assert_eq!(ledger.rows[3].kind, LedgerEventKind::OperatorPlan);
}

fn lifecycle_event(
    source: u128,
    correlation: CorrelationId,
    kind: LedgerEventKind,
) -> AutomaticLedgerEventCommand {
    AutomaticLedgerEventCommand {
        source_key: SourceEventKey(source),
        case_policy: AutomaticCasePolicy::Correlate,
        correlation_id: correlation,
        actor_id: ActorId(77),
        actor_kind: ActorKind::Venue,
        kind,
        status: LedgerEventStatus::Observed,
        flags: LedgerFlags::MATERIAL,
        instrument_id: instruments::US100,
        account_id: 91,
        ts_event_ns: source as i64,
        ts_received_ns: source as i64,
        canonical_sequences: None,
        schema_version: SchemaVersion(1),
        body: Arc::from(format!("{} lifecycle evidence", kind.label()).into_bytes()),
    }
}

#[test]
fn automatic_lifecycle_reuses_case_after_retry_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let correlation = CorrelationId(44);
    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    runtime
        .submit_automatic_event(lifecycle_event(1, correlation, LedgerEventKind::Order))
        .unwrap();
    let fill = lifecycle_event(2, correlation, LedgerEventKind::Fill);
    runtime.submit_automatic_event(fill.clone()).unwrap();
    runtime.submit_automatic_event(fill).unwrap();
    drop(runtime);

    let mapped = LedgerMappedStore::open(
        dir.path().join("ledger.hot"),
        dir.path().join("ledger.body"),
    )
    .unwrap();
    assert_eq!(mapped.len(), 2);
    let cases: Vec<_> = mapped.iter().map(|entry| entry.header.case_id()).collect();
    assert_ne!(cases[0], TradeCaseId::UNKNOWN);
    assert_eq!(cases[0], cases[1]);
    drop(mapped);

    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    runtime
        .submit_automatic_event(lifecycle_event(3, correlation, LedgerEventKind::Position))
        .unwrap();
    drop(runtime);
    let mapped = LedgerMappedStore::open(
        dir.path().join("ledger.hot"),
        dir.path().join("ledger.body"),
    )
    .unwrap();
    assert_eq!(mapped.len(), 3);
    assert!(mapped
        .iter()
        .all(|entry| entry.header.case_id() == cases[0]));
}

#[test]
fn recovered_canonical_macro_batch_backfills_ledger_once() {
    const CPI: &[u8] = include_bytes!("../../tests/fixtures/bls/cpi_v1.json");
    let dir = tempfile::tempdir().unwrap();
    let mut macro_plane = MacroPlane::open_or_create(dir.path(), 1).unwrap();
    let receipts = BLS_SERIES
        .iter()
        .map(|binding| FetchedMacroReceipt {
            provider_code: binding.provider_code,
            status_code: 200,
            ts_started_ns: 100,
            ts_received_ns: 200,
            payload: if binding.provider_code == "CUUR0000SA0" {
                CPI.to_vec()
            } else {
                String::from_utf8_lossy(CPI)
                    .replace("CUUR0000SA0", binding.provider_code)
                    .into_bytes()
            },
        })
        .collect();
    macro_plane
        .ingest_complete_bls_refresh(receipts, 1_000)
        .unwrap();
    drop(macro_plane);

    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    let snapshot = runtime.current();
    assert_eq!(snapshot.ledger.entry_count, 1);
    assert_eq!(snapshot.ledger.machine_count, 1);
    assert_eq!(snapshot.ledger.rows[0].kind, LedgerEventKind::MacroRelease);
    assert_eq!(snapshot.ledger.rows[0].canonical_first_sequence.get(), 1);
    assert_eq!(snapshot.ledger.rows[0].canonical_last_sequence.get(), 8);
    drop(runtime);

    let runtime = NorthstarRuntime::with_ledger(OperatingMode::LiveData, dir.path()).unwrap();
    assert_eq!(runtime.current().ledger.entry_count, 1);
}

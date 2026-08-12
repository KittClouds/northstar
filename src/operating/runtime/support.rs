use super::*;
use crate::operating::Availability;

pub(super) fn tradelocker_supervisor(
    mut client: TradeLockerReadClient,
    decoder: TradeLockerDecoder,
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
) {
    let mut wait = Duration::ZERO;
    loop {
        if !wait.is_zero() {
            match cancel.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
        } else if cancel.try_recv().is_ok() {
            return;
        }
        match crate::operating::tradelocker_source::poll_epoch(&mut client, &decoder) {
            Ok(epoch) => {
                if commands
                    .send(RuntimeCommand::TradeLockerEpoch(epoch))
                    .is_err()
                {
                    return;
                }
                wait = TRADELOCKER_REFRESH_INTERVAL;
            }
            Err(error) => {
                let availability = error.availability();
                eprintln!("NORTHSTAR_TRADELOCKER_REFRESH_REJECTED error={error}");
                if commands
                    .send(RuntimeCommand::TradeLockerFailure(
                        availability,
                        Arc::from(format!("TradeLocker refresh rejected: {error}")),
                    ))
                    .is_err()
                {
                    return;
                }
                wait = TRADELOCKER_FAILURE_RETRY;
            }
        }
    }
}

pub(super) fn massive_supervisor(
    mut adapter: MassiveRestSnapshotAdapter,
    commands: Sender<RuntimeCommand>,
    cancel: Receiver<()>,
) {
    let mut wait = Duration::ZERO;
    loop {
        if !wait.is_zero() {
            match cancel.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
        } else if cancel.try_recv().is_ok() {
            return;
        }
        let mut receipts = smallvec::SmallVec::new();
        wait = match adapter.poll(&mut receipts) {
            Ok(()) => {
                if commands
                    .send(RuntimeCommand::MassiveRefresh(receipts.into_vec()))
                    .is_err()
                {
                    return;
                }
                MASSIVE_SNAPSHOT_INTERVAL
            }
            Err(error) => {
                let availability = match error {
                    SourceError::RateLimited => Availability::Stale,
                    SourceError::Disconnected => Availability::Disconnected,
                    SourceError::SchemaDrift(_) | SourceError::Rejected(_) => Availability::Invalid,
                };
                if commands
                    .send(RuntimeCommand::DeskFailure(availability))
                    .is_err()
                {
                    return;
                }
                MASSIVE_FAILURE_RETRY
            }
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn publish_macro_result(
    result: Result<MacroIngestReport, MacroPlaneError>,
    label: &str,
    source: crate::data_plane::ids::SourceId,
    stream: crate::data_plane::ids::StreamId,
    failure_retry: Duration,
    macro_plane: &mut MacroPlane,
    store: &mut LedgerStore,
    projector: &mut LedgerProjector,
    publisher: &mut SnapshotPublisher,
) -> bool {
    match result {
        Ok(mut report) => {
            macro_plane.clear_feed_failure(source, stream);
            report.snapshot = macro_plane.published_snapshot(
                report.snapshot.availability,
                Arc::clone(&report.snapshot.health),
                report.snapshot.next_refresh_ns,
            );
            publish_macro_ingest(report, store, projector, publisher);
            true
        }
        Err(error) => {
            publisher.publish_macro(macro_plane.failure_snapshot(
                source,
                stream,
                Arc::from(format!("{label} refresh rejected: {error}")),
                now_ns().saturating_add(duration_ns(failure_retry)),
            ));
            false
        }
    }
}

fn publish_macro_ingest(
    report: MacroIngestReport,
    store: &mut LedgerStore,
    projector: &mut LedgerProjector,
    publisher: &mut SnapshotPublisher,
) {
    for material in [report.material, report.provenance_material]
        .into_iter()
        .flatten()
    {
        let command = AutomaticLedgerEventCommand::from_macro_batch(material);
        match append_automatic_event(store, projector, &command, now_ns()) {
            Ok(true) => {
                publisher.publish_ledger(Arc::new(projector.snapshot()));
            }
            Ok(false) => {}
            Err(error) => publish_ledger_failure(
                projector,
                publisher,
                format!("Macro material receipt failed: {error}"),
            ),
        }
    }
    publisher.publish_macro(report.snapshot);
}

pub(super) fn publish_ledger_failure(
    projector: &LedgerProjector,
    publisher: &mut SnapshotPublisher,
    message: String,
) {
    let mut snapshot = projector.snapshot();
    snapshot.availability = Availability::Invalid;
    snapshot.health = Arc::from(message);
    publisher.publish_ledger(Arc::new(snapshot));
}

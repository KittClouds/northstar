use crate::contracts::GateStatus;
use crate::office::{
    CoverageRow, IncidentReceipt, OperationGroup, OperationNode, OperationsSnapshot,
};

pub(super) fn fixture() -> OperationsSnapshot {
    OperationsSnapshot {
        publication_sequence: 8_412,
        snapshot_age_ms: 47,
        catalog_coverage_pct: 96,
        storage_megabytes: 184,
        replay_exact: true,
        nodes: vec![
            node(
                OperationGroup::Data,
                "Massive",
                "streaming",
                "42 ms",
                "6 / 6 bindings",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Data,
                "Official macro",
                "published",
                "11 adapters",
                "1 late release",
                GateStatus::Observe,
            ),
            node(
                OperationGroup::Data,
                "CFTC TFF",
                "current",
                "5 / 6 mappings",
                "DE40 under review",
                GateStatus::Wait,
            ),
            node(
                OperationGroup::Engine,
                "Nautilus",
                "reconciled",
                "epoch 9",
                "event queue 0",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Engine,
                "Snapshot publisher",
                "atomic",
                "seq 8412",
                "47 ms old",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Venue,
                "TradeLocker",
                "synced",
                "SyncEnd",
                "orders 2 / positions 1",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Venue,
                "Instrument contracts",
                "agree",
                "6 / 6",
                "route/session checked",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Machine,
                "Clock",
                "synchronized",
                "3 ms",
                "America/New_York",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Machine,
                "Replay",
                "exact",
                "1,842 receipts",
                "schema v2",
                GateStatus::Pass,
            ),
            node(
                OperationGroup::Machine,
                "Storage",
                "healthy",
                "184 MB",
                "mmap generation 14",
                GateStatus::Pass,
            ),
        ]
        .into(),
        coverage: vec![
            coverage(
                "US official",
                "34 / 34",
                "18.2y",
                "12m",
                "current",
                "6 vintages",
            ),
            coverage("CFTC TFF", "5 / 6", "9.4y", "3d", "current", "weekly"),
            coverage(
                "Euro area",
                "18 / 18",
                "16.0y",
                "1d",
                "current",
                "4 vintages",
            ),
            coverage(
                "United Kingdom",
                "14 / 14",
                "14.8y",
                "2d",
                "current",
                "2 vintages",
            ),
            coverage(
                "Japan",
                "15 / 15",
                "17.1y",
                "late 7m",
                "observe",
                "3 vintages",
            ),
        ]
        .into(),
        incidents: vec![
            IncidentReceipt {
                incident_id: 5_301,
                time_label: "09:35:12",
                system: "e-Stat calendar",
                summary: "Industrial production release late by 7 minutes",
                recovery: "holding prior value / next check 09:45",
                status: GateStatus::Observe,
            },
            IncidentReceipt {
                incident_id: 5_300,
                time_label: "08:42:01",
                system: "Macro publisher",
                summary: "US CPI vintage 114 published atomically",
                recovery: "no action / receipt bls-91f2",
                status: GateStatus::Pass,
            },
            IncidentReceipt {
                incident_id: 5_299,
                time_label: "08:29:58",
                system: "TradeLocker",
                summary: "Execution snapshot reconciled before session",
                recovery: "epoch 9 / no unknown outcomes",
                status: GateStatus::Pass,
            },
        ]
        .into(),
    }
}

fn node(
    group: OperationGroup,
    name: &'static str,
    state: &'static str,
    metric: &'static str,
    detail: &'static str,
    status: GateStatus,
) -> OperationNode {
    OperationNode {
        group,
        name,
        state,
        metric,
        detail,
        status,
    }
}

fn coverage(
    family: &'static str,
    cataloged: &'static str,
    history: &'static str,
    latest: &'static str,
    freshness: &'static str,
    revisions: &'static str,
) -> CoverageRow {
    CoverageRow {
        family,
        cataloged,
        history,
        latest,
        freshness,
        revisions,
        replay: GateStatus::Pass,
    }
}

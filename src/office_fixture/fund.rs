use crate::contracts::{GateStatus, IndexKey};
use crate::office::{AttributionRow, FundDetailSnapshot, IndexExposure, StressScenario};

pub(super) fn fixture() -> FundDetailSnapshot {
    FundDetailSnapshot {
        realized_today: 358.0,
        unrealized_today: 128.0,
        remaining_daily_r: 1.26,
        exposures: vec![
            IndexExposure {
                index: IndexKey::Us100,
                notional_dollars: 20_118.0,
                risk_r: 0.31,
                pnl_dollars: 128.0,
                cluster: "US growth",
                positions: 1,
            },
            IndexExposure {
                index: IndexKey::Us500,
                notional_dollars: 0.0,
                risk_r: 0.18,
                pnl_dollars: 214.0,
                cluster: "US broad",
                positions: 0,
            },
            IndexExposure {
                index: IndexKey::Us30,
                notional_dollars: 0.0,
                risk_r: 0.0,
                pnl_dollars: 16.0,
                cluster: "US value",
                positions: 0,
            },
            IndexExposure {
                index: IndexKey::De40,
                notional_dollars: 0.0,
                risk_r: 0.0,
                pnl_dollars: 0.0,
                cluster: "Europe",
                positions: 0,
            },
            IndexExposure {
                index: IndexKey::Uk100,
                notional_dollars: 0.0,
                risk_r: 0.0,
                pnl_dollars: 0.0,
                cluster: "United Kingdom",
                positions: 0,
            },
            IndexExposure {
                index: IndexKey::Jp225,
                notional_dollars: 0.0,
                risk_r: 0.0,
                pnl_dollars: 0.0,
                cluster: "Japan",
                positions: 0,
            },
        ]
        .into(),
        stress: vec![
            StressScenario {
                name: "All stops",
                assumption: "defined stops + 1.5pt slippage",
                loss_dollars: 186.0,
                loss_r: 0.31,
                status: GateStatus::Pass,
            },
            StressScenario {
                name: "US cluster shock",
                assumption: "US indices -1.0% simultaneous",
                loss_dollars: 274.0,
                loss_r: 0.46,
                status: GateStatus::Observe,
            },
            StressScenario {
                name: "Venue disconnect",
                assumption: "position held / no new route",
                loss_dollars: 186.0,
                loss_r: 0.31,
                status: GateStatus::Pass,
            },
        ]
        .into(),
        attribution: vec![
            AttributionRow {
                label: "US100 / Momentum-5m",
                realized_dollars: 0.0,
                unrealized_dollars: 128.0,
                fees_dollars: -2.4,
                total_r: 0.21,
            },
            AttributionRow {
                label: "US500 / Open-range",
                realized_dollars: 214.0,
                unrealized_dollars: 0.0,
                fees_dollars: -3.1,
                total_r: 0.36,
            },
            AttributionRow {
                label: "US30 / Mean-revert",
                realized_dollars: 144.0,
                unrealized_dollars: 0.0,
                fees_dollars: -1.8,
                total_r: 0.24,
            },
        ]
        .into(),
    }
}

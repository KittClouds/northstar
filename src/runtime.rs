use crate::contracts::DashboardSnapshot;
#[cfg(feature = "fixtures")]
use crate::contracts::{
    Bar, BarSeries, DecisionReceipt, FundSnapshot, GateStatus, IndexKey, IndexSnapshot,
    MarketSession, PromotionGate, RiskSnapshot, StrategySnapshot, SystemHealth,
};
#[cfg(feature = "fixtures")]
use hashbrown::HashMap;
#[cfg(feature = "fixtures")]
use memchr::memchr;
#[cfg(feature = "fixtures")]
use smallvec::smallvec;
use std::sync::Arc;

pub trait MarketDataPort: Send + Sync {
    fn dashboard_snapshot(&self) -> Arc<DashboardSnapshot>;
}

pub trait TradingControlPort: Send + Sync {
    fn live_gate(&self) -> LiveGate;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveGate {
    pub massive_fresh: bool,
    pub tradelocker_synced: bool,
    pub nautilus_reconciled: bool,
    pub clock_synchronized: bool,
    pub no_unknown_orders: bool,
    pub instrument_contracts_agree: bool,
}

impl LiveGate {
    #[inline]
    pub fn ready(self) -> bool {
        self.massive_fresh
            && self.tradelocker_synced
            && self.nautilus_reconciled
            && self.clock_synchronized
            && self.no_unknown_orders
            && self.instrument_contracts_agree
    }
}

#[cfg(feature = "fixtures")]
pub struct PrototypeRuntime {
    snapshot: Arc<DashboardSnapshot>,
    index_by_key: HashMap<IndexKey, usize>,
}

#[cfg(feature = "fixtures")]
impl PrototypeRuntime {
    pub fn fixture() -> Self {
        let office = crate::office_fixture::office_snapshots();
        let indices: Arc<[IndexSnapshot]> = vec![
            index_fixture(
                IndexKey::Us100,
                "NASDAQ 100",
                "I:NDX",
                "US100",
                20_118.4,
                0.42,
                0.8,
                1.6,
                MarketSession::Open,
                "EXPANSION",
                18,
            ),
            index_fixture(
                IndexKey::Us500,
                "S&P 500",
                "I:SPX",
                "US500",
                5_924.8,
                0.18,
                -0.2,
                0.7,
                MarketSession::Open,
                "BALANCED",
                9,
            ),
            index_fixture(
                IndexKey::Us30,
                "DOW 30",
                "I:DJI",
                "US30",
                43_784.0,
                -0.11,
                1.3,
                2.2,
                MarketSession::Open,
                "COMPRESSION",
                27,
            ),
            index_fixture(
                IndexKey::De40,
                "DAX 40",
                "I:DAX",
                "DE40",
                21_504.0,
                0.31,
                0.0,
                1.9,
                MarketSession::Closed,
                "CLOSED",
                14,
            ),
            index_fixture(
                IndexKey::Uk100,
                "FTSE 100",
                "I:UKX",
                "UK100",
                8_706.0,
                -0.07,
                0.0,
                1.1,
                MarketSession::Closed,
                "CLOSED",
                33,
            ),
            index_fixture(
                IndexKey::Jp225,
                "NIKKEI 225",
                "I:NKY",
                "JP225",
                40_228.0,
                0.62,
                0.0,
                3.4,
                MarketSession::PreMarket,
                "PRE-MARKET",
                21,
            ),
        ]
        .into();

        let decisions: Arc<[DecisionReceipt]> = vec![
            DecisionReceipt {
                receipt_id: 1_842,
                time_label: "09:41:05",
                system: "Momentum-5m",
                index: IndexKey::Us100,
                summary: "Held LONG after spread gate passed",
                evidence: "signal 0.62 / threshold 0.58 / basis 0.8",
                outcome: GateStatus::Pass,
                risk_r: 0.31,
            },
            DecisionReceipt {
                receipt_id: 1_841,
                time_label: "09:36:11",
                system: "Mean-revert",
                index: IndexKey::Us30,
                summary: "Entry suppressed by regime conflict",
                evidence: "expansion probability 0.81 / ceiling 0.55",
                outcome: GateStatus::Block,
                risk_r: 0.0,
            },
            DecisionReceipt {
                receipt_id: 1_840,
                time_label: "09:31:42",
                system: "Open-range",
                index: IndexKey::Us500,
                summary: "Breakout candidate retained in shadow",
                evidence: "breadth 0.74 / liquidity healthy / no order",
                outcome: GateStatus::Observe,
                risk_r: 0.18,
            },
        ]
        .into();

        let strategies: Arc<[StrategySnapshot]> = vec![
            StrategySnapshot {
                name: "Momentum-5m",
                index: IndexKey::Us100,
                disposition: "ACTIVE LONG",
                return_r: 0.46,
                health: "healthy",
                gates: smallvec![
                    gate("Replay determinism", "exact", GateStatus::Pass),
                    gate("Shadow agreement", "98.7% / min 98%", GateStatus::Pass),
                    gate("Slippage budget", "12 / 100 fills", GateStatus::Observe),
                    gate("Tail loss", "14 / 30 sessions", GateStatus::Wait),
                ],
            },
            StrategySnapshot {
                name: "Open-range",
                index: IndexKey::Us500,
                disposition: "FLAT",
                return_r: 0.18,
                health: "waiting",
                gates: smallvec![gate(
                    "Session coverage",
                    "19 / 30 sessions",
                    GateStatus::Wait
                )],
            },
            StrategySnapshot {
                name: "Mean-revert",
                index: IndexKey::Us30,
                disposition: "SUPPRESSED",
                return_r: -0.06,
                health: "regime gate",
                gates: smallvec![gate("Regime conflict", "expansion 0.81", GateStatus::Block)],
            },
        ]
        .into();

        let snapshot = Arc::new(DashboardSnapshot {
            snapshot_id: 1_188_402,
            indices,
            decisions,
            strategies,
            fund: FundSnapshot {
                net_liquidation: 101_842.0,
                today_pnl: 486.0,
                inception_pct: 1.84,
                gross_exposure: 0.63,
                risk: RiskSnapshot {
                    daily_used_r: 0.74,
                    daily_limit_r: 2.0,
                    open_exposure_r: 0.31,
                    max_loss_dollars: 186.0,
                    portfolio_heat_r: 0.63,
                    portfolio_limit_r: 1.50,
                    drawdown_pct: -1.26,
                    drawdown_limit_pct: -4.0,
                },
            },
            health: SystemHealth {
                massive_latency_ms: 42,
                tradelocker_synced: true,
                nautilus_reconciled: true,
                clock_drift_ms: 3,
                unknown_orders: 0,
            },
            macro_office: office.macro_office,
            operations: office.operations,
            lifecycle: office.ledger,
            fund_detail: office.fund_detail,
        });
        let index_by_key = snapshot
            .indices
            .iter()
            .enumerate()
            .map(|(index, item)| (item.key, index))
            .collect();
        Self {
            snapshot,
            index_by_key,
        }
    }

    #[inline]
    pub fn index_snapshot(&self, key: IndexKey) -> &IndexSnapshot {
        &self.snapshot.indices[self.index_by_key[&key]]
    }
}

#[cfg(feature = "fixtures")]
impl MarketDataPort for PrototypeRuntime {
    fn dashboard_snapshot(&self) -> Arc<DashboardSnapshot> {
        Arc::clone(&self.snapshot)
    }
}

#[cfg(feature = "fixtures")]
impl TradingControlPort for PrototypeRuntime {
    fn live_gate(&self) -> LiveGate {
        let health = &self.snapshot.health;
        LiveGate {
            massive_fresh: health.massive_latency_ms < 1_000,
            tradelocker_synced: health.tradelocker_synced,
            nautilus_reconciled: health.nautilus_reconciled,
            clock_synchronized: health.clock_drift_ms < 250,
            no_unknown_orders: health.unknown_orders == 0,
            instrument_contracts_agree: true,
        }
    }
}

#[cfg(feature = "fixtures")]
fn gate(label: &'static str, detail: &'static str, status: GateStatus) -> PromotionGate {
    PromotionGate {
        label,
        detail,
        status,
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "fixtures")]
fn index_fixture(
    key: IndexKey,
    display_name: &'static str,
    massive_ticker: &'static str,
    tradelocker_symbol: &'static str,
    reference_price: f32,
    daily_change_pct: f32,
    basis_points: f32,
    spread_points: f32,
    session: MarketSession,
    regime: &'static str,
    phase: u32,
) -> IndexSnapshot {
    debug_assert!(memchr(b':', massive_ticker.as_bytes()).is_some());
    let bars = generate_bars(reference_price, phase);
    IndexSnapshot {
        key,
        display_name,
        massive_ticker,
        tradelocker_symbol,
        reference_price,
        execution_price: reference_price + basis_points,
        daily_change_pct,
        basis_points,
        spread_points,
        session,
        regime,
        bars: BarSeries::new(bars),
    }
}

#[cfg(feature = "fixtures")]
fn generate_bars(base: f32, phase: u32) -> Vec<Bar> {
    const COUNT: usize = 48;
    let amplitude = (base * 0.00048).max(1.2);
    let drift = amplitude * 0.10;
    let mut bars = Vec::with_capacity(COUNT);
    let mut previous = base - amplitude * 2.7;
    for index in 0..COUNT {
        let angle = (index as f32 + phase as f32) * 0.58;
        let open = previous;
        let close = open + angle.sin() * amplitude * 0.46 + drift;
        let wick = amplitude * (0.32 + 0.18 * (angle * 1.7).cos().abs());
        bars.push(Bar {
            timestamp_ns: 1_786_289_400_000_000_000 + index as u64 * 300_000_000_000,
            open,
            high: open.max(close) + wick,
            low: open.min(close) - wick * 0.82,
            close,
            volume: 2_000.0 + 900.0 * (angle * 0.7).sin().abs(),
            flags: 0,
        });
        previous = close;
    }
    bars
}

#[cfg(all(test, feature = "fixtures"))]
mod tests {
    use super::*;

    #[test]
    fn live_gate_fails_closed_and_fixture_is_ready() {
        assert!(!LiveGate {
            massive_fresh: true,
            tradelocker_synced: true,
            nautilus_reconciled: false,
            clock_synchronized: true,
            no_unknown_orders: true,
            instrument_contracts_agree: true,
        }
        .ready());
        assert!(PrototypeRuntime::fixture().live_gate().ready());
    }

    #[test]
    fn symbol_lookup_is_stable() {
        let runtime = PrototypeRuntime::fixture();
        assert_eq!(
            runtime.index_snapshot(IndexKey::Us100).massive_ticker,
            "I:NDX"
        );
        assert_eq!(
            runtime.index_snapshot(IndexKey::Us100).tradelocker_symbol,
            "US100"
        );
    }

    #[test]
    fn fixture_bars_are_dense_and_finite() {
        let runtime = PrototypeRuntime::fixture();
        for index in runtime.snapshot.indices.iter() {
            assert_eq!(index.bars.bars().len(), 48);
            assert!(index.bars.extents().low.is_finite());
            assert!(index.bars.extents().high > index.bars.extents().low);
        }
    }

    #[test]
    fn office_publication_is_single_immutable_snapshot() {
        let runtime = PrototypeRuntime::fixture();
        let first = runtime.dashboard_snapshot();
        let second = runtime.dashboard_snapshot();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.snapshot_id, first.macro_office.snapshot_id);
        assert_eq!(first.operations.publication_sequence, 8_412);
    }
}

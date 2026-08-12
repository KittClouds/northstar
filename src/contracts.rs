use bytemuck::{Pod, Zeroable};
use pulp::{Arch, Simd, WithSimd};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum IndexKey {
    Us100,
    Us500,
    Us30,
    De40,
    Uk100,
    Jp225,
}

impl IndexKey {
    pub const ALL: [Self; 6] = [
        Self::Us100,
        Self::Us500,
        Self::Us30,
        Self::De40,
        Self::Uk100,
        Self::Jp225,
    ];

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Us100 => "US100",
            Self::Us500 => "US500",
            Self::Us30 => "US30",
            Self::De40 => "DE40",
            Self::Uk100 => "UK100",
            Self::Jp225 => "JP225",
        }
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Bar {
    pub timestamp_ns: u64,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: f32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceExtents {
    pub low: f32,
    pub high: f32,
}

#[derive(Clone, Debug)]
pub struct BarSeries {
    bars: Arc<[Bar]>,
    lows: Arc<[f32]>,
    highs: Arc<[f32]>,
    extents: PriceExtents,
}

impl BarSeries {
    pub fn new(bars: Vec<Bar>) -> Self {
        let mut lows = Vec::with_capacity(bars.len());
        let mut highs = Vec::with_capacity(bars.len());
        lows.extend(bars.iter().map(|bar| bar.low));
        highs.extend(bars.iter().map(|bar| bar.high));
        let extents = simd_extents(&lows, &highs);
        Self {
            bars: bars.into(),
            lows: lows.into(),
            highs: highs.into(),
            extents,
        }
    }

    #[inline]
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    #[inline]
    pub fn extents(&self) -> PriceExtents {
        self.extents
    }

    #[inline]
    pub fn low_lane(&self) -> &[f32] {
        &self.lows
    }

    #[inline]
    pub fn high_lane(&self) -> &[f32] {
        &self.highs
    }
}

fn simd_extents(lows: &[f32], highs: &[f32]) -> PriceExtents {
    if lows.is_empty() || highs.is_empty() {
        return PriceExtents {
            low: 0.0,
            high: 1.0,
        };
    }
    Arch::new().dispatch(ExtentsKernel { lows, highs })
}

struct ExtentsKernel<'a> {
    lows: &'a [f32],
    highs: &'a [f32],
}

impl WithSimd for ExtentsKernel<'_> {
    type Output = PriceExtents;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (low_blocks, low_tail) = S::as_simd_f32s(self.lows);
        let (high_blocks, high_tail) = S::as_simd_f32s(self.highs);
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for values in low_blocks.iter().copied() {
            low = low.min(simd.reduce_min_f32s(values));
        }
        for &value in low_tail {
            low = low.min(value);
        }
        for values in high_blocks.iter().copied() {
            high = high.max(simd.reduce_max_f32s(values));
        }
        for &value in high_tail {
            high = high.max(value);
        }
        PriceExtents { low, high }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MarketSession {
    Open,
    Closed,
    PreMarket,
}

#[derive(Clone, Debug)]
pub struct IndexSnapshot {
    pub key: IndexKey,
    pub display_name: &'static str,
    pub massive_ticker: &'static str,
    pub tradelocker_symbol: &'static str,
    pub reference_price: f32,
    pub execution_price: f32,
    pub daily_change_pct: f32,
    pub basis_points: f32,
    pub spread_points: f32,
    pub session: MarketSession,
    pub regime: &'static str,
    pub bars: BarSeries,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GateStatus {
    Pass,
    Observe,
    Wait,
    Block,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromotionGate {
    pub label: &'static str,
    pub detail: &'static str,
    pub status: GateStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionReceipt {
    pub receipt_id: u64,
    pub time_label: &'static str,
    pub system: &'static str,
    pub index: IndexKey,
    pub summary: &'static str,
    pub evidence: &'static str,
    pub outcome: GateStatus,
    pub risk_r: f32,
}

#[derive(Clone, Debug)]
pub struct StrategySnapshot {
    pub name: &'static str,
    pub index: IndexKey,
    pub disposition: &'static str,
    pub return_r: f32,
    pub health: &'static str,
    pub gates: SmallVec<[PromotionGate; 6]>,
}

#[derive(Clone, Copy, Debug)]
pub struct RiskSnapshot {
    pub daily_used_r: f32,
    pub daily_limit_r: f32,
    pub open_exposure_r: f32,
    pub max_loss_dollars: f32,
    pub portfolio_heat_r: f32,
    pub portfolio_limit_r: f32,
    pub drawdown_pct: f32,
    pub drawdown_limit_pct: f32,
}

#[derive(Clone, Debug)]
pub struct FundSnapshot {
    pub net_liquidation: f32,
    pub today_pnl: f32,
    pub inception_pct: f32,
    pub gross_exposure: f32,
    pub risk: RiskSnapshot,
}

#[derive(Clone, Debug)]
pub struct SystemHealth {
    pub massive_latency_ms: u32,
    pub tradelocker_synced: bool,
    pub nautilus_reconciled: bool,
    pub clock_drift_ms: u32,
    pub unknown_orders: u32,
}

#[derive(Clone, Debug)]
pub struct DashboardSnapshot {
    pub snapshot_id: u64,
    pub indices: Arc<[IndexSnapshot]>,
    pub decisions: Arc<[DecisionReceipt]>,
    pub strategies: Arc<[StrategySnapshot]>,
    pub fund: FundSnapshot,
    pub health: SystemHealth,
    pub macro_office: crate::office::MacroOfficeSnapshot,
    pub operations: crate::office::OperationsSnapshot,
    pub lifecycle: crate::office::LifecycleLedgerSnapshot,
    pub fund_detail: crate::office::FundDetailSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simd_extents_match_scalar_truth() {
        let lows = [9.0, 4.0, 8.0, 1.5, 3.0, 7.0, 5.0, 2.0, 6.0];
        let highs = [10.0, 14.0, 12.0, 18.5, 11.0, 13.0, 19.0, 16.0, 15.0];
        assert_eq!(
            simd_extents(&lows, &highs),
            PriceExtents {
                low: 1.5,
                high: 19.0
            }
        );
    }

    #[test]
    fn bars_are_stable_zero_copy_records() {
        assert_eq!(std::mem::size_of::<Bar>(), 32);
        assert_eq!(std::mem::align_of::<Bar>(), 8);
    }
}

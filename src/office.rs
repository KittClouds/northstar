use crate::contracts::{GateStatus, IndexKey};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum RegionKey {
    UnitedStates,
    EuroArea,
    UnitedKingdom,
    Japan,
}

impl RegionKey {
    pub const ALL: [Self; 4] = [
        Self::UnitedStates,
        Self::EuroArea,
        Self::UnitedKingdom,
        Self::Japan,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::UnitedStates => "United States",
            Self::EuroArea => "Euro area / Germany",
            Self::UnitedKingdom => "United Kingdom",
            Self::Japan => "Japan",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum MacroFamily {
    Growth,
    Inflation,
    Labor,
    Rates,
    Positioning,
    EventRisk,
}

impl MacroFamily {
    pub const ALL: [Self; 6] = [
        Self::Growth,
        Self::Inflation,
        Self::Labor,
        Self::Rates,
        Self::Positioning,
        Self::EventRisk,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub const fn label(self) -> &'static str {
        match self {
            Self::Growth => "Growth",
            Self::Inflation => "Inflation",
            Self::Labor => "Labor",
            Self::Rates => "Rates",
            Self::Positioning => "Positioning",
            Self::EventRisk => "Event risk",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MacroRegionSnapshot {
    pub region: RegionKey,
    pub growth: &'static str,
    pub inflation: &'static str,
    pub labor: &'static str,
    pub policy_rate: &'static str,
    pub rates_context: &'static str,
    pub currency_context: &'static str,
    pub next_release: &'static str,
    pub freshness: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct MacroContextCell {
    pub index: IndexKey,
    pub family: MacroFamily,
    pub state: &'static str,
    pub value: &'static str,
    pub freshness: &'static str,
    pub status: GateStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseStatus {
    Scheduled,
    Received,
    Revised,
    Late,
}

#[derive(Clone, Copy, Debug)]
pub struct MacroRelease {
    pub release_id: u64,
    pub time_label: &'static str,
    pub region: RegionKey,
    pub name: &'static str,
    pub actual: &'static str,
    pub previous: &'static str,
    pub revised: &'static str,
    pub change: &'static str,
    pub source: &'static str,
    pub freshness: &'static str,
    pub status: ReleaseStatus,
    pub provenance: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct PositioningSnapshot {
    pub index: IndexKey,
    pub contract: &'static str,
    pub asset_manager_net: i32,
    pub leveraged_money_net: i32,
    pub dealer_net: i32,
    pub weekly_delta: i32,
    pub percentile: u8,
    pub open_interest: u32,
    pub report_age_days: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceDocument {
    pub document_id: u64,
    pub institution: &'static str,
    pub kind: &'static str,
    pub title: &'static str,
    pub published: &'static str,
    pub scope: &'static str,
    pub receipt: &'static str,
    pub official: bool,
}

#[derive(Clone, Debug)]
pub struct MacroOfficeSnapshot {
    pub snapshot_id: u64,
    pub catalog_version: u32,
    pub as_of: &'static str,
    pub next_release: &'static str,
    pub late_releases: u16,
    pub stale_families: u16,
    pub regions: Arc<[MacroRegionSnapshot]>,
    /// Dense index-major storage: index ordinal * family count + family ordinal.
    pub context: Arc<[MacroContextCell]>,
    pub releases: Arc<[MacroRelease]>,
    pub positioning: Arc<[PositioningSnapshot]>,
    pub documents: Arc<[SourceDocument]>,
}

impl MacroOfficeSnapshot {
    #[inline]
    pub fn context_cell(&self, index: IndexKey, family: MacroFamily) -> &MacroContextCell {
        let slot = index as usize * MacroFamily::COUNT + family as usize;
        &self.context[slot]
    }

    #[inline]
    pub fn region(&self, key: RegionKey) -> &MacroRegionSnapshot {
        &self.regions[key as usize]
    }

    #[inline]
    pub fn positioning(&self, key: IndexKey) -> Option<&PositioningSnapshot> {
        self.positioning.iter().find(|item| item.index == key)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationGroup {
    Data,
    Engine,
    Venue,
    Machine,
}

#[derive(Clone, Copy, Debug)]
pub struct OperationNode {
    pub group: OperationGroup,
    pub name: &'static str,
    pub state: &'static str,
    pub metric: &'static str,
    pub detail: &'static str,
    pub status: GateStatus,
}

#[derive(Clone, Copy, Debug)]
pub struct CoverageRow {
    pub family: &'static str,
    pub cataloged: &'static str,
    pub history: &'static str,
    pub latest: &'static str,
    pub freshness: &'static str,
    pub revisions: &'static str,
    pub replay: GateStatus,
}

#[derive(Clone, Copy, Debug)]
pub struct IncidentReceipt {
    pub incident_id: u64,
    pub time_label: &'static str,
    pub system: &'static str,
    pub summary: &'static str,
    pub recovery: &'static str,
    pub status: GateStatus,
}

#[derive(Clone, Debug)]
pub struct OperationsSnapshot {
    pub publication_sequence: u64,
    pub snapshot_age_ms: u32,
    pub catalog_coverage_pct: u8,
    pub storage_megabytes: u32,
    pub replay_exact: bool,
    pub nodes: Arc<[OperationNode]>,
    pub coverage: Arc<[CoverageRow]>,
    pub incidents: Arc<[IncidentReceipt]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptKind {
    Observation,
    Decision,
    RiskEvaluation,
    CommandIntent,
    VenueOrder,
    Fill,
    Position,
    Exit,
    Reconciliation,
    Incident,
    OperatorAction,
}

impl ReceiptKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observation => "Observation",
            Self::Decision => "Decision",
            Self::RiskEvaluation => "Risk",
            Self::CommandIntent => "Intent",
            Self::VenueOrder => "Order",
            Self::Fill => "Fill",
            Self::Position => "Position",
            Self::Exit => "Exit",
            Self::Reconciliation => "Reconcile",
            Self::Incident => "Incident",
            Self::OperatorAction => "Operator",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LifecycleReceipt {
    pub receipt_id: u64,
    pub parent_id: Option<u64>,
    pub time_label: &'static str,
    pub index: Option<IndexKey>,
    pub kind: ReceiptKind,
    pub origin: &'static str,
    pub summary: &'static str,
    pub evidence: &'static str,
    pub risk_r: Option<f32>,
    pub venue_truth: &'static str,
    pub latency_us: u32,
    pub status: GateStatus,
    pub provenance: &'static str,
}

#[derive(Clone, Debug)]
pub struct LifecycleLedgerSnapshot {
    pub receipts: Arc<[LifecycleReceipt]>,
    pub decisions: u16,
    pub routed: u16,
    pub blocked: u16,
    pub fills: u16,
    pub unknown: u16,
    pub incidents: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexExposure {
    pub index: IndexKey,
    pub notional_dollars: f32,
    pub risk_r: f32,
    pub pnl_dollars: f32,
    pub cluster: &'static str,
    pub positions: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct StressScenario {
    pub name: &'static str,
    pub assumption: &'static str,
    pub loss_dollars: f32,
    pub loss_r: f32,
    pub status: GateStatus,
}

#[derive(Clone, Copy, Debug)]
pub struct AttributionRow {
    pub label: &'static str,
    pub realized_dollars: f32,
    pub unrealized_dollars: f32,
    pub fees_dollars: f32,
    pub total_r: f32,
}

#[derive(Clone, Debug)]
pub struct FundDetailSnapshot {
    pub realized_today: f32,
    pub unrealized_today: f32,
    pub remaining_daily_r: f32,
    pub exposures: Arc<[IndexExposure]>,
    pub stress: Arc<[StressScenario]>,
    pub attribution: Arc<[AttributionRow]>,
}

#[derive(Clone, Debug)]
pub struct OfficeSnapshots {
    pub macro_office: MacroOfficeSnapshot,
    pub operations: OperationsSnapshot,
    pub ledger: LifecycleLedgerSnapshot,
    pub fund_detail: FundDetailSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_family_layout_is_dense_and_stable() {
        assert_eq!(MacroFamily::ALL.len(), 6);
        assert_eq!(MacroFamily::Growth as usize, 0);
        assert_eq!(MacroFamily::EventRisk as usize, 5);
    }

    #[test]
    fn hot_rows_remain_compact() {
        assert!(std::mem::size_of::<MacroContextCell>() <= 64);
        assert!(std::mem::size_of::<LifecycleReceipt>() <= 144);
    }
}

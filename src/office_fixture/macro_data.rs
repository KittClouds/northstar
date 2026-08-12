use crate::contracts::{GateStatus, IndexKey};
use crate::office::{
    MacroContextCell, MacroFamily, MacroOfficeSnapshot, MacroRegionSnapshot, MacroRelease,
    PositioningSnapshot, RegionKey, ReleaseStatus, SourceDocument,
};
use std::sync::Arc;

pub(super) fn fixture() -> MacroOfficeSnapshot {
    let regions: Arc<[MacroRegionSnapshot]> = vec![
        MacroRegionSnapshot {
            region: RegionKey::UnitedStates,
            growth: "GDP 3.8% / firm",
            inflation: "CPI 2.9% / sticky",
            labor: "Claims 218k / balanced",
            policy_rate: "Fed 4.25–4.50%",
            rates_context: "2Y 4.08 / 10Y 4.31",
            currency_context: "DXY 102.4 / +0.2%",
            next_release: "Initial claims / T-18m",
            freshness: "calendar current / 08:42",
        },
        MacroRegionSnapshot {
            region: RegionKey::EuroArea,
            growth: "GDP 0.2% QoQ / soft",
            inflation: "HICP 2.4% / easing",
            labor: "Unemployment 6.3%",
            policy_rate: "ECB deposit 2.00%",
            rates_context: "Bund 2Y 2.21 / 10Y 2.59",
            currency_context: "EURUSD 1.086 / -0.1%",
            next_release: "DE industrial output / 1d",
            freshness: "Eurostat current / 07:00",
        },
        MacroRegionSnapshot {
            region: RegionKey::UnitedKingdom,
            growth: "GDP 0.3% QoQ / stable",
            inflation: "CPI 2.8% / sticky",
            labor: "Wages 5.1% / cooling",
            policy_rate: "BoE 4.25%",
            rates_context: "Gilt 2Y 4.03 / 10Y 4.41",
            currency_context: "GBPUSD 1.274 / flat",
            next_release: "Retail sales / 2d",
            freshness: "ONS current / 02:00",
        },
        MacroRegionSnapshot {
            region: RegionKey::Japan,
            growth: "GDP -0.4% QoQ / soft",
            inflation: "CPI 2.1% / cooling",
            labor: "Unemployment 2.5%",
            policy_rate: "BOJ 0.50%",
            rates_context: "JGB 2Y 0.79 / 10Y 1.43",
            currency_context: "USDJPY 151.8 / +0.4%",
            next_release: "Tankan / 3d",
            freshness: "BOJ/e-Stat current / 23:50",
        },
    ]
    .into();

    let mut context = Vec::with_capacity(IndexKey::ALL.len() * MacroFamily::COUNT);
    push_context(
        &mut context,
        IndexKey::Us100,
        [
            ("firm", "GDP 3.8%", GateStatus::Pass),
            ("sticky", "CPI 2.9%", GateStatus::Observe),
            ("balanced", "claims 218k", GateStatus::Pass),
            ("restrictive", "2Y 4.08%", GateStatus::Observe),
            ("extended", "82nd pct", GateStatus::Observe),
            ("near", "claims T-18m", GateStatus::Wait),
        ],
    );
    push_context(
        &mut context,
        IndexKey::Us500,
        [
            ("firm", "retail +0.4%", GateStatus::Pass),
            ("sticky", "PCE 2.7%", GateStatus::Observe),
            ("balanced", "NFP 22k", GateStatus::Observe),
            ("restrictive", "10Y 4.31%", GateStatus::Observe),
            ("supported", "67th pct", GateStatus::Pass),
            ("near", "claims T-18m", GateStatus::Wait),
        ],
    );
    push_context(
        &mut context,
        IndexKey::Us30,
        [
            ("mixed", "industry -0.2%", GateStatus::Observe),
            ("sticky", "PPI 2.6%", GateStatus::Observe),
            ("cooling", "wages 3.7%", GateStatus::Pass),
            ("restrictive", "curve +23bp", GateStatus::Observe),
            ("neutral", "51st pct", GateStatus::Pass),
            ("near", "claims T-18m", GateStatus::Wait),
        ],
    );
    push_context(
        &mut context,
        IndexKey::De40,
        [
            ("soft", "GDP +0.2%", GateStatus::Observe),
            ("easing", "HICP 2.4%", GateStatus::Pass),
            ("stable", "unemp 6.3%", GateStatus::Pass),
            ("easing", "ECB 2.00%", GateStatus::Pass),
            ("unavailable", "mapping review", GateStatus::Wait),
            ("clear", "next 1d", GateStatus::Pass),
        ],
    );
    push_context(
        &mut context,
        IndexKey::Uk100,
        [
            ("stable", "GDP +0.3%", GateStatus::Pass),
            ("sticky", "CPI 2.8%", GateStatus::Observe),
            ("cooling", "wages 5.1%", GateStatus::Pass),
            ("restrictive", "BoE 4.25%", GateStatus::Observe),
            ("balanced", "56th pct", GateStatus::Pass),
            ("clear", "next 2d", GateStatus::Pass),
        ],
    );
    push_context(
        &mut context,
        IndexKey::Jp225,
        [
            ("soft", "GDP -0.4%", GateStatus::Observe),
            ("cooling", "CPI 2.1%", GateStatus::Pass),
            ("stable", "unemp 2.5%", GateStatus::Pass),
            ("normalizing", "BOJ 0.50%", GateStatus::Observe),
            ("supported", "74th pct", GateStatus::Pass),
            ("clear", "Tankan 3d", GateStatus::Pass),
        ],
    );

    MacroOfficeSnapshot {
        snapshot_id: 1_188_402,
        catalog_version: 42,
        as_of: "09:42:18 ET",
        next_release: "Initial claims / T-18m",
        late_releases: 1,
        stale_families: 0,
        regions,
        context: context.into(),
        releases: releases(),
        positioning: positioning(),
        documents: documents(),
    }
}

fn push_context(
    target: &mut Vec<MacroContextCell>,
    index: IndexKey,
    values: [(&'static str, &'static str, GateStatus); MacroFamily::COUNT],
) {
    target.extend(MacroFamily::ALL.into_iter().zip(values).map(
        |(family, (state, value, status))| MacroContextCell {
            index,
            family,
            state,
            value,
            freshness: "current",
            status,
        },
    ));
}

fn releases() -> Arc<[MacroRelease]> {
    vec![
        release(
            7_031,
            "10:00",
            RegionKey::UnitedStates,
            "Initial jobless claims",
            "—",
            "218k",
            "—",
            "scheduled",
            "BLS/FRED",
            "T-18m",
            ReleaseStatus::Scheduled,
            "calendar v42 / series ICSA",
        ),
        release(
            7_030,
            "08:30",
            RegionKey::UnitedStates,
            "CPI YoY",
            "2.9%",
            "2.7%",
            "—",
            "+0.2pp",
            "BLS",
            "12m ago",
            ReleaseStatus::Received,
            "receipt bls-91f2 / vintage 114",
        ),
        release(
            7_029,
            "08:30",
            RegionKey::UnitedStates,
            "Nonfarm payrolls",
            "22k",
            "75k",
            "68k",
            "-53k",
            "BLS",
            "5d ago",
            ReleaseStatus::Revised,
            "receipt bls-77ac / vintage 203",
        ),
        release(
            7_028,
            "07:00",
            RegionKey::EuroArea,
            "German retail sales",
            "+0.6%",
            "+0.5%",
            "+0.4%",
            "+0.1pp",
            "Eurostat",
            "1d ago",
            ReleaseStatus::Received,
            "receipt estat-44be / vintage 39",
        ),
        release(
            7_027,
            "02:00",
            RegionKey::UnitedKingdom,
            "Average weekly earnings",
            "5.1%",
            "5.4%",
            "—",
            "-0.3pp",
            "ONS",
            "2d ago",
            ReleaseStatus::Received,
            "receipt ons-330a / vintage 18",
        ),
        release(
            7_026,
            "23:50",
            RegionKey::Japan,
            "Industrial production",
            "—",
            "+1.2%",
            "—",
            "late 7m",
            "e-Stat",
            "late",
            ReleaseStatus::Late,
            "calendar jp-14 / no receipt",
        ),
    ]
    .into()
}

#[allow(clippy::too_many_arguments)]
fn release(
    release_id: u64,
    time_label: &'static str,
    region: RegionKey,
    name: &'static str,
    actual: &'static str,
    previous: &'static str,
    revised: &'static str,
    change: &'static str,
    source: &'static str,
    freshness: &'static str,
    status: ReleaseStatus,
    provenance: &'static str,
) -> MacroRelease {
    MacroRelease {
        release_id,
        time_label,
        region,
        name,
        actual,
        previous,
        revised,
        change,
        source,
        freshness,
        status,
        provenance,
    }
}

fn positioning() -> Arc<[PositioningSnapshot]> {
    vec![
        position(
            IndexKey::Us100,
            "NQ TFF",
            82_400,
            41_200,
            -18_700,
            4_400,
            82,
        ),
        position(
            IndexKey::Us500,
            "ES TFF",
            104_800,
            28_900,
            -44_100,
            2_100,
            67,
        ),
        position(IndexKey::Us30, "YM TFF", 18_200, -2_600, -9_100, -800, 51),
        position(IndexKey::De40, "mapping review", 0, 0, 0, 0, 0),
        position(IndexKey::Uk100, "FTSE TFF", 22_800, 7_600, -11_400, 600, 56),
        position(
            IndexKey::Jp225,
            "Nikkei TFF",
            36_400,
            21_100,
            -17_900,
            3_200,
            74,
        ),
    ]
    .into()
}

fn position(
    index: IndexKey,
    contract: &'static str,
    asset_manager_net: i32,
    leveraged_money_net: i32,
    dealer_net: i32,
    weekly_delta: i32,
    percentile: u8,
) -> PositioningSnapshot {
    PositioningSnapshot {
        index,
        contract,
        asset_manager_net,
        leveraged_money_net,
        dealer_net,
        weekly_delta,
        percentile,
        open_interest: 428_000,
        report_age_days: 3,
    }
}

fn documents() -> Arc<[SourceDocument]> {
    vec![
        document(
            22_104,
            "Federal Reserve",
            "Policy release",
            "Minutes of the Federal Open Market Committee",
            "Yesterday / 14:00 ET",
            "US100 / US500 / US30",
            "fed-18f3",
            true,
        ),
        document(
            22_103,
            "ECB",
            "Speech",
            "Monetary policy transmission and the euro area outlook",
            "Today / 06:10 ET",
            "DE40",
            "ecb-a771",
            true,
        ),
        document(
            22_102,
            "Bank of Japan",
            "Statistics notice",
            "Time-series publication schedule updated",
            "Today / 01:20 ET",
            "JP225",
            "boj-59c0",
            true,
        ),
        document(
            22_101,
            "GDELT",
            "Broad antenna",
            "Global policy and credit headlines / 14 matched",
            "Current / 5m window",
            "context only",
            "gdelt-window-551",
            false,
        ),
    ]
    .into()
}

#[allow(clippy::too_many_arguments)]
fn document(
    document_id: u64,
    institution: &'static str,
    kind: &'static str,
    title: &'static str,
    published: &'static str,
    scope: &'static str,
    receipt: &'static str,
    official: bool,
) -> SourceDocument {
    SourceDocument {
        document_id,
        institution,
        kind,
        title,
        published,
        scope,
        receipt,
        official,
    }
}

use std::path::Path;

use northstar_mt5_corpus::{MappedTsv, Row, parse_u64};

use crate::{OracleError, Result};

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub sequence: u64,
    pub market_time: u64,
    pub calculation_bar_time: u64,
    pub closed_bar_time: u64,
    pub bid: f64,
    pub ask: f64,
    pub reference_price: f64,
    pub closed_bar_price: f64,
    pub atr: f64,
    pub structure_snapshot_hash: u64,
    pub structure_generation: u64,
    pub auction_event_sequence_before: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Level {
    pub source_key: u64,
    pub producer: i32,
    pub producer_instance: i32,
    pub local_id: u64,
    pub family: i32,
    pub source_kind: i32,
    pub role: i32,
    pub lower: f64,
    pub price: f64,
    pub upper: f64,
    pub normalized_lower: f64,
    pub normalized_price: f64,
    pub normalized_upper: f64,
    pub width_atr: f64,
    pub created_at: u64,
    pub updated_at: u64,
    pub developing: bool,
    pub frozen: bool,
    pub state: i32,
    pub evidence_weight: f64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceRef {
    pub source_key: u64,
    pub producer: i32,
    pub producer_instance: i32,
    pub local_id: u64,
    pub family: i32,
    pub source_kind: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ExpectedNode {
    pub node_id: u64,
    pub evidence_id: u64,
    pub cluster_id: i32,
    pub lower: f64,
    pub price: f64,
    pub upper: f64,
    pub normalized_lower: f64,
    pub normalized_price: f64,
    pub normalized_upper: f64,
    pub width_atr: f64,
    pub distance_atr: f64,
    pub region: i32,
    pub median_distance_atr: f64,
    pub median_distance_sigma: f64,
    pub cog_distance_sigma: f64,
    pub width_sigma: f64,
    pub contains_cog: bool,
    pub role: i32,
    pub family_mask: u64,
    pub role_mask: u64,
    pub family_count: i32,
    pub member_count: i32,
    pub developing_count: i32,
    pub frozen_count: i32,
    pub broken_count: i32,
    pub corridor_up_levels: i32,
    pub corridor_down_levels: i32,
    pub corridor_up_noise: i32,
    pub corridor_down_noise: i32,
    pub oldest_source_time: u64,
    pub newest_update_time: u64,
    pub provenance_offset: i32,
    pub provenance_count: i32,
    pub existence: i32,
    pub created_at: u64,
    pub last_seen_at: u64,
    pub state_changed_at: u64,
    pub missed_rebuilds: i32,
    pub revision: i32,
    pub primary_parent_id: u64,
    pub secondary_parent_id: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ExpectedFeature {
    pub frozen_bar_time: u64,
    pub has_cog: bool,
    pub has_c3: bool,
    pub has_lattice: bool,
    pub has_field: bool,
    pub has_profile: bool,
    pub cog_price: f64,
    pub c3_price: f64,
    pub cog_distance_atr: f64,
    pub c3_distance_atr: f64,
    pub cog_velocity_atr: f64,
    pub c3_velocity_atr: f64,
    pub lattice_width_atr: f64,
    pub field_width_atr: f64,
    pub profile_poc: f64,
    pub profile_vah: f64,
    pub profile_val: f64,
    pub poc_distance_atr: f64,
    pub vah_distance_atr: f64,
    pub val_distance_atr: f64,
    pub spread_atr: f64,
    pub raw_level_count: i32,
    pub noise_level_count: i32,
    pub active_node_count: i32,
    pub regional_valid: bool,
    pub sigma_valid: bool,
    pub regional_has_cog: bool,
    pub basis_changed: bool,
    pub regional_frozen_bar_time: u64,
    pub structure_snapshot_hash: u64,
    pub regional_basis_hash: u64,
    pub structure_generation: u64,
    pub population_count: i32,
    pub population_count_delta: i32,
    pub velocity_elapsed_bars: i32,
    pub median_price: f64,
    pub mean_price: f64,
    pub structural_sigma: f64,
    pub regional_reference_price: f64,
    pub regional_cog_price: f64,
    pub price_from_median_atr: f64,
    pub price_from_median_sigma: f64,
    pub price_from_cog_atr: f64,
    pub price_from_cog_sigma: f64,
    pub cog_median_gap_atr: f64,
    pub cog_median_gap_sigma: f64,
    pub cog_velocity_price: f64,
    pub regional_cog_velocity_atr: f64,
    pub cog_velocity_sigma: f64,
    pub median_velocity_price: f64,
    pub median_velocity_atr: f64,
    pub median_velocity_sigma: f64,
    pub sigma_log_change: f64,
    pub price_region: i32,
    pub cog_region: i32,
}

#[derive(Debug, Default)]
pub struct FrameBundle {
    pub frame: Frame,
    pub levels: Vec<Level>,
    pub nodes: Vec<ExpectedNode>,
    pub sources: Vec<SourceRef>,
    pub feature: ExpectedFeature,
}

pub fn load_oracle(root: &Path) -> Result<Vec<FrameBundle>> {
    let frame_table = MappedTsv::open(root.join("frames.tsv"))?;
    let mut bundles = Vec::new();
    for row in frame_table.rows() {
        let row = row?;
        let frame = Frame {
            sequence: u64_field(row, &frame_table, "sequence")?,
            market_time: u64_field(row, &frame_table, "market_time")?,
            calculation_bar_time: u64_field(row, &frame_table, "calculation_bar_time")?,
            closed_bar_time: u64_field(row, &frame_table, "closed_bar_time")?,
            bid: f64_field(row, &frame_table, "bid")?,
            ask: f64_field(row, &frame_table, "ask")?,
            reference_price: f64_field(row, &frame_table, "reference_price")?,
            closed_bar_price: f64_field(row, &frame_table, "closed_bar_price")?,
            atr: f64_field(row, &frame_table, "atr")?,
            structure_snapshot_hash: u64_field(row, &frame_table, "structure_snapshot_hash")?,
            structure_generation: u64_field(row, &frame_table, "structure_generation")?,
            auction_event_sequence_before: u64_field(row, &frame_table, "auction_event_sequence")?,
        };
        if frame.sequence != bundles.len() as u64 + 1 {
            return Err(contract("frame sequence is not contiguous"));
        }
        bundles.push(FrameBundle {
            frame,
            ..FrameBundle::default()
        });
    }
    if bundles.is_empty() {
        return Err(contract("B-D oracle has no frames"));
    }
    load_levels(root, &mut bundles)?;
    load_nodes(root, &mut bundles)?;
    load_sources(root, &mut bundles)?;
    load_features(root, &mut bundles)?;
    Ok(bundles)
}

fn load_levels(root: &Path, bundles: &mut [FrameBundle]) -> Result<()> {
    let table = MappedTsv::open(root.join("levels.tsv"))?;
    for row in table.rows() {
        let row = row?;
        let target = bundle_mut(bundles, u64_field(row, &table, "sequence")?)?;
        target.levels.push(Level {
            source_key: u64_field(row, &table, "source_key")?,
            producer: i32_field(row, &table, "producer")?,
            producer_instance: i32_field(row, &table, "producer_instance")?,
            local_id: u64_field(row, &table, "local_id")?,
            family: i32_field(row, &table, "family")?,
            source_kind: i32_field(row, &table, "source_kind")?,
            role: i32_field(row, &table, "role")?,
            lower: f64_field(row, &table, "lower")?,
            price: f64_field(row, &table, "price")?,
            upper: f64_field(row, &table, "upper")?,
            normalized_lower: f64_field(row, &table, "normalized_lower")?,
            normalized_price: f64_field(row, &table, "normalized_price")?,
            normalized_upper: f64_field(row, &table, "normalized_upper")?,
            width_atr: f64_field(row, &table, "width_atr")?,
            created_at: u64_field(row, &table, "created_at")?,
            updated_at: u64_field(row, &table, "updated_at")?,
            developing: bool_field(row, &table, "developing")?,
            frozen: bool_field(row, &table, "frozen")?,
            state: i32_field(row, &table, "state")?,
            evidence_weight: f64_field(row, &table, "evidence_weight")?,
        });
    }
    Ok(())
}

fn load_nodes(root: &Path, bundles: &mut [FrameBundle]) -> Result<()> {
    let table = MappedTsv::open(root.join("nodes.tsv"))?;
    for row in table.rows() {
        let row = row?;
        let target = bundle_mut(bundles, u64_field(row, &table, "sequence")?)?;
        target.nodes.push(ExpectedNode {
            node_id: u64_field(row, &table, "node_id")?,
            evidence_id: u64_field(row, &table, "evidence_id")?,
            cluster_id: i32_field(row, &table, "cluster_id")?,
            lower: f64_field(row, &table, "lower")?,
            price: f64_field(row, &table, "price")?,
            upper: f64_field(row, &table, "upper")?,
            normalized_lower: f64_field(row, &table, "normalized_lower")?,
            normalized_price: f64_field(row, &table, "normalized_price")?,
            normalized_upper: f64_field(row, &table, "normalized_upper")?,
            width_atr: f64_field(row, &table, "width_atr")?,
            distance_atr: f64_field(row, &table, "distance_atr")?,
            region: i32_field(row, &table, "region")?,
            median_distance_atr: f64_field(row, &table, "median_distance_atr")?,
            median_distance_sigma: f64_field(row, &table, "median_distance_sigma")?,
            cog_distance_sigma: f64_field(row, &table, "cog_distance_sigma")?,
            width_sigma: f64_field(row, &table, "width_sigma")?,
            contains_cog: bool_field(row, &table, "contains_cog")?,
            role: i32_field(row, &table, "role")?,
            family_mask: u64_field(row, &table, "family_mask")?,
            role_mask: u64_field(row, &table, "role_mask")?,
            family_count: i32_field(row, &table, "family_count")?,
            member_count: i32_field(row, &table, "member_count")?,
            developing_count: i32_field(row, &table, "developing_count")?,
            frozen_count: i32_field(row, &table, "frozen_count")?,
            broken_count: i32_field(row, &table, "broken_count")?,
            corridor_up_levels: i32_field(row, &table, "corridor_up_levels")?,
            corridor_down_levels: i32_field(row, &table, "corridor_down_levels")?,
            corridor_up_noise: i32_field(row, &table, "corridor_up_noise")?,
            corridor_down_noise: i32_field(row, &table, "corridor_down_noise")?,
            oldest_source_time: u64_field(row, &table, "oldest_source_time")?,
            newest_update_time: u64_field(row, &table, "newest_update_time")?,
            provenance_offset: i32_field(row, &table, "provenance_offset")?,
            provenance_count: i32_field(row, &table, "provenance_count")?,
            existence: i32_field(row, &table, "existence")?,
            created_at: u64_field(row, &table, "created_at")?,
            last_seen_at: u64_field(row, &table, "last_seen_at")?,
            state_changed_at: u64_field(row, &table, "state_changed_at")?,
            missed_rebuilds: i32_field(row, &table, "missed_rebuilds")?,
            revision: i32_field(row, &table, "revision")?,
            primary_parent_id: u64_field(row, &table, "primary_parent_id")?,
            secondary_parent_id: u64_field(row, &table, "secondary_parent_id")?,
        });
    }
    Ok(())
}

fn load_sources(root: &Path, bundles: &mut [FrameBundle]) -> Result<()> {
    let table = MappedTsv::open(root.join("sources.tsv"))?;
    for row in table.rows() {
        let row = row?;
        let target = bundle_mut(bundles, u64_field(row, &table, "sequence")?)?;
        target.sources.push(SourceRef {
            source_key: u64_field(row, &table, "source_key")?,
            producer: i32_field(row, &table, "producer")?,
            producer_instance: i32_field(row, &table, "producer_instance")?,
            local_id: u64_field(row, &table, "local_id")?,
            family: i32_field(row, &table, "family")?,
            source_kind: i32_field(row, &table, "source_kind")?,
        });
    }
    Ok(())
}

fn load_features(root: &Path, bundles: &mut [FrameBundle]) -> Result<()> {
    let table = MappedTsv::open(root.join("features.tsv"))?;
    let mut seen = vec![false; bundles.len()];
    for row in table.rows() {
        let row = row?;
        let sequence = u64_field(row, &table, "sequence")?;
        let index = sequence
            .checked_sub(1)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|&value| value < bundles.len())
            .ok_or_else(|| contract("feature references an unknown frame"))?;
        if std::mem::replace(&mut seen[index], true) {
            return Err(contract("duplicate feature frame"));
        }
        bundles[index].feature = ExpectedFeature {
            frozen_bar_time: u64_field(row, &table, "frozen_bar_time")?,
            has_cog: bool_field(row, &table, "has_cog")?,
            has_c3: bool_field(row, &table, "has_c3")?,
            has_lattice: bool_field(row, &table, "has_lattice")?,
            has_field: bool_field(row, &table, "has_field")?,
            has_profile: bool_field(row, &table, "has_profile")?,
            cog_price: f64_field(row, &table, "cog_price")?,
            c3_price: f64_field(row, &table, "c3_price")?,
            cog_distance_atr: f64_field(row, &table, "cog_distance_atr")?,
            c3_distance_atr: f64_field(row, &table, "c3_distance_atr")?,
            cog_velocity_atr: f64_field(row, &table, "cog_velocity_atr")?,
            c3_velocity_atr: f64_field(row, &table, "c3_velocity_atr")?,
            lattice_width_atr: f64_field(row, &table, "lattice_width_atr")?,
            field_width_atr: f64_field(row, &table, "field_width_atr")?,
            profile_poc: f64_field(row, &table, "profile_poc")?,
            profile_vah: f64_field(row, &table, "profile_vah")?,
            profile_val: f64_field(row, &table, "profile_val")?,
            poc_distance_atr: f64_field(row, &table, "poc_distance_atr")?,
            vah_distance_atr: f64_field(row, &table, "vah_distance_atr")?,
            val_distance_atr: f64_field(row, &table, "val_distance_atr")?,
            spread_atr: f64_field(row, &table, "spread_atr")?,
            raw_level_count: i32_field(row, &table, "raw_level_count")?,
            noise_level_count: i32_field(row, &table, "noise_level_count")?,
            active_node_count: i32_field(row, &table, "active_node_count")?,
            regional_valid: bool_field(row, &table, "regional_valid")?,
            sigma_valid: bool_field(row, &table, "sigma_valid")?,
            regional_has_cog: bool_field(row, &table, "regional_has_cog")?,
            basis_changed: bool_field(row, &table, "basis_changed")?,
            regional_frozen_bar_time: u64_field(row, &table, "regional_frozen_bar_time")?,
            structure_snapshot_hash: u64_field(row, &table, "structure_snapshot_hash")?,
            regional_basis_hash: u64_field(row, &table, "regional_basis_hash")?,
            structure_generation: u64_field(row, &table, "structure_generation")?,
            population_count: i32_field(row, &table, "population_count")?,
            population_count_delta: i32_field(row, &table, "population_count_delta")?,
            velocity_elapsed_bars: i32_field(row, &table, "velocity_elapsed_bars")?,
            median_price: f64_field(row, &table, "median_price")?,
            mean_price: f64_field(row, &table, "mean_price")?,
            structural_sigma: f64_field(row, &table, "structural_sigma")?,
            regional_reference_price: f64_field(row, &table, "regional_reference_price")?,
            regional_cog_price: f64_field(row, &table, "regional_cog_price")?,
            price_from_median_atr: f64_field(row, &table, "price_from_median_atr")?,
            price_from_median_sigma: f64_field(row, &table, "price_from_median_sigma")?,
            price_from_cog_atr: f64_field(row, &table, "price_from_cog_atr")?,
            price_from_cog_sigma: f64_field(row, &table, "price_from_cog_sigma")?,
            cog_median_gap_atr: f64_field(row, &table, "cog_median_gap_atr")?,
            cog_median_gap_sigma: f64_field(row, &table, "cog_median_gap_sigma")?,
            cog_velocity_price: f64_field(row, &table, "cog_velocity_price")?,
            regional_cog_velocity_atr: f64_field(row, &table, "regional_cog_velocity_atr")?,
            cog_velocity_sigma: f64_field(row, &table, "cog_velocity_sigma")?,
            median_velocity_price: f64_field(row, &table, "median_velocity_price")?,
            median_velocity_atr: f64_field(row, &table, "median_velocity_atr")?,
            median_velocity_sigma: f64_field(row, &table, "median_velocity_sigma")?,
            sigma_log_change: f64_field(row, &table, "sigma_log_change")?,
            price_region: i32_field(row, &table, "price_region")?,
            cog_region: i32_field(row, &table, "cog_region")?,
        };
    }
    if seen.iter().any(|value| !value) {
        return Err(contract("missing frame feature"));
    }
    Ok(())
}

fn bundle_mut(bundles: &mut [FrameBundle], sequence: u64) -> Result<&mut FrameBundle> {
    sequence
        .checked_sub(1)
        .and_then(|value| usize::try_from(value).ok())
        .and_then(|index| bundles.get_mut(index))
        .ok_or_else(|| contract("child row references an unknown frame"))
}

fn bytes<'a>(row: Row<'a>, table: &MappedTsv, name: &str) -> Result<&'a [u8]> {
    let index = table.column(name)?;
    row.field(index)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| contract(format!("missing {name} in {}", table.path().display())))
}

fn u64_field(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<u64> {
    parse_u64(bytes(row, table, name)?)
        .ok_or_else(|| contract(format!("invalid u64 {name} in {}", table.path().display())))
}

fn i32_field(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<i32> {
    text(row, table, name)?
        .parse()
        .map_err(|_| contract(format!("invalid i32 {name} in {}", table.path().display())))
}

fn f64_field(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<f64> {
    text(row, table, name)?
        .parse()
        .map_err(|_| contract(format!("invalid f64 {name} in {}", table.path().display())))
}

fn bool_field(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<bool> {
    match bytes(row, table, name)? {
        b"0" => Ok(false),
        b"1" => Ok(true),
        _ => Err(contract(format!(
            "invalid bool {name} in {}",
            table.path().display()
        ))),
    }
}

fn text<'a>(row: Row<'a>, table: &MappedTsv, name: &str) -> Result<&'a str> {
    std::str::from_utf8(bytes(row, table, name)?)
        .map_err(|_| contract(format!("non-UTF8 {name} in {}", table.path().display())))
}

fn contract(detail: impl Into<String>) -> OracleError {
    OracleError::Contract(detail.into())
}

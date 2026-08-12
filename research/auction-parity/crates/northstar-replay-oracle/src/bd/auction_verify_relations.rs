use std::path::Path;

use northstar_mt5_corpus::MappedTsv;

use crate::Result;

use super::{
    auction::AuctionLedger,
    auction_types::{Attempt, ContextRow, Episode, Transit},
    auction_verify::{close_at, fail_at, float, int, suffixed, uint},
    input::ExpectedFeature,
};

pub fn verify(prefix: &Path, ledger: &AuctionLedger) -> Result<()> {
    verify_attempts(&suffixed(prefix, "_attempts.tsv"), &ledger.attempts)?;
    verify_episodes(&suffixed(prefix, "_episodes.tsv"), &ledger.episodes)?;
    verify_context(&suffixed(prefix, "_context.tsv"), &ledger.context)?;
    verify_features(&suffixed(prefix, "_features.tsv"), &ledger.features)?;
    verify_transits(&suffixed(prefix, "_transits.tsv"), &ledger.transits)
}

macro_rules! fields {
    ($row:expr, $table:expr, $n:expr, $value:expr; u: $(($uf:ident,$uc:literal)),*; i: $(($if:ident,$ic:literal)),*; f: $(($ff:ident,$fc:literal)),* $(;)?) => {{
        $(same($n, $uc, $value.$uf, uint($row, $table, $uc)?)?;)*
        $(same($n, $ic, $value.$if, int($row, $table, $ic)?)?;)*
        $(close_at($n, $fc, $value.$ff, float($row, $table, $fc)?)?;)*
    }};
}

fn verify_attempts(path: &Path, actual: &[Attempt]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let mut rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    count("attempt", actual.len(), rows.len())?;
    rows.sort_by_key(|row| uint(*row, &table, "attempt_id").unwrap_or(0));
    let mut ordered: Vec<_> = actual.iter().collect();
    ordered.sort_by_key(|attempt| attempt.attempt_id);
    for (index, (a, row)) in ordered.into_iter().zip(rows).enumerate() {
        let n = index + 1;
        fields!(row, &table, n, a;
            u: (attempt_id,"attempt_id"),(episode_id,"episode_id"),(node_id,"node_id"),(evidence_id,"evidence_id"),
               (started_at,"start"),(contact_at,"contact"),(break_at,"break"),(accepted_at,"accepted"),(resolved_at,"end"),
               (nearest_above_id,"nearest_above_id"),(nearest_below_id,"nearest_below_id"),(family_mask,"family_mask");
            i: (attempt_ordinal,"ordinal"),(direction,"direction"),(resolution,"resolution_code"),
               (start_bar_sequence,"start_bar"),(contact_bar_sequence,"contact_bar"),(resolved_bar_sequence,"end_bar"),
               (inside_updates,"inside_updates"),(qualified_far_closes,"far_closes"),(family_count,"family_count"),
               (member_count,"member_count"),(developing_count,"developing_count"),(frozen_count,"frozen_count"),
               (node_revision,"node_revision"),(completion_status,"completion_status_code"),(censor_reason,"censor_reason_code"),
               (corridor_up_level_count,"corridor_up_level_count"),(corridor_down_level_count,"corridor_down_level_count"),
               (corridor_up_noise_count,"corridor_up_noise_count"),(corridor_down_noise_count,"corridor_down_noise_count"),
               (start_region,"start_region_code"),(end_region,"end_region_code"),(node_region,"node_region_code");
            f: (start_price,"approach_start_price"),(contact_price,"contact_price"),(frozen_atr,"frozen_atr"),
               (frozen_lower,"frozen_lower"),(frozen_price,"frozen_price"),(frozen_upper,"frozen_upper"),
               (node_width_atr,"node_width_atr"),(initial_distance_atr,"initial_distance_atr"),
               (approach_efficiency,"approach_efficiency"),(max_penetration_atr,"penetration_atr"),
               (max_penetration_node,"penetration_node"),(max_above_node_atr,"max_above_atr"),
               (max_below_node_atr,"max_below_atr"),(rejection_excursion_atr,"rejection_excursion_atr"),
               (start_median_sigma,"start_median_sigma"),(end_median_sigma,"end_median_sigma"),
               (min_median_sigma,"min_median_sigma"),(max_median_sigma,"max_median_sigma"),
               (node_from_median_sigma,"node_from_median_sigma"),(node_from_cog_sigma,"node_from_cog_sigma"),
               (node_width_sigma,"node_width_sigma");
        );
        same(
            n,
            "is_retest",
            i32::from(a.is_retest),
            int(row, &table, "is_retest")?,
        )?;
        same(
            n,
            "inside_seconds",
            a.inside_seconds as u64,
            uint(row, &table, "inside_seconds")?,
        )?;
        close_at(
            n,
            "corridor_up_atr",
            a.corridor_up_atr,
            nullable_float(row, &table, "corridor_up_atr", -1.0)?,
        )?;
        close_at(
            n,
            "corridor_down_atr",
            a.corridor_down_atr,
            nullable_float(row, &table, "corridor_down_atr", -1.0)?,
        )?;
    }
    Ok(())
}

fn verify_context(path: &Path, actual: &[ContextRow]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let mut rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    count("context", actual.len(), rows.len())?;
    rows.sort_by_key(|row| {
        (
            uint(*row, &table, "attempt_id").unwrap_or(0),
            uint(*row, &table, "source_key").unwrap_or(0),
        )
    });
    let mut ordered: Vec<_> = actual.iter().collect();
    ordered.sort_by_key(|row| (row.attempt_id, row.source.source_key));
    for (index, (a, row)) in ordered.into_iter().zip(rows).enumerate() {
        let n = index + 1;
        for (name, left, right) in [
            ("attempt_id", a.attempt_id, uint(row, &table, "attempt_id")?),
            ("episode_id", a.episode_id, uint(row, &table, "episode_id")?),
            ("node_id", a.node_id, uint(row, &table, "node_id")?),
            ("frozen_at", a.frozen_at, uint(row, &table, "frozen_at")?),
            (
                "source_key",
                a.source.source_key,
                uint(row, &table, "source_key")?,
            ),
            (
                "local_id",
                a.source.local_id,
                uint(row, &table, "local_id")?,
            ),
        ] {
            same(n, name, left, right)?;
        }
        same(
            n,
            "producer_code",
            a.source.producer,
            int(row, &table, "producer_code")?,
        )?;
        same(
            n,
            "producer_instance",
            a.source.producer_instance,
            int(row, &table, "producer_instance")?,
        )?;
        same(
            n,
            "family_code",
            a.source.family,
            int(row, &table, "family_code")?,
        )?;
        same(
            n,
            "source_kind",
            a.source.source_kind,
            int(row, &table, "source_kind")?,
        )?;
    }
    Ok(())
}

fn verify_features(path: &Path, actual: &[(u64, ExpectedFeature)]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let mut rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    count("feature", actual.len(), rows.len())?;
    rows.sort_by_key(|row| uint(*row, &table, "attempt_id").unwrap_or(0));
    let mut ordered: Vec<_> = actual.iter().collect();
    ordered.sort_by_key(|row| row.0);
    for (index, ((attempt_id, a), row)) in ordered.into_iter().zip(rows).enumerate() {
        let n = index + 1;
        same(
            n,
            "attempt_id",
            *attempt_id,
            uint(row, &table, "attempt_id")?,
        )?;
        fields!(row, &table, n, a;
            u: (frozen_bar_time,"frozen_bar_time"),(structure_snapshot_hash,"structure_snapshot_hash"),
               (regional_basis_hash,"regional_basis_hash"),(structure_generation,"structure_generation");
            i: (raw_level_count,"raw_level_count"),(noise_level_count,"noise_level_count"),
               (active_node_count,"active_node_count"),(population_count,"population_count"),
               (population_count_delta,"population_count_delta"),(velocity_elapsed_bars,"velocity_elapsed_bars"),
               (price_region,"price_region_code"),(cog_region,"cog_region_code");
            f: (cog_price,"cog_price"),(cog_distance_atr,"cog_distance_atr"),(cog_velocity_atr,"cog_velocity_atr"),
               (c3_price,"c3_price"),(c3_distance_atr,"c3_distance_atr"),(c3_velocity_atr,"c3_velocity_atr"),
               (lattice_width_atr,"lattice_width_atr"),(field_width_atr,"field_width_atr"),
               (profile_poc,"poc_price"),(profile_vah,"vah_price"),(profile_val,"val_price"),
               (poc_distance_atr,"poc_distance_atr"),(vah_distance_atr,"vah_distance_atr"),
               (val_distance_atr,"val_distance_atr"),(spread_atr,"spread_atr"),
               (median_price,"median_price"),(mean_price,"mean_price"),(structural_sigma,"structural_sigma"),
               (regional_reference_price,"reference_price"),(price_from_median_atr,"price_from_median_atr"),
               (price_from_median_sigma,"price_from_median_sigma"),(price_from_cog_atr,"price_from_cog_atr"),
               (price_from_cog_sigma,"price_from_cog_sigma"),(cog_median_gap_atr,"cog_median_gap_atr"),
               (cog_median_gap_sigma,"cog_median_gap_sigma"),(cog_velocity_price,"regional_cog_velocity_price"),
               (regional_cog_velocity_atr,"regional_cog_velocity_atr"),(cog_velocity_sigma,"regional_cog_velocity_sigma"),
               (median_velocity_price,"regional_median_velocity_price"),(median_velocity_atr,"regional_median_velocity_atr"),
               (median_velocity_sigma,"regional_median_velocity_sigma"),(sigma_log_change,"regional_sigma_log_change_per_bar");
        );
        for (name, actual, expected) in [
            ("has_cog", a.has_cog, int(row, &table, "has_cog")? != 0),
            ("has_c3", a.has_c3, int(row, &table, "has_c3")? != 0),
            (
                "has_lattice",
                a.has_lattice,
                int(row, &table, "has_lattice")? != 0,
            ),
            (
                "has_field",
                a.has_field,
                int(row, &table, "has_field")? != 0,
            ),
            (
                "has_profile",
                a.has_profile,
                int(row, &table, "has_profile")? != 0,
            ),
            (
                "regional_valid",
                a.regional_valid,
                int(row, &table, "regional_valid")? != 0,
            ),
            (
                "sigma_valid",
                a.sigma_valid,
                int(row, &table, "sigma_valid")? != 0,
            ),
            (
                "basis_changed",
                a.basis_changed,
                int(row, &table, "basis_changed")? != 0,
            ),
        ] {
            same(n, name, actual, expected)?;
        }
    }
    Ok(())
}

fn verify_episodes(path: &Path, actual: &[Episode]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let mut rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    count("episode", actual.len(), rows.len())?;
    rows.sort_by_key(|row| uint(*row, &table, "episode_id").unwrap_or(0));
    let mut ordered: Vec<_> = actual.iter().collect();
    ordered.sort_by_key(|episode| episode.episode_id);
    for (index, (a, row)) in ordered.into_iter().zip(rows).enumerate() {
        let n = index + 1;
        fields!(row, &table, n, a;
            u: (episode_id,"episode_id"),(node_id,"node_id"),(started_at,"start"),(ended_at,"end"),
               (next_node_id,"next_node_id"),(family_mask,"family_mask");
            i: (first_direction,"first_direction"),(attempts,"attempts"),(breaks,"breaks"),
               (reclaims,"reclaims"),(retests,"retests"),(family_count,"family_count"),
               (member_count,"member_count"),(resolution,"resolution_code"),
               (completion_status,"completion_status_code"),(censor_reason,"censor_reason_code"),
               (initial_region,"initial_region_code"),(terminal_region,"terminal_region_code");
            f: (node_width_atr,"node_width_atr"),(max_up_excursion_atr,"max_up_excursion_atr"),
               (max_down_excursion_atr,"max_down_excursion_atr");
        );
        close_at(
            n,
            "corridor_up_atr",
            a.corridor_up_atr,
            nullable_float(row, &table, "corridor_up_atr", -1.0)?,
        )?;
        close_at(
            n,
            "corridor_down_atr",
            a.corridor_down_atr,
            nullable_float(row, &table, "corridor_down_atr", -1.0)?,
        )?;
        same(
            n,
            "regional_valid",
            i32::from(a.regional_valid),
            int(row, &table, "regional_valid")?,
        )?;
        same(
            n,
            "sigma_valid",
            i32::from(a.sigma_valid),
            int(row, &table, "sigma_valid")?,
        )?;
        same(
            n,
            "duration_bars",
            (a.last_attempt_end_bar - a.start_bar_sequence).max(0),
            int(row, &table, "duration_bars")?,
        )?;
        same(
            n,
            "duration_seconds",
            a.ended_at.saturating_sub(a.started_at),
            uint(row, &table, "duration_seconds")?,
        )?;
    }
    Ok(())
}

fn verify_transits(path: &Path, actual: &[Transit]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let mut rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    count("transit", actual.len(), rows.len())?;
    rows.sort_by_key(|row| uint(*row, &table, "transit_id").unwrap_or(0));
    let mut ordered: Vec<_> = actual.iter().collect();
    ordered.sort_by_key(|transit| transit.transit_id);
    for (index, (a, row)) in ordered.into_iter().zip(rows).enumerate() {
        let n = index + 1;
        fields!(row, &table, n, a;
            u: (transit_id,"transit_id"),(attempt_id,"attempt_id"),(episode_id,"episode_id"),
               (source_node_id,"source_node_id"),(destination_node_id,"destination_node_id"),
               (started_at,"start"),(ended_at,"end"),(regional_basis_hash,"regional_basis_hash"),
               (structure_snapshot_hash,"structure_snapshot_hash");
            i: (direction,"direction"),(start_bar_sequence,"start_bar"),(end_bar_sequence,"end_bar"),
               (completion_status,"completion_status_code"),(censor_reason,"censor_reason_code"),
               (resolution,"resolution_code"),(source_region,"source_region_code"),
               (destination_region,"destination_region_code"),(start_price_region,"start_price_region_code"),
               (end_price_region,"end_price_region_code");
            f: (start_price,"start_price"),(end_price,"end_price"),(source_lower,"source_lower"),
               (source_upper,"source_upper"),(destination_lower,"destination_lower"),
               (destination_upper,"destination_upper"),(frozen_atr,"frozen_atr"),(distance_atr,"distance_atr"),
               (path_length,"path_length"),(path_efficiency,"path_efficiency"),(max_adverse_atr,"max_adverse_atr"),
               (start_median_price,"start_median_price"),(start_structural_sigma,"start_structural_sigma"),
               (start_price_from_median_sigma,"start_price_from_median_sigma"),
               (end_price_from_median_sigma,"end_price_from_median_sigma");
        );
        same(
            n,
            "regional_valid",
            i32::from(a.regional_valid),
            int(row, &table, "regional_valid")?,
        )?;
        same(
            n,
            "sigma_valid",
            i32::from(a.sigma_valid),
            int(row, &table, "sigma_valid")?,
        )?;
        same(
            n,
            "duration_bars",
            (a.end_bar_sequence - a.start_bar_sequence).max(0),
            int(row, &table, "duration_bars")?,
        )?;
        same(
            n,
            "duration_seconds",
            a.ended_at.saturating_sub(a.started_at),
            uint(row, &table, "duration_seconds")?,
        )?;
    }
    Ok(())
}

fn nullable_float(
    row: northstar_mt5_corpus::Row<'_>,
    table: &MappedTsv,
    name: &str,
    null_value: f64,
) -> Result<f64> {
    let bytes = row
        .field(table.column(name)?)
        .ok_or_else(|| crate::OracleError::Contract(format!("missing {name}")))?;
    if bytes == b"\\N" {
        return Ok(null_value);
    }
    std::str::from_utf8(bytes)
        .map_err(|_| crate::OracleError::Contract(format!("non-UTF8 {name}")))?
        .parse()
        .map_err(|_| crate::OracleError::Contract(format!("invalid f64 {name}")))
}

fn same<T: PartialEq + std::fmt::Display>(
    row: usize,
    field: &str,
    actual: T,
    expected: T,
) -> Result<()> {
    if actual != expected {
        return fail_at(row, field, actual, expected);
    }
    Ok(())
}

fn count(name: &str, actual: usize, expected: usize) -> Result<()> {
    same(0, name, actual, expected)
}

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use northstar_mt5_corpus::{MappedTsv, Row, parse_u64};

use crate::{OracleError, Result};

use super::{auction::AuctionLedger, auction_types::Event};

pub fn verify(prefix: &Path, ledger: &AuctionLedger) -> Result<()> {
    verify_events(&suffixed(prefix, "_events.tsv"), &ledger.events)?;
    super::auction_verify_relations::verify(prefix, ledger)?;
    verify_run(&suffixed(prefix, "_runs.tsv"), ledger)
}

fn verify_events(path: &Path, actual: &[Event]) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let rows: Vec<_> = table.rows().collect::<std::result::Result<_, _>>()?;
    if rows.len() != actual.len() {
        return fail("event count", actual.len(), rows.len());
    }
    for (index, (event, row)) in actual.iter().zip(rows).enumerate() {
        let sequence = index + 1;
        macro_rules! u {
            ($field:ident, $column:literal) => {
                if event.$field != uint(row, &table, $column)? {
                    return fail_at(sequence, $column, event.$field, uint(row, &table, $column)?);
                }
            };
        }
        macro_rules! i {
            ($field:ident, $column:literal) => {
                if event.$field != int(row, &table, $column)? {
                    return fail_at(sequence, $column, event.$field, int(row, &table, $column)?);
                }
            };
        }
        macro_rules! f {
            ($field:ident, $column:literal) => {
                close_at(
                    sequence,
                    $column,
                    event.$field,
                    float(row, &table, $column)?,
                )?;
            };
        }
        u!(event_id, "event_id");
        u!(event_sequence, "event_sequence");
        u!(episode_id, "episode_id");
        u!(attempt_id, "attempt_id");
        u!(node_id, "node_id");
        u!(related_node_id, "related_node_id");
        u!(market_time, "timestamp");
        u!(bar_time, "bar_time");
        i!(kind, "event_type_code");
        i!(direction, "direction");
        f!(price, "price");
        f!(atr, "atr");
        f!(distance_atr, "distance_atr");
        f!(penetration_atr, "penetration_atr");
        i!(bar_sequence, "bar_sequence");
        i!(attempt_event_sequence, "attempt_event_sequence");
        if event.regional.structure_snapshot_hash != uint(row, &table, "structure_snapshot_hash")? {
            return fail_at(
                sequence,
                "structure_snapshot_hash",
                event.regional.structure_snapshot_hash,
                uint(row, &table, "structure_snapshot_hash")?,
            );
        }
        if event.regional.regional_basis_hash != uint(row, &table, "regional_basis_hash")? {
            return fail_at(
                sequence,
                "regional_basis_hash",
                event.regional.regional_basis_hash,
                uint(row, &table, "regional_basis_hash")?,
            );
        }
        if event.regional.price_region != int(row, &table, "price_region_code")? {
            return fail_at(
                sequence,
                "price_region_code",
                event.regional.price_region,
                int(row, &table, "price_region_code")?,
            );
        }
        close_at(
            sequence,
            "median_price",
            event.regional.median_price,
            float(row, &table, "median_price")?,
        )?;
        close_at(
            sequence,
            "structural_sigma",
            event.regional.structural_sigma,
            float(row, &table, "structural_sigma")?,
        )?;
        close_at(
            sequence,
            "price_from_median_atr",
            event.regional.price_from_median_atr,
            float(row, &table, "price_from_median_atr")?,
        )?;
        close_at(
            sequence,
            "price_from_median_sigma",
            event.regional.price_from_median_sigma,
            float(row, &table, "price_from_median_sigma")?,
        )?;
        close_at(
            sequence,
            "price_from_cog_atr",
            event.regional.price_from_cog_atr,
            float(row, &table, "price_from_cog_atr")?,
        )?;
        close_at(
            sequence,
            "price_from_cog_sigma",
            event.regional.price_from_cog_sigma,
            float(row, &table, "price_from_cog_sigma")?,
        )?;
        close_at(
            sequence,
            "cog_median_gap_sigma",
            event.regional.cog_median_gap_sigma,
            float(row, &table, "cog_median_gap_sigma")?,
        )?;
    }
    Ok(())
}

fn verify_run(path: &Path, ledger: &AuctionLedger) -> Result<()> {
    let table = MappedTsv::open(path)?;
    let row = table
        .rows()
        .last()
        .transpose()?
        .ok_or_else(|| OracleError::Contract("run ledger is empty".into()))?;
    let fields = [
        ("terminal_hash", ledger.terminal_hash),
        ("event_sequence", ledger.event_sequence),
        ("attempts_started", ledger.attempts_started),
        ("attempts_resolved", ledger.attempts_resolved),
        ("attempts_censored", ledger.attempts_censored),
        ("episodes_started", ledger.episodes_started),
        ("episodes_resolved", ledger.episodes_resolved),
        ("episodes_censored", ledger.episodes_censored),
        ("transits_started", ledger.transits_started),
        ("transits_resolved", ledger.transits_resolved),
        ("transits_censored", ledger.transits_censored),
        ("events_emitted", ledger.event_sequence),
    ];
    for (name, actual) in fields {
        let expected = uint(row, &table, name)?;
        if actual != expected {
            return fail(name, actual, expected);
        }
    }
    Ok(())
}

pub(super) fn suffixed(prefix: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = prefix.as_os_str().to_owned();
    value.push(suffix);
    value.into()
}
pub(super) fn uint(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<u64> {
    let bytes = row
        .field(table.column(name)?)
        .ok_or_else(|| OracleError::Contract(format!("missing {name}")))?;
    if bytes == b"\\N" {
        return Ok(0);
    }
    parse_u64(bytes).ok_or_else(|| OracleError::Contract(format!("invalid u64 {name}")))
}
pub(super) fn int(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<i32> {
    text(row, table, name)?
        .parse()
        .map_err(|_| OracleError::Contract(format!("invalid i32 {name}")))
}
pub(super) fn float(row: Row<'_>, table: &MappedTsv, name: &str) -> Result<f64> {
    text(row, table, name)?
        .parse()
        .map_err(|_| OracleError::Contract(format!("invalid f64 {name}")))
}
fn text<'a>(row: Row<'a>, table: &MappedTsv, name: &str) -> Result<&'a str> {
    let bytes = row
        .field(table.column(name)?)
        .ok_or_else(|| OracleError::Contract(format!("missing {name}")))?;
    if bytes == b"\\N" {
        return Ok("0");
    }
    std::str::from_utf8(bytes).map_err(|_| OracleError::Contract(format!("non-UTF8 {name}")))
}
pub(super) fn close_at(sequence: usize, field: &str, actual: f64, expected: f64) -> Result<()> {
    let price_field = field.contains("price")
        || field.contains("lower")
        || field.contains("upper")
        || field.contains("structural_sigma")
        || matches!(field, "atr" | "frozen_atr" | "path_length" | "mean_price");
    let tolerance = if price_field { 0.005_000_001 } else { 5.1e-8 };
    if (actual - expected).abs() > tolerance {
        return fail_at(sequence, field, actual, expected);
    }
    Ok(())
}
pub(super) fn fail_at<T, A: std::fmt::Display, E: std::fmt::Display>(
    sequence: usize,
    field: &str,
    actual: A,
    expected: E,
) -> Result<T> {
    Err(OracleError::Contract(format!(
        "B ledger divergence row={sequence} field={field} actual={actual} expected={expected}"
    )))
}
fn fail<T, A: std::fmt::Display, E: std::fmt::Display>(
    field: &str,
    actual: A,
    expected: E,
) -> Result<T> {
    Err(OracleError::Contract(format!(
        "B ledger divergence field={field} actual={actual} expected={expected}"
    )))
}

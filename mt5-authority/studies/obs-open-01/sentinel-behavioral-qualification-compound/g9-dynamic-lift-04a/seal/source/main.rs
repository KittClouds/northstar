mod authority;
mod compare;
mod model;
mod output;
mod replay;

use crate::model::AccessAudit;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let authority_repo = PathBuf::from(args.next().ok_or("AUTHORITY_REPO_ARGUMENT_MISSING")?);
    let output = PathBuf::from(args.next().ok_or("OUTPUT_ARGUMENT_MISSING")?);
    if args.next().is_some() {
        return Err("UNEXPECTED_ARGUMENT".into());
    }
    let crate_root = authority::crate_root_from_manifest();
    let opened = authority::bind_and_open(&crate_root, &authority_repo)?;
    let access = AccessAudit {
        raw_source_sha256: opened.authority.raw_source_hash.clone(),
        firewall_manifest_sha256: opened.authority.firewall_manifest_hash.clone(),
        source_prefix_rows_scanned: opened.authority.access.source_prefix_rows_scanned as u64,
        source_prefix_sha256: opened.authority.access.source_prefix_sha256.clone(),
        real_04a_history_reads: opened.authority.access.d_a_retained_causal_bars as u64,
        d_a_sessions_decoded: opened.authority.access.d_a_sessions_decoded as u64,
        d_a_bars_replayed: opened.authority.access.d_a_retained_causal_bars as u64,
        d_a_path_gap_sessions: opened.authority.access.d_a_path_gap_sessions as u64,
        d_a_path_gap_records: opened.authority.access.d_a_path_gaps.len() as u64,
        next_source_row_requested: opened.authority.access.next_source_row_requested,
        inst_seal_members_verified: opened.authority.inst_members as u64,
        measurement_seal_members_verified: opened.authority.meas_members as u64,
        b2_seal_members_verified: opened.authority.b2_members as u64,
        b2_state: opened.authority.b2_state.clone(),
        d_b_session_ids_decoded_for_outcomes: opened
            .authority
            .access
            .d_b_session_ids_decoded_for_outcomes
            as u64,
        d_b_reads: 0,
        d_c_reads: 0,
        d_d_reads: 0,
        target_reads: 0,
        outcome_reads: 0,
        external_optic_reads_before_primary_seal: 0,
    };
    let replay = replay::replay_sessions(&opened.authority.sessions)?;
    if replay.len() != opened.authority.access.d_a_retained_causal_bars {
        return Err("G9_REPLAY_CARDINALITY_DRIFT".into());
    }
    let products = compare::attack(&replay)?;
    let seal = output::write_seal(
        &output,
        &crate_root,
        &products,
        &opened.roots,
        access,
        replay.len(),
        &opened.permit,
    )?;
    println!("G9_PRIMARY_SCIENCE_ROOT={}", seal.scientific_root);
    println!("G9_ROOT={}", seal.complete_root);
    println!("G9_FILES={}", seal.files);
    Ok(())
}

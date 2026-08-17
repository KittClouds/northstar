mod authority;
mod engine;
mod model;
mod output;
mod replay;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let phase = args
        .next()
        .ok_or("PHASE_MISSING")?
        .to_string_lossy()
        .to_string();
    let authority_repo = PathBuf::from(args.next().ok_or("AUTHORITY_REPO_MISSING")?);
    let output_dir = PathBuf::from(args.next().ok_or("OUTPUT_MISSING")?);
    let census_dir = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err("UNEXPECTED_ARGUMENT".into());
    }
    let root = authority::crate_root();
    match phase.as_str() {
        "census" => {
            if census_dir.is_some() {
                return Err("CENSUS_EXTRA_ARGUMENT".into());
            }
            let opened = authority::bind_and_open(&root, &authority_repo, "census", None)?;
            let access = output::access(&opened.authority);
            let records = replay::replay_sessions(&opened.authority.sessions)?;
            if records.len() != opened.authority.access.d_a_retained_causal_bars {
                return Err("REPLAY_CARDINALITY_DRIFT".into());
            }
            let products = engine::census(&records)?;
            let seal = output::write_census(
                &output_dir,
                &root,
                &products,
                &opened.roots,
                &opened.permit,
                access,
            )?;
            println!("G9_1_CENSUS_ROOT={}", seal.root);
            println!("G9_1_CENSUS_FILES={}", seal.files);
            println!("E0={}", products.summary.e0_exact_cardinality);
            println!("E1={}", products.summary.e1_exact_cardinality);
            println!("E2={}", products.summary.e2_exact_cardinality);
        }
        "discover" => {
            let census_dir = census_dir.ok_or("CENSUS_DIR_MISSING")?;
            let census_root =
                output::read_phase_root(&census_dir, "G9_1_CENSUS_ROOT_RECEIPT.json")?;
            let census = output::read_census_summary(&census_dir)?;
            let opened =
                authority::bind_and_open(&root, &authority_repo, "discover", Some(&census_root))?;
            let access = output::access(&opened.authority);
            let records = replay::replay_sessions(&opened.authority.sessions)?;
            let products = engine::discover(&records, &census)?;
            let seal = output::write_discovery(
                &output_dir,
                &root,
                &products,
                &opened.roots,
                &opened.permit,
                access,
                &census_root,
                &census,
            )?;
            println!("G9_1_DISCOVERY_ROOT={}", seal.root);
            println!("G9_1_DISCOVERY_FILES={}", seal.files);
            println!("DELAYED_FRACTURES={}", products.fractures.len());
            println!("MECHANISM_CLASSES={}", products.mechanisms.len());
        }
        _ => return Err("UNKNOWN_PHASE".into()),
    }
    Ok(())
}

use hashbrown::HashSet;
use northstar_market_objects::{
    CandidateAssignment, DATASETS, RawCorpus, discover_trajectory_families,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

fn find_raw_directories(root: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if DATASETS
        .iter()
        .all(|name| root.join(format!("{name}.tsv")).is_file())
    {
        output.push(root.to_path_buf());
        return Ok(());
    }
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            find_raw_directories(&path, output)?;
        }
    }
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BufWriter::with_capacity(64 * 1024, File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn write_assignments(path: &Path, rows: &[CandidateAssignment]) -> std::io::Result<()> {
    let mut writer = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writer.write_all(b"object_kind\trun_key\tcanonical_instrument\tobject_id\tterminal_reason_code\tcensored\teligible\trepresentation\tcandidate_id\n")?;
    for row in rows {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_kind,
            row.run_key,
            row.canonical_instrument,
            row.object_id,
            row.terminal_reason_code,
            u8::from(row.censored),
            u8::from(row.eligible),
            row.representation,
            row.candidate_id.as_deref().unwrap_or("\\N")
        )?;
    }
    writer.flush()
}

fn sha256(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Serialize)]
struct DiscoveryReceipt {
    contract: &'static str,
    status: String,
    research_generation: u32,
    report_sha256: String,
    assignments_sha256: String,
    source_run_count: usize,
    recipe_sha256: String,
    epistemic_status: &'static str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let output = PathBuf::from(
        args.next()
            .ok_or("usage: northstar-market-object-discover OUTPUT INPUT_ROOT [INPUT_ROOT...]")?,
    );
    let roots: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if roots.is_empty() {
        return Err("at least one INPUT_ROOT is required".into());
    }
    let mut raw_directories = Vec::new();
    for root in &roots {
        find_raw_directories(root, &mut raw_directories)?;
    }
    raw_directories.sort();
    raw_directories.dedup();
    if raw_directories.is_empty() {
        return Err("no complete nine-dataset raw directories found".into());
    }
    let mut corpora = Vec::with_capacity(raw_directories.len());
    let mut run_keys = HashSet::with_capacity(raw_directories.len());
    for directory in &raw_directories {
        let corpus = RawCorpus::open(directory, "")?;
        if !run_keys.insert(corpus.report().run_key.clone()) {
            return Err(format!("duplicate semantic run_key: {}", corpus.report().run_key).into());
        }
        corpora.push(corpus);
    }
    fs::create_dir_all(&output)?;
    let report_path = output.join("gate15-discovery-report.json");
    let assignments_path = output.join("candidate-assignments.tsv");
    if report_path.exists() || assignments_path.exists() {
        return Err("derived output already exists; use a new output directory".into());
    }
    let (report, assignments) = discover_trajectory_families(&corpora)?;
    write_json(&report_path, &report)?;
    write_assignments(&assignments_path, &assignments)?;
    let receipt = DiscoveryReceipt {
        contract: "NORTHSTAR_RG3_GATE15_DISCOVERY_RECEIPT_V1",
        status: report.status.clone(),
        research_generation: 3,
        report_sha256: sha256(&report_path)?,
        assignments_sha256: sha256(&assignments_path)?,
        source_run_count: report.source_run_count,
        recipe_sha256: report.recipe_sha256.clone(),
        epistemic_status: "CANDIDATE_ONLY_NOT_MARKET_TRUTH",
    };
    write_json(&output.join("discovery-receipt.json"), &receipt)?;
    let mut roots_writer = BufWriter::new(File::create(output.join("source-raw-directories.txt"))?);
    for directory in raw_directories {
        writeln!(roots_writer, "{}", directory.display())?;
    }
    roots_writer.flush()?;
    println!(
        "RG3_GATE15 status={} runs={} objects={} assignments={}",
        report.status,
        report.source_run_count,
        report.total_objects,
        assignments.len()
    );
    Ok(())
}

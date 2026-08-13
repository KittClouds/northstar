use northstar_market_objects::{
    DATASETS, Gate16Inputs, Gate16Package, Gate16Reference, RawCorpus, build_gate16_laboratory,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

fn find_raw(root: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if DATASETS
        .iter()
        .all(|name| root.join(format!("{name}.tsv")).is_file())
    {
        output.push(root.to_path_buf());
        return Ok(());
    }
    if root.is_dir() {
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() {
                find_raw(&path, output)?;
            }
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    Ok(serde_json::from_slice(bytes)?)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BufWriter::with_capacity(128 * 1024, File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn references(
    path: &Path,
) -> Result<BTreeMap<String, Gate16Reference>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty Gate15.5 coordinate file")?;
    let columns = header
        .split('\t')
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect::<BTreeMap<_, _>>();
    let required = |name: &str| {
        columns
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing coordinate field {name}"))
    };
    let kind_col = required("object_kind")?;
    let run_col = required("run_key")?;
    let id_col = required("object_id")?;
    let comp_col = required("compression_summary_family_id")?;
    let comp_shape_col = required("compression_shape_family_id")?;
    let comp_hybrid_col = required("compression_hybrid_family_id")?;
    let exp_col = required("expansion_summary_family_id")?;
    let exp_shape_col = required("expansion_shape_family_id")?;
    let exp_hybrid_col = required("expansion_hybrid_family_id")?;
    let stratum_col = required("structural_stratum")?;
    let mut output = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let kind = fields[kind_col];
        let family = |local: &str| match local {
            "NOT_APPLICABLE" => None,
            "NULL_FAMILY" => Some("NULL".into()),
            value => Some(value.into()),
        };
        let (summary_family, shape_family, hybrid_family) = if kind == "COMPRESSION" {
            (
                family(fields[comp_col]),
                family(fields[comp_shape_col]),
                family(fields[comp_hybrid_col]),
            )
        } else {
            (
                family(fields[exp_col]),
                family(fields[exp_shape_col]),
                family(fields[exp_hybrid_col]),
            )
        };
        let key = format!("{}::{kind}::{}", fields[run_col], fields[id_col]);
        if output
            .insert(
                key,
                Gate16Reference {
                    summary_family,
                    shape_family,
                    hybrid_family,
                    structural_stratum: fields[stratum_col].into(),
                },
            )
            .is_some()
        {
            return Err("duplicate Gate15.5 coordinate key".into());
        }
    }
    Ok(output)
}

fn write_receipts(path: &Path, package: &Gate16Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "left_object_key\tright_object_key\trepresentation_id\tdistance_contract\tcomparison_mode\tstatus\treason\tdistance\tcomparable_support_points\tcomparable_support_bars\tcomparable_support_seconds\tobserved_fraction_left\tobserved_fraction_right\tleft_censored\tright_censored"
    )?;
    for row in &package.comparison_receipts {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.left_object_key,
            row.right_object_key,
            row.representation_id,
            row.distance_contract,
            row.comparison_mode,
            row.status,
            row.reason,
            row.distance
                .map_or_else(|| r"\N".into(), |value| value.to_string()),
            row.comparable_support_points,
            row.comparable_support_bars,
            row.comparable_support_seconds,
            row.observed_fraction_left,
            row.observed_fraction_right,
            u8::from(row.left_censored),
            u8::from(row.right_censored)
        )?;
    }
    out.flush()
}

fn write_neighbors(path: &Path, package: &Gate16Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "object_key\trepresentation_id\tdistance_contract\tcomparison_mode\trank\tneighbor_key\tdistance\tcomparable_support_points\tcomparable_support_bars\tcomparable_support_seconds\tobject_censored\tneighbor_censored"
    )?;
    for row in &package.neighbors {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_key,
            row.representation_id,
            row.distance_contract,
            row.comparison_mode,
            row.rank,
            row.neighbor_key,
            row.distance,
            row.comparable_support_points,
            row.comparable_support_bars,
            row.comparable_support_seconds,
            u8::from(row.object_censored),
            u8::from(row.neighbor_censored)
        )?;
    }
    out.flush()
}

fn write_index(path: &Path, package: &Gate16Package) -> std::io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(
        out,
        "object_key\trepresentation_id\tavailability\tbyte_offset\tbyte_length\tdimensions"
    )?;
    for row in &package.packed_vector_index {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}",
            row.object_key,
            row.representation_id,
            row.availability,
            row.byte_offset,
            row.byte_length,
            row.dimensions
        )?;
    }
    out.flush()
}

fn write_package(output: &Path, package: &Gate16Package) -> Result<(), Box<dyn std::error::Error>> {
    write_json(&output.join("gate16_exit_report.json"), &package.report)?;
    write_json(
        &output.join("gate16_authority_census.json"),
        &package.authority_census,
    )?;
    write_json(
        &output.join("gate16_representation_manifests.json"),
        &package.manifests,
    )?;
    write_json(&output.join("gate16_robust_scales.json"), &package.scales)?;
    write_receipts(&output.join("gate16_comparison_receipts.tsv"), package)?;
    write_neighbors(&output.join("gate16_neighborhoods.tsv"), package)?;
    write_json(
        &output.join("gate16_metric_diagnostics.json"),
        &package.metric_diagnostics,
    )?;
    write_json(
        &output.join("gate16_distance_probes.json"),
        &package.distance_probes,
    )?;
    write_json(
        &output.join("gate16_partial_distance_vectors.json"),
        &package.partial_distance_vectors,
    )?;
    write_json(
        &output.join("gate16_collision_audit.json"),
        &package.collisions,
    )?;
    write_json(
        &output.join("gate16_local_geometry.json"),
        &package.local_geometry,
    )?;
    write_json(
        &output.join("gate16_neighborhood_agreement.json"),
        &package.neighborhood_agreement,
    )?;
    write_json(
        &output.join("gate16_capability_profiles.json"),
        &package.capability_profiles,
    )?;
    write_json(
        &output.join("gate16_retrospective_diagnostics.json"),
        &package.retrospective,
    )?;
    fs::write(
        output.join("gate16_packed_vectors.f32le"),
        &package.packed_vectors,
    )?;
    write_index(&output.join("gate16_packed_vector_index.tsv"), package)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let usage = "usage: northstar-market-object-geometry OUTPUT GATE15_EXIT GATE15_5_RECEIPT GATE15_5_COORDINATES CONFIRMATION_MANIFEST GATE16_PROTOCOL RG3_SCHEMA INPUT_ROOT [INPUT_ROOT...]";
    let output = PathBuf::from(args.next().ok_or(usage)?);
    let gate15_exit = PathBuf::from(args.next().ok_or(usage)?);
    let atlas_receipt_path = PathBuf::from(args.next().ok_or(usage)?);
    let coordinate_path = PathBuf::from(args.next().ok_or(usage)?);
    let confirmation_path = PathBuf::from(args.next().ok_or(usage)?);
    let protocol_path = PathBuf::from(args.next().ok_or(usage)?);
    let schema_path = PathBuf::from(args.next().ok_or(usage)?);
    let roots = args.map(PathBuf::from).collect::<Vec<_>>();
    if roots.is_empty() || output.exists() {
        return Err("input root missing or output already exists".into());
    }
    let gate15 = read_json(&gate15_exit)?;
    let atlas = read_json(&atlas_receipt_path)?;
    let confirmation = read_json(&confirmation_path)?;
    let protocol = read_json(&protocol_path)?;
    if gate15["status"] != "PASS" || atlas["status"] != "PASS" || protocol["status"] != "FROZEN" {
        return Err("frozen ancestry is not PASS/FROZEN".into());
    }
    if confirmation["status"] != "FROZEN_UNOPENED"
        || confirmation["authorization"] != "NOT_AUTHORIZED_IN_GATE15"
        || atlas["confirmation_windows_opened"] != false
    {
        return Err("confirmation boundary is not unopened".into());
    }
    let corpus_sha = gate15["canonical_corpus_sha256"]
        .as_str()
        .ok_or("Gate15 canonical corpus hash missing")?;
    if atlas["source_corpus_sha256"] != corpus_sha {
        return Err("Gate15/Gate15.5 corpus ancestry mismatch".into());
    }
    let mut raw_dirs = Vec::new();
    for root in &roots {
        find_raw(root, &mut raw_dirs)?;
    }
    raw_dirs.sort();
    raw_dirs.dedup();
    let mut corpora = Vec::with_capacity(raw_dirs.len());
    let mut run_keys = BTreeSet::new();
    for directory in &raw_dirs {
        let corpus = RawCorpus::open(directory, "")?;
        if !run_keys.insert(corpus.report().run_key.clone()) {
            return Err("duplicate RG3 run key".into());
        }
        corpora.push(corpus);
    }
    let references = references(&coordinate_path)?;
    let schema_sha = hash_file(&schema_path)?;
    let package = build_gate16_laboratory(Gate16Inputs {
        corpora: &corpora,
        source_corpus_sha256: corpus_sha,
        input_schema_sha256: &schema_sha,
        references: &references,
    })?;
    fs::create_dir_all(&output)?;
    write_package(&output, &package)?;
    let mut source_list = BufWriter::new(File::create(output.join("source-raw-directories.txt"))?);
    for directory in &raw_dirs {
        writeln!(source_list, "{}", directory.display())?;
    }
    source_list.flush()?;
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(&output)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|name| name != "gate16_receipt.json")
        {
            files.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                hash_file(&path)?,
            );
        }
    }
    let mut canonical = Sha256::new();
    canonical.update(b"NORTHSTAR_RG3_GATE16_CANONICAL_LAB_V1\0");
    for (name, hash) in &files {
        canonical.update(name.as_bytes());
        canonical.update([0]);
        canonical.update(hash.as_bytes());
        canonical.update([0xff]);
    }
    let receipt = json!({
        "contract":"NORTHSTAR_RG3_GATE16_RECEIPT_V1", "status":package.report.status,
        "source_corpus_sha256":corpus_sha, "source_gate15_exit_sha256":hash_file(&gate15_exit)?,
        "source_gate15_5_receipt_sha256":hash_file(&atlas_receipt_path)?, "source_gate15_5_coordinates_sha256":hash_file(&coordinate_path)?,
        "source_confirmation_manifest_sha256":hash_file(&confirmation_path)?, "source_gate16_protocol_sha256":hash_file(&protocol_path)?,
        "source_rg3_schema_sha256":schema_sha, "files":files, "canonical_lab_sha256":format!("{:x}", canonical.finalize()),
        "confirmation_windows_opened":false, "distance_vector_composed":false, "scientific_claim":"PARTIAL_GEOMETRIES_ONLY"
    });
    write_json(&output.join("gate16_receipt.json"), &receipt)?;
    println!(
        "GATE16 status={} runs={} objects={} complete={} censored={} representations={} distances={} neighbors={}",
        package.report.status,
        package.report.source_run_count,
        package.report.source_object_count,
        package.report.completed_objects,
        package.report.censored_objects,
        package.report.representation_count,
        package.report.distance_contract_count,
        package.report.neighborhood_row_count
    );
    Ok(())
}

use hashbrown::HashSet;
use northstar_market_objects::{
    DATASETS, Gate155Package, RawCorpus, build_gate155_atlas, fit_trajectory_family_systems,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
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
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            find_raw(&path, output)?;
        }
    }
    Ok(())
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

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    Ok(serde_json::from_slice(bytes)?)
}

fn optional<T: ToString>(value: Option<T>) -> String {
    value.map_or(r"\N".into(), |value| value.to_string())
}

fn write_bridge(path: &Path, package: &Gate155Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "receipt_id\tobject_kind\trun_key\tcanonical_instrument\tobject_id\tobject_event_type\tobject_event_time\tmaster_snapshot_time\tsnapshot_age_seconds\tjoin_mode\tmax_gap_seconds\tavailability_code\tmaster_generation\tmaster_snapshot_hash\tregional_basis_hash\treference_price\treference_atr\tmedian_price\tmean_price\tstructural_sigma\tcog_price\tprice_region_code\tnearest_node_id\tnearest_node_lower\tnearest_node_price\tnearest_node_upper\tnearest_node_region_code\tnode_contact"
    )?;
    for row in &package.bridge_receipts {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.receipt_id,
            row.object_kind,
            row.run_key,
            row.canonical_instrument,
            row.object_id,
            row.object_event_type,
            row.object_event_time,
            optional(row.master_snapshot_time),
            optional(row.snapshot_age_seconds),
            row.join_mode,
            row.max_gap_seconds,
            row.availability_code,
            optional(row.master_generation),
            optional(row.master_snapshot_hash.as_deref()),
            optional(row.regional_basis_hash.as_deref()),
            optional(row.reference_price),
            optional(row.reference_atr),
            optional(row.median_price),
            optional(row.mean_price),
            optional(row.structural_sigma),
            optional(row.cog_price),
            optional(row.price_region_code),
            optional(row.nearest_node_id.as_deref()),
            optional(row.nearest_node_lower),
            optional(row.nearest_node_price),
            optional(row.nearest_node_upper),
            optional(row.nearest_node_region_code),
            optional(row.node_contact.map(u8::from))
        )?;
    }
    out.flush()
}

fn write_coordinates(path: &Path, package: &Gate155Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(128 * 1024, File::create(path)?);
    writeln!(
        out,
        "object_kind\trun_key\tcanonical_instrument\tobject_id\tterminal_reason_code\tcensored\tcompression_summary_family_id\tcompression_shape_family_id\tcompression_hybrid_family_id\texpansion_summary_family_id\texpansion_shape_family_id\texpansion_hybrid_family_id\tstructural_stratum\tterminal_bridge_receipt_id"
    )?;
    for row in &package.object_coordinates {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_kind,
            row.run_key,
            row.canonical_instrument,
            row.object_id,
            row.terminal_reason_code,
            u8::from(row.censored),
            row.compression_summary_family_id,
            row.compression_shape_family_id,
            row.compression_hybrid_family_id,
            row.expansion_summary_family_id,
            row.expansion_shape_family_id,
            row.expansion_hybrid_family_id,
            row.structural_stratum,
            row.terminal_bridge_receipt_id
        )?;
    }
    out.flush()
}

fn write_lineages(path: &Path, package: &Gate155Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(128 * 1024, File::create(path)?);
    writeln!(
        out,
        "lineage_id\trun_key\tcanonical_instrument\torigin_compression_id\texpansion_id\tdestination_compression_id\torigin_compression_summary_family_id\torigin_compression_shape_family_id\torigin_compression_hybrid_family_id\texpansion_summary_family_id\texpansion_shape_family_id\texpansion_hybrid_family_id\tdestination_compression_summary_family_id\tdestination_compression_shape_family_id\tdestination_compression_hybrid_family_id\torigin_structural_stratum\tterminal_structural_stratum\tstructural_node_contacts\tdestination_availability_code"
    )?;
    for row in &package.lineage_coordinates {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.lineage_id,
            row.run_key,
            row.canonical_instrument,
            row.origin_compression_id,
            row.expansion_id,
            optional(row.destination_compression_id),
            row.origin_compression_summary_family_id,
            row.origin_compression_shape_family_id,
            row.origin_compression_hybrid_family_id,
            row.expansion_summary_family_id,
            row.expansion_shape_family_id,
            row.expansion_hybrid_family_id,
            row.destination_compression_summary_family_id,
            row.destination_compression_shape_family_id,
            row.destination_compression_hybrid_family_id,
            row.origin_structural_stratum,
            row.terminal_structural_stratum,
            row.structural_node_contacts,
            row.destination_availability_code
        )?;
    }
    out.flush()
}

fn write_graph(output: &Path, package: &Gate155Package) -> std::io::Result<()> {
    let mut nodes = BufWriter::with_capacity(
        128 * 1024,
        File::create(output.join("gate15_5_atlas_nodes.tsv"))?,
    );
    writeln!(nodes, "node_key\tnode_type\tlocal_label\tauthority")?;
    for row in &package.atlas_nodes {
        writeln!(
            nodes,
            "{}\t{}\t{}\t{}",
            row.node_key, row.node_type, row.local_label, row.authority
        )?;
    }
    nodes.flush()?;
    let mut edges = BufWriter::with_capacity(
        256 * 1024,
        File::create(output.join("gate15_5_atlas_edges.tsv"))?,
    );
    writeln!(
        edges,
        "edge_key\tedge_type\tsource_key\tdestination_key\tevent_time\tweight\tavailability_code"
    )?;
    for row in &package.atlas_edges {
        writeln!(
            edges,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.edge_key,
            row.edge_type,
            row.source_key,
            row.destination_key,
            optional(row.event_time),
            row.weight,
            row.availability_code
        )?;
    }
    edges.flush()
}

fn write_intersections(path: &Path, package: &Gate155Package) -> std::io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(
        out,
        "object_kind\trun_key\tobject_id\tobject_start_time\tobject_end_time\tintersection_status\tauthority_status\toverlap_start\toverlap_end\toverlap_seconds\tfraction_of_object_lifetime\tfraction_of_episode_lifetime"
    )?;
    for row in &package.auction_intersections {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_kind,
            row.run_key,
            row.object_id,
            row.object_start_time,
            row.object_end_time,
            row.intersection_status,
            row.authority_status,
            optional(row.overlap_start),
            optional(row.overlap_end),
            optional(row.overlap_seconds),
            optional(row.fraction_of_object_lifetime),
            optional(row.fraction_of_episode_lifetime)
        )?;
    }
    out.flush()
}

fn confirmation_protocol(package: &Gate155Package, source: &Value, artifact_sha: &str) -> Value {
    let mut tolerances = Vec::new();
    let mut grouped = BTreeMap::<(String, String, String), Vec<f64>>::new();
    for row in package
        .subcohort_stability
        .iter()
        .filter(|row| row.support_class == "SUPPORTED_COMPARISON")
    {
        grouped
            .entry((
                row.object_kind.clone(),
                row.left_representation.clone(),
                row.right_representation.clone(),
            ))
            .or_default()
            .push(row.js_divergence_bits);
    }
    for ((kind, left, right), mut values) in grouped {
        values.sort_by(f64::total_cmp);
        let p95 = values[((values.len() - 1) * 95) / 100].max(0.05);
        tolerances.push(json!({"object_kind":kind,"left_representation":left,"right_representation":right,"metric":"JS_DIVERGENCE_BITS","maximum":p95,"derivation":"exploratory supported instrument/window blocked empirical p95 with 0.05 floor"}));
    }
    json!({
        "contract":"NORTHSTAR_RG3_GATE15_5_CONFIRMATION_PROTOCOL_V1","status":"FROZEN_UNOPENED","authorization":"NOT_AUTHORIZED_IN_GATE15_5",
        "family_system_artifact_sha256":artifact_sha,"confirmation_windows":source["windows"],"expected_runs":source["expected_runs"],
        "support_floors":{"SUPPORTED_COMPARISON":30,"DESCRIPTIVE_ONLY":8,"INSUFFICIENT_SUPPORT":"0..7"},
        "layers":{
            "frozen_system_transport":{"fit":"FORBIDDEN","assignment":"nearest frozen centroid; no OOS rejection radius authorized","null_rule":"future NULL cannot be invented from distance in Gate15.5"},
            "independent_rediscovery":{"recipe":"frozen Gate15 V2 discovery recipe","comparison":"label-invariant partition and atlas correspondence","claim":"distinct from frozen-system transport"}
        },
        "atlas_tests":["family recurrence","correspondence transport with and without NULL_FAMILY","structural-stratum correspondence transport","lineage phenotype support","population drift"],
        "tolerances":tolerances,"holdout_rule":"the 12 exact windows remain unopened until explicit future authorization"
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let output = PathBuf::from(args.next().ok_or("usage: northstar-market-object-atlas OUTPUT GATE15_EXIT CONFIRMATION_MANIFEST RG2_PROOF GATE15_5_PROTOCOL INPUT_ROOT [INPUT_ROOT...]")?);
    let gate15_exit = PathBuf::from(args.next().ok_or("missing Gate15 exit report")?);
    let confirmation_path = PathBuf::from(args.next().ok_or("missing confirmation manifest")?);
    let rg2_proof_path = PathBuf::from(args.next().ok_or("missing RG2 proof")?);
    let protocol_path = PathBuf::from(args.next().ok_or("missing Gate15.5 protocol")?);
    let roots: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if roots.is_empty() {
        return Err("at least one input root is required".into());
    }
    if output.exists() {
        return Err("atlas output already exists; use a fresh directory".into());
    }
    let gate15_exit_value = read_json(&gate15_exit)?;
    let confirmation = read_json(&confirmation_path)?;
    let rg2_proof = read_json(&rg2_proof_path)?;
    let protocol = read_json(&protocol_path)?;
    if confirmation["status"] != "FROZEN_UNOPENED"
        || confirmation["authorization"] != "NOT_AUTHORIZED_IN_GATE15"
    {
        return Err("confirmation boundary is not frozen and unopened".into());
    }
    if rg2_proof["status"] != "PASS" {
        return Err("RG2 ancestry proof is not PASS".into());
    }
    if protocol["status"] != "FROZEN" {
        return Err("Gate15.5 protocol is not frozen".into());
    }
    let mut raw_dirs = Vec::new();
    for root in &roots {
        find_raw(root, &mut raw_dirs)?;
    }
    raw_dirs.sort();
    raw_dirs.dedup();
    let mut corpora = Vec::with_capacity(raw_dirs.len());
    let mut run_keys = HashSet::with_capacity(raw_dirs.len());
    for directory in &raw_dirs {
        let corpus = RawCorpus::open(directory, "")?;
        if !run_keys.insert(corpus.report().run_key.clone()) {
            return Err(format!("duplicate run key {}", corpus.report().run_key).into());
        }
        corpora.push(corpus);
    }
    let (gate15_report, assignments, fitted) = fit_trajectory_family_systems(&corpora)?;
    let expected: BTreeMap<_, _> = confirmation["family_systems"]
        .as_array()
        .ok_or("family systems")?
        .iter()
        .map(|row| {
            (
                (
                    row["object_kind"].as_str().unwrap().to_owned(),
                    row["representation"].as_str().unwrap().to_owned(),
                ),
                row["sha256"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    for system in &fitted {
        if expected.get(&(system.object_kind.clone(), system.representation.clone()))
            != Some(&system.family_system_sha256)
        {
            return Err(format!(
                "family system drift: {} {}",
                system.object_kind, system.representation
            )
            .into());
        }
    }
    let corpus_sha = gate15_exit_value["canonical_corpus_sha256"]
        .as_str()
        .ok_or("canonical corpus hash")?;
    let package = build_gate155_atlas(&corpora, &gate15_report, &assignments, fitted, corpus_sha)?;
    fs::create_dir_all(&output)?;
    write_json(&output.join("gate15_5_exit_report.json"), &package.report)?;
    write_json(
        &output.join("gate15_5_family_system_artifacts.json"),
        &package.fitted_family_systems,
    )?;
    let family_artifact_sha = hash_file(&output.join("gate15_5_family_system_artifacts.json"))?;
    write_bridge(
        &output.join("gate15_5_structural_bridge_receipts.tsv"),
        &package,
    )?;
    write_coordinates(&output.join("gate15_5_object_coordinates.tsv"), &package)?;
    write_lineages(&output.join("gate15_5_lineage_coordinates.tsv"), &package)?;
    write_graph(&output, &package)?;
    write_intersections(
        &output.join("gate15_5_auction_interval_intersections.tsv"),
        &package,
    )?;
    write_json(
        &output.join("gate15_5_correspondence.json"),
        &json!({"global":package.global_correspondence,"conditioned":package.conditioned_correspondence}),
    )?;
    write_json(
        &output.join("gate15_5_constraint_motion_correspondence.json"),
        &package.constraint_motion_correspondence,
    )?;
    write_json(
        &output.join("gate15_5_subcohort_stability.json"),
        &package.subcohort_stability,
    )?;
    write_json(
        &output.join("gate15_5_conditioned_support_audit.json"),
        &package.conditioned_support_audit,
    )?;
    write_json(
        &output.join("gate15_5_constraint_motion_support_audit.json"),
        &package.constraint_motion_support_audit,
    )?;
    write_json(
        &output.join("gate15_5_object_phenotype_census.json"),
        &package.object_phenotype_census,
    )?;
    write_json(
        &output.join("gate15_5_lineage_phenotype_census.json"),
        &package.lineage_phenotype_census,
    )?;
    write_json(
        &output.join("gate15_5_null_audit.json"),
        &package.null_audit,
    )?;
    let protocol = confirmation_protocol(&package, &confirmation, &family_artifact_sha);
    write_json(
        &output.join("gate15_5_confirmation_protocol.json"),
        &protocol,
    )?;
    let mut sources = BufWriter::new(File::create(output.join("source-raw-directories.txt"))?);
    for dir in &raw_dirs {
        writeln!(sources, "{}", dir.display())?;
    }
    sources.flush()?;
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(&output)? {
        let path = entry?.path();
        if path.is_file() && path.file_name().unwrap() != "gate15_5_receipt.json" {
            files.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                hash_file(&path)?,
            );
        }
    }
    let protocol_sha = hash_file(&protocol_path)?;
    let mut semantic = Sha256::new();
    semantic.update(b"NORTHSTAR_RG3_GATE15_5_CANONICAL_ATLAS_V1\0");
    semantic.update(protocol_sha.as_bytes());
    semantic.update([0xff]);
    for (name, hash) in &files {
        semantic.update(name.as_bytes());
        semantic.update([0]);
        semantic.update(hash.as_bytes());
        semantic.update([0xff]);
    }
    let receipt = json!({"contract":"NORTHSTAR_RG3_GATE15_5_ATLAS_RECEIPT_V1","status":package.report.status,"source_corpus_sha256":corpus_sha,"rg2_source_corpus_sha256":rg2_proof["source_corpus_sha256"],
        "source_gate15_exit_sha256":hash_file(&gate15_exit)?,"source_confirmation_manifest_sha256":hash_file(&confirmation_path)?,"source_rg2_proof_sha256":hash_file(&rg2_proof_path)?,
        "source_gate15_5_protocol_sha256":protocol_sha,
        "files":files,"canonical_atlas_sha256":format!("{:x}",semantic.finalize()),"confirmation_windows_opened":false});
    write_json(&output.join("gate15_5_receipt.json"), &receipt)?;
    println!(
        "GATE15_5 status={} runs={} objects={} lineages={} receipts={} exact={} asof={} null_structural={} nodes={} edges={}",
        package.report.status,
        package.report.source_run_count,
        package.report.source_object_count,
        package.report.lineage_count,
        package.report.structural_bridge_receipts,
        package.report.exact_receipts,
        package.report.asof_receipts,
        package.report.null_structural_receipts,
        package.report.graph_nodes,
        package.report.graph_edges
    );
    if package.report.status != "PASS" {
        return Err("Gate15.5 exit checks failed".into());
    }
    Ok(())
}

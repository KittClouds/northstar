use blake3::Hasher as Blake3Hasher;
use northstar_market_objects::{
    DATASETS, DIFFERENCE_AXES, Gate165Inputs, Gate165Package, Gate165Reference, RawCorpus,
    build_gate165_census,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
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
    let mut writer = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn logical_census_hash(
    package: &Gate165Package,
    corpus_sha256: &str,
    gate16_lab_sha256: &str,
    protocol_sha256: &str,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"NORTHSTAR_RG3_GATE16_5_LOGICAL_CENSUS_V1\0");
    for value in [corpus_sha256, gate16_lab_sha256, protocol_sha256] {
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    for row in &package.pair_summaries {
        for value in [
            row.object_kind.as_str(),
            row.contract_id.as_str(),
            row.exhaustive_stream_blake3.as_str(),
        ] {
            hash.update(value.as_bytes());
            hash.update([0]);
        }
        for value in [
            row.total_same_kind_pairs,
            row.comparable_pairs,
            row.exact_zero_pairs,
            row.epsilon_near_pairs,
            row.nonzero_pairs,
            row.not_comparable_pairs,
        ] {
            hash.update((value as u64).to_le_bytes());
        }
    }
    for row in &package.raw_differences {
        hash.update(row.pair_id.as_bytes());
        hash.update([0]);
        hash.update(row.object_kind.as_bytes());
        hash.update([0]);
        hash.update(row.axis_status_bits.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn columns(header: &str) -> BTreeMap<&str, usize> {
    header
        .split('\t')
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect()
}

fn required(
    columns: &BTreeMap<&str, usize>,
    name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    columns
        .get(name)
        .copied()
        .ok_or_else(|| format!("missing field {name}").into())
}

fn references(
    coordinate_path: &Path,
    bridge_path: &Path,
) -> Result<BTreeMap<String, Gate165Reference>, Box<dyn std::error::Error>> {
    let bridge_text = fs::read_to_string(bridge_path)?;
    let mut bridge_lines = bridge_text.lines();
    let bridge_columns = columns(bridge_lines.next().ok_or("empty bridge file")?);
    let receipt_col = required(&bridge_columns, "receipt_id")?;
    let join_col = required(&bridge_columns, "join_mode")?;
    let age_col = required(&bridge_columns, "snapshot_age_seconds")?;
    let mut bridges = BTreeMap::<String, (String, Option<u32>)>::new();
    for line in bridge_lines.filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let age = (fields[age_col] != r"\N")
            .then(|| fields[age_col].parse())
            .transpose()?;
        bridges.insert(fields[receipt_col].into(), (fields[join_col].into(), age));
    }
    let coordinate_text = fs::read_to_string(coordinate_path)?;
    let mut lines = coordinate_text.lines();
    let coordinate_columns = columns(lines.next().ok_or("empty coordinate file")?);
    let kind_col = required(&coordinate_columns, "object_kind")?;
    let run_col = required(&coordinate_columns, "run_key")?;
    let id_col = required(&coordinate_columns, "object_id")?;
    let summary_cols = [
        required(&coordinate_columns, "compression_summary_family_id")?,
        required(&coordinate_columns, "expansion_summary_family_id")?,
    ];
    let shape_cols = [
        required(&coordinate_columns, "compression_shape_family_id")?,
        required(&coordinate_columns, "expansion_shape_family_id")?,
    ];
    let hybrid_cols = [
        required(&coordinate_columns, "compression_hybrid_family_id")?,
        required(&coordinate_columns, "expansion_hybrid_family_id")?,
    ];
    let stratum_col = required(&coordinate_columns, "structural_stratum")?;
    let terminal_receipt_col = required(&coordinate_columns, "terminal_bridge_receipt_id")?;
    let family = |value: &str| match value {
        "NOT_APPLICABLE" => None,
        "NULL_FAMILY" => Some("NULL".into()),
        value => Some(value.into()),
    };
    let mut output = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let kind_index = usize::from(fields[kind_col] == "EXPANSION");
        let key = format!(
            "{}::{}::{}",
            fields[run_col], fields[kind_col], fields[id_col]
        );
        let (join, age) = bridges
            .get(fields[terminal_receipt_col])
            .cloned()
            .unwrap_or_else(|| ("UNAVAILABLE".into(), None));
        if output
            .insert(
                key,
                Gate165Reference {
                    summary_family: family(fields[summary_cols[kind_index]]),
                    shape_family: family(fields[shape_cols[kind_index]]),
                    hybrid_family: family(fields[hybrid_cols[kind_index]]),
                    structural_stratum: fields[stratum_col].into(),
                    terminal_join_mode: join,
                    terminal_snapshot_age_seconds: age,
                },
            )
            .is_some()
        {
            return Err("duplicate Gate 15.5 coordinate key".into());
        }
    }
    Ok(output)
}

fn write_pair_census(path: &Path, package: &Gate165Package) -> std::io::Result<()> {
    let writer = BufWriter::with_capacity(512 * 1024, File::create(path)?);
    let mut out = zstd::stream::write::Encoder::new(writer, 9)?;
    writeln!(
        out,
        "pair_id\tobject_kind\tleft_object_key\tright_object_key\trepresentation_id\tdistance_contract\tcomparison_mode\tstatus\treason\tdistance\tzero_class\tsupport_points\tsupport_bars\tsupport_seconds\tleft_censored\tright_censored"
    )?;
    for row in &package.pair_census {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.pair_id,
            row.object_kind,
            row.left_object_key,
            row.right_object_key,
            row.representation_id,
            row.distance_contract,
            row.comparison_mode,
            row.status,
            row.reason,
            row.distance
                .map_or_else(|| r"\N".into(), |value| value.to_string()),
            row.zero_class,
            row.support_points,
            row.support_bars,
            row.support_seconds,
            u8::from(row.left_censored),
            u8::from(row.right_censored)
        )?;
    }
    let mut writer = out.finish()?;
    writer.flush()
}

fn write_differences(
    path: &Path,
    package: &Gate165Package,
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = BufWriter::with_capacity(512 * 1024, File::create(path)?);
    let mut out = zstd::stream::write::Encoder::new(writer, 9)?;
    writeln!(
        out,
        "pair_id\tobject_kind\trepresentation_id\tdistance_contract\tcomparison_mode\taxis_status_bits_hex"
    )?;
    for row in &package.raw_differences {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{:08x}",
            row.pair_id,
            row.object_kind,
            row.representation_id,
            row.distance_contract,
            row.comparison_mode,
            row.axis_status_bits,
        )?;
    }
    let mut writer = out.finish()?;
    writer.flush()?;
    Ok(())
}

fn write_object_authority(path: &Path, package: &Gate165Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "object_key\tobject_kind\traw_history_sha256\tduration_seconds\tobserved_bars\traw_direction\tcensored\tterminal_reason_code\tevent_multiplicity\tevent_order_and_type_blake3\tevent_timing_gaps_blake3\traw_summary_coordinates_blake3\traw_continuous_trajectory_blake3"
    )?;
    for row in &package.object_authority {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_key,
            row.object_kind,
            row.raw_history_sha256,
            row.duration_seconds,
            row.observed_bars,
            row.raw_direction,
            u8::from(row.censored),
            row.terminal_reason_code,
            row.event_multiplicity,
            row.event_order_and_type_blake3,
            row.event_timing_gaps_blake3,
            row.raw_summary_coordinates_blake3,
            row.raw_continuous_trajectory_blake3
        )?;
    }
    out.flush()
}

fn write_classes(path: &Path, package: &Gate165Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "contract_id\tobject_kind\tclass_id\tclass_size\tobject_key"
    )?;
    for row in &package.equivalence_classes {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            row.contract_id, row.object_kind, row.class_id, row.class_size, row.object_key
        )?;
    }
    out.flush()
}

fn write_turnover(path: &Path, package: &Gate165Package) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(256 * 1024, File::create(path)?);
    writeln!(
        out,
        "object_kind\tobject_key\traw_contract_id\tcanonical_contract_id\tretained_neighbors\tentered_neighbors\texited_neighbors\tunion_neighbors\tmembership_jaccard\tshared_rank_correlation\traw_direction\tduration_seconds\tterminal_reason_code\tcensored\tinstrument\trun_key"
    )?;
    for row in &package.canonicalization_turnover {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.object_kind,
            row.object_key,
            row.raw_contract_id,
            row.canonical_contract_id,
            row.retained_neighbors,
            row.entered_neighbors,
            row.exited_neighbors,
            row.union_neighbors,
            row.membership_jaccard,
            row.shared_rank_correlation
                .map_or_else(|| r"\N".into(), |value| value.to_string()),
            row.raw_direction,
            row.duration_seconds,
            row.terminal_reason_code,
            u8::from(row.censored),
            row.instrument,
            row.run_key
        )?;
    }
    out.flush()
}

fn write_package(
    output: &Path,
    package: &Gate165Package,
) -> Result<(), Box<dyn std::error::Error>> {
    write_json(&output.join("gate16_5_exit_report.json"), &package.report)?;
    write_pair_census(&output.join("gate16_5_pair_census.tsv.zst"), package)?;
    write_json(
        &output.join("gate16_5_pair_census_summary.json"),
        &package.pair_summaries,
    )?;
    write_differences(
        &output.join("gate16_5_raw_difference_receipts.tsv.zst"),
        package,
    )?;
    write_object_authority(&output.join("gate16_5_object_authority.tsv"), package)?;
    let sources = [
        "RG3_RAW_HISTORY_HASH",
        "EXACT_DERIVATION_TERMINAL_MINUS_START",
        "RG3_SAMPLE_AGE",
        "RG3_OBJECT_DIRECTION",
        "RG3_TERMINAL_SEMANTICS",
        "RG3_TERMINAL_REASON",
        "RG3_EVENT_ROWS",
        "EXACT_HASH_OF_ORDERED_RG3_EVENT_AUTHORITY",
        "EXACT_HASH_OF_RG3_EVENT_BAR_AND_SECOND_GAPS",
        "EXACT_HASH_OF_GATE16_RAW_SUMMARY_COORDINATES",
        "EXACT_HASH_OF_ORDERED_RG3_DERIVED_RAW_COORDINATES",
        "RG3_DID_NOT_EMIT_ATTEMPT_IDENTITIES",
        "RG3_DID_NOT_EMIT_BRANCH_MERGE_TOPOLOGY",
    ];
    let axes = DIFFERENCE_AXES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            json!({
                "index":index, "axis":name, "authority":sources[index], "encoding":{
                    "0":"EXACT_SAME", "1":"EXACT_DIFFERENT", "2":"NOT_EVALUABLE", "3":"RESERVED"
                }
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &output.join("gate16_5_difference_axis_dictionary.json"),
        &json!({
            "contract":"NORTHSTAR_GATE16_5_DIFFERENCE_AXIS_DICTIONARY_V1",
            "packing":"two bits per axis at index*2, least-significant axis first",
            "raw_values":"normalized once per object in gate16_5_object_authority.tsv",
            "axes":axes
        }),
    )?;
    write_json(
        &output.join("gate16_5_loss_profiles.json"),
        &package.loss_profiles,
    )?;
    write_classes(&output.join("gate16_5_equivalence_classes.tsv"), package)?;
    write_json(
        &output.join("gate16_5_cross_representation_sets.json"),
        &package.cross_representation_sets,
    )?;
    write_turnover(
        &output.join("gate16_5_canonicalization_turnover.tsv"),
        package,
    )?;
    write_json(
        &output.join("gate16_5_graph_incremental_audit.json"),
        &package.graph_incremental_audit,
    )?;
    write_json(
        &output.join("gate16_5_gate15_geometry_audit.json"),
        &package.gate15_geometry_audit,
    )?;
    write_json(
        &output.join("gate16_5_master_blocked_audit.json"),
        &package.master_blocked_audit,
    )?;
    Ok(())
}

fn verify_compact_artifacts(
    output: &Path,
    package: &Gate165Package,
) -> Result<Value, Box<dyn std::error::Error>> {
    let authority = package
        .object_authority
        .iter()
        .map(|row| row.object_key.as_str())
        .collect::<BTreeSet<_>>();
    let pair_file = File::open(output.join("gate16_5_pair_census.tsv.zst"))?;
    let pair_decoder = zstd::stream::read::Decoder::new(pair_file)?;
    let mut pair_reader = BufReader::with_capacity(512 * 1024, pair_decoder);
    let mut line = Vec::with_capacity(768);
    pair_reader.read_until(b'\n', &mut line)?;
    let mut pair_exact = 0usize;
    let mut pair_near = 0usize;
    let mut pair_not_comparable = 0usize;
    let mut pair_hash = Blake3Hasher::new();
    loop {
        line.clear();
        if pair_reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        while line
            .last()
            .is_some_and(|byte| matches!(*byte, b'\n' | b'\r'))
        {
            line.pop();
        }
        let mut fields = line.split(|byte| *byte == b'\t');
        let pair_id = fields.next().ok_or("compact pair row missing pair_id")?;
        let kind = fields.next().ok_or("compact pair row missing kind")?;
        let left = fields
            .next()
            .ok_or("compact pair row missing left object")?;
        let right = fields
            .next()
            .ok_or("compact pair row missing right object")?;
        for _ in 0..6 {
            fields.next().ok_or("compact pair row truncated")?;
        }
        let zero_class = fields.next().ok_or("compact pair row missing zero class")?;
        let left = std::str::from_utf8(left)?;
        let right = std::str::from_utf8(right)?;
        if !authority.contains(left) || !authority.contains(right) {
            return Err("compact pair row cannot rehydrate against object authority".into());
        }
        match zero_class {
            b"EXACT_ZERO" => {
                pair_exact += 1;
                pair_hash.update(pair_id);
                pair_hash.update(&[0]);
                pair_hash.update(kind);
                pair_hash.update(&[0xff]);
            }
            b"EPSILON_NEAR" => pair_near += 1,
            b"NOT_COMPARABLE" => pair_not_comparable += 1,
            _ => return Err("compact pair ledger contains an unretained class".into()),
        }
    }
    let receipt_file = File::open(output.join("gate16_5_raw_difference_receipts.tsv.zst"))?;
    let receipt_decoder = zstd::stream::read::Decoder::new(receipt_file)?;
    let mut receipt_reader = BufReader::with_capacity(512 * 1024, receipt_decoder);
    line.clear();
    receipt_reader.read_until(b'\n', &mut line)?;
    let mut receipt_count = 0usize;
    let mut receipt_hash = Blake3Hasher::new();
    loop {
        line.clear();
        if receipt_reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        while line
            .last()
            .is_some_and(|byte| matches!(*byte, b'\n' | b'\r'))
        {
            line.pop();
        }
        let mut fields = line.split(|byte| *byte == b'\t');
        let pair_id = fields.next().ok_or("difference row missing pair_id")?;
        let kind = fields.next().ok_or("difference row missing kind")?;
        receipt_hash.update(pair_id);
        receipt_hash.update(&[0]);
        receipt_hash.update(kind);
        receipt_hash.update(&[0xff]);
        receipt_count += 1;
    }
    let pair_exact_hash = pair_hash.finalize().to_hex().to_string();
    let receipt_exact_hash = receipt_hash.finalize().to_hex().to_string();
    let pass = pair_exact == package.report.exact_zero_relations
        && pair_near == package.report.epsilon_near_relations
        && pair_not_comparable == package.report.not_comparable_relations
        && receipt_count == pair_exact
        && pair_exact_hash == receipt_exact_hash
        && authority.len() == package.report.source_object_count;
    if !pass {
        return Err("compact artifact rehydration proof failed".into());
    }
    Ok(json!({
        "contract":"NORTHSTAR_GATE16_5_COMPACT_REHYDRATION_PROOF_V1",
        "status":"PASS",
        "object_authority_rows":authority.len(),
        "exact_zero_pair_rows":pair_exact,
        "epsilon_near_pair_rows":pair_near,
        "not_comparable_pair_rows":pair_not_comparable,
        "raw_difference_rows":receipt_count,
        "exact_pair_stream_blake3":pair_exact_hash,
        "difference_pair_stream_blake3":receipt_exact_hash,
        "statement":"Every retained pair resolves to two admitted authority objects; every exact-zero pair has exactly one compact raw-difference receipt."
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let usage = "usage: northstar-market-object-loss-census OUTPUT GATE15_EXIT GATE15_5_RECEIPT GATE15_5_COORDINATES GATE15_5_BRIDGES CONFIRMATION_MANIFEST GATE16_PROTOCOL GATE16_RECEIPT GATE16_5_PROTOCOL RG3_SCHEMA INPUT_ROOT [INPUT_ROOT...]";
    let output = PathBuf::from(args.next().ok_or(usage)?);
    let gate15_exit_path = PathBuf::from(args.next().ok_or(usage)?);
    let atlas_receipt_path = PathBuf::from(args.next().ok_or(usage)?);
    let coordinate_path = PathBuf::from(args.next().ok_or(usage)?);
    let bridge_path = PathBuf::from(args.next().ok_or(usage)?);
    let confirmation_path = PathBuf::from(args.next().ok_or(usage)?);
    let gate16_protocol_path = PathBuf::from(args.next().ok_or(usage)?);
    let gate16_receipt_path = PathBuf::from(args.next().ok_or(usage)?);
    let gate165_protocol_path = PathBuf::from(args.next().ok_or(usage)?);
    let schema_path = PathBuf::from(args.next().ok_or(usage)?);
    let roots = args.map(PathBuf::from).collect::<Vec<_>>();
    if roots.is_empty() || output.exists() {
        return Err("input root missing or output already exists".into());
    }
    let gate15 = read_json(&gate15_exit_path)?;
    let atlas = read_json(&atlas_receipt_path)?;
    let confirmation = read_json(&confirmation_path)?;
    let gate16_protocol = read_json(&gate16_protocol_path)?;
    let gate16_receipt = read_json(&gate16_receipt_path)?;
    let gate165_protocol = read_json(&gate165_protocol_path)?;
    if gate15["status"] != "PASS" || atlas["status"] != "PASS" || gate16_receipt["status"] != "PASS"
    {
        return Err("Gate 15, 15.5, or 16 ancestry is not sealed PASS".into());
    }
    if gate16_protocol["status"] != "FROZEN" || gate165_protocol["status"] != "FROZEN" {
        return Err("Gate 16 or Gate 16.5 protocol is not frozen".into());
    }
    if confirmation["status"] != "FROZEN_UNOPENED"
        || confirmation["authorization"] != "NOT_AUTHORIZED_IN_GATE15"
        || atlas["confirmation_windows_opened"] != false
        || gate16_receipt["confirmation_windows_opened"] != false
    {
        return Err("confirmation boundary is not unopened".into());
    }
    let corpus_sha = gate15["canonical_corpus_sha256"]
        .as_str()
        .ok_or("Gate 15 corpus hash missing")?;
    let expected_run_count = gate15["corpus"]["runs"]
        .as_u64()
        .ok_or("Gate 15 run count missing")? as usize;
    let expected_object_count = gate15["corpus"]["objects"]
        .as_u64()
        .ok_or("Gate 15 object count missing")? as usize;
    if atlas["source_corpus_sha256"] != corpus_sha
        || gate16_receipt["source_corpus_sha256"] != corpus_sha
    {
        return Err("RG3 ancestry hash mismatch".into());
    }
    let gate16_lab_sha = gate16_receipt["canonical_lab_sha256"]
        .as_str()
        .ok_or("Gate 16 lab hash missing")?;
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
    let references = references(&coordinate_path, &bridge_path)?;
    let package = build_gate165_census(Gate165Inputs {
        corpora: &corpora,
        expected_run_count,
        expected_object_count,
        source_corpus_sha256: corpus_sha,
        source_gate16_lab_sha256: gate16_lab_sha,
        references: &references,
    })?;
    fs::create_dir_all(&output)?;
    write_package(&output, &package)?;
    let rehydration = verify_compact_artifacts(&output, &package)?;
    write_json(
        &output.join("gate16_5_compact_rehydration_proof.json"),
        &rehydration,
    )?;
    let mut sources = BufWriter::new(File::create(output.join("source-raw-directories.txt"))?);
    for directory in &raw_dirs {
        writeln!(sources, "{}", directory.display())?;
    }
    sources.flush()?;
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(&output)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|name| name != "gate16_5_receipt.json")
        {
            files.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                hash_file(&path)?,
            );
        }
    }
    let mut physical = Sha256::new();
    physical.update(b"NORTHSTAR_RG3_GATE16_5_PHYSICAL_ARTIFACT_SET_V1\0");
    for (name, hash) in &files {
        physical.update(name.as_bytes());
        physical.update([0]);
        physical.update(hash.as_bytes());
        physical.update([0xff]);
    }
    let gate165_protocol_sha = hash_file(&gate165_protocol_path)?;
    let logical_census_sha =
        logical_census_hash(&package, corpus_sha, gate16_lab_sha, &gate165_protocol_sha);
    let physical_artifact_set_sha = format!("{:x}", physical.finalize());
    let receipt = json!({
        "contract":"NORTHSTAR_RG3_GATE16_5_RECEIPT_V1", "status":package.report.status,
        "source_corpus_sha256":corpus_sha, "source_gate15_exit_sha256":hash_file(&gate15_exit_path)?,
        "source_gate15_5_receipt_sha256":hash_file(&atlas_receipt_path)?, "source_gate15_5_coordinates_sha256":hash_file(&coordinate_path)?,
        "source_gate15_5_bridges_sha256":hash_file(&bridge_path)?, "source_confirmation_manifest_sha256":hash_file(&confirmation_path)?,
        "source_gate16_protocol_sha256":hash_file(&gate16_protocol_path)?, "source_gate16_receipt_sha256":hash_file(&gate16_receipt_path)?,
        "source_gate16_lab_sha256":gate16_lab_sha, "source_gate16_5_protocol_sha256":gate165_protocol_sha,
        "source_rg3_schema_sha256":hash_file(&schema_path)?, "files":files,
        "canonical_gate16_5_sha256":logical_census_sha.clone(),
        "logical_census_sha256":logical_census_sha,
        "physical_artifact_set_sha256":physical_artifact_set_sha,
        "compressed_containers":{
            "gate16_5_pair_census.tsv.zst":{"codec":"ZSTD","level":9,"implementation":"zstd-rs 0.13.3; exact implementation pinned by Cargo.lock"},
            "gate16_5_raw_difference_receipts.tsv.zst":{"codec":"ZSTD","level":9,"implementation":"zstd-rs 0.13.3; exact implementation pinned by Cargo.lock"}
        },
        "confirmation_windows_opened":false, "hash_buckets_authoritative":false,
        "exhaustive_compare_authoritative":true, "scientific_claim":"REPRESENTATION_LOSS_OBSERVATION_ONLY"
    });
    write_json(&output.join("gate16_5_receipt.json"), &receipt)?;
    println!(
        "GATE16_5 status={} runs={} objects={} same_kind_pairs={} contract_evaluations={} zero={} near={} not_comparable={}",
        package.report.status,
        package.report.source_run_count,
        package.report.source_object_count,
        package.report.total_same_kind_pairs,
        package.report.contract_pair_evaluations,
        package.report.exact_zero_relations,
        package.report.epsilon_near_relations,
        package.report.not_comparable_relations
    );
    Ok(())
}

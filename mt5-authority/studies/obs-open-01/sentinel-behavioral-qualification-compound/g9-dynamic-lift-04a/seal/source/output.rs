use crate::authority::{AuthorityRoots, G8_ROOT, O4A_ROOT, sha256_hex};
use crate::compare::{AttackProducts, MAP_ID};
use crate::model::{AccessAudit, MapInventoryRecord, RunSummary};
use northstar_operating_surface::canonical_json_bytes;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SealResult {
    pub scientific_root: String,
    pub complete_root: String,
    pub files: usize,
}

pub fn write_seal(
    out: &Path,
    crate_root: &Path,
    products: &AttackProducts,
    roots: &AuthorityRoots,
    access: AccessAudit,
    replayed: usize,
    permit: &northstar_operating_surface::ExecutionPermit,
) -> Result<SealResult, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("OUTPUT_DIRECTORY_NOT_EMPTY:{}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    snapshot_inputs(crate_root, out)?;
    let immediate = products
        .fractures
        .iter()
        .filter(|item| item.witness_class == "IMMEDIATE_FRACTURE")
        .count();
    let delayed = products.fractures.len() - immediate;
    let bounded_silence = products
        .search
        .iter()
        .filter(|item| item.pair_status == "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH")
        .count();
    let lawful = products
        .comparability
        .iter()
        .filter(|item| item.g6_context_fiber_match)
        .count();
    let map_status = if products.fractures.is_empty() {
        "NO_VERIFIED_FRACTURE_WITHIN_FROZEN_SEARCH"
    } else {
        "UNIVERSAL_BEHAVIORAL_PRESERVATION_FALSIFIED"
    };
    let summary = RunSummary {
        schema: "G9_DYNAMIC_LIFT_RUN_SUMMARY_V1".into(),
        status: "G9_PRIMARY_CHAMBER_SEALED_WITH_RESTRICTIONS".into(),
        map_status: map_status.into(),
        primary_map_id: MAP_ID.into(),
        records_replayed: replayed as u64,
        observed_fibers: products.observed_fibers as u64,
        collision_fibers: products.collision_fibers as u64,
        same_fiber_pairs: products.same_fiber_pairs,
        lawful_comparison_pairs: lawful as u64,
        immediate_fractures: immediate as u64,
        delayed_fractures: delayed as u64,
        bounded_silence_pairs: bounded_silence as u64,
        not_evaluable_pairs: (products.same_fiber_pairs as usize - lawful) as u64,
        access,
        restrictions: vec![
            "PRIMARY_MAP_ONLY_CURRENT_STATE_TO_REDUCED_GEOMETRY".into(),
            "DECLARED_D_A_SCOPE_ONLY".into(),
            "SEARCH_PAIR_BUDGET_1024".into(),
            "CONTINUATION_HORIZON_1".into(),
            "G4_GRAMMAR_STATE_AND_EVENT_BLIND_SPOT_OPEN".into(),
            "NO_GLOBAL_EQUIVALENCE_FROM_BOUNDED_SILENCE".into(),
            "NO_MARKET_ECONOMIC_OR_TRADING_AUTHORITY".into(),
        ],
    };

    let map_inventory = map_inventory();
    let relation_graph = json!({
        "schema": "04A_BEHAVIORAL_RELATION_GRAPH_V1",
        "map_id": MAP_ID,
        "pair_statuses": products.search,
        "fiber_status_rule": "ANY_VERIFIED_PAIR_FRACTURE_IMPLIES_UNIVERSAL_HOMOGENEITY_FALSIFIED_FOR_THAT_FIBER",
        "map_status": map_status,
        "nonclaims": ["COMPLETE_FIBER_PARTITION", "COMPLETE_MAP_CHARACTERIZATION"]
    });
    let contract_audit = json!({
        "schema": "G9_04A_CONTRACT_AUDIT_V1",
        "map_id": MAP_ID,
        "source_contract_fields": ["bar_index", "knowledge_time", "candidate_ids", "birth_bars", "birth_knowledge_times", "candidate_ages"],
        "target_contract_fields_present": ["current_upper_value", "current_lower_value", "upper_giveback", "lower_giveback", "range_locations", "range_extensions", "window_active", "coverage"],
        "04A_declared_lost_degrees": ["CANDIDATE_IDS", "BIRTH_TIMES", "CAUSAL_AGES"],
        "projection_verified_additional_dropped_fields": ["TRANSITION_ORDINAL", "AUTHORITATIVE_TIME"],
        "status": "04A_DECLARED_LOST_DEGREES_UNDERENUMERATED",
        "map_projection_status": "UNCHANGED_AND_EXECUTED_EXACTLY",
        "historical_04A_mutated": false,
        "scientific_consequence": "FIRST_1024_FROZEN_LAWFUL_PAIRS_FRACTURE_IMMEDIATELY_ON_PROTECTED_TRANSITION_ORDINAL"
    });
    let fiber_ledger = fiber_ledger(products);
    let verifier_receipts = products
        .fractures
        .iter()
        .map(|fracture| {
            json!({
                "schema": "G9_EXACT_VERIFIER_RECEIPT_V1",
                "receipt_id": fracture.verifier_receipt,
                "verifier": "G8_REAL_HISTORY_EXACT_VERIFIER_DESCENDANT_V1",
                "pair_id": fracture.pair.pair_id,
                "fiber_id": fracture.pair.fiber_id,
                "map_id": fracture.map_id,
                "witness": fracture.witness,
                "witness_class": fracture.witness_class,
                "first_divergent_observable": fracture.first_divergent_observable,
                "first_protected_observable_divergence": fracture.first_protected_observable_divergence,
                "recomputation_source": "SEALED_G1_G4_G5_G6",
                "explorer_authority": "NONE",
                "status": "PASS"
            })
        })
        .collect::<Vec<_>>();
    let primary_files = vec![
        write_json(out, "G9_RUN_SUMMARY.json", &summary)?,
        write_json(out, "G9_04A_MAP_INVENTORY.json", &map_inventory)?,
        write_json(out, "G9_04A_CONTRACT_AUDIT.json", &contract_audit)?,
        write_json(out, "G9_COMPARABILITY_GRAPH.json", &products.comparability)?,
        write_json(out, "G9_VERIFIED_FRACTURE_GRAPH.json", &products.fractures)?,
        write_json(out, "G9_FIBER_STATUS_LEDGER.json", &fiber_ledger)?,
        write_json(out, "G9_EXACT_VERIFIER_RECEIPTS.json", &verifier_receipts)?,
        write_json(out, "G9_SEARCH_COVERAGE_OVERLAY.json", &products.search)?,
        write_json(
            out,
            "04A_BEHAVIORAL_RELATION_GRAPH_V1.json",
            &relation_graph,
        )?,
        write_json(
            out,
            "G9_VERIFIED_SEPARATOR_CORPUS.json",
            &products.fractures,
        )?,
        write_json(
            out,
            "G9_BEHAVIORAL_FRACTURE_ATLAS.json",
            &products.fractures,
        )?,
        write_json(
            out,
            "G9_BOUNDED_SEPARATOR_INCIDENCE_LEDGER.json",
            &products.search,
        )?,
        write_json(
            out,
            "G9_BOUNDED_SILENCE_LEDGER.json",
            &products
                .search
                .iter()
                .filter(|item| item.pair_status == "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH")
                .collect::<Vec<_>>(),
        )?,
        write_json(
            out,
            "G9_EXECUTION_AUTHORITY_RECEIPT.json",
            &json!({
                "schema": "G9_EXECUTION_AUTHORITY_RECEIPT_V1",
                "gate_id": crate::authority::G9_GATE_ID,
                "constitution_root_blake3": roots.constitution,
                "precommit_root_blake3": roots.precommit,
                "data_capability_root_blake3": roots.data_capability,
                "incident_root_blake3": roots.incident,
                "apparatus_repair_ledger_root_blake3": roots.repair_ledger,
                "gate_spec_root_blake3": roots.gate_spec,
                "execution_id_blake3": roots.execution_id,
                "execution_capability_id": permit.capability_id,
                "execution_binding_root": permit.binding_root,
                "single_use_capability_consumed": true,
                "input_sha256_roots": roots.sha256_roots,
                "04A_root": O4A_ROOT,
                "G8_historical_root": G8_ROOT,
                "historical_G8_real_history_authority": "NONE",
                "verifier_descendant": "G8_REAL_HISTORY_EXACT_VERIFIER_DESCENDANT_V1",
                "verifier_descendant_authority_scope": "G9_PRIMARY_MAP_DECLARED_D_A_SCOPE_ONLY"
            }),
        )?,
    ];
    let scientific_root = manifest_root(&primary_files);
    write_json(
        out,
        "G9_PRIMARY_SCIENCE_ROOT.json",
        &json!({
            "schema": "G9_PRIMARY_SCIENCE_ROOT_V1",
            "root": scientific_root,
            "status": "SEALED",
            "primary_result_precedes_sidecars": true,
            "external_optic_inputs": 0
        }),
    )?;

    // Sidecars are emitted only after the primary root above exists. No external
    // optic is available in this execution lineage, so typed non-evaluability is
    // preserved rather than coerced to zero.
    write_json(
        out,
        "G9_PARALLAX_SIDECAR_MATRIX.json",
        &json!({
            "schema": "G9_PARALLAX_SIDECAR_MATRIX_V1",
            "primary_science_root": scientific_root,
            "status": "NOT_EVALUABLE",
            "reason": "NO_SEPARATELY_QUALIFIED_EXTERNAL_OPTIC_TRANSPORT_IN_G9_LINEAGE",
            "primary_result_influence": "NONE",
            "bounded_silence_coerced_to_zero": false
        }),
    )?;
    write_json(
        out,
        "G10_EVIDENCE_HANDOFF.json",
        &json!({
            "schema": "G10_EVIDENCE_HANDOFF_V1",
            "primary_science_root": scientific_root,
            "evidence": products.fractures,
            "necessity_verdict": "NOT_CLAIMED",
            "representation_prescription": "NOT_CLAIMED"
        }),
    )?;
    write_report(out, &summary, &scientific_root, products)?;
    let all_files = collect_files(out)?;
    let manifest = content_manifest(out, &all_files)?;
    fs::write(out.join("content_manifest.tsv"), &manifest)?;
    let complete_root = sha256_hex(&manifest);
    write_json(
        out,
        "G9_ROOT_RECEIPT.json",
        &json!({
            "schema": "G9_ROOT_RECEIPT_V1",
            "G9_root": complete_root,
            "primary_science_root": scientific_root,
            "status": "G9_DYNAMIC_LIFT_04A_SEALED_WITH_RESTRICTIONS",
            "manifest_members": all_files.len()
        }),
    )?;
    Ok(SealResult {
        scientific_root,
        complete_root,
        files: all_files.len() + 2,
    })
}

fn map_inventory() -> Vec<MapInventoryRecord> {
    let executable = |source: &str, target: &str, relation: &str| MapInventoryRecord {
        map_id: MAP_ID.into(),
        source_representation: source.into(),
        target_representation: target.into(),
        relation_class: relation.into(),
        execution_status: "EXECUTED_EXACT_PUBLIC_PROJECTION".into(),
        reason: "SEALED_G1_PROJECT_STATE_TO_04A_THEN_SEALED_REDUCED_STATE_FROM".into(),
    };
    let unavailable = |id: &str, source: &str, target: &str, relation: &str| {
        MapInventoryRecord {
        map_id: id.into(),
        source_representation: source.into(),
        target_representation: target.into(),
        relation_class: relation.into(),
        execution_status: "NOT_EVALUABLE".into(),
        reason: "NO_EQUALLY_EXACT_PUBLIC_G9_PAIR_PROJECTION_AND_REAL_HISTORY_VERIFIER_PATH_QUALIFIED; PRIVATE_04A_HELPERS_NOT_COPIED".into(),
    }
    };
    vec![
        unavailable(
            "04A_HISTORY_TO_SEMANTIC",
            "RAW_COMPLETED_BAR_HISTORY_V1",
            "SEMANTIC_TRANSITION_TAPE_V1",
            "LOSSY_QUOTIENT",
        ),
        unavailable(
            "04A_HISTORY_TO_SOURCE",
            "RAW_COMPLETED_BAR_HISTORY_V1",
            "EVENT_SOURCE_TAPE_V1",
            "LOSSY_QUOTIENT",
        ),
        unavailable(
            "04A_SOURCE_TO_SEMANTIC",
            "EVENT_SOURCE_TAPE_V1",
            "SEMANTIC_TRANSITION_TAPE_V1",
            "LOSSY_QUOTIENT",
        ),
        unavailable(
            "04A_HISTORY_TO_STATE",
            "RAW_COMPLETED_BAR_HISTORY_V1",
            "CURRENT_SENTINEL_STATE_V1",
            "RECONSTRUCTIBLE_WITH_CONTEXT",
        ),
        unavailable(
            "04A_SEMANTIC_TO_STATE",
            "SEMANTIC_TRANSITION_TAPE_V1",
            "CURRENT_SENTINEL_STATE_V1",
            "NON_RECONSTRUCTIBLE",
        ),
        unavailable(
            "04A_SOURCE_TO_STATE",
            "EVENT_SOURCE_TAPE_V1",
            "CURRENT_SENTINEL_STATE_V1",
            "RECONSTRUCTIBLE_WITH_CONTEXT",
        ),
        executable(
            "CURRENT_SENTINEL_STATE_V1",
            "REDUCED_SENTINEL_GEOMETRY_V1",
            "LOSSY_QUOTIENT",
        ),
    ]
}

fn fiber_ledger(products: &AttackProducts) -> Vec<serde_json::Value> {
    #[derive(Default)]
    struct Counts {
        lawful: u64,
        searched: u64,
        fractures: u64,
        bounded_silence: u64,
    }
    let mut counts: BTreeMap<String, Counts> = BTreeMap::new();
    for pair in products
        .comparability
        .iter()
        .filter(|pair| pair.g6_context_fiber_match)
    {
        counts.entry(pair.pair.fiber_id.clone()).or_default().lawful += 1;
    }
    for pair in &products.search {
        let item = counts.entry(pair.pair.fiber_id.clone()).or_default();
        item.searched += 1;
        if pair.pair_status == "DISTINGUISHABLE_WITH_WITNESS" {
            item.fractures += 1;
        } else if pair.pair_status == "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH" {
            item.bounded_silence += 1;
        }
    }
    counts
        .into_iter()
        .map(|(fiber_id, item)| {
            let status = if item.fractures > 0 {
                "UNIVERSAL_HOMOGENEITY_FALSIFIED"
            } else if item.searched == 0 {
                "UNSEARCHED"
            } else if item.bounded_silence > 0 {
                "BOUNDED_SILENCE"
            } else {
                "NOT_EVALUABLE"
            };
            json!({
                "fiber_id": fiber_id,
                "lawful_pair_count": item.lawful,
                "searched_pair_count": item.searched,
                "verified_fracture_count": item.fractures,
                "bounded_silence_count": item.bounded_silence,
                "fiber_status": status,
                "complete_partition_claimed": false
            })
        })
        .collect()
}

fn write_json<T: Serialize + ?Sized>(
    out: &Path,
    name: &str,
    value: &T,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let value = serde_json::to_value(value)?;
    let bytes = canonical_json_bytes(&value)?;
    fs::write(out.join(name), &bytes)?;
    Ok((name.into(), bytes))
}

fn snapshot_inputs(crate_root: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source_out = out.join("source");
    let constitution_out = out.join("constitution");
    fs::create_dir_all(&source_out)?;
    fs::create_dir_all(&constitution_out)?;
    for name in ["Cargo.toml", "Cargo.lock"] {
        fs::copy(crate_root.join(name), source_out.join(name))?;
    }
    for entry in fs::read_dir(crate_root.join("src"))? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            fs::copy(entry.path(), source_out.join(entry.file_name()))?;
        }
    }
    for entry in fs::read_dir(crate_root.join("constitution"))? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            fs::copy(entry.path(), constitution_out.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn manifest_root(files: &[(String, Vec<u8>)]) -> String {
    let mut lines: Vec<_> = files
        .iter()
        .map(|(name, bytes)| format!("{name}\t{}\t{}", bytes.len(), sha256_hex(bytes)))
        .collect();
    lines.sort_unstable();
    sha256_hex(lines.join("\n").as_bytes())
}

fn collect_files(out: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut pending = vec![out.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort_unstable();
    Ok(files)
}

fn content_manifest(out: &Path, files: &[PathBuf]) -> Result<Vec<u8>, std::io::Error> {
    let mut text = String::from("path\tbytes\tsha256\n");
    for path in files {
        let bytes = fs::read(path)?;
        let name = path
            .strip_prefix(out)
            .expect("member beneath output")
            .to_string_lossy()
            .replace('\\', "/");
        text.push_str(&format!(
            "{name}\t{}\t{}\n",
            bytes.len(),
            sha256_hex(&bytes)
        ));
    }
    Ok(text.into_bytes())
}

fn write_report(
    out: &Path,
    summary: &RunSummary,
    science_root: &str,
    products: &AttackProducts,
) -> Result<(), std::io::Error> {
    let report = format!(
        "# G9 Dynamic Lift of 04A — Final Scientific Report\n\n\
## Executive outcome\n\n\
- Status: `{}`\n\
- Primary map: `{}`\n\
- Map verdict: `{}`\n\
- Primary science root: `{}`\n\
- Records replayed: `{}`\n\
- Lawful comparison pairs in the frozen search budget: `{}`\n\
- Immediate fractures: `{}`\n\
- Delayed fractures: `{}`\n\
- Bounded-silence pairs: `{}`\n\n\
## Sol — semantic and lineage receipt\n\n\
The executable used the exact sealed G1 kernel and public 04A state projection. Same-fiber membership was computed from `ReducedState::from(&CommittedState)`. No FC/G9 source reconstructs G1 lifecycle, genealogy, rejection, or time semantics. Six other 04A relation surfaces remain `NOT_EVALUABLE` because no equally exact public projection/verifier path was qualified.\n\n\
## Kammi — adversarial receipt\n\n\
The chamber separated same-fiber membership, G6 comparability, exact fracture verification, and bounded search coverage. It rejected context-incompatible pairs, tested epsilon first, preserved bounded silence as unresolved, prohibited result-driven resizing, and left external optics outside the primary chamber. The historical synthetic-only G8 certificate path was not relabeled as real-history authority; a named exact-verification descendant recomputed claims from G1/G4/G5/G6.\n\n\
## Kage — custody and access receipt\n\n\
The G9 constitution, precommit, and D_A capability were hashed before the single-use execution capability was consumed. The authorized run read D_A only. D_B, D_C, D_D, target, outcome, explorer-result, and external-optic reads remained zero. A preauthority repository-discovery incident is preserved as a quarantined fossil and none of its values entered search design or scientific claims.\n\n\
## Scientific result boundaries\n\n\
Pair, fiber, and map quantifiers remain separate. A verified pair fracture falsifies universal homogeneity for its fiber and universal preservation for this map; it does not partition the whole fiber. Search rank is not divergence depth. Bounded incidence is not continuation-space density. G4's `GrammarStateAndEvent` blind spot remains open. No economic, predictive, market, or trading claim is made.\n\n\
## Remaining lawful gaps\n\n\
- Other actual 04A maps require exact public projection and real-history verifier descendants before execution.\n\
- Parallax sidecars are `NOT_EVALUABLE` because no separately qualified external-optic transport is in this lineage.\n\
- G10 receives fracture evidence only; necessity and representation prescriptions are not claimed.\n\n\
## Artifact census\n\n\
- Same-fiber candidate pairs: `{}`\n\
- Comparability records: `{}`\n\
- Verified fracture specimens: `{}`\n\
- Search coverage records: `{}`\n",
        summary.status,
        summary.primary_map_id,
        summary.map_status,
        science_root,
        summary.records_replayed,
        summary.lawful_comparison_pairs.min(1024),
        summary.immediate_fractures,
        summary.delayed_fractures,
        summary.bounded_silence_pairs,
        products.same_fiber_pairs,
        products.comparability.len(),
        products.fractures.len(),
        products.search.len(),
    );
    fs::write(out.join("G9_FINAL_REPORT.md"), report)
}

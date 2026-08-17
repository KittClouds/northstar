use crate::authority::{
    AuthorityRoots, G9_PRIMARY_SCIENCE_ROOT, G9_QUALIFICATION_ROOT, sha256_hex,
};
use crate::engine::{CensusProducts, DiscoveryProducts, MAP_ID, SEARCH_BOX_ID};
use crate::model::{AccessAudit, CensusSummary};
use northstar_operating_surface::canonical_json_bytes;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PhaseSeal {
    pub root: String,
    pub files: usize,
}

pub fn access(opened: &obs_open_04a::authority::BoundAuthority) -> AccessAudit {
    AccessAudit {
        raw_source_sha256: opened.raw_source_hash.clone(),
        real_04a_history_reads: opened.access.d_a_retained_causal_bars as u64,
        d_a_sessions_decoded: opened.access.d_a_sessions_decoded as u64,
        d_a_bars_replayed: opened.access.d_a_retained_causal_bars as u64,
        d_a_path_gap_sessions: opened.access.d_a_path_gap_sessions as u64,
        d_b_reads: 0,
        d_c_reads: 0,
        d_d_reads: 0,
        target_reads: 0,
        outcome_reads: 0,
        external_optic_reads: 0,
        g10_claims: 0,
    }
}

pub fn write_census(
    out: &Path,
    crate_root: &Path,
    p: &CensusProducts,
    roots: &AuthorityRoots,
    permit: &northstar_operating_surface::ExecutionPermit,
    access: AccessAudit,
) -> Result<PhaseSeal, Box<dyn std::error::Error>> {
    prepare(out)?;
    snapshot(crate_root, out)?;
    write_json(
        out,
        "G9_1_RESIDUAL_PAIR_STRATA.json",
        &json!({
            "schema":"G9_1_RESIDUAL_PAIR_STRATA_V1", "map_id":MAP_ID,
            "summary":p.summary, "fiber_strata":p.fibers,
            "cardinality_authority":"EXACT_COMPLETE_ENUMERATION",
            "pair_orientation":"CANONICAL_UNORDERED", "continuation_evaluations":0
        }),
    )?;
    write_json(out, "G9_1_E2_PAIR_REGISTRY.json", &p.e2_pairs)?;
    write_json(out, "G9_1_RESIDUAL_BOUNDARY_ATLAS.json", &p.residual)?;
    write_json(out, "G9_1_CENSUS_ACCESS_AUDIT.json", &access)?;
    write_json(
        out,
        "G9_1_CENSUS_EXECUTION_RECEIPT.json",
        &execution_receipt("CENSUS", roots, permit, None),
    )?;
    write_json(
        out,
        "G9_1_CENSUS_NONCLAIMS.json",
        &json!({
            "continuation_search_performed":false,"incidence_authority":"NONE",
            "causal_necessity":"NOT_CLAIMED","raw_state_difference_as_tau_comp":false,
            "G4_GrammarStateAndEvent_blind_spot":"OPEN","market_or_prediction_authority":"NONE"
        }),
    )?;
    let root = seal_root(
        out,
        "G9_1_CENSUS_ROOT_RECEIPT.json",
        "G9_1_EXACT_CENSUS_ROOT_V1",
    )?;
    Ok(PhaseSeal {
        root,
        files: collect_files(out)?.len(),
    })
}

#[allow(clippy::too_many_arguments)] // Explicit sealed inputs are kept visible at this custody boundary.
pub fn write_discovery(
    out: &Path,
    crate_root: &Path,
    p: &DiscoveryProducts,
    roots: &AuthorityRoots,
    permit: &northstar_operating_surface::ExecutionPermit,
    access: AccessAudit,
    census_root: &str,
    census: &CensusSummary,
) -> Result<PhaseSeal, Box<dyn std::error::Error>> {
    prepare(out)?;
    snapshot(crate_root, out)?;
    write_json(out, "G9_1_DELAYED_FRACTURE_ATLAS.json", &p.fractures)?;
    write_json(
        out,
        "G9_1_BOUNDED_SILENCE_LEDGER.json",
        &p.exposure
            .iter()
            .filter(|x| x.status == "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH")
            .collect::<Vec<_>>(),
    )?;
    write_json(out, "G9_1_SEARCH_COVERAGE_OVERLAY.json", &p.exposure)?;
    write_json(
        out,
        "G9_1_MECHANISM_CENSUS.json",
        &json!({
            "schema":"G9_1_EXACT_MECHANISM_CENSUS_V1","tranche":"DISCOVERY",
            "classes":p.mechanisms,"not_prevalence_authority":true
        }),
    )?;
    write_json(
        out,
        "G9_1_DISCOVERY_SUMMARY.json",
        &json!({
            "schema":"G9_1_DISCOVERY_SUMMARY_V1","status":"SEALED_WITH_RESTRICTIONS",
            "scientific_outcome":typed_outcome(census,p),
            "census_root":census_root,"E0":census.e0_exact_cardinality,"E1":census.e1_exact_cardinality,
            "E2":census.e2_exact_cardinality,"search_box_id":SEARCH_BOX_ID,
            "searched_pairs":p.searched_pairs,"verified_delayed_fractures":p.fractures.len(),
            "exact_mechanism_classes":p.mechanisms.len(),"bounded_silence_pairs":p.exposure.iter().filter(|x|x.status=="NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH").count(),
            "continuation_evaluations":p.exposure.iter().map(|x|u64::from(x.continuations_tested)).sum::<u64>(),
            "saturation_stopped":p.saturation_stopped,"discovery_counts_are_prevalence":false,
            "null_distinctions":{
                "E1_EMPTY":"NO_ORDINAL_MATCHED_LAWFUL_SAME_FIBER_SPECIMENS",
                "E2_EMPTY_WHEN_E1_NONEMPTY":"RESIDUAL_IMMEDIATE_HETEROGENEITY_AFTER_ORDINAL_CONTROL",
                "E2_SEARCHED_WITHOUT_WITNESS":"BOUNDED_SILENCE_NOT_EQUIVALENCE"
            }
        }),
    )?;
    write_json(out, "G9_1_DISCOVERY_ACCESS_AUDIT.json", &access)?;
    write_json(
        out,
        "G9_1_DISCOVERY_EXECUTION_RECEIPT.json",
        &execution_receipt("DISCOVERY", roots, permit, Some(census_root)),
    )?;
    write_json(
        out,
        "G9_1_X_CROSS_FIBER_MECHANISM_TRANSPORT.json",
        &json!({
            "schema":"G9_1_X_CROSS_FIBER_MECHANISM_TRANSPORT_V1","status":"NOT_EXECUTED",
            "reason":"REQUIRES_SEPARATE_POST_PRIMARY_DESCENDANT_PRECOMMIT_AFTER_PRIMARY_MECHANISM_ROOT_SEALED",
            "primary_influence":"NONE","necessity_claimed":false
        }),
    )?;
    write_json(
        out,
        "G10_EVIDENCE_HANDOFF.json",
        &json!({
            "schema":"G10_EVIDENCE_HANDOFF_FROM_G9_1_V1","evidence":p.fractures,
            "necessity_verdict":"NOT_CLAIMED","representation_prescription":"NOT_CLAIMED"
        }),
    )?;
    let root = seal_root(
        out,
        "G9_1_DISCOVERY_ROOT_RECEIPT.json",
        "G9_1_DISCOVERY_ROOT_V1",
    )?;
    Ok(PhaseSeal {
        root,
        files: collect_files(out)?.len(),
    })
}

fn typed_outcome(census: &CensusSummary, p: &DiscoveryProducts) -> &'static str {
    if census.e1_exact_cardinality == 0 {
        "NO_ORDINAL_MATCHED_LAWFUL_SAME_FIBER_SPECIMENS"
    } else if census.e2_exact_cardinality == 0 {
        "ORDINAL_MATCHED_PAIRS_EXIST_BUT_NONE_ARE_EPSILON_BOUNDARY_EQUAL"
    } else if p.fractures.is_empty() {
        "NO_DELAYED_FRACTURE_FOUND_UNDER_PRECOMMITTED_BOUNDED_DISCOVERY"
    } else {
        "VERIFIED_DELAYED_FRACTURE_MECHANISM_DISCOVERED"
    }
}

fn execution_receipt(
    phase: &str,
    roots: &AuthorityRoots,
    permit: &northstar_operating_surface::ExecutionPermit,
    census_root: Option<&str>,
) -> serde_json::Value {
    json!({
        "schema":format!("G9_1_{}_EXECUTION_RECEIPT_V1",phase),"phase":phase,
        "parent_G9_qualification_root":G9_QUALIFICATION_ROOT,"parent_G9_primary_science_root":G9_PRIMARY_SCIENCE_ROOT,
        "constitution_root_blake3":roots.constitution,"census_precommit_root_blake3":roots.census_precommit,
        "discovery_precommit_root_blake3":roots.discovery_precommit,"gate_spec_root_blake3":roots.gate_spec,
        "execution_id_blake3":roots.execution_id,"execution_capability_id":permit.capability_id,
        "execution_binding_root":permit.binding_root,"single_use_capability_consumed":true,
        "bound_census_root":census_root,"input_sha256_roots":roots.input_sha256
    })
}

fn prepare(out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("OUTPUT_DIRECTORY_NOT_EMPTY:{}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    Ok(())
}
fn snapshot(crate_root: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let s = out.join("source");
    let c = out.join("constitution");
    fs::create_dir_all(&s)?;
    fs::create_dir_all(&c)?;
    for name in ["Cargo.toml", "Cargo.lock"] {
        fs::copy(crate_root.join(name), s.join(name))?;
    }
    for entry in fs::read_dir(crate_root.join("src"))? {
        let e = entry?;
        if e.path().extension().is_some_and(|x| x == "rs") {
            fs::copy(e.path(), s.join(e.file_name()))?;
        }
    }
    for entry in fs::read_dir(crate_root.join("constitution"))? {
        let e = entry?;
        if e.path().extension().is_some_and(|x| x == "json") {
            fs::copy(e.path(), c.join(e.file_name()))?;
        }
    }
    Ok(())
}
fn write_json<T: Serialize + ?Sized>(
    out: &Path,
    name: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = canonical_json_bytes(&serde_json::to_value(value)?)?;
    fs::write(out.join(name), bytes)?;
    Ok(())
}
fn collect_files(out: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut pending = vec![out.to_path_buf()];
    let mut files = Vec::new();
    while let Some(d) = pending.pop() {
        for e in fs::read_dir(d)? {
            let p = e?.path();
            if p.is_dir() {
                pending.push(p)
            } else if p.is_file() {
                files.push(p)
            }
        }
    }
    files.sort();
    Ok(files)
}
fn seal_root(
    out: &Path,
    receipt: &str,
    schema: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let files = collect_files(out)?;
    let mut manifest = String::from("path\tbytes\tsha256\n");
    for p in &files {
        let b = fs::read(p)?;
        let n = p.strip_prefix(out)?.to_string_lossy().replace('\\', "/");
        manifest.push_str(&format!("{n}\t{}\t{}\n", b.len(), sha256_hex(&b)));
    }
    fs::write(out.join("content_manifest.tsv"), manifest.as_bytes())?;
    let root = sha256_hex(manifest.as_bytes());
    write_json(
        out,
        receipt,
        &json!({"schema":schema,"root":root,"manifest_members":files.len(),"status":"SEALED"}),
    )?;
    Ok(root)
}

pub fn read_census_summary(path: &Path) -> Result<CensusSummary, Box<dyn std::error::Error>> {
    let v: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("G9_1_RESIDUAL_PAIR_STRATA.json"))?)?;
    Ok(serde_json::from_value(
        v.get("summary").ok_or("CENSUS_SUMMARY_MISSING")?.clone(),
    )?)
}
pub fn read_phase_root(path: &Path, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let v: serde_json::Value = serde_json::from_slice(&fs::read(path.join(name))?)?;
    Ok(v.get("root")
        .and_then(|x| x.as_str())
        .ok_or("PHASE_ROOT_MISSING")?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn census(e1: u64, e2: u64) -> CensusSummary {
        CensusSummary {
            schema: String::new(),
            status: String::new(),
            records_replayed: 0,
            observed_fibers: 0,
            collision_fibers: 0,
            same_fiber_candidate_pairs: 0,
            e0_exact_cardinality: 0,
            e1_exact_cardinality: e1,
            e2_exact_cardinality: e2,
            e1_minus_e2_exact_cardinality: e1 - e2,
            not_g6_comparable: 0,
            ordinal_mismatch: 0,
            semantic_time_control_mismatch: 0,
            other_epsilon_protected_fracture: 0,
            continuation_evaluations: 0,
            exact_enumeration: true,
        }
    }

    #[test]
    fn empty_e1_is_not_bounded_silence() {
        let p = DiscoveryProducts {
            fractures: vec![],
            exposure: vec![],
            mechanisms: vec![],
            searched_pairs: 0,
            saturation_stopped: false,
        };
        assert_eq!(
            typed_outcome(&census(0, 0), &p),
            "NO_ORDINAL_MATCHED_LAWFUL_SAME_FIBER_SPECIMENS"
        );
    }
}

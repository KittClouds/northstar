use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const TOPOLOGY_ROOT: &str = "5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1";
const INTERVENTION_ROOT: &str = "2fc9a4c150040b0f5167c341503583f1caeb298241eb1ec04b9614cc3837065f";
const CLOSURE_ROOT: &str = "b96ad31eb43f30ec5026a71a1d607d95e0041543f67418dd20180fabf3c7db71";
const RUNTIME_ROOT: &str = "03fc7cc1733f771573637329d9236ba548c1d29b69846cee655b84b746ed9901";
const INTERVENTION_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/intervention-topology/seal/OTO_I_INTERVENTION_REGISTRY.json";
const CLOSURE_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/covariation-closure/seal/OTO_C_CLOSURE_REGISTRY.json";

fn read_json(repo: &Path, rel: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(repo.join(rel))?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn artifact_entry(dir: &Path, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(dir.join(name))?;
    Ok(json!({"path": name, "bytes": bytes.len(), "sha256": sha256_bytes(&bytes)}))
}

fn is_renderer(record: &Value) -> bool {
    record["intervention_status"].as_str() == Some("LOCALLY_REWRITEABLE")
        && record["target_group"].as_str() == Some("LEGACY_RENDERER")
        || record["intervention_status"].as_str() == Some("LOCALLY_REWRITEABLE")
            && record["target_joint"].as_str() == Some("InpShowExtremeSentinels")
        || record["intervention_status"].as_str() == Some("LOCALLY_REWRITEABLE")
            && record["target_group"].as_str() == Some("SENTINEL")
}

fn gate_vector(
    record: &Value,
    closure: &Value,
) -> (
    Value,
    &'static str,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    bool,
    &'static str,
) {
    let status = record["intervention_status"].as_str().unwrap_or("UNKNOWN");
    let closure_status = closure["closure_status"].as_str().unwrap_or("UNKNOWN");
    let domain = status == "CHANGES_INPUT_DOMAIN" || closure_status == "OPEN_REVIEW_REQUIRED";
    let platform = status == "NOT_SEPARABLY_INTERVENABLE";
    let unknown = status == "UNKNOWN" || closure_status == "UNKNOWN";
    if platform || unknown {
        let unknowns = ["G0", "G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        return (
            json!({
                "G0_INPUT_AUTHORITY":"UNKNOWN", "G1_KERNEL_EXTRACTION":"UNKNOWN", "G2_ROLE_CENSUS":"UNKNOWN",
                "G3_REACHABILITY":"UNKNOWN", "G4_OBSERVER_SURFACE":"UNKNOWN", "G5_CONTINUATION_GRAMMAR":"UNKNOWN",
                "G6_EQUIVALENCE_CONTRACT":"UNKNOWN", "G7_FORMAL_ANALYSIS_BOUNDARY":"UNKNOWN", "G8_LABORATORY":"UNKNOWN"
            }),
            "UNKNOWN",
            Vec::new(),
            Vec::new(),
            unknowns,
            true,
            "UNRESOLVED_PREDECESSOR_CLASSIFICATION",
        );
    }
    if domain {
        let cone = (0..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
        return (
            json!({
                "G0_INPUT_AUTHORITY":"REQUALIFY", "G1_KERNEL_EXTRACTION":"REEXTRACT", "G2_ROLE_CENSUS":"REEXECUTE",
                "G3_REACHABILITY":"REPROVE_NAMED_COMPONENTS", "G4_OBSERVER_SURFACE":"REFREEZE", "G5_CONTINUATION_GRAMMAR":"REQUALIFY",
                "G6_EQUIVALENCE_CONTRACT":"NEW_FIBER_CONTRACT_REQUIRED", "G7_FORMAL_ANALYSIS_BOUNDARY":"THEOREM_TRANSFER_REAUDIT",
                "G8_LABORATORY":"AUTHORITY_BUNDLE_INVALIDATED"
            }),
            "G0",
            Vec::new(),
            cone,
            Vec::new(),
            true,
            "DOMAIN_OR_COMMON_DOMAIN_AUTHORITY_NOT_CLOSED",
        );
    }
    if is_renderer(record) {
        let preserved = (0..=3).map(|i| format!("G{i}")).collect::<Vec<_>>();
        let cone = (4..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
        return (
            json!({
                "G0_INPUT_AUTHORITY":"PRESERVED", "G1_KERNEL_EXTRACTION":"PRESERVED", "G2_ROLE_CENSUS":"PRESERVED",
                "G3_REACHABILITY":"PRESERVED", "G4_OBSERVER_SURFACE":"REFREEZE", "G5_CONTINUATION_GRAMMAR":"REQUALIFY",
                "G6_EQUIVALENCE_CONTRACT":"NEW_FIBER_CONTRACT_REQUIRED", "G7_FORMAL_ANALYSIS_BOUNDARY":"THEOREM_TRANSFER_REAUDIT",
                "G8_LABORATORY":"AUTHORITY_BUNDLE_INVALIDATED"
            }),
            "G4",
            preserved,
            cone,
            Vec::new(),
            false,
            "STATIC_PUBLICATION_ONLY_RESTRICTION",
        );
    }
    let cone = (1..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
    (
        json!({
            "G0_INPUT_AUTHORITY":"PRESERVED", "G1_KERNEL_EXTRACTION":"REEXTRACT", "G2_ROLE_CENSUS":"REEXECUTE",
            "G3_REACHABILITY":"INVALIDATED", "G4_OBSERVER_SURFACE":"REFREEZE", "G5_CONTINUATION_GRAMMAR":"REQUALIFY",
            "G6_EQUIVALENCE_CONTRACT":"NEW_FIBER_CONTRACT_REQUIRED", "G7_FORMAL_ANALYSIS_BOUNDARY":"INVALIDATED",
            "G8_LABORATORY":"AUTHORITY_BUNDLE_INVALIDATED"
        }),
        "G1",
        vec!["G0".to_owned()],
        cone,
        Vec::new(),
        true,
        "STATIC_SEMANTIC_DEFORMATION_REQUIRES_KERNEL_REEXTRACTION",
    )
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.is_file());
    paths.sort();
    Ok(paths)
}

pub fn build(repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!(
            "output directory must not already contain files: {}",
            out.display()
        )
        .into());
    }
    fs::create_dir_all(out)?;
    let interventions = read_json(repo, INTERVENTION_REL)?;
    let closures = read_json(repo, CLOSURE_REL)?;
    let records = interventions["records"]
        .as_array()
        .ok_or("intervention records missing")?;
    let closure_records = closures["closures"]
        .as_array()
        .ok_or("closure records missing")?;
    if records.len() != 58 || closure_records.len() != 58 {
        return Err(format!(
            "expected 58 intervention and closure records, found {} and {}",
            records.len(),
            closure_records.len()
        )
        .into());
    }
    let mut closure_map = BTreeMap::new();
    for closure in closure_records {
        closure_map.insert(
            closure["intervention_id"].as_str().unwrap_or("UNKNOWN"),
            closure,
        );
    }
    let mut impacts = Vec::with_capacity(records.len());
    let mut counts = BTreeMap::<String, usize>::new();
    let mut gate_counts = BTreeMap::<String, usize>::new();
    let mut runtime_count = 0usize;
    for record in records {
        let id = record["intervention_id"].as_str().unwrap_or("UNKNOWN");
        let closure = closure_map
            .get(id)
            .ok_or_else(|| format!("closure missing for {id}"))?;
        let (gates, earliest, preserved, cone, unknown, runtime_required, basis) =
            gate_vector(record, closure);
        if runtime_required {
            runtime_count += 1;
        }
        *counts.entry(earliest.to_owned()).or_insert(0) += 1;
        for gate in &cone {
            *gate_counts.entry(gate.clone()).or_insert(0) += 1;
        }
        let mut roots_invalidated = cone
            .iter()
            .map(|gate| format!("{gate}_ROOT"))
            .collect::<Vec<_>>();
        roots_invalidated.sort();
        let mut roots_preserved = preserved
            .iter()
            .map(|gate| format!("{gate}_ROOT"))
            .collect::<Vec<_>>();
        roots_preserved.sort();
        impacts.push(json!({
            "intervention_id": id,
            "target_joint": record["target_joint"],
            "target_class": record["target_class"],
            "target_group": record["target_group"],
            "intervention_status": record["intervention_status"],
            "closure_status": closure["closure_status"],
            "closure_set": closure["closure_set"],
            "authority_impact": gates,
            "earliest_invalidated_gate": earliest,
            "descendant_requalification_cone": cone,
            "unchanged_ancestry": preserved,
            "roots_invalidated": roots_invalidated,
            "roots_preserved": roots_preserved,
            "static_proof_basis": ["OTO_I_INTERVENTION_REGISTRY", "OTO_C_CLOSURE_REGISTRY", "intervention_status", "closure_status", "preservation_requirements", "downstream_affected_nodes"],
            "preservation_requirements": record["preservation_requirements"],
            "downstream_affected_nodes": record["downstream_affected_nodes"],
            "runtime_evidence_required": runtime_required,
            "runtime_evidence_basis": if runtime_required {
                if record["runtime_confirmation_required"].as_bool().unwrap_or(false) {
                    "OTO-I runtime confirmation or OTO-C open/runtime status"
                } else {
                    "OTO-C open/runtime status"
                }
            } else {
                "no runtime evidence required for static publication restriction"
            },
            "unknown_fields": unknown,
            "authority_impact_basis": basis,
            "numeric_authority_distance": "NOT_CLAIMED",
            "dynamic_dof_authority": "NONE"
        }));
    }
    impacts.sort_by(|a, b| {
        a["intervention_id"]
            .as_str()
            .cmp(&b["intervention_id"].as_str())
    });
    let protocol = json!({
        "schema":"OTO_A_PROTOCOL_V1", "gate":"OTO-A_AUTHORITY_IMPACT_TOPOLOGY", "execution_state":"SEALED",
        "question_status":"CLOSED", "result":"STATIC_AUTHORITY_IMPACT_TOPOLOGY_SEALED", "disposition":"NONE",
        "question":"For each static observer intervention closure, which previously earned authority layers remain applicable and which require requalification?",
        "doctrine":"A small source deformation does not imply a small qualification debt.",
        "scope":"STATIC_LINEAGE_ONLY", "dynamic_dof_authority":"NONE", "market_or_outcome_access":"FORBIDDEN",
        "classification_note":"Authority impact is conservative static topology, not execution of a counterfactual observer and not a numeric distance. Renderer-only status preserves G0-G3 under explicit restriction; semantic/domain changes reopen the named descendant cone."
    });
    let cones = json!({"schema":"OTO_A_AUTHORITY_CONES_V1", "intervention_count":58, "earliest_invalidated_gate_counts":counts, "gate_requalification_counts":gate_counts, "runtime_evidence_required_count":runtime_count, "numeric_authority_distance":"NOT_CLAIMED", "independent_dimension_count":"NOT_EVALUABLE"});
    let matrix = json!({"schema":"OTO_A_ROOT_INVALIDATION_MATRIX_V1", "rows":impacts.iter().map(|item| json!({"intervention_id":item["intervention_id"],"earliest_invalidated_gate":item["earliest_invalidated_gate"],"roots_invalidated":item["roots_invalidated"],"roots_preserved":item["roots_preserved"]})).collect::<Vec<_>>()});
    let access = json!({
        "schema":"OTO_A_ACCESS_AUDIT_V1", "static_receipts_read":true, "intervention_metadata_read":true, "closure_metadata_read":true,
        "source_artifact_bytes_read":0, "market_rows_read":0, "outcome_rows_read":0, "indicator_buffers_read":0, "runtime_probe_executed":false,
        "d_b_reads":0, "d_c_reads":0, "d_d_reads":0, "trading_com_editor_invoked":false, "dynamic_dof_authority":"NONE"
    });
    let findings = json!({
        "schema":"OTO_A_TYPED_FINDINGS_V1", "execution_state":"SEALED", "question_status":"CLOSED", "result":"STATIC_AUTHORITY_IMPACT_TOPOLOGY_SEALED",
        "status":"QUALIFIED_STATIC_TOPOLOGY", "intervention_count":58, "earliest_invalidated_gate_counts":counts,
        "runtime_evidence_required_count":runtime_count, "independent_dimension_count":"NOT_EVALUABLE", "counterfactual_execution":"NONE",
        "maximum_authority":"OBS_OPEN_OTO_STATIC_AUTHORITY_IMPACT_TOPOLOGY_V1"
    });
    let artifact_values = [
        ("OTO_A_PROTOCOL_V1.json", protocol),
        (
            "OTO_A_AUTHORITY_IMPACT_REGISTRY.json",
            json!({"schema":"OTO_A_AUTHORITY_IMPACT_REGISTRY_V1","records":impacts}),
        ),
        ("OTO_A_AUTHORITY_CONES.json", cones),
        ("OTO_A_ROOT_INVALIDATION_MATRIX.json", matrix),
        ("OTO_A_ACCESS_AUDIT.json", access),
        ("OTO_A_TYPED_FINDINGS.json", findings),
    ];
    for (name, value) in &artifact_values {
        write_json(&out.join(name), value)?;
    }
    let entries = artifact_values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({"schema":"OTO_A_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_OTO_STATIC_AUTHORITY_IMPACT_TOPOLOGY_V1","parent_static_root":STATIC_ROOT,"parent_topology_root":TOPOLOGY_ROOT,"parent_intervention_root":INTERVENTION_ROOT,"parent_closure_root":CLOSURE_ROOT,"runtime_transport_root":RUNTIME_ROOT,"intervention_count":58,"artifacts":entries});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_A_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_A_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
    )?;
    Ok(root)
}

pub fn finalize(a: &Path, b: &Path, seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if seal.exists() {
        return Err(format!("seal destination already exists: {}", seal.display()).into());
    }
    let af = files(a)?;
    let bf = files(b)?;
    let an = af
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    let bn = bf
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    if an != bn {
        return Err("independent build artifact names differ".into());
    }
    for (left, right) in af.iter().zip(&bf) {
        if fs::read(left)? != fs::read(right)? {
            return Err(format!("independent build mismatch: {}", left.display()).into());
        }
    }
    fs::create_dir_all(seal)?;
    for path in &af {
        fs::copy(path, seal.join(path.file_name().unwrap()))?;
    }
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_A_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_A_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_A_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_A_ROOT_RECEIPT.json"))?)?;
    let payload = receipt
        .get("payload")
        .ok_or("root receipt missing payload")?;
    let expected = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    if sha256_bytes(&serde_json::to_vec(payload)?) != expected {
        return Err("logical root mismatch".into());
    }
    let mut names = HashSet::new();
    for artifact in payload["artifacts"]
        .as_array()
        .ok_or("artifacts not array")?
    {
        let name = artifact["path"].as_str().ok_or("artifact path missing")?;
        if !names.insert(name) {
            return Err(format!("duplicate artifact: {name}").into());
        }
        let bytes = fs::read(seal.join(name))?;
        if bytes.len() != artifact["bytes"].as_u64().ok_or("artifact bytes missing")? as usize {
            return Err(format!("artifact size mismatch: {name}").into());
        }
        if sha256_bytes(&bytes) != artifact["sha256"].as_str().ok_or("artifact hash missing")? {
            return Err(format!("artifact hash mismatch: {name}").into());
        }
    }
    Ok(expected.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_are_sha256() {
        for value in [
            STATIC_ROOT,
            TOPOLOGY_ROOT,
            INTERVENTION_ROOT,
            CLOSURE_ROOT,
            RUNTIME_ROOT,
        ] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}

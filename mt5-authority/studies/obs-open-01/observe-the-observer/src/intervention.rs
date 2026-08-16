use crate::source::sha256_bytes;
use hashbrown::{HashMap, HashSet};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const TOPOLOGY_ROOT: &str = "5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1";
const RT_ROOT: &str = "03fc7cc1733f771573637329d9236ba548c1d29b69846cee655b84b746ed9901";
const GRAPH_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/dof-topology/seal/OTO_DOF_DEPENDENCY_GRAPH.json";
const FINDINGS_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/dof-topology/seal/OTO_DOF_TOPOLOGY_FINDINGS.json";
const EXPLICIT_REL: &str =
    "mt5-authority/studies/obs-open-01/observe-the-observer/seal/EXPLICIT_DOF_CENSUS.json";
const AMBIENT_REL: &str =
    "mt5-authority/studies/obs-open-01/observe-the-observer/seal/AMBIENT_COORDINATE_CENSUS.json";
const POLICY_REL: &str =
    "mt5-authority/studies/obs-open-01/observe-the-observer/seal/LATENT_POLICY_CENSUS.json";

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

fn strings(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|xs| {
            xs.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn node_status(class: &str, id: &str, effects: &[String]) -> (&'static str, bool, &'static str) {
    if class == "AMBIENT_COORDINATE" {
        if id == "PLATFORM_FLOATING_ARITHMETIC" {
            return (
                "NOT_SEPARABLY_INTERVENABLE",
                true,
                "PLATFORM_SEMANTICS_NOT_SOURCE_REWRITE",
            );
        }
        return ("CHANGES_INPUT_DOMAIN", true, "AMBIENT_CONTEXT_SUBSTITUTION");
    }
    if effects
        .iter()
        .all(|e| matches!(e.as_str(), "PUBLICATION_ONLY" | "RENDER_ONLY"))
    {
        return (
            "LOCALLY_REWRITEABLE",
            false,
            "PUBLICATION_ONLY_OR_RENDERING_ONLY",
        );
    }
    if effects.iter().any(|e| {
        matches!(
            e.as_str(),
            "CHANGES_TIME_SEMANTICS"
                | "CHANGES_ADMISSION"
                | "AVAILABLE_DOMAIN"
                | "INITIALIZATION"
                | "CHANGES_STATE_TRANSITION"
                | "UPDATE_PATH_SELECTION"
                | "KNOWLEDGE_TIME"
                | "IDENTITY"
                | "SESSION_OWNERSHIP"
                | "MISSINGNESS_SEMANTICS"
                | "NUMERICAL_SEMANTICS"
        )
    }) {
        return (
            "REQUIRES_COUPLED_REWRITE",
            true,
            "SEMANTIC_EFFECT_CROSSES_STATE_OR_DOMAIN_BOUNDARY",
        );
    }
    (
        "UNKNOWN",
        true,
        "NO_STATIC_INTERVENTION_CLASSIFICATION_RULE",
    )
}

fn preservation_requirements(effects: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for effect in effects {
        let node = match effect.as_str() {
            "CHANGES_TIME_SEMANTICS" | "SESSION_OWNERSHIP" => "ARCH_CLOCK_SESSION_RESOLUTION",
            "INITIALIZATION" => "ARCH_INITIALIZATION",
            "AVAILABLE_DOMAIN" | "CHANGES_ADMISSION" => "ARCH_COVERAGE_EVALUABILITY",
            "CHANGES_STATE_TRANSITION" => "ARCH_CAUSAL_STATE_TRANSITION",
            "KNOWLEDGE_TIME" => "ARCH_PROVISIONAL_COMMITTED_SPLIT",
            "IDENTITY" => "ARCH_CANDIDATE_GENEALOGY",
            "UPDATE_PATH_SELECTION" => "ARCH_UPDATE_DISPATCH",
            "PUBLICATION" | "PUBLICATION_ONLY" | "RENDER_ONLY" => "ARCH_PUBLICATION_SURFACE",
            "NUMERICAL_SEMANTICS" => "ARCH_NUMERICAL_DOMAIN",
            _ => continue,
        };
        if !out.iter().any(|x| x == node) {
            out.push(node.to_owned());
        }
    }
    out.sort();
    out
}

fn intervention_record(
    node: &Value,
    incoming: &HashMap<String, Vec<String>>,
    outgoing: &HashMap<String, Vec<String>>,
    policy_map: &HashMap<String, Value>,
    ambient_map: &HashMap<String, Value>,
    group_map: &HashMap<String, String>,
) -> Value {
    let id = node["id"].as_str().unwrap_or("UNKNOWN");
    let class = node["class"].as_str().unwrap_or("UNKNOWN");
    let effects = strings(node, "effects");
    let (status, runtime_required, status_basis) = node_status(class, id, &effects);
    let current_rule = if let Some(policy) = policy_map.get(id) {
        policy["choice"]
            .as_str()
            .unwrap_or("HARD_CODED_CHOICE")
            .to_owned()
    } else if let Some(coord) = ambient_map.get(id) {
        coord["role"]
            .as_str()
            .unwrap_or("AMBIENT_COORDINATE")
            .to_owned()
    } else {
        let group = node["group"].as_str().unwrap_or("UNGROUPED");
        format!("DECLARED_INPUT:{group}")
    };
    let counterfactual = match class {
        "EXPLICIT_INPUT" => "INPUT_VALUE_OR_VALIDATION_RULE_REPLACEMENT",
        "LATENT_POLICY" => "POLICY_BRANCH_OR_UPDATE_RULE_REWRITE",
        "AMBIENT_COORDINATE" => "AMBIENT_CONTEXT_SUBSTITUTION_OR_PLATFORM_SEAM_REPLACEMENT",
        _ => "UNSUPPORTED_NODE_CLASS",
    };
    let mut siblings = Vec::new();
    if let Some(group) = group_map.get(id) {
        for (other, other_group) in group_map {
            if other != id && other_group == group {
                siblings.push(other.clone());
            }
        }
    }
    siblings.sort();
    let mut downstream = outgoing.get(id).cloned().unwrap_or_default();
    downstream.sort();
    downstream.dedup();
    let mut deps = incoming.get(id).cloned().unwrap_or_default();
    deps.sort();
    deps.dedup();
    let preservation = preservation_requirements(&effects);
    let side_effects = if preservation.is_empty() {
        vec!["NO_SEMANTIC_CONE_IDENTIFIED".to_owned()]
    } else {
        vec![
            "MAY_CHANGE_PROTECTED_SURFACE".to_owned(),
            "PRESERVATION_NOT_PROVEN".to_owned(),
        ]
    };
    let source_location = policy_map.get(id).and_then(|p| p.get("evidence")).cloned()
        .or_else(|| ambient_map.get(id).and_then(|p| p.get("evidence")).cloned())
        .unwrap_or_else(|| {
            if class == "AMBIENT_COORDINATE" {
                json!({"source": AMBIENT_REL, "line": Value::Null, "authority": "AMBIENT_COORDINATE_CENSUS"})
            } else {
                json!({"source": EXPLICIT_REL, "line": Value::Null, "authority": "EXPLICIT_DOF_CENSUS"})
            }
        });
    json!({
        "intervention_id": format!("OTO_I_{id}"),
        "target_joint": id,
        "target_class": class,
        "target_group": node.get("group").and_then(Value::as_str).unwrap_or("UNGROUPED"),
        "source_location": source_location,
        "current_rule": current_rule,
        "counterfactual_rule_schema": counterfactual,
        "direct_dependencies": deps,
        "downstream_affected_nodes": downstream,
        "required_covariations": siblings,
        "covariation_status": if siblings.is_empty() { "NONE_IDENTIFIED" } else { "REVIEW_REQUIRED_NOT_PROVEN" },
        "preservation_requirements": preservation,
        "invalidating_side_effects": side_effects,
        "intervention_status": status,
        "status_basis": status_basis,
        "semantic_effect_authority": "SOURCE_DEPENDENCY_TOPOLOGY_ONLY",
        "runtime_confirmation_required": runtime_required,
        "dynamic_dof_authority": "NONE",
        "independent_dimension_claim": "NOT_EARNED"
    })
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
    let graph = read_json(repo, GRAPH_REL)?;
    let findings = read_json(repo, FINDINGS_REL)?;
    let explicit = read_json(repo, EXPLICIT_REL)?;
    let ambient = read_json(repo, AMBIENT_REL)?;
    let policies = read_json(repo, POLICY_REL)?;
    let nodes = graph["nodes"].as_array().ok_or("graph nodes missing")?;
    let edges = graph["edges"].as_array().ok_or("graph edges missing")?;
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        let source = edge["source"].as_str().unwrap_or("UNKNOWN").to_owned();
        let target = edge["target"].as_str().unwrap_or("UNKNOWN").to_owned();
        outgoing
            .entry(source.clone())
            .or_default()
            .push(target.clone());
        incoming.entry(target).or_default().push(source);
    }
    let mut policy_map = HashMap::new();
    for policy in policies["policies"].as_array().ok_or("policies missing")? {
        if let Some(id) = policy["id"].as_str() {
            policy_map.insert(id.to_owned(), policy.clone());
        }
    }
    let mut ambient_map = HashMap::new();
    for coord in ambient["coordinates"]
        .as_array()
        .ok_or("coordinates missing")?
    {
        if let Some(id) = coord["id"].as_str() {
            ambient_map.insert(id.to_owned(), coord.clone());
        }
    }
    let mut group_map = HashMap::new();
    for group in explicit["groups"].as_array().ok_or("groups missing")? {
        let group_id = group["id"].as_str().unwrap_or("UNGROUPED");
        for field in group["fields"].as_array().unwrap_or(&Vec::new()) {
            if let Some(id) = field.as_str() {
                group_map.insert(id.to_owned(), group_id.to_owned());
            }
        }
    }
    let joint_nodes: Vec<&Value> = nodes
        .iter()
        .filter(|n| {
            matches!(
                n["class"].as_str(),
                Some("EXPLICIT_INPUT" | "LATENT_POLICY" | "AMBIENT_COORDINATE")
            )
        })
        .collect();
    if joint_nodes.len() != 58 {
        return Err(format!("expected 58 identified joints, found {}", joint_nodes.len()).into());
    }
    let records = joint_nodes
        .iter()
        .map(|n| {
            intervention_record(
                n,
                &incoming,
                &outgoing,
                &policy_map,
                &ambient_map,
                &group_map,
            )
        })
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::new();
    let mut effects = BTreeMap::new();
    let empty_values: Vec<Value> = Vec::new();
    for record in &records {
        *counts
            .entry(record["intervention_status"].as_str().unwrap_or("UNKNOWN"))
            .or_insert(0usize) += 1;
        for effect in record["preservation_requirements"]
            .as_array()
            .unwrap_or(&empty_values)
        {
            *effects
                .entry(effect.as_str().unwrap_or("UNKNOWN"))
                .or_insert(0usize) += 1;
        }
    }
    let artifacts: [(&str, Value); 6] = [
        (
            "OTO_I_PROTOCOL_V1.json",
            json!({
                "schema":"OTO_I_PROTOCOL_V1",
                "gate":"OTO-I_STATIC_INTERVENTION_TOPOLOGY",
                "question":"What source-level interventions can be described over the 58 identified joints without claiming independent dimensions or runtime behavior?",
                "authority":"STATIC_SOURCE_DEPENDENCY_ONLY",
                "dynamic_dof_authority":"NONE",
                "parent_topology_findings":findings,
                "rules":["CLASSIFY_INTERVENTION_NOT_IMPORTANCE","DO_NOT_CLAIM_INDEPENDENT_DIMENSIONS","DO_NOT_EXECUTE_RUNTIME","DO_NOT_READ_MARKET_OR_OUTCOME_ROWS","PRESERVE_COUPLING_AND_INVALIDATING_SIDE_EFFECTS"]
            }),
        ),
        (
            "OTO_I_INTERVENTION_REGISTRY.json",
            json!({"schema":"OTO_I_INTERVENTION_REGISTRY_V1","identified_joint_count":58,"records":records}),
        ),
        (
            "OTO_I_DEPENDENCY_CONES.json",
            json!({"schema":"OTO_I_DEPENDENCY_CONES_V1","graph_root":TOPOLOGY_ROOT,"nodes":nodes,"edges":edges,"intervention_status_counts":counts,"preservation_requirement_counts":effects}),
        ),
        (
            "OTO_I_PRESERVATION_MATRIX.json",
            json!({"schema":"OTO_I_PRESERVATION_MATRIX_V1","preservation_surface":"CAUSAL_OBSERVER_BEHAVIOR","architecture_nodes":["ARCH_CLOCK_SESSION_RESOLUTION","ARCH_INITIALIZATION","ARCH_COVERAGE_EVALUABILITY","ARCH_CAUSAL_STATE_TRANSITION","ARCH_PROVISIONAL_COMMITTED_SPLIT","ARCH_CANDIDATE_GENEALOGY","ARCH_UPDATE_DISPATCH","ARCH_NUMERICAL_DOMAIN","ARCH_PUBLICATION_SURFACE"],"matrix_authority":"SOURCE_DEPENDENCY_TOPOLOGY_ONLY","records":records.iter().map(|r| json!({"intervention_id":r["intervention_id"],"target_joint":r["target_joint"],"preservation_requirements":r["preservation_requirements"],"invalidating_side_effects":r["invalidating_side_effects"]})).collect::<Vec<_>>()}),
        ),
        (
            "OTO_I_ACCESS_AUDIT.json",
            json!({"schema":"OTO_I_ACCESS_AUDIT_V1","static_receipts_read":true,"source_census_metadata_read":true,"source_artifact_bytes_read":0,"market_rows_read":0,"outcome_rows_read":0,"runtime_probe_executed":false,"indicator_buffers_read":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"dynamic_dof_authority":"NONE","runtime_transport_root":RT_ROOT}),
        ),
        (
            "OTO_I_TYPED_FINDINGS.json",
            json!({"schema":"OTO_I_TYPED_FINDINGS_V1","execution_state":"SEALED","question_status":"CLOSED","result":"STATIC_INTERVENTION_TOPOLOGY_SEALED","identified_joint_count":58,"independent_dimension_count":"NOT_EVALUABLE","lawful_isolated_intervention_count":"NOT_EVALUABLE","runtime_confirmation_count":records.iter().filter(|r| r["runtime_confirmation_required"] == true).count(),"dynamic_dof_authority":"NONE","market_or_outcome_rows_read":0,"maximum_authority":"OBS_OPEN_OTO_STATIC_INTERVENTION_TOPOLOGY_V1"}),
        ),
    ];
    for (name, value) in &artifacts {
        write_json(&out.join(name), value)?;
    }
    let entries = artifacts
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({"schema":"OTO_I_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_OTO_STATIC_INTERVENTION_TOPOLOGY_V1","parent_static_root":STATIC_ROOT,"parent_topology_root":TOPOLOGY_ROOT,"runtime_transport_root":RT_ROOT,"identified_joint_count":58,"artifacts":entries});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_I_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_I_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
    )?;
    Ok(root)
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(dir)?
        .map(|e| e.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|p| p.is_file());
    paths.sort();
    Ok(paths)
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
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_I_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_I_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_I_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_I_ROOT_RECEIPT.json"))?)?;
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
        if bytes.len() as u64 != artifact["bytes"].as_u64().ok_or("artifact bytes missing")? {
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
        for value in [STATIC_ROOT, TOPOLOGY_ROOT, RT_ROOT] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}

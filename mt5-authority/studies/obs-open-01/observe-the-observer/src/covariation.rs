use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const INTERVENTION_ROOT: &str = "2fc9a4c150040b0f5167c341503583f1caeb298241eb1ec04b9614cc3837065f";
const TOPOLOGY_ROOT: &str = "5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1";
const RT_ROOT: &str = "03fc7cc1733f771573637329d9236ba548c1d29b69846cee655b84b746ed9901";
const REGISTRY_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/intervention-topology/seal/OTO_I_INTERVENTION_REGISTRY.json";
const FINDINGS_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/intervention-topology/seal/OTO_I_TYPED_FINDINGS.json";

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

fn array_strings(value: &Value, key: &str) -> Vec<String> {
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

fn edge_kind(target: &Value, _candidate: &str) -> &'static str {
    let id = target["target_joint"].as_str().unwrap_or_default();
    let group = target["target_group"].as_str().unwrap_or_default();
    if group == "CLOCK" {
        return "MUST_REPROVE_WITH";
    }
    if id == "InpEnableExtremeSentinels" {
        return "MUST_REPROVE_WITH";
    }
    if group == "COVERAGE" {
        return "DOMAIN_COUPLING";
    }
    if id == "LEGACY_REPAINT_EVERY_TICK" || id == "CAUSAL_REBUILD_ON_NEW_BAR" {
        return "MUST_REPROVE_WITH";
    }
    "NO_COVARIATION_PROVEN"
}

fn domain_class(record: &Value) -> &'static str {
    let class = record["target_class"].as_str().unwrap_or_default();
    let effects = array_strings(record, "preservation_requirements");
    if class == "AMBIENT_COORDINATE" {
        return "DOMAIN_DEFORMATION";
    }
    if effects.iter().any(|x| x == "ARCH_COVERAGE_EVALUABILITY") {
        return "MIXED_DOMAIN_AND_SEMANTIC";
    }
    "SEMANTIC_DEFORMATION_WITHIN_FIXED_DOMAIN"
}

fn closure_status(
    record: &Value,
    edge_kinds: &[&str],
    closure_size: usize,
) -> (&'static str, &'static str) {
    let status = record["intervention_status"].as_str().unwrap_or_default();
    if status == "NOT_SEPARABLY_INTERVENABLE" {
        return (
            "NOT_SEPARABLY_INTERVENABLE",
            "platform seam is not a source-level intervention",
        );
    }
    if status == "UNKNOWN" {
        return (
            "UNKNOWN",
            "predecessor intervention classification remains unknown",
        );
    }
    if edge_kinds.contains(&"DOMAIN_COUPLING") {
        return (
            "OPEN_REVIEW_REQUIRED",
            "domain coupling requires a separate common-domain contract",
        );
    }
    if status == "LOCALLY_REWRITEABLE" {
        return (
            "CLOSED_UNDER_EXPLICIT_RESTRICTION",
            "publication/rendering surface only; causal state held fixed",
        );
    }
    if edge_kinds.contains(&"MUST_REPROVE_WITH") && closure_size > 1 {
        return (
            "CLOSED_UNDER_EXPLICIT_RESTRICTION",
            "closure is a coupled package under an explicit preservation restriction",
        );
    }
    (
        "REQUIRES_RUNTIME_EVIDENCE",
        "source cone is identified but behavioral closure is not runtime-qualified",
    )
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
    let registry = read_json(repo, REGISTRY_REL)?;
    let findings = read_json(repo, FINDINGS_REL)?;
    let records = registry["records"]
        .as_array()
        .ok_or("intervention records missing")?;
    if records.len() != 58 {
        return Err(format!("expected 58 intervention records, found {}", records.len()).into());
    }
    let mut edges = Vec::new();
    let mut closures = Vec::new();
    let mut domains = Vec::new();
    let mut status_counts = BTreeMap::new();
    let mut edge_counts = BTreeMap::new();
    for record in records {
        let id = record["target_joint"].as_str().unwrap_or("UNKNOWN");
        let mut closure = vec![id.to_owned()];
        let mut kinds = Vec::new();
        for target in array_strings(record, "downstream_affected_nodes") {
            edges.push(json!({"source":id,"target":target,"kind":"AUTOMATIC_DOWNSTREAM_EFFECT","authority":"OTO_I_TOPOLOGY","claim":"value/rule change propagates to this node; rewrite requirement not implied"}));
            *edge_counts
                .entry("AUTOMATIC_DOWNSTREAM_EFFECT")
                .or_insert(0usize) += 1;
        }
        for candidate in array_strings(record, "required_covariations") {
            let kind = edge_kind(record, &candidate);
            kinds.push(kind);
            if matches!(kind, "MUST_REPROVE_WITH" | "DOMAIN_COUPLING") {
                closure.push(candidate.clone());
            }
            edges.push(json!({"source":id,"target":candidate,"kind":kind,"authority":if kind == "NO_COVARIATION_PROVEN" {"NOT_PROVEN"} else {"STATIC_REVIEW_RULE"},"claim":if kind == "MUST_REPROVE_WITH" {"preservation must be re-proven jointly under the declared restriction"} else if kind == "DOMAIN_COUPLING" {"common-domain analysis is required"} else {"candidate relationship only; no required covariation proven"}}));
            *edge_counts.entry(kind).or_insert(0usize) += 1;
        }
        closure.sort();
        closure.dedup();
        let (status, reason) = closure_status(record, &kinds, closure.len());
        *status_counts.entry(status).or_insert(0usize) += 1;
        closures.push(json!({
            "intervention_id":record["intervention_id"],
            "seed":id,
            "closure_set":closure,
            "closure_iterations":if kinds.iter().any(|k| *k == "MUST_REPROVE_WITH" || *k == "DOMAIN_COUPLING") { 1 } else { 0 },
            "closure_status":status,
            "closure_reason":reason,
            "minimality_claim":"NONE",
            "independent_dimension_claim":"NOT_EARNED"
        }));
        let domain = domain_class(record);
        domains.push(json!({
            "intervention_id":record["intervention_id"],
            "target_joint":id,
            "deformation_class":domain,
            "domain_before":"G0_G1_AUTHORIZED_OBSERVER_INPUT_CONFIGURATION_DOMAIN",
            "domain_after":if domain == "SEMANTIC_DEFORMATION_WITHIN_FIXED_DOMAIN" {"COMMON_AUTHORIZED_DOMAIN"} else {"COUNTERFACTUAL_DOMAIN_CONTRACT_REQUIRED"},
            "common_domain":"NOT_EVALUABLE",
            "behavior_on_common_domain":"NOT_EVALUABLE_WITHOUT_RUNTIME_OR_FORMAL_KERNEL_PROOF",
            "newly_admitted_region":if domain == "SEMANTIC_DEFORMATION_WITHIN_FIXED_DOMAIN" {"NONE_IDENTIFIED"} else {"NOT_EVALUABLE"},
            "newly_excluded_region":if domain == "SEMANTIC_DEFORMATION_WITHIN_FIXED_DOMAIN" {"NONE_IDENTIFIED"} else {"NOT_EVALUABLE"},
            "authority":"STATIC_SCHEMA_AND_DEPENDENCY_ONLY"
        }));
    }
    let artifacts: [(&str, Value); 7] = [
        (
            "OTO_C_PROTOCOL_V1.json",
            json!({
                "schema":"OTO_C_PROTOCOL_V1",
                "gate":"OTO-C_COVARIATION_CLOSURE_QUALIFICATION",
                "question":"When one manufactured choice is altered, what else must change or be re-proven for the resulting observer to remain coherent?",
                "closure_rule":"S_0={j}; S_(n+1)=S_n union RequiredCovariates(S_n) until fixed point or typed failure",
                "edge_kinds":["MUST_REWRITE_WITH","MUST_REPROVE_WITH","AUTOMATIC_DOWNSTREAM_EFFECT","DOMAIN_COUPLING","INITIALIZATION_COUPLING","PUBLICATION_COUPLING","AUTHORITY_COUPLING","OPTIONAL_COORDINATED_REWRITE","INVALIDATES_IF_UNCHANGED","NO_COVARIATION_PROVEN","UNKNOWN"],
                "prohibitions":["NO_MINIMALITY_CLAIM","NO_INDEPENDENT_DIMENSION_CLAIM","NO_RUNTIME_EXECUTION","NO_MARKET_OR_OUTCOME_READS"]
            }),
        ),
        (
            "OTO_C_COVARIATION_EDGE_REGISTRY.json",
            json!({"schema":"OTO_C_COVARIATION_EDGE_REGISTRY_V1","edges":edges,"edge_counts":edge_counts}),
        ),
        (
            "OTO_C_CLOSURE_REGISTRY.json",
            json!({"schema":"OTO_C_CLOSURE_REGISTRY_V1","seed_count":58,"closures":closures,"status_counts":status_counts}),
        ),
        (
            "OTO_C_DOMAIN_DEFORMATION_ATLAS.json",
            json!({"schema":"OTO_C_DOMAIN_DEFORMATION_ATLAS_V1","records":domains,"semantic_vs_domain_authority":"STATIC_CLASSIFICATION_ONLY"}),
        ),
        (
            "OTO_C_PRESERVATION_CONFLICTS.json",
            json!({"schema":"OTO_C_PRESERVATION_CONFLICTS_V1","contradictory_preservation_requirements":[],"unresolved_seams":["common_domain_behavior","runtime-transport-not-evaluable","platform-floating-arithmetic"],"conflict_status":"NO_CONTRADICTION_PROVEN"}),
        ),
        (
            "OTO_C_ACCESS_AUDIT.json",
            json!({"schema":"OTO_C_ACCESS_AUDIT_V1","intervention_root":INTERVENTION_ROOT,"runtime_transport_root":RT_ROOT,"static_receipts_read":true,"market_rows_read":0,"outcome_rows_read":0,"indicator_buffers_read":0,"runtime_probe_executed":false,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"dynamic_dof_authority":"NONE"}),
        ),
        (
            "OTO_C_TYPED_FINDINGS.json",
            json!({"schema":"OTO_C_TYPED_FINDINGS_V1","execution_state":"SEALED","question_status":"CLOSED","result":"STATIC_COVARIATION_CLOSURE_SEALED","seed_count":58,"status_counts":status_counts,"contradictory_preservation_requirements":false,"independent_dimension_count":"NOT_EVALUABLE","lawful_intervention_count":"NOT_EVALUABLE","dynamic_dof_authority":"NONE","market_or_outcome_rows_read":0,"parent_intervention_findings":findings,"maximum_authority":"OBS_OPEN_OTO_STATIC_COVARIATION_CLOSURE_V1"}),
        ),
    ];
    for (name, value) in &artifacts {
        write_json(&out.join(name), value)?;
    }
    let entries = artifacts
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({"schema":"OTO_C_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_OTO_STATIC_COVARIATION_CLOSURE_V1","parent_static_root":STATIC_ROOT,"parent_intervention_root":INTERVENTION_ROOT,"parent_topology_root":TOPOLOGY_ROOT,"runtime_transport_root":RT_ROOT,"seed_count":58,"artifacts":entries});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_C_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_C_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_C_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_C_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_C_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_C_ROOT_RECEIPT.json"))?)?;
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
        for value in [STATIC_ROOT, INTERVENTION_ROOT, TOPOLOGY_ROOT, RT_ROOT] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}

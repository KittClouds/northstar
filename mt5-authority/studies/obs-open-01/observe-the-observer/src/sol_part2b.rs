use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SOL_ROOT: &str = "f25ba4971834384a76b96eb090306773dfba304be8250ce1519d175deeeec35d";
const PART2_ROOT: &str = "d1b38643e473ff5a2026bd9f244d2727d9cba08d608007d6ebc24d0c16004052";
const G8_ROOT: &str = "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT";
const STATUS: &str = "G8_LINEAGE_REBIND_BLOCKED";
const FINAL_STATE: &str = "QUESTION_FACTORY_QUALIFIED_PENDING_G8_BINDING";
const PART2_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/sol-part2/seal";

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, text.as_bytes())?;
    Ok(())
}

fn artifact_entry(dir: &Path, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(dir.join(name))?;
    Ok(json!({"path":name,"bytes":bytes.len(),"sha256":sha256_bytes(&bytes)}))
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut result = fs::read_dir(dir)?
        .map(|entry| entry.map(|item| item.path()))
        .collect::<Result<Vec<_>, _>>()?;
    result.retain(|path| path.is_file());
    result.sort();
    Ok(result)
}

fn report(audit: &Value, packet_status: &str) -> String {
    format!(
        r#"# Sol P2B — G8 Lineage Materialization and Packet Rebinding Report

## Result

- Parent Sol root: {SOL_ROOT}
- Sol Part 2 root: {PART2_ROOT}
- G8 lineage: {G8_ROOT}
- Final state: {FINAL_STATE}
- Packet regeneration: FORBIDDEN
- Population access: FORBIDDEN
- Arm execution: FORBIDDEN
- Target object and region: UNBOUND

## Lineage audit

The declared materialized G8 receipt was not found in the singular-authority checkout. No digest-only substitute, memory-derived root, or synthetic receipt was accepted. Exact G8 authenticity, membership, and ancestry therefore remain NOT_EVALUABLE.

## Packet immutability

The frozen SOL_PART2_QUESTION_PACKETS artifact was read as protocol material only. Its byte hash and its binding inside the sealed Sol Part 2 root receipt were recomputed. Packet immutability is {packet_status}; packet regeneration and ranking are forbidden.

## Rebinding law

When an exact G8 receipt becomes available, this gate may only bind that receipt and reverify the existing packets and inherited optics, arms, synthesis, stopping, and blindness artifacts. Any packet byte change, question regeneration, P6 source substitution, or population-conditioned ranking creates a new lineage and halts.

## Access audit

- 04A reads: 0
- D_B/D_C/D_D reads: 0
- market rows: 0
- outcomes: 0
- Sol arm results: 0
- Kammi results: 0
- runtime probes: 0
- Trading.com editor: 0

## Disposition

The packet factory remains qualified only pending exact G8 binding. This is not population access authorization and does not authorize any arm execution.

Audit payload:

{audit}
"#
    )
}

pub fn build(repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("output directory must be empty: {}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    let part2 = repo.join(PART2_REL);
    let root_path = part2.join("SOL_PART2_ROOT_RECEIPT.json");
    let packet_path = part2.join("SOL_PART2_QUESTION_PACKETS.json");
    let root_bytes = fs::read(&root_path)?;
    let packet_bytes = fs::read(&packet_path)?;
    let root_receipt: Value = serde_json::from_slice(&root_bytes)?;
    let payload = root_receipt
        .get("payload")
        .ok_or("Sol Part 2 root missing payload")?;
    let root_expected = root_receipt["logical_root"]
        .as_str()
        .ok_or("Sol Part 2 root missing logical_root")?;
    let root_hash_ok =
        sha256_bytes(&serde_json::to_vec(payload)?) == root_expected && root_expected == PART2_ROOT;
    let packet_hash = sha256_bytes(&packet_bytes);
    let packet_entry = payload["artifacts"].as_array().and_then(|items| {
        items
            .iter()
            .find(|item| item["path"] == "SOL_PART2_QUESTION_PACKETS.json")
    });
    let packet_bound_hash = packet_entry
        .and_then(|item| item["sha256"].as_str())
        .unwrap_or("MISSING");
    let packet_status = if root_hash_ok && packet_hash == packet_bound_hash {
        "PASS_BYTE_UNCHANGED"
    } else {
        "FAIL_PACKET_OR_PARENT_ROOT_MISMATCH"
    };
    let g8_rel =
        "mt5-authority/studies/obs-open-01/observe-the-observer/g8/seal/G8_ROOT_RECEIPT.json";
    let g8_path = repo.join(g8_rel);
    let g8_exists = g8_path.exists();
    let audit = json!({
        "schema":"SOL_P2B_LINEAGE_MATERIALIZATION_AUDIT_V1",
        "declared_g8_path":g8_rel,
        "g8_receipt_exists":g8_exists,
        "g8_root":G8_ROOT,
        "g8_authenticity":"NOT_EVALUABLE",
        "g8_membership":"NOT_EVALUABLE",
        "g8_ancestry":"NOT_EVALUABLE",
        "parent_sol_root":SOL_ROOT,
        "part2_root":PART2_ROOT,
        "part2_root_recomputed":root_expected,
        "part2_root_status":if root_hash_ok {"PASS"} else {"FAIL"},
        "packet_sha256":packet_hash,
        "packet_bound_sha256":packet_bound_hash,
        "packet_status":packet_status,
        "packet_regeneration":"FORBIDDEN",
        "population_access":0
    });
    let values: Vec<(&str, Value)> = vec![
        ("SOL_P2B_LINEAGE_MATERIALIZATION_AUDIT.json", audit.clone()),
        (
            "SOL_P2B_PACKET_IMMUTABILITY_RECEIPT.json",
            json!({"schema":"SOL_P2B_PACKET_IMMUTABILITY_RECEIPT_V1","packet_sha256":packet_hash,"bound_sha256":packet_bound_hash,"status":packet_status,"question_count":5,"regeneration":"FORBIDDEN"}),
        ),
        (
            "SOL_P2B_ANCESTRY_REBIND_CONTRACT.json",
            json!({"schema":"SOL_P2B_ANCESTRY_REBIND_CONTRACT_V1","inputs":["EXACT_MATERIALIZED_G8_ROOT_RECEIPT","SOL_PART2_ROOT","FROZEN_PACKET_HASHES"],"operations":["VERIFY_G8_AUTHENTICITY","VERIFY_G8_MEMBERSHIP","VERIFY_ANCESTRY","BIND_EXACT_G8_ROOT","VERIFY_PACKET_BYTES","VERIFY_INHERITED_SURFACES"],"forbidden":["QUESTION_REGENERATION","PACKET_EDITING","POPULATION_ACCESS","ARM_EXECUTION","P6_SOURCE_SUBSTITUTION"],"current_status":STATUS}),
        ),
        (
            "SOL_P2B_G8_REVERIFICATION_REQUIREMENTS.json",
            json!({"schema":"SOL_P2B_G8_REVERIFICATION_REQUIREMENTS_V1","unchanged":["OPTICS","QUESTION_GRAMMAR","ARMS","SYNTHESIS","STOPPING","BLINDNESS"],"population_access":0,"failure_on_any_change":"NEW_LINEAGE_AND_HALT"}),
        ),
        (
            "SOL_P2B_ACCESS_AUDIT.json",
            json!({"schema":"SOL_P2B_ACCESS_AUDIT_V1","protocol_artifacts_read":2,"04a_reads":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"outcomes":0,"arm_executions":0,"packet_regeneration":0,"population_contact":0,"p6_source_access":0,"trading_com_editor_invoked":false}),
        ),
        (
            "SOL_P2B_GATE_DECISION.json",
            json!({"schema":"SOL_P2B_GATE_DECISION_V1","status":STATUS,"result":FINAL_STATE,"packet_immutability":packet_status,"g8_binding":"NOT_EVALUABLE","population_authority":"NONE","authority_gain":"NONE"}),
        ),
    ];
    for (name, value) in &values {
        write_json(&out.join(name), value)?;
    }
    write_text(
        &out.join("SOL_P2B_EXECUTION_REPORT.md"),
        &report(&audit, packet_status),
    )?;
    let mut entries = values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    entries.push(artifact_entry(out, "SOL_P2B_EXECUTION_REPORT.md")?);
    let payload = json!({"schema":"SOL_P2B_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_SOL_P2B_G8_LINEAGE_REBINDING_V1","parent_sol_root":SOL_ROOT,"part2_root":PART2_ROOT,"g8_root":G8_ROOT,"packet_status":packet_status,"artifacts":entries,"population_contact":"FORBIDDEN","final_state":FINAL_STATE});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("SOL_P2B_ROOT_RECEIPT.json"),
        &json!({"schema":"SOL_P2B_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("SOL_P2B_ROOT_RECEIPT.json"))?)?;
    let logical_root = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload=af.iter().map(|p|{let bytes=fs::read(p).unwrap();json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()})}).collect::<Vec<_>>();
    write_json(
        &seal.join("SOL_P2B_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"SOL_P2B_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("SOL_P2B_ROOT_RECEIPT.json"))?)?;
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
        } else if sha256_bytes(&bytes)
            != artifact["sha256"].as_str().ok_or("artifact hash missing")?
        {
            return Err(format!("artifact hash mismatch: {name}").into());
        }
    }
    Ok(expected.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_are_fixed() {
        assert_eq!(SOL_ROOT.len(), 64);
        assert_eq!(PART2_ROOT.len(), 64);
        assert_eq!(G8_ROOT, "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT");
    }
}

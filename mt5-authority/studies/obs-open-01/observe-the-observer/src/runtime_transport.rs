use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const DOF_ROOT: &str = "5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1";
const INST01_ROOT: &str = "5930c7d4f5b2de4549bfedacbd2c3b9df7da78f68c7d765b9c0669d08a17490f";
const PARENT_ROOT: &str = "32e1207717afc29533e8a8fb0a7e0f329f9e092f58f3fb3c58d5661c5b7dd0fa";
const PARENT_BUILD: u64 = 6106;
const CURRENT_BUILD: u64 = 6116;
const FORBIDDEN_BUILD: u64 = 6094;

const LINEAGE_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/runtime-probe/RUNTIME_LINEAGE_AUDIT.json";
const QUARANTINE_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/runtime-probe/RUNTIME_TOOL_QUARANTINE.json";
const AUTHORITY_MANIFEST_REL: &str =
    "mt5-authority/studies/obs-open-01/instrument-qualification/seal/authority_manifest.json";
const CONTROLLED_AUDIT_REL: &str = "mt5-authority/studies/obs-open-01/instrument-qualification/receipts/controlled_fixture_audit.json";

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn read_json(repo: &Path, rel: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(repo.join(rel))?)?)
}

fn artifact_entry(dir: &Path, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(dir.join(name))?;
    Ok(json!({"path": name, "bytes": bytes.len(), "sha256": sha256_bytes(&bytes)}))
}

fn surface(
    id: &str,
    operations: &[&str],
    reference: &str,
    reference_strength: &str,
    reason: &str,
) -> Value {
    json!({
        "surface_id": id,
        "operations": operations,
        "reference_class": reference,
        "reference_strength": reference_strength,
        "6116_test_artifact": "NOT_AVAILABLE",
        "comparison_status": "NOT_EVALUABLE",
        "authority": "NONE",
        "reason": reason,
        "market_or_outcome_rows_read": 0
    })
}

fn scope() -> Value {
    json!({
        "schema": "OTO_RT_TRANSPORT_SCOPE_V1",
        "gate": "OBS_OPEN_OTO_6106_TO_6116_RUNTIME_TRANSPORT_V1",
        "question": "For the exact platform operations consumed by the admitted observer, does same-origin build 6116 preserve the sealed 6106 semantics needed for observer metrology?",
        "prohibited_claims": ["META_TRADER_6106_EQUIVALENT_TO_6116", "GENERAL_MT5_RUNTIME_EQUIVALENCE"],
        "origin": "MetaTrader-origin",
        "parent_build": PARENT_BUILD,
        "candidate_build": CURRENT_BUILD,
        "quarantined_build": FORBIDDEN_BUILD,
        "surfaces": [
            {"id":"RT-1", "name":"ENVIRONMENT_AND_CLOCK_SEMANTICS", "operations":["_Symbol","_Period","PeriodSeconds","_Digits","TimeCurrent","TimeToStruct","StructToTime","civil_day_construction","DayStart","PreviousDayStart","AtMinuteOfDay"], "semantic_scope":"observer-consumed clock and session ownership semantics"},
            {"id":"RT-2", "name":"SERIES_ADDRESSING_SEMANTICS", "operations":["Bars","CopyTime","CopyHigh","CopyLow","iTime","iHigh","iLow"], "semantic_scope":"resolved containing bar, boundaries, gaps and overnight transitions"},
            {"id":"RT-3", "name":"CUSTOM_INDICATOR_INSTANTIATION", "operations":["iCustom","BarsCalculated","CopyBuffer","IndicatorParameters","MqlParam"], "semantic_scope":"handle creation, buffer transfer and effective parameter introspection"},
            {"id":"RT-4", "name":"COMPLETED_KERNEL_REPLAY", "operations":["OnCalculate","legacy_buffers_0_35","sentinel_buffers_36_53","fixture_replay"], "semantic_scope":"completed-bar observer semantics on controlled fixtures"},
            {"id":"RT-5", "name":"FORMING_BAR_PROVISIONAL_SEMANTICS", "operations":["bar_zero","forming_high_low_close","provisional_sentinels","commit_transition"], "semantic_scope":"live/provisional versus committed causal knowledge"}
        ],
        "composition_rule":"Transport authority is compositional only where each consumed seam is qualified.",
        "reference_rule":"Downstream authority is capped by the weaker reference/test side.",
        "execution_boundary":"No market or outcome data; no parameter sweep; no observer source mutation."
    })
}

fn reference_manifest(
    repo: &Path,
    lineage: &Value,
    quarantine: &Value,
    authority: &Value,
    audit: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let capture_count = authority["captures"].as_array().map_or(0, Vec::len);
    Ok(json!({
        "schema":"OTO_RT_REFERENCE_AUTHORITY_MATRIX_V1",
        "parent_inst01_root": INST01_ROOT,
        "static_oto_root": STATIC_ROOT,
        "dof_topology_root": DOF_ROOT,
        "reference_classes": {
            "SEALED_6106_RUNTIME_OUTPUT":"not present for the transport comparison",
            "SEALED_6106_CAPTURE":"sealed controlled fixture metadata identifies 6106-origin capture authority; capture bytes are not read by this build",
            "SEALED_G0_G8_SEMANTIC_ARTIFACT":"static semantic lineage only; cannot substitute for a paired runtime observation",
            "SOURCE_DERIVED_EXPECTATION":"source expectations are non-empirical and cannot upgrade transport status",
            "SYNTHETIC_CONSTRUCTIVE_ORACLE":"permitted for comparator qualification only",
            "NOT_AVAILABLE":"no admissible paired evidence"
        },
        "metadata_only_inputs": {
            "lineage_sha256": sha256_bytes(&fs::read(repo.join(LINEAGE_REL))?),
            "quarantine_sha256": sha256_bytes(&fs::read(repo.join(QUARANTINE_REL))?),
            "authority_manifest_sha256": sha256_bytes(&fs::read(repo.join(AUTHORITY_MANIFEST_REL))?),
            "controlled_fixture_audit_sha256": sha256_bytes(&fs::read(repo.join(CONTROLLED_AUDIT_REL))?),
            "sealed_capture_metadata_count": capture_count,
            "sealed_6106_capture_metadata_consumed": true,
            "capture_rows_read": 0,
            "capture_bytes_read": 0
        },
        "lineage_summary": {
            "parent_runtime_origin": lineage["parent_runtime_origin"],
            "parent_build": PARENT_BUILD,
            "same_origin_candidate_build": CURRENT_BUILD,
            "exact_6106_runtime_found": lineage["exact_6106_runtime_found"],
            "alternate_6094_disposition": quarantine["status"],
            "trading_com_editor_authority":"NONE_QUARANTINED"
        },
        "authority_cap":"No source-derived expectation or sealed 6106 capture alone proves 6106-to-6116 transport. A paired 6116 result is required.",
        "audit_schema": audit["schema"]
    }))
}

fn access_audit() -> Value {
    json!({
        "schema":"OTO_RT_ACCESS_AUDIT_V1",
        "source_and_receipt_metadata_read": true,
        "market_or_outcome_rows_read": 0,
        "market_or_outcome_bytes_read": 0,
        "d_b_target_reads": 0,
        "d_c_reads": 0,
        "d_d_reads": 0,
        "indicator_output_buffers_read_during_this_build": 0,
        "runtime_probe_executed": false,
        "parameter_sweep_executed": false,
        "trading_com_editor_invoked": false,
        "trading_com_editor_path":"C:/Program Files/Trading.com Markets MT5/metaeditor64.exe",
        "trading_com_editor_status":"QUARANTINED_DO_NOT_INVOKE",
        "observer_source_mutated": false,
        "existing_terminal_function_modified": false
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
    let lineage = read_json(repo, LINEAGE_REL)?;
    let quarantine = read_json(repo, QUARANTINE_REL)?;
    let authority = read_json(repo, AUTHORITY_MANIFEST_REL)?;
    let audit = read_json(repo, CONTROLLED_AUDIT_REL)?;
    let artifacts: [(&str, Value); 6] = [
        ("OTO_RT_TRANSPORT_SCOPE_V1.json", scope()),
        (
            "OTO_RT_REFERENCE_AUTHORITY_MATRIX.json",
            reference_manifest(repo, &lineage, &quarantine, &authority, &audit)?,
        ),
        (
            "OTO_RT_CONTROLLED_FIXTURE_MANIFEST.json",
            json!({
                "schema":"OTO_RT_CONTROLLED_FIXTURE_MANIFEST_V1",
                "source_authority":"SEALED_G0_G8_SEMANTIC_ARTIFACT",
                "fixture_audit_schema":audit["schema"],
                "fixtures":[
                    {"id":"FIXTURE_FULL_M1","reference_class":"SEALED_6106_CAPTURE","capture_metadata_only":true,"capture_rows_read":0,"purpose":"completed-bar and sentinel invariant reference"},
                    {"id":"FIXTURE_GAP_M1","reference_class":"SEALED_6106_CAPTURE","capture_metadata_only":true,"capture_rows_read":0,"purpose":"coverage and fail-closed reference"},
                    {"id":"FIXTURE_FULL_M5","reference_class":"SEALED_6106_CAPTURE","capture_metadata_only":true,"capture_rows_read":0,"purpose":"completed-bar timeframe reference"},
                    {"id":"FIXTURE_GAP_M5","reference_class":"SEALED_6106_CAPTURE","capture_metadata_only":true,"capture_rows_read":0,"purpose":"M5 coverage reference"}
                ],
                "6116_fixture_execution":"NOT_PERFORMED",
                "comparison_authority":"NOT_EVALUABLE_WITHOUT_PAIRED_6116_ARTIFACT"
            }),
        ),
        (
            "OTO_RT_SURFACE_DECISIONS.json",
            json!({
                "schema":"OTO_RT_SURFACE_DECISIONS_V1",
                "surfaces":[
                    surface("RT-1", &["_Symbol","_Period","PeriodSeconds","_Digits","TimeCurrent","TimeToStruct","StructToTime","civil_day_construction"], "SEALED_G0_G8_SEMANTIC_ARTIFACT", "SOURCE_AND_STATIC_LINEAGE_ONLY", "No paired 6106/6116 clock fixture or runtime metadata comparison was executed."),
                    surface("RT-2", &["Bars","CopyTime","CopyHigh","CopyLow","iTime","iHigh","iLow"], "SEALED_6106_CAPTURE", "FIXTURE_METADATA_ONLY", "6106 capture metadata exists, but no 6116 series-addressing replay artifact is available."),
                    surface("RT-3", &["iCustom","BarsCalculated","CopyBuffer","IndicatorParameters","MqlParam"], "NOT_AVAILABLE", "NONE", "Effective 6116 parameter introspection and paired 6106 reference output are both unopened."),
                    surface("RT-4", &["OnCalculate","legacy_buffers_0_35","sentinel_buffers_36_53","fixture_replay"], "SEALED_6106_CAPTURE", "FIXTURE_METADATA_ONLY", "Sealed 6106 fixture receipts cannot be treated as a paired 6116 execution."),
                    surface("RT-5", &["bar_zero","forming_high_low_close","provisional_sentinels","commit_transition"], "SEALED_6106_CAPTURE", "FIXTURE_METADATA_ONLY", "No 6116 live/provisional controlled fixture was executed; INST-01 trigger limitations remain.")
                ],
                "top_level":"TRANSPORT_NOT_QUALIFIED",
                "dynamic_dof_authority":"NONE",
            "source_and_runtime_unmodified":true
            }),
        ),
        ("OTO_RT_ACCESS_AUDIT.json", access_audit()),
        (
            "OTO_RT_TYPED_FINDINGS.json",
            json!({
                "schema":"OTO_RT_TYPED_FINDINGS_V1",
                "execution_state":"SEALED",
                "question_status":"NOT_EVALUABLE",
                "surface_result":"TRANSPORT_NOT_QUALIFIED",
                "completed_bar_dynamic_dof_authority":"NONE",
                "live_forming_bar_dof_authority":"NONE",
                "effective_parameter_binding":"NOT_EVALUABLE",
                "general_mt5_runtime_equivalence":"NOT_CLAIMED",
                "trading_com_editor":"QUARANTINED",
                "disposition":"HALT_DYNAMIC_BRANCH_PENDING_PAIRED_6116_ARTIFACTS",
                "maximum_authority":"OBS_OPEN_OTO_6106_TO_6116_RUNTIME_TRANSPORT_V1_PREOPEN_AUDIT_ONLY"
            }),
        ),
    ];
    for (name, value) in &artifacts {
        write_json(&out.join(name), value)?;
    }
    let entries = artifacts
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({
        "schema":"OTO_RT_ROOT_PAYLOAD_V1",
        "authority":"OBS_OPEN_OTO_6106_TO_6116_RUNTIME_TRANSPORT_V1",
        "parent_static_root":STATIC_ROOT,
        "parent_dof_root":DOF_ROOT,
        "parent_inst01_root":INST01_ROOT,
        "parent_inst01_authority_root":PARENT_ROOT,
        "candidate":"MetaTrader-origin build 6116",
        "reference_build":PARENT_BUILD,
        "artifacts":entries
    });
    let logical_root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_RT_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_RT_ROOT_RECEIPT_V1","logical_root":logical_root,"payload":payload}),
    )?;
    Ok(logical_root)
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
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_RT_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_RT_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_RT_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_RT_ROOT_RECEIPT.json"))?)?;
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
    fn roots_and_builds_are_fixed() {
        for value in [STATIC_ROOT, DOF_ROOT, INST01_ROOT, PARENT_ROOT] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
        assert_eq!(PARENT_BUILD, 6106);
        assert_eq!(CURRENT_BUILD, 6116);
        assert_eq!(FORBIDDEN_BUILD, 6094);
    }
}

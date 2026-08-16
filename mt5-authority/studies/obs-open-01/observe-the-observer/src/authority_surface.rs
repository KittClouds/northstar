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
const CLOSURE_ROOT: &str = "b96ad31eb43f30ec5026a71a1d607d95e0041543f67418dd20180fabf3c7db71";
const AUTHORITY_IMPACT_ROOT: &str =
    "a6a8dc1b01dcb97fcd75dde29594d472fbcc526b44add2263c89c4a89a1accc0";
const INTERVENTION_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/intervention-topology/seal/OTO_I_INTERVENTION_REGISTRY.json";
const EXPLICIT_REL: &str =
    "mt5-authority/studies/obs-open-01/observe-the-observer/seal/EXPLICIT_DOF_CENSUS.json";
const IMPACT_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/authority-impact/seal/OTO_A_AUTHORITY_IMPACT_REGISTRY.json";

fn read_json(repo: &Path, rel: &str) -> Result<(Value, Vec<u8>), Box<dyn std::error::Error>> {
    let bytes = fs::read(repo.join(rel))?;
    Ok((serde_json::from_slice(&bytes)?, bytes))
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

fn display_only(id: &str) -> bool {
    matches!(
        id,
        "InpShowLegacyLines"
            | "InpPeriodColor"
            | "InpAreaColor"
            | "InpPeriodWidth"
            | "InpAreaWidth"
            | "InpPeriodStyle"
            | "InpAreaStyle"
            | "InpShowExtremeSentinels"
            | "InpUpperSentinelColor"
            | "InpLowerSentinelColor"
            | "InpExtremeSentinelStyle"
            | "InpExtremeSentinelWidth"
    )
}

fn corrected_vector(record: &Value) -> (Value, String, Vec<String>, Vec<String>, &'static str) {
    let id = record["target_joint"].as_str().unwrap_or("UNKNOWN");
    let status = record["intervention_status"].as_str().unwrap_or("UNKNOWN");
    if display_only(id) {
        let all = (0..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
        return (
            json!({
                "G0_INPUT_AUTHORITY":"PRESERVED", "G1_KERNEL_EXTRACTION":"PRESERVED", "G2_ROLE_CENSUS":"PRESERVED",
                "G3_REACHABILITY":"PRESERVED", "G4_OBSERVER_SURFACE":"PRESERVED", "G5_CONTINUATION_GRAMMAR":"PRESERVED",
                "G6_EQUIVALENCE_CONTRACT":"PRESERVED", "G7_FORMAL_ANALYSIS_BOUNDARY":"PRESERVED", "G8_LABORATORY":"PRESERVED"
            }),
            "NONE".to_owned(),
            all.clone(),
            Vec::new(),
            "STATIC_PLOT_ONLY_PROOF_NO_G4_PROTECTED_OBSERVABLE_CHANGE",
        );
    }
    if status == "NOT_SEPARABLY_INTERVENABLE" || status == "UNKNOWN" {
        let unknown = (0..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
        return (
            json!({
                "G0_INPUT_AUTHORITY":"UNKNOWN", "G1_KERNEL_EXTRACTION":"UNKNOWN", "G2_ROLE_CENSUS":"UNKNOWN",
                "G3_REACHABILITY":"UNKNOWN", "G4_OBSERVER_SURFACE":"UNKNOWN", "G5_CONTINUATION_GRAMMAR":"UNKNOWN",
                "G6_EQUIVALENCE_CONTRACT":"UNKNOWN", "G7_FORMAL_ANALYSIS_BOUNDARY":"UNKNOWN", "G8_LABORATORY":"UNKNOWN"
            }),
            "UNKNOWN".to_owned(),
            Vec::new(),
            unknown,
            "PREDECESSOR_IMPACT_REMAINS_UNKNOWN",
        );
    }
    let earliest = record["earliest_invalidated_gate"]
        .as_str()
        .unwrap_or("UNKNOWN")
        .to_owned();
    let preserved = match earliest.as_str() {
        "G0" => Vec::new(),
        "G1" => vec!["G0".to_owned()],
        _ => Vec::new(),
    };
    let start = if earliest == "G0" { 0 } else { 1 };
    let cone = (start..=8).map(|i| format!("G{i}")).collect::<Vec<_>>();
    (
        record["authority_impact"].clone(),
        earliest,
        preserved,
        cone,
        "INHERITED_OTO_A_IMPACT_FOR_NON_DISPLAY_INTERVENTION",
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
    let (intervention, _) = read_json(repo, INTERVENTION_REL)?;
    let (explicit, explicit_bytes) = read_json(repo, EXPLICIT_REL)?;
    let (impact, _) = read_json(repo, IMPACT_REL)?;
    let records = intervention["records"]
        .as_array()
        .ok_or("intervention records missing")?;
    let impacts = impact["records"]
        .as_array()
        .ok_or("impact records missing")?;
    if records.len() != 58 || impacts.len() != 58 {
        return Err(format!(
            "expected 58 records, found {} and {}",
            records.len(),
            impacts.len()
        )
        .into());
    }
    let mut impact_map = BTreeMap::new();
    for item in impacts {
        impact_map.insert(item["target_joint"].as_str().unwrap_or("UNKNOWN"), item);
    }
    let declared_inputs = explicit["declared_inputs"]
        .as_array()
        .ok_or("declared inputs missing")?;
    if declared_inputs.len() != 24 {
        return Err(format!(
            "expected 24 declared inputs, found {}",
            declared_inputs.len()
        )
        .into());
    }
    let mut mappings = Vec::with_capacity(records.len());
    let mut corrected = Vec::with_capacity(records.len());
    let mut surface_counts = BTreeMap::<String, usize>::new();
    let mut corrected_counts = BTreeMap::<String, usize>::new();
    for record in records {
        let id = record["target_joint"].as_str().unwrap_or("UNKNOWN");
        let old = impact_map
            .get(id)
            .ok_or_else(|| format!("impact record missing for {id}"))?;
        let is_display = display_only(id);
        let unknown = old["earliest_invalidated_gate"].as_str() == Some("UNKNOWN");
        let implementation_data = if is_display {
            "NO"
        } else if record["downstream_affected_nodes"]
            .as_array()
            .map(|v| {
                v.iter()
                    .any(|x| x.as_str() == Some("ARCH_PUBLICATION_SURFACE"))
            })
            .unwrap_or(false)
        {
            "YES"
        } else {
            "NO"
        };
        let g4 = if is_display {
            "PROVEN_NO"
        } else if unknown {
            "UNKNOWN"
        } else {
            "NOT_PROVEN"
        };
        let capture = if is_display || unknown || implementation_data == "YES" {
            "CONDITIONAL"
        } else {
            "NO"
        };
        let display = if is_display { "YES" } else { "NO" };
        let surface_class = if is_display {
            "PURE_DISPLAY_RENDERING"
        } else if unknown {
            "UNRESOLVED"
        } else if implementation_data == "YES" {
            "IMPLEMENTATION_DATA_PUBLICATION"
        } else {
            "NON_PUBLICATION"
        };
        *surface_counts.entry(surface_class.to_owned()).or_insert(0) += 1;
        let (vector, earliest, preserved, cone, basis) = corrected_vector(old);
        *corrected_counts.entry(earliest.to_owned()).or_insert(0) += 1;
        mappings.push(json!({
            "intervention_id": record["intervention_id"], "target_joint": id, "target_class": record["target_class"], "target_group": record["target_group"],
            "touches_implementation_data_publication": implementation_data,
            "touches_g4_protected_observation": g4,
            "touches_display_rendering": display,
            "touches_capture_visibility": capture,
            "surface_class": surface_class,
            "display_specimen": if is_display { "CHANGED" } else { "UNCHANGED_OR_NOT_DISPLAY" },
            "state_transition_change": if is_display { "PROVEN_NO" } else { "NOT_EVALUABLE" },
            "buffer_value_change": if is_display { "PROVEN_NO" } else { "NOT_EVALUABLE" },
            "buffer_order_change": if is_display { "PROVEN_NO" } else { "NOT_EVALUABLE" },
            "g4_named_observable_change": if is_display { "PROVEN_NO" } else { "NOT_EVALUABLE" },
            "only_plot_attribute_change": if is_display { "PROVEN_YES" } else { "NOT_EVALUABLE" },
            "mapping_basis": if is_display { "OTO-I publication-only restriction plus sealed explicit effect census" } else { "OTO-I/OTO-A metadata only" },
            "capture_note": "instrument capture visibility is channel-dependent; buffer capture and visual rendering are not conflated"
        }));
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
        corrected.push(json!({
            "intervention_id": record["intervention_id"], "target_joint": id,
            "prior_oto_a_earliest_invalidated_gate": old["earliest_invalidated_gate"],
            "corrected_authority_impact": vector,
            "corrected_earliest_invalidated_gate": earliest,
            "corrected_descendant_requalification_cone": cone,
            "corrected_unchanged_ancestry": preserved,
            "corrected_roots_invalidated": roots_invalidated,
            "corrected_roots_preserved": roots_preserved,
            "oto_display_specimen": if is_display { "CHANGED" } else { "NOT_APPLICABLE" },
            "static_proof_basis": ["OTO_A_AUTHORITY_IMPACT_REGISTRY", "OTO_I_INTERVENTION_REGISTRY", "EXPLICIT_DOF_CENSUS", basis],
            "runtime_evidence_required": old["runtime_evidence_required"]
        }));
    }
    mappings.sort_by(|a, b| a["target_joint"].as_str().cmp(&b["target_joint"].as_str()));
    corrected.sort_by(|a, b| a["target_joint"].as_str().cmp(&b["target_joint"].as_str()));
    let protocol = json!({
        "schema":"OTO_A1_PROTOCOL_V1", "gate":"OTO-A1_AUTHORITY_SURFACE_ALIGNMENT", "execution_state":"SEALED", "question_status":"CLOSED",
        "result":"AUTHORITY_SURFACE_ALIGNMENT_SEALED", "disposition":"NONE",
        "question":"Does an OTO architectural publication node denote the G4-protected observation surface, display rendering, or instrument capture visibility?",
        "firewall":"DISPLAY_RENDERING != G4_PROTECTED_OBSERVATION", "scope":"ALIGNMENT_ONLY", "oto_i_and_oto_c_mutated":false,
        "g4_mutated":false, "dynamic_dof_authority":"NONE", "numeric_authority_distance":"NOT_CLAIMED"
    });
    let mapping = json!({"schema":"OTO_A1_SURFACE_MAPPING_REGISTRY_V1", "record_count":58, "surface_counts":surface_counts, "records":mappings});
    let impact = json!({"schema":"OTO_A1_CORRECTED_AUTHORITY_IMPACT_V1", "record_count":58, "corrected_earliest_invalidated_gate_counts":corrected_counts, "records":corrected});
    let access = json!({
        "schema":"OTO_A1_ACCESS_AUDIT_V1", "oto_a_root_read":true, "oto_i_metadata_read":true, "explicit_census_read":true,
        "source_artifact_bytes_read":0, "market_rows_read":0, "outcome_rows_read":0, "indicator_buffers_read":0,
        "runtime_probe_executed":false, "d_b_reads":0, "d_c_reads":0, "d_d_reads":0, "trading_com_editor_invoked":false,
        "oto_i_reopened":false, "oto_c_reopened":false, "g4_mutated":false, "dynamic_dof_authority":"NONE",
        "explicit_census_sha256":sha256_bytes(&explicit_bytes)
    });
    let findings = json!({
        "schema":"OTO_A1_TYPED_FINDINGS_V1", "execution_state":"SEALED", "question_status":"CLOSED", "result":"AUTHORITY_SURFACE_ALIGNMENT_SEALED",
        "display_only_cases_reclassified":12, "corrected_earliest_invalidated_gate_counts":corrected_counts,
        "independent_dimension_count":"NOT_EVALUABLE", "dynamic_dof_authority":"NONE",
        "maximum_authority":"OBS_OPEN_OTO_STATIC_AUTHORITY_SURFACE_ALIGNMENT_V1"
    });
    let artifact_values = [
        ("OTO_A1_PROTOCOL_V1.json", protocol),
        ("OTO_A1_SURFACE_MAPPING_REGISTRY.json", mapping),
        ("OTO_A1_CORRECTED_AUTHORITY_IMPACT.json", impact),
        ("OTO_A1_ACCESS_AUDIT.json", access),
        ("OTO_A1_TYPED_FINDINGS.json", findings),
    ];
    for (name, value) in &artifact_values {
        write_json(&out.join(name), value)?;
    }
    let entries = artifact_values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({"schema":"OTO_A1_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_OTO_STATIC_AUTHORITY_SURFACE_ALIGNMENT_V1","parent_static_root":STATIC_ROOT,"parent_intervention_root":INTERVENTION_ROOT,"parent_closure_root":CLOSURE_ROOT,"parent_oto_a_root":AUTHORITY_IMPACT_ROOT,"artifact_count":artifact_values.len(),"artifacts":entries});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_A1_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_A1_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_A1_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_A1_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_A1_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_A1_ROOT_RECEIPT.json"))?)?;
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
            INTERVENTION_ROOT,
            CLOSURE_ROOT,
            AUTHORITY_IMPACT_ROOT,
        ] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
    #[test]
    fn display_set_has_twelve_members() {
        let values = [
            "InpShowLegacyLines",
            "InpPeriodColor",
            "InpAreaColor",
            "InpPeriodWidth",
            "InpAreaWidth",
            "InpPeriodStyle",
            "InpAreaStyle",
            "InpShowExtremeSentinels",
            "InpUpperSentinelColor",
            "InpLowerSentinelColor",
            "InpExtremeSentinelStyle",
            "InpExtremeSentinelWidth",
        ];
        assert_eq!(values.iter().filter(|id| display_only(id)).count(), 12);
    }
}

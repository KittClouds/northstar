use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const OTO_A1_ROOT: &str = "6f5209a690d0a860bf9916741d5298275a7b18d1c7fe984ab2d5aefb962a0157";
const OTO_L_ROOT: &str = "24003a167236071b27ddab86769023528e76185fe60825608dbf605cfe13a472";
const G8_ROOT: &str = "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT";
const OTO_L_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/literature-adjacency/seal/OTO_L_ROOT_RECEIPT.json";

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
    Ok(json!({"path": name, "bytes": bytes.len(), "sha256": sha256_bytes(&bytes)}))
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.is_file());
    paths.sort();
    Ok(paths)
}

fn optics() -> Value {
    let defs = [
        (
            "CLOCK_COORDINATE_PRISM",
            "deterministic coordinate projection",
            "AUTHORIZED_04A_COORDINATES",
            "LOSSLESS_OR_TYPED_UNEVALUABLE",
            "NONE",
        ),
        (
            "RESOLUTION_LADDER",
            "predeclared lossy merge/discard operator",
            "AUTHORIZED_04A_TRACE",
            "LOSSY_BY_DECLARED_OPERATOR",
            "NONE",
        ),
        (
            "PARTIAL_INFORMATION_LENS",
            "mask/widen/remove without imputation",
            "AUTHORIZED_04A_TRACE",
            "KNOWN_POSSIBLE_SET_UNDETERMINED",
            "NONE",
        ),
        (
            "TEMPORAL_DEFORMATION_LENS",
            "monotone timing deformation with separate value comparison",
            "AUTHORIZED_04A_TRACE",
            "TIME_VALUE_SEPARATE",
            "NONE",
        ),
        (
            "FIXED_MULTISCALE_LENS",
            "frozen nonlearned transform bank",
            "AUTHORIZED_04A_TRACE",
            "TRANSFORM_DEPENDENT_VIEW",
            "NONE",
        ),
        (
            "EXTERNAL_MICROFILM_OVERLAY",
            "separately sourced finer chronology overlay",
            "SEPARATE_SOURCE_REQUIRED",
            "BLOCKED",
            "BLOCKED_PENDING_SEPARATE_SOURCE",
        ),
        (
            "IDENTITY_CONTROL_LENS",
            "identity-preserving control projection",
            "AUTHORIZED_04A_IDENTITY",
            "DECLARED_CONTROL",
            "NONE",
        ),
    ];
    json!({"schema":"OPTICAL_TOOL_REGISTRY_V1","tools":defs.into_iter().map(|(id, transform, input, loss, status)| json!({
        "optic_id":id,"input_authority":input,"transform_definition":transform,"output_type":"DESCRIPTIVE_VIEW_AND_RECEIPT",
        "loss_contract":loss,"time_authority":"INHERITED_ONLY","ordering_authority":"INHERITED_ONLY","invertibility_status":"MUST_BE_TYPED_PER_USE",
        "precision_effect":"NO_RETROSPECTIVE_UPGRADE","ontology_authority":"NONE","target_region_authority":"NONE","scientific_verdict_authority":"NONE",
        "feedback_to_observer":"FORBIDDEN","status":status
    })).collect::<Vec<_>>()})
}

fn question_contract() -> Value {
    json!({
        "schema":"QUESTION_QUALIFICATION_CONTRACT_V1",
        "value_test":{"required_for_each_result":["YES","NO","MIXED","UNKNOWN","NOT_EVALUABLE"],"condition":"V(Q|r)>0"},
        "tests":[
            {"id":"ONTOLOGY_STRIPPING","rule":"remove interpretive nouns; reject if mathematical precision disappears"},
            {"id":"OPTIC_OUTPUT_SEPARATION","required":["OPTICS_USED","OPTIC_DEPENDENCY","MEASURED_QUANTITY","DESCRIPTIVE_OUTPUTS","AUTHORIZED_INFERENCES","FORBIDDEN_INFERENCES"]},
            {"id":"BLINDSPOT_VALUE","rule":"UNKNOWN and NOT_EVALUABLE must identify the stopping reason"},
            {"id":"RESOLUTION_NEUTRALITY","forbidden_assumptions":["FINER_IS_BETTER","COARSER_IS_WORSE","STABLE_ACROSS_SCALE_IS_IMPORTANT","UNSTABLE_ACROSS_SCALE_IS_IMPORTANT"]},
            {"id":"INTERPRETATION_SYMMETRY","rule":"protocol is result-independent except predeclared termination"}
        ],
        "result_vocabulary":["YES","NO","MIXED","UNKNOWN","NOT_EVALUABLE"],
        "question_selection":"NOT_PERFORMED_IN_G8_2",
        "ontology_authority":"NONE"
    })
}

fn arms() -> Vec<Value> {
    vec![
        json!({"arm_id":"SOL-P1","name":"CLOCK_PARALLAX","primitive":"coordinate representation","coordinates":["t_event","t_knowledge","t_commit","j_experiment","t_causal","a_candidate"],"status":"PROTOCOL_FREEZE_ONLY","authority":"NONE"}),
        json!({"arm_id":"SOL-P2","name":"GRANULARITY_PARALLAX","primitive":"predeclared lossy resolution operators","status":"PROTOCOL_FREEZE_ONLY","authority":"NONE"}),
        json!({"arm_id":"SOL-P3","name":"PARTIAL_INFORMATION_PARALLAX","primitive":"controlled ignorance","states":["KNOWN","POSSIBLE_SET","UNDETERMINED"],"status":"PROTOCOL_FREEZE_ONLY","authority":"NONE"}),
        json!({"arm_id":"SOL-P4","name":"TEMPORAL_DEFORMATION_PARALLAX","primitive":"monotone temporal perturbation","decomposition":["D_t","D_v","rho"],"status":"PROTOCOL_FREEZE_ONLY","authority":"NONE"}),
        json!({"arm_id":"SOL-P5","name":"FIXED_MULTISCALE_OPTICS","primitive":"nonlearned transform coefficients","constraints":["NO_LEARNED_FILTERS","NO_ADAPTIVE_BANDWIDTH","NO_RESULT_CONDITIONED_SCALE_SELECTION"],"status":"PROTOCOL_FREEZE_ONLY","authority":"NONE"}),
        json!({"arm_id":"SOL-P6","name":"EXTERNAL_MICROFILM","primitive":"separately sourced higher-resolution chronology","status":"BLOCKED_PENDING_SEPARATE_SOURCE","authority":"NONE"}),
    ]
}

fn controls() -> Value {
    let ids = [
        "IDENTITY_TRANSFORM_CONTROL",
        "REVERSIBLE_ROUNDTRIP_CONTROL",
        "KNOWN_LOSS_COLLISION_CONTROL",
        "ORDER_PRESERVATION_CONTROL",
        "TIME_AUTHORITY_BARRIER_CONTROL",
        "SYNTHETIC_NULL_TRACE_CONTROL",
        "SYNTHETIC_KNOWN_DIFFERENCE_CONTROL",
    ];
    json!({"schema":"NEGATIVE_CONTROL_CONTRACT_V1","controls":ids.into_iter().map(|id| json!({"control_id":id,"purpose":"detect structure introduced by the optical apparatus","result_authority":"NONE","execution":"NOT_STARTED"})).collect::<Vec<_>>()})
}

fn independence(arm_list: &[Value]) -> Value {
    let mut rows = Vec::new();
    for i in 0..arm_list.len() {
        for j in (i + 1)..arm_list.len() {
            let a = arm_list[i]["arm_id"].as_str().unwrap_or("UNKNOWN");
            let b = arm_list[j]["arm_id"].as_str().unwrap_or("UNKNOWN");
            rows.push(json!({"arm_a":a,"arm_b":b,"primitive_overlap":"NONE_IDENTIFIED","shared_assumptions":["G0_G8_ANCESTRY","A1_SURFACE_SEPARATION","STOPPING_AND_BLINDNESS_CONTRACT"],"independence_status":"UNKNOWN","common_mode_risk":"PARTIALLY_SHARED"}));
        }
    }
    json!({"schema":"ARM_INDEPENDENCE_MATRIX_V1","arms":arm_list.iter().map(|x| x["arm_id"].clone()).collect::<Vec<_>>(),"pair_count":rows.len(),"pairs":rows,"independence_claim":"NOT_EARNED"})
}

fn report(gate_status: &str, root: &str, arm_list: &[Value]) -> String {
    let arms = arm_list
        .iter()
        .map(|x| format!("- `{}` — {} (`{}`)", x["arm_id"], x["name"], x["status"]))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# Sol Blind Campaign E2E Report

## Result

- Campaign root: `{root}`
- Gate status: `{gate_status}`
- Execution: `NOT_STARTED`
- 04A access: `CLOSED`
- Scientific authority: `NONE`
- Ontology authority: `NONE`
- Target-region authority: `NONE`
- Interpretation authority: `NONE`
- G8 parent root: `{G8_ROOT}`

The package freezes a campaign constitution. It does not open 04A, execute a parallax arm, select a question from a result, or synthesize an unknown object.

## G8.1 — Null ontology and optical tools

Observer, optic, and interpretation are separate types. Seven optical tool classes are registered. The external microfilm overlay is blocked pending a separately qualified source. Every optic carries an explicit loss, time, ordering, invertibility, precision, and feedback contract. No optic can feed back into the observer.

## G8.2 — Question generator

The question factory is qualified only as a factory. It requires five result branches, strips interpretive nouns, separates optic outputs from authorized inference, explains blind spots, remains resolution-neutral, and is invariant to the first result. No actual 04A question was selected.

## G8.3 — Temporal parallax architecture

Six independent instrumentation arms are frozen:

{arms}

Their pairwise independence is `UNKNOWN`; they share constitutional ancestry and common-mode failure risks. Seven negative controls are frozen. No arm has executed.

## G8.4 — Non-ontological synthesis

Only sealed arm packets may be synthesized. Legal operations are agreement, disagreement, conditional compatibility, orthogonal findings, shared blind spot, common-mode dependency, unresolved tension, not-comparable, and not-evaluable. Majority voting, forced coordinates, latent-object construction, region naming, and master-geometry construction are forbidden.

## G8.5 — Blind launch seal

The campaign manifest binds the parent roots, gate artifacts, arm protocols, optic registry, negative controls, synthesis contract, stopping rules, and blindness contract. Cross-arm result visibility is locked until synthesis unlock. Moxie access is forbidden. Stop conditions are protocol completion, declared bound exhaustion, authority exhaustion, not-evaluable, resource bound, or implementation failure.

## Methodological adjacency

The recorded methodological references are adjacency only. Partial-information runtime verification explicitly treats gaps as sets of possible traces; fixed scattering uses prescribed rather than learned filters; sampled timed semantics and Skorokhod conformance motivate separating resolution/timing effects; black-box checking motivates proposal/validation separation. None transfers a theorem to Northstar.

## Access audit

- 04A source/content reads: `0`
- market/outcome reads: `0`
- D_B/D_C/D_D reads: `0`
- runtime probes: `0`
- Trading.com editor: `0`
- arm executions: `0`
- result-conditioned retuning: `0`

## Disposition

`{gate_status}` is intentionally restricted by the absence of a materialized parent G8 root in this checkout. The campaign is launch-ready only as a sealed protocol package; a later execution authorization must bind the missing G8 lineage before opening any population.
"#
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
    let literature_bytes = fs::read(repo.join(OTO_L_REL))?;
    let arm_list = arms();
    let gate_status = "SEALED_WITH_DECLARED_RESTRICTIONS";
    let g81 = json!({"schema":"G8_1_GATE_DECISION_V1","gate":"G8.1","status":gate_status,"question_status":"CLOSED","result":"OPTIC_CONSTITUTION_SEALED","authority_gain":"NONE","optics":"OPTIC_REGISTRY_SEALED","external_microfilm":"BLOCKED_PENDING_SEPARATELY_QUALIFIED_SOURCE"});
    let g82 = json!({"schema":"G8_2_GATE_DECISION_V1","gate":"G8.2","status":gate_status,"question_status":"CLOSED","result":"QUESTION_FACTORY_SEALED","authority_gain":"NONE","question_selection":"NOT_PERFORMED"});
    let g83 = json!({"schema":"G8_3_GATE_DECISION_V1","gate":"G8.3","status":gate_status,"question_status":"CLOSED","result":"PARALLAX_ARCHITECTURE_SEALED","authority_gain":"NONE","arm_count":arm_list.len(),"arm_execution":"NOT_STARTED"});
    let g84 = json!({"schema":"G8_4_GATE_DECISION_V1","gate":"G8.4","status":gate_status,"question_status":"CLOSED","result":"NON_ONTOLOGICAL_SYNTHESIS_SEALED","authority_gain":"NONE","majority_vote":"FORBIDDEN","latent_object":"FORBIDDEN"});
    let g85 = json!({"schema":"G8_5_GATE_DECISION_V1","gate":"G8.5","status":gate_status,"question_status":"CLOSED","result":"BLIND_CAMPAIGN_LAUNCH_SEALED","authority_gain":"NONE","campaign_execution":"NOT_STARTED","04A_access":"CLOSED","moxie_access":"FORBIDDEN"});
    let optic = optics();
    let questions = question_contract();
    let architecture = json!({"schema":"SOL_PARALLAX_ARCHITECTURE_V1","arms":arm_list,"primitive_independence":"NOT_EARNED","shared_ancestry":["G0_G8","OTO_A1"],"execution":"NOT_STARTED"});
    let synthesis = json!({"schema":"TRIANGULATION_SYNTHESIS_CONTRACT_V1","legal_operations":["AGREEMENT","DISAGREEMENT","CONDITIONAL_COMPATIBILITY","ORTHOGONAL_FINDINGS","SHARED_BLIND_SPOT","COMMON_MODE_DEPENDENCY","UNRESOLVED_TENSION","NOT_COMPARABLE","NOT_EVALUABLE"],"agreement_independence_status":["STRONGLY_INDEPENDENT","PARTIALLY_SHARED","COMMON_MODE_DOMINATED","UNKNOWN"],"no_majority_vote":true,"no_forced_common_coordinate":true,"latent_object":"FORBIDDEN","region":"FORBIDDEN","master_geometry":"FORBIDDEN","authority_gain":"NONE"});
    let stopping = json!({"schema":"ARM_STOPPING_RULES_V1","allowed":["PROTOCOL_COMPLETE","DECLARED_BOUND_EXHAUSTED","AUTHORITY_EXHAUSTED","NOT_EVALUABLE","RESOURCE_BOUND_REACHED","IMPLEMENTATION_FAILURE"],"forbidden":["FOUND_SOMETHING_INTERESTING","RESULT_LOOKS_CLEAR","ENOUGH_TO_TELL_A_STORY"]});
    let controls = controls();
    let access = json!({"schema":"SOL_CAMPAIGN_ACCESS_AUDIT_V1","static_ancestry_read":true,"literature_metadata_read":true,"literature_root_sha256":sha256_bytes(&literature_bytes),"04a_content_read":0,"market_rows_read":0,"outcome_rows_read":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"runtime_probe_executed":false,"arm_executions":0,"result_conditioned_retuning":0,"trading_com_editor_invoked":false,"moxie_access":"FORBIDDEN","campaign_execution":"NOT_STARTED"});
    let artifact_values = vec![
        ("G8_1_GATE_DECISION.json", g81),
        ("G8_1_OPTICAL_TOOL_REGISTRY.json", optic),
        ("G8_2_GATE_DECISION.json", g82),
        ("G8_2_QUESTION_QUALIFICATION_CONTRACT.json", questions),
        ("G8_3_GATE_DECISION.json", g83),
        ("G8_3_SOL_PARALLAX_ARCHITECTURE.json", architecture.clone()),
        ("G8_3_ARM_INDEPENDENCE_MATRIX.json", independence(&arm_list)),
        ("G8_3_NEGATIVE_CONTROL_CONTRACT.json", controls),
        ("G8_4_GATE_DECISION.json", g84),
        ("G8_4_SYNTHESIS_CONTRACT.json", synthesis),
        ("G8_5_GATE_DECISION.json", g85),
        (
            "G8_5_CAMPAIGN_MANIFEST.json",
            json!({"schema":"SOL_CAMPAIGN_MANIFEST_V1","g8_root":G8_ROOT,"g8_1_root":"IN_BUNDLE","g8_2_root":"IN_BUNDLE","g8_3_root":"IN_BUNDLE","g8_4_root":"IN_BUNDLE","question_set_hash":"NOT_SELECTED","arm_protocol_hashes":"FROZEN_IN_ARCHITECTURE_ARTIFACT","optic_registry_hash":"FROZEN_IN_BUNDLE","negative_control_hash":"FROZEN_IN_BUNDLE","synthesis_contract_hash":"FROZEN_IN_BUNDLE","stopping_rules_hash":"FROZEN_IN_BUNDLE","blindness_contract_hash":"FROZEN_IN_BUNDLE","campaign_execution":"NOT_STARTED","04a_access":"CLOSED"}),
        ),
        ("G8_5_STOPPING_RULES.json", stopping),
        ("SOL_CAMPAIGN_ACCESS_AUDIT.json", access),
    ];
    for (name, value) in &artifact_values {
        write_json(&out.join(name), value)?;
    }
    write_text(
        &out.join("SOL_CAMPAIGN_EXECUTION_REPORT.md"),
        &report(gate_status, "SEE_SOL_CAMPAIGN_ROOT_RECEIPT", &arm_list),
    )?;
    let mut entries = artifact_values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    entries.push(artifact_entry(out, "SOL_CAMPAIGN_EXECUTION_REPORT.md")?);
    let payload = json!({"schema":"SOL_CAMPAIGN_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_SOL_BLIND_CAMPAIGN_LAUNCH_PROTOCOL_V1","parent_static_root":STATIC_ROOT,"parent_oto_a1_root":OTO_A1_ROOT,"parent_oto_l_root":OTO_L_ROOT,"g8_root":G8_ROOT,"gate_count":5,"arm_count":6,"artifacts":entries});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("SOL_CAMPAIGN_ROOT_RECEIPT.json"),
        &json!({"schema":"SOL_CAMPAIGN_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
    let root: Value =
        serde_json::from_slice(&fs::read(seal.join("SOL_CAMPAIGN_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("SOL_CAMPAIGN_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"SOL_CAMPAIGN_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("SOL_CAMPAIGN_ROOT_RECEIPT.json"))?)?;
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
    fn roots_are_sha256_or_typed_placeholder() {
        for value in [STATIC_ROOT, OTO_A1_ROOT, OTO_L_ROOT] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
        assert_eq!(G8_ROOT, "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT");
    }
    #[test]
    fn six_arms_and_seven_controls() {
        assert_eq!(arms().len(), 6);
        assert_eq!(controls()["controls"].as_array().unwrap().len(), 7);
    }
}

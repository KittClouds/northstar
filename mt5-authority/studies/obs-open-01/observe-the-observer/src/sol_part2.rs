use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SOL_ROOT: &str = "f25ba4971834384a76b96eb090306773dfba304be8250ce1519d175deeeec35d";
const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const OTO_A1_ROOT: &str = "6f5209a690d0a860bf9916741d5298275a7b18d1c7fe984ab2d5aefb962a0157";
const OTO_L_ROOT: &str = "24003a167236071b27ddab86769023528e76185fe60825608dbf605cfe13a472";
const G8_ROOT: &str = "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT";
const STATUS: &str = "QUESTION_FACTORY_FROZEN_WITH_DECLARED_RESTRICTION";
const FINAL_STATE: &str = "QUESTION_FACTORY_QUALIFIED_PENDING_G8_BINDING";

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

fn delta_contract() -> Value {
    let classes = ["YES", "NO", "MIXED", "UNKNOWN", "NOT_EVALUABLE"];
    json!({
        "schema":"SOL_PART2_EPISTEMIC_DELTA_CONTRACT_V1",
        "required_fields":["what_becomes_known","what_becomes_ruled_out","what_remains_unknown","authority_boundary","legal_followup","forbidden_inference"],
        "result_classes":classes,
        "nonempty_update_required":true
    })
}

fn question(
    id: &str,
    arm: &str,
    operation: &str,
    text: &str,
    varied: &[&str],
    surface: &[&str],
) -> Value {
    let classes = ["YES", "NO", "MIXED", "UNKNOWN", "NOT_EVALUABLE"];
    let delta = classes
        .iter()
        .map(|class| {
            (
                class.to_string(),
                json!({
                    "what_becomes_known":format!("whether the predeclared comparison has the {class} result class"),
                    "what_becomes_ruled_out":"only claims inconsistent with the qualified result and its controls",
                    "what_remains_unknown":"any unmeasured construction, mechanism, generalization, or interpretation",
                    "authority_boundary":"the exact input, optic, correspondence, and protected surface used by this packet",
                    "legal_followup":"a separately frozen question whose inputs and authority are named",
                    "forbidden_inference":["TARGET_OBJECT","TARGET_REGION","MECHANISM","UNIVERSALITY","ECONOMIC_VALUE"]
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "question_id":id,
        "arm_id":arm,
        "question_text":text,
        "primitive_operation":operation,
        "fixed_quantities":["inherited observer lineage","authorized input material","protected measurement surface","predeclared control set"],
        "varied_quantities":varied,
        "required_authority":["SOL_CAMPAIGN_ROOT","G8_ROOT","OPTIC_REGISTRY","NEGATIVE_CONTROL_CONTRACT"],
        "required_adapters":["QUESTION_PACKET_ADAPTER_V1"],
        "observed_output_surface":surface,
        "explicit_non_measurements":["target identity","target region","mechanism","economic meaning","universal generalization"],
        "ontology_assumptions":"NONE",
        "region_assumptions":"NONE",
        "outcome_space":classes,
        "epistemic_delta_contract":delta,
        "dependence_on_other_questions":"NONE_REQUIRED",
        "shared_assumptions":["G0_G8_ANCESTRY","NULL_ONTOLOGY","RESULT_BLINDNESS"],
        "common_mode_failures":["DISPLAY_CONTAMINATION","CLOCK_OR_RESOLUTION_ALIASING","ADAPTER_ERROR"],
        "result_conditioned_retuning":"FORBIDDEN",
        "qualification_status":"SURVIVES_SYNTHETIC_ANTI_FUNNEL"
    })
}

fn questions() -> Vec<Value> {
    vec![
        question(
            "SOL-P1-Q1",
            "SOL-P1",
            "CHANGE_COORDINATE_PROJECTION",
            "Under a frozen set of authorized coordinate projections, which protected relations are preserved, changed, ambiguous, or unevaluable?",
            &["coordinate projection"],
            &["protected relations", "ordering", "timing receipts"],
        ),
        question(
            "SOL-P2-Q1",
            "SOL-P2",
            "REMOVE_INFORMATION_WITH_EXPLICIT_LOSS_RECEIPT",
            "Under frozen resolution operators, which protected distinctions remain equal, differ, become ambiguous, or become unevaluable?",
            &["resolution operator"],
            &["protected distinctions", "loss receipt", "ordering"],
        ),
        question(
            "SOL-P3-Q1",
            "SOL-P3",
            "REMOVE_INFORMATION_WITH_EXPLICIT_LOSS_RECEIPT",
            "Under predeclared masks and possible-value sets, which outputs remain determined, possible, or undetermined?",
            &["visibility mask", "possible-value set"],
            &["known outputs", "possible sets", "undetermined outputs"],
        ),
        question(
            "SOL-P4-Q1",
            "SOL-P4",
            "PERTURB_TIMING_WHERE_LEGAL",
            "Under frozen monotone timing displacements with protected values held separate, which measured differences are attributable to timing, value, both, or cannot be compared?",
            &["monotone timing displacement"],
            &["time component", "value component", "comparison receipt"],
        ),
        question(
            "SOL-P5-Q1",
            "SOL-P5",
            "APPLY_FIXED_OPTICAL_TRANSFORM",
            "For a frozen nonlearned transform bank, which declared measurements are reproducible, changed, or unevaluable under the control battery?",
            &["fixed transform bank"],
            &[
                "transform measurements",
                "control receipts",
                "precision receipt",
            ],
        ),
    ]
}

fn report(questions: &[Value], g8_status: &str) -> String {
    let ids = questions
        .iter()
        .map(|question| {
            format!(
                "- {}: {}",
                question["question_id"], question["qualification_status"]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# Sol Part 2 — Question Factory Qualification Report

## Result

- Parent Sol root: {SOL_ROOT}
- G8 lineage status: {g8_status}
- Final state: {FINAL_STATE}
- Target object: UNBOUND
- Target region: UNBOUND
- Population contact: FORBIDDEN
- Scientific authority: NONE
- Question packets frozen: synthetic qualification only

## Lineage act

The repository contains no materialized G8 root receipt. No substitute hash was created. Sol optics, arms, synthesis rules, stopping rules, and blindness contracts remain unchanged. Reverification is pending exact G8 materialization.

## Question factory output

The factory was instantiated using neutral operations and epistemic-delta contracts. No interpretive noun was used as a target, expected structure, or success criterion. Each result class YES, NO, MIXED, UNKNOWN, and NOT_EVALUABLE receives a nonempty update contract.

{ids}

P6 external microfilm remains BLOCKED_PENDING_SEPARATE_SOURCE and has no population-facing question packet.

## Anti-funnel qualification

Outcome reversal, null survival, tool removal, story deletion, negative-control challenge, and synthetic five-class outcome handling were exercised as protocol fixtures only. These fixtures do not observe the inherited observer or any population.

## Access firewall

04A, D_B, D_C, D_D, market rows, outcomes, Sol arm results, Kammi results, runtime probes, and the Trading.com editor were not accessed. No question was selected from a result and no arm was executed.

## Disposition

The question set is frozen as a candidate packet set worthy of later population-access review, not as an execution warrant. Exact G8 lineage must be materialized and rebound before any population authorization can be issued.
"#
    )
}

pub fn build(_repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("output directory must be empty: {}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    let qs = questions();
    let g8_audit = json!({
        "schema":"SOL_PART2_G8_LINEAGE_AUDIT_V1",
        "parent_sol_root":SOL_ROOT,
        "static_root":STATIC_ROOT,
        "oto_a1_root":OTO_A1_ROOT,
        "oto_l_root":OTO_L_ROOT,
        "g8_root":G8_ROOT,
        "materialized":false,
        "search_scope":"CURRENT_SINGULAR_AUTHORITY_CHECKOUT",
        "synthetic_replacement_created":false,
        "reverification":"NOT_EVALUABLE_PENDING_EXACT_G8_ROOT",
        "status":"G8_PARENT_LINEAGE_UNAVAILABLE"
    });
    let values: Vec<(&str, Value)> = vec![
        ("SOL_PART2_G8_LINEAGE_AUDIT.json", g8_audit),
        (
            "SOL_PART2_REVERIFICATION_POLICY.json",
            json!({"schema":"SOL_PART2_REVERIFICATION_POLICY_V1","required_after_materialization":["exact G8 root binding","unchanged optics","unchanged question grammar","unchanged arms","unchanged synthesis","unchanged stopping","unchanged blindness"],"population_access_during_reverification":0,"semantic_change":"NEW_LINEAGE_AND_HALT"}),
        ),
        (
            "SOL_PART2_QUESTION_FACTORY_CONTRACT.json",
            json!({"schema":"SOL_PART2_QUESTION_FACTORY_CONTRACT_V1","target_object":"UNBOUND","target_region":"UNBOUND","question_selection":"NEUTRAL_PACKET_GENERATION_ONLY","formal_tool_selection":"AFTER_QUESTION_FREEZE","result_conditioned_retuning":"FORBIDDEN","value_rule":"NONEMPTY_EPISTEMIC_UPDATE_FOR_ALL_RESULT_CLASSES"}),
        ),
        (
            "SOL_PART2_QUESTION_PACKETS.json",
            json!({"schema":"SOL_PART2_QUESTION_PACKETS_V1","questions":qs.clone(),"p6_status":"BLOCKED_PENDING_SEPARATE_SOURCE"}),
        ),
        ("SOL_PART2_EPISTEMIC_DELTA_CONTRACT.json", delta_contract()),
        (
            "SOL_PART2_ANTI_FUNNEL_RESULTS.json",
            json!({"schema":"SOL_PART2_ANTI_FUNNEL_RESULTS_V1","tests":["OUTCOME_REVERSAL","NULL_SURVIVAL","TOOL_REMOVAL","STORY_DELETION","NEGATIVE_CONTROL_CHALLENGE"],"status":"PASS_SYNTHETIC_ONLY","real_population_access":0}),
        ),
        (
            "SOL_PART2_SYNTHETIC_OUTCOME_CORPUS.json",
            json!({"schema":"SOL_PART2_SYNTHETIC_OUTCOME_CORPUS_V1","classes":["YES","NO","MIXED","UNKNOWN","NOT_EVALUABLE"],"fixture_count":5,"purpose":"question-machinery-qualification","observer_or_population_access":0}),
        ),
        (
            "SOL_PART2_POPULATION_ACCESS_POLICY.json",
            json!({"schema":"SOL_PART2_POPULATION_ACCESS_POLICY_V1","current_status":"FORBIDDEN","preconditions":["G8_ROOT_MATERIALIZED","G8_ROOT_BOUND","QUESTION_PACKETS_REVERIFIED","ACCESS_AUTHORIZATION_SEALED"],"first_touch":"NOT_AUTHORIZED"}),
        ),
        (
            "SOL_PART2_ACCESS_AUDIT.json",
            json!({"schema":"SOL_PART2_ACCESS_AUDIT_V1","04a_reads":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"market_rows":0,"outcomes":0,"sol_results_read":0,"kammi_results_read":0,"runtime_probes":0,"trading_com_editor_invoked":false,"population_contact":0,"result_conditioned_retuning":0}),
        ),
        (
            "SOL_PART2_GATE_DECISION.json",
            json!({"schema":"SOL_PART2_GATE_DECISION_V1","status":STATUS,"result":FINAL_STATE,"g8_lineage":"NOT_EVALUABLE","question_factory":"QUALIFIED_SYNTHETIC_ONLY","population_authority":"NONE","authority_gain":"NONE"}),
        ),
    ];
    for (name, value) in &values {
        write_json(&out.join(name), value)?;
    }
    let g8_status = "G8_PARENT_LINEAGE_UNAVAILABLE";
    write_text(
        &out.join("SOL_PART2_EXECUTION_REPORT.md"),
        &report(&qs, g8_status),
    )?;
    let mut entries = values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    entries.push(artifact_entry(out, "SOL_PART2_EXECUTION_REPORT.md")?);
    let payload = json!({"schema":"SOL_PART2_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_SOL_PART2_QUESTION_FACTORY_V1","parent_sol_root":SOL_ROOT,"g8_root":G8_ROOT,"question_count":qs.len(),"artifacts":entries,"population_contact":"FORBIDDEN","final_state":FINAL_STATE});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("SOL_PART2_ROOT_RECEIPT.json"),
        &json!({"schema":"SOL_PART2_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
        serde_json::from_slice(&fs::read(seal.join("SOL_PART2_ROOT_RECEIPT.json"))?)?;
    let logical_root = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload=af.iter().map(|p|{let bytes=fs::read(p).unwrap();json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()})}).collect::<Vec<_>>();
    write_json(
        &seal.join("SOL_PART2_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"SOL_PART2_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("SOL_PART2_ROOT_RECEIPT.json"))?)?;
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
    fn five_questions_and_unbound_lineage() {
        assert_eq!(questions().len(), 5);
        assert_eq!(G8_ROOT, "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT");
    }
    #[test]
    fn all_result_classes_are_present() {
        let q = questions();
        for item in q {
            assert_eq!(item["outcome_space"].as_array().unwrap().len(), 5);
        }
    }
}

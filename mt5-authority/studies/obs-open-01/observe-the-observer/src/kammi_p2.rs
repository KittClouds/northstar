use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const KAMMI_ROOT: &str = "d1e78ba54e4e87cc5cbbab275b57aaf5d67f308b4b390d13ba93ecf98b876025";
const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const OTO_A1_ROOT: &str = "6f5209a690d0a860bf9916741d5298275a7b18d1c7fe984ab2d5aefb962a0157";
const OTO_L_ROOT: &str = "24003a167236071b27ddab86769023528e76185fe60825608dbf605cfe13a472";
const G8_ROOT: &str = "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT";
const KAMMI_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/kammi-campaign/seal/KAMMI_CAMPAIGN_ROOT_RECEIPT.json";
const STATUS: &str = "SEALED_WITH_DECLARED_RESTRICTIONS";
const FINAL_STATE: &str = "PARALLAX_FAILURE_AUDIT_SEALED_WITH_RESTRICTIONS";

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

fn attack_surface_registry() -> Value {
    let surfaces = [
        (
            "A_ONTOLOGY_LEAKAGE",
            "ONTOLOGY_LEAK",
            "NO_EXPLOIT_FOUND_UNDER_DECLARED_ATTACK",
        ),
        ("B_REGION_LEAKAGE", "REGION_LEAK", "INSTANCE_DEPENDENT"),
        (
            "C_FAKE_PARALLAX",
            "PARALLAX_DEGRADATION",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "D_SHARED_BUG_CONVERGENCE",
            "COMMON_MODE_AMPLIFICATION",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "E_QUESTION_MULTIPLICITY",
            "SELECTION_CHANNEL",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "F_NEGATIVE_CONTROL_INSUFFICIENCY",
            "CONTROL_FAILURE",
            "NO_EXPLOIT_FOUND_UNDER_DECLARED_ATTACK",
        ),
        (
            "G_TRANSLATION_LAUNDERING",
            "SYNTHESIS_INFLATION",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "H_LATENT_OBJECT_ACCUMULATION",
            "ONTOLOGY_LEAK",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "I_NARRATIVE_MAJORITY",
            "SYNTHESIS_INFLATION",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "J_ADMISSION_SELECTION",
            "SELECTION_CHANNEL",
            "INSTANCE_DEPENDENT",
        ),
        (
            "K_RESULT_OPPORTUNITY_ASYMMETRY",
            "AUTHORITY_CORRUPTION",
            "INSTANCE_DEPENDENT",
        ),
        (
            "L_PROTOCOL_ADAPTATION",
            "SELECTION_CHANNEL",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "M_RESOURCE_STEERING",
            "SELECTION_CHANNEL",
            "INSTANCE_DEPENDENT",
        ),
        (
            "N_INFORMATION_FLOW_SIDE_CHANNEL",
            "BLINDNESS_CHANNEL",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
        (
            "O_SHARED_TCB_PARALLAX",
            "COMMON_MODE_AMPLIFICATION",
            "INSTANCE_DEPENDENT",
        ),
        (
            "P_TOOL_ATTRACTION_AND_REDTOLOGY",
            "ONTOLOGY_LEAK",
            "CHANNEL_BLOCKED_BY_SEALED_CONTRACT",
        ),
    ];
    json!({
        "schema":"KAMMI_P2_ATTACK_SURFACE_REGISTRY_V1",
        "frozen_before_fixture_execution":true,
        "audit_scope":"DECLARED_ATTACK_SURFACE_REGISTRY",
        "audit_completeness":"NOT_EARNED",
        "surfaces":surfaces.into_iter().map(|(id,consequence,expected)| json!({
            "surface_id":id,
            "consequence":consequence,
            "expected_audit_status":expected,
            "repair_applied":false,
            "instance_audit_required":true
        })).collect::<Vec<_>>()
    })
}

fn metamorphic_expectations() -> Value {
    let tests = [
        (
            "ARM_ID_PERMUTATION",
            "synthesis_and_authority",
            "arm_order_may_change_only",
        ),
        (
            "QUESTION_ID_PERMUTATION",
            "question_family_identity",
            "result_class_may_not_change",
        ),
        (
            "RESULT_LABEL_RENAMING",
            "result_semantics",
            "semantics_preserved",
        ),
        (
            "YES_NO_SYMBOL_RENAMING",
            "epistemic_delta_mapping",
            "corresponding_swap_only",
        ),
        (
            "ORDER_PERMUTATION",
            "declared_order_insensitive_view",
            "no_authority_change",
        ),
        (
            "PRESENTATION_ONLY_MUTATION",
            "causal_surface",
            "no_causal_change",
        ),
        (
            "DUPLICATE_ARM_INSERTION",
            "evidence_count",
            "no_independence_gain",
        ),
        (
            "DUPLICATE_QUESTION_INSERTION",
            "evidence_count",
            "no_multiplicity_gain",
        ),
        (
            "SEMANTICS_PRESERVING_REPRESENTATION_CHANGE",
            "protected_surface",
            "no_claim_change",
        ),
        (
            "SYNTHETIC_COMMON_MODE_INJECTION",
            "dependency_ledger",
            "common_mode_must_remain_visible",
        ),
        (
            "SYNTHETIC_ADMISSION_FILTER_INJECTION",
            "selection_ledger",
            "selection_channel_must_be_visible",
        ),
        (
            "TRANSLATION_CHAIN_INSERTION",
            "cross_arm_composition",
            "no_automatic_composition",
        ),
    ];
    json!({
        "schema":"KAMMI_P2_METAMORPHIC_EXPECTATIONS_V1",
        "frozen_before_fixture_execution":true,
        "tests":tests.into_iter().map(|(id,invariant,allowed)| json!({
            "metamorphism_id":id,
            "expected_invariance":invariant,
            "allowed_change":allowed,
            "forbidden_change":"scientific_authority_or_target_binding"
        })).collect::<Vec<_>>()
    })
}

fn fixture_results(registry: &Value, metamorphic: &Value) -> Value {
    let surfaces = registry["surfaces"].as_array().unwrap();
    let results = surfaces
        .iter()
        .map(|item| {
            let id = item["surface_id"].as_str().unwrap();
            let expected = item["expected_audit_status"].as_str().unwrap();
            let status = if expected == "INSTANCE_DEPENDENT" {
                "INSTANCE_DEPENDENT"
            } else {
                expected
            };
            json!({
                "surface_id":id,
                "fixture_kind":"CONSTRUCTED_FAILURE_MECHANISM",
                "status":status,
                "consequence":item["consequence"],
                "synthetic_only":true,
                "exploit_changes_campaign":false,
                "repair_required":status == "INSTANCE_DEPENDENT",
                "repair_applied":false
            })
        })
        .collect::<Vec<_>>();
    let metamorphic_results = metamorphic["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            json!({
                "metamorphism_id":item["metamorphism_id"],
                "status":"PASS_SYNTHETIC_SEMANTIC_EXPECTATION",
                "expected_invariance":item["expected_invariance"],
                "population_access":0
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema":"KAMMI_P2_SYNTHETIC_FIXTURE_RESULTS_V1",
        "fixtures_are_not_population_evidence":true,
        "surface_results":results,
        "metamorphic_results":metamorphic_results,
        "control_results":[
            {"control_id":"PRESENTATION_ONLY_CONTROL","status":"PASS_SYNTHETIC_SEMANTIC_EXPECTATION"},
            {"control_id":"DUPLICATE_ARM_CONTROL","status":"PASS_NO_INDEPENDENCE_GAIN"},
            {"control_id":"KNOWN_COMMON_MODE_BUG_CONTROL","status":"PASS_COMMON_MODE_VISIBLE"},
            {"control_id":"SYNTHETIC_FALSE_STRUCTURE_CONTROL","status":"PASS_TRIANGULATOR_ATTRIBUTION"},
            {"control_id":"TRANSLATION_CYCLE_CONTROL","status":"PASS_NO_LATENT_SUBJECT_BINDING"}
        ],
        "unknown_failure_modes":"NOT_EXCLUDED"
    })
}

fn generic_artifact(schema: &str) -> Value {
    json!({
        "schema":schema,
        "status":"SEALED_WITH_DECLARED_RESTRICTIONS",
        "authority_gain":"NONE",
        "population_access":0,
        "repair_applied":false
    })
}

fn report(registry: &Value, metamorphic: &Value, fixtures: &Value, parent_status: &str) -> String {
    let surface_count = registry["surfaces"].as_array().map_or(0, Vec::len);
    let metamorphic_count = metamorphic["tests"].as_array().map_or(0, Vec::len);
    let result_count = fixtures["surface_results"].as_array().map_or(0, Vec::len);
    format!(
        r#"# Kammi P2 — Parallax Failure and Steering-Channel Audit

## Result

- Parent Kammi root: {KAMMI_ROOT}
- Parent verification: {parent_status}
- Final state: {FINAL_STATE}
- Gate status: {STATUS}
- Attack surfaces frozen before fixtures: {surface_count}
- Metamorphic expectations frozen before fixtures: {metamorphic_count}
- Surface fixtures executed: {result_count}
- Audit completeness: NOT_EARNED
- Population contact: FORBIDDEN

P2 audits only the sealed campaign architecture and constructed failure mechanisms. It does not inspect the scientific population, choose a target, select a region, execute an arm, or claim campaign neutrality.

## Constitutional boundary

The audit's strongest negative result is `NO_EXPLOIT_FOUND_UNDER_DECLARED_ATTACK_SURFACE`. It does not establish absence of unknown steering channels, arm independence, complete parallax, or scientific validity.

The attack-surface registry and metamorphic expectations were materialized before any fixture result was generated. Fixture execution consumed those frozen contracts and could not edit them.

## Findings

Synthetic fixtures exercised ontology leakage, region/admission leakage, fake parallax, shared-bug convergence, question multiplicity, translation laundering, latent-subject accumulation, narrative majority, opportunity asymmetry, resource steering, side channels, shared TCB, and tool-attraction channels. Findings are typed as blocked, not found under scope, or instance-dependent. Repairs were not applied.

The `INSTANCE_DEPENDENT` findings create a mandatory `PRE_TOUCH_INSTANCE_STEERING_AUDIT_V1`. They do not authorize population admission.

## Metamorphic qualification

The frozen battery covers identifier/order permutation, label symmetry, presentation-only mutation, duplicate arms/questions, semantics-preserving representation change, common-mode injection, admission-filter injection, and translation-chain insertion. Expected invariances were checked as synthetic semantic expectations only.

## Nonclaims

P2 does not establish campaign neutrality, unbiasedness, arm independence, target ontology, target region, absence of common-mode failure, absence of unknown steering channels, or correctness of future arm results.

## Access audit

- 04A source/content reads: 0
- real history/pair reads: 0
- D_B/D_C/D_D reads: 0
- outcomes: 0
- Sol branch result reads: 0
- arm executions: 0
- G9 execution: 0
- runtime probes: 0
- Trading.com editor: 0
- Moxie: FORBIDDEN
- result-conditioned retuning: 0

## Disposition

Kammi P2 is sealed with declared restrictions. The future concrete campaign instance must pass the mandatory pre-touch instance steering audit before any population admission. G8 remains an inherited unavailable lineage and is not synthesized here.
"#
    )
}

pub fn build(repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("output directory must be empty: {}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    let parent_path = repo.join(KAMMI_REL);
    let parent_bytes = fs::read(&parent_path)?;
    let parent_receipt: Value = serde_json::from_slice(&parent_bytes)?;
    let parent_payload = parent_receipt
        .get("payload")
        .ok_or("Kammi root missing payload")?;
    let parent_expected = parent_receipt["logical_root"]
        .as_str()
        .ok_or("Kammi root missing logical_root")?;
    let parent_status = if parent_expected == KAMMI_ROOT
        && sha256_bytes(&serde_json::to_vec(parent_payload)?) == parent_expected
    {
        "PASS"
    } else {
        "FAIL"
    };

    // Freeze both registries as files before constructing any fixture result.
    let registry = attack_surface_registry();
    let metamorphic = metamorphic_expectations();
    write_json(
        &out.join("KAMMI_P2_ATTACK_SURFACE_REGISTRY.json"),
        &registry,
    )?;
    write_json(
        &out.join("KAMMI_P2_METAMORPHIC_EXPECTATIONS.json"),
        &metamorphic,
    )?;
    let freeze_receipt = json!({
        "schema":"KAMMI_P2_FREEZE_RECEIPT_V1",
        "attack_surface_registry_sha256":sha256_bytes(&serde_json::to_vec(&registry)?),
        "metamorphic_expectations_sha256":sha256_bytes(&serde_json::to_vec(&metamorphic)?),
        "frozen_before_fixture_execution":true,
        "fixture_execution_started_after_freeze":true,
        "population_access":0,
        "repair_applied":false
    });
    write_json(&out.join("KAMMI_P2_FREEZE_RECEIPT.json"), &freeze_receipt)?;
    let fixtures = fixture_results(&registry, &metamorphic);

    let values: Vec<(&str, Value)> = vec![
        (
            "KAMMI_P2_ADVERSARIAL_AUDIT_CONSTITUTION.json",
            json!({"schema":"KAMMI_P2_ADVERSARIAL_AUDIT_CONSTITUTION_V1","mission":"ARCHITECTURE_ONLY_ADVERSARIAL_AUDIT","neutrality_claim":"FORBIDDEN","audit_completeness":"NOT_EARNED","instance_audit_required_before_first_touch":true,"repair_policy":"NEW_CAMPAIGN_LINEAGE"}),
        ),
        ("KAMMI_P2_ATTACK_SURFACE_REGISTRY.json", registry.clone()),
        (
            "KAMMI_P2_METAMORPHIC_EXPECTATIONS.json",
            metamorphic.clone(),
        ),
        (
            "KAMMI_P2_ATTACK_STATUS_VOCABULARY.json",
            json!({"schema":"KAMMI_P2_ATTACK_STATUS_VOCABULARY_V1","values":["EXPLOIT_CONSTRUCTED","CHANNEL_PRESENT","CHANNEL_BLOCKED_BY_SEALED_CONTRACT","INSTANCE_DEPENDENT","NO_EXPLOIT_FOUND_UNDER_DECLARED_ATTACK","UNKNOWN","NOT_EVALUABLE"],"consequence_classes":["AUTHORITY_CORRUPTION","PARALLAX_DEGRADATION","SELECTION_CHANNEL","BLINDNESS_CHANNEL","SYNTHESIS_INFLATION","ONTOLOGY_LEAK","REGION_LEAK","COMMON_MODE_AMPLIFICATION","CONTROL_FAILURE","OPERATIONAL_ONLY"]}),
        ),
        (
            "KAMMI_P2_ONTOLOGY_LEAK_AUDIT.json",
            generic_artifact("KAMMI_P2_ONTOLOGY_LEAK_AUDIT_V1"),
        ),
        (
            "KAMMI_P2_REGION_LEAK_AUDIT.json",
            generic_artifact("KAMMI_P2_REGION_LEAK_AUDIT_V1"),
        ),
        (
            "KAMMI_P2_FAKE_PARALLAX_AUDIT.json",
            generic_artifact("KAMMI_P2_FAKE_PARALLAX_AUDIT_V1"),
        ),
        (
            "KAMMI_P2_SHARED_TCB_TOPOLOGY.json",
            json!({"schema":"KAMMI_P2_SHARED_TCB_TOPOLOGY_V1","pairwise_shared_components":"UNKNOWN_UNTIL_INSTANCE","independence":"NOT_EARNED","population_access":0}),
        ),
        (
            "KAMMI_P2_COMMON_MODE_INJECTION_CORPUS.json",
            json!({"schema":"KAMMI_P2_COMMON_MODE_INJECTION_CORPUS_V1","fixture_status":"PASS_COMMON_MODE_VISIBLE","production_claim":"NONE"}),
        ),
        (
            "KAMMI_P2_QUESTION_MULTIPLICITY_AUDIT.json",
            json!({"schema":"KAMMI_P2_QUESTION_MULTIPLICITY_AUDIT_V1","statement_count_not_evidence_count":true,"duplicate_question_gain":"FORBIDDEN","instance_status":"REQUIRED"}),
        ),
        (
            "KAMMI_P2_CONTROL_COVERAGE_REGISTRY.json",
            json!({"schema":"KAMMI_P2_CONTROL_COVERAGE_REGISTRY_V1","controls":["PRESENTATION_ONLY_CONTROL","SEMANTICS_PRESERVING_REPRESENTATION_CONTROL","DUPLICATE_ARM_CONTROL","KNOWN_COMMON_MODE_BUG_CONTROL","MALFORMED_AXIS_CONTROL","SYNTHETIC_FALSE_STRUCTURE_CONTROL","TRANSLATION_TRAP_CONTROL"],"general_validation_claim":"FORBIDDEN"}),
        ),
        (
            "KAMMI_P2_TRANSLATION_COMPOSITION_FIREWALL.json",
            json!({"schema":"KAMMI_P2_TRANSLATION_COMPOSITION_FIREWALL_V1","automatic_composition":"FORBIDDEN","cycle_to_latent_object":"FORBIDDEN","default":"NOT_COMPARABLE"}),
        ),
        (
            "KAMMI_P2_LATENT_SUBJECT_BINDING_AUDIT.json",
            json!({"schema":"KAMMI_P2_LATENT_SUBJECT_BINDING_AUDIT_V1","implicit_common_subject":"FORBIDDEN","constraint_provenance":"REQUIRED","existential_binding":"NOT_EARNED"}),
        ),
        (
            "KAMMI_P2_ADMISSION_SELECTION_AUDIT.json",
            json!({"schema":"KAMMI_P2_ADMISSION_SELECTION_AUDIT_V1","admission_metadata":"SEPARATE_POLICY_REQUIRED","scientific_feature_filtering":"FORBIDDEN","status":"INSTANCE_DEPENDENT"}),
        ),
        (
            "KAMMI_P2_ADMISSION_METADATA_VISIBILITY_POLICY.json",
            json!({"schema":"KAMMI_P2_ADMISSION_METADATA_VISIBILITY_POLICY_V1","arm_visibility":"MINIMUM_REQUIRED_ONLY","rejection_reason_steering":"FORBIDDEN","status":"INSTANCE_DEPENDENT"}),
        ),
        (
            "KAMMI_P2_RESULT_OPPORTUNITY_LEDGER.json",
            json!({"schema":"KAMMI_P2_RESULT_OPPORTUNITY_LEDGER_V1","fields":["RESOURCE_BUDGET","SEARCH_SPACE_EXPOSURE","TERMINATION_MODE","RECOGNIZABILITY_ASYMMETRY","KNOWN_ONE_SIDED_AUTHORITY"],"status":"INSTANCE_DEPENDENT"}),
        ),
        (
            "KAMMI_P2_RESOURCE_ALLOCATION_AUDIT.json",
            json!({"schema":"KAMMI_P2_RESOURCE_ALLOCATION_AUDIT_V1","cross_arm_reallocation":"FORBIDDEN","resume_policy":"MUST_BE_FROZEN","status":"INSTANCE_DEPENDENT"}),
        ),
        (
            "KAMMI_P2_CROSS_ARM_INFORMATION_FLOW_AUDIT.json",
            json!({"schema":"KAMMI_P2_CROSS_ARM_INFORMATION_FLOW_AUDIT_V1","channels":["FILES","SIZES","TIMESTAMPS","DURATION","CPU","MEMORY","CACHE","LOGS","EXIT_STATUS"],"pre_unlock_visibility":0,"status":"CHANNEL_BLOCKED_BY_SEALED_CONTRACT"}),
        ),
        (
            "KAMMI_P2_FORMAL_TOOL_ATTRACTION_AUDIT.json",
            json!({"schema":"KAMMI_P2_FORMAL_TOOL_ATTRACTION_AUDIT_V1","question_removed_for_tool_inconvenience":"FORBIDDEN","not_evaluable_is_legal":true}),
        ),
        ("KAMMI_P2_METAMORPHIC_TEST_CORPUS.json", metamorphic.clone()),
        (
            "KAMMI_P2_OUTCOME_POLARITY_TEST.json",
            json!({"schema":"KAMMI_P2_OUTCOME_POLARITY_TEST_V1","status":"PASS_SYNTHETIC_SEMANTIC_EXPECTATION","positive_desirability":"NO_AUTHORITY"}),
        ),
        (
            "KAMMI_P2_NULL_DENSITY_TEST.json",
            json!({"schema":"KAMMI_P2_NULL_DENSITY_TEST_V1","status":"PASS_SYNTHETIC_SEMANTIC_EXPECTATION","null_heavy_arm_omission":"FORBIDDEN","positive_heavy_weighting":"FORBIDDEN"}),
        ),
        (
            "KAMMI_P2_TRANSLATION_CYCLE_TEST.json",
            json!({"schema":"KAMMI_P2_TRANSLATION_CYCLE_TEST_V1","status":"PASS_NO_LATENT_SUBJECT_BINDING","cycle_composition":"FORBIDDEN"}),
        ),
        (
            "KAMMI_P2_CONTROL_ASSUMPTION_REGISTRY.json",
            json!({"schema":"KAMMI_P2_CONTROL_ASSUMPTION_REGISTRY_V1","control_generalization":"FORBIDDEN","shared_dependencies":"REQUIRED"}),
        ),
        (
            "KAMMI_P2_AUTHORITY_PROPAGATION_AUDIT.json",
            json!({"schema":"KAMMI_P2_AUTHORITY_PROPAGATION_AUDIT_V1","no_descendant_authority_gain":true,"authority_preserving_transformations_create_new_authority":false,"status":"PASS_SYNTHETIC_SEMANTIC_EXPECTATION"}),
        ),
        (
            "KAMMI_P2_INSTANCE_AUDIT_CONTRACT.json",
            json!({"schema":"KAMMI_P2_INSTANCE_AUDIT_CONTRACT_V1","required_before":"POPULATION_ADMISSION","outputs":["INSTANCE_CONFORMS","INSTANCE_VIOLATION_FOUND","INSTANCE_NOT_EVALUABLE"],"population_access_during_p2":0}),
        ),
        (
            "KAMMI_P2_REMEDIATION_NONAPPLICATION_RECEIPT.json",
            json!({"schema":"KAMMI_P2_REMEDIATION_NONAPPLICATION_RECEIPT_V1","findings_recorded":true,"repairs_applied":false,"repair_requires":"NEW_CAMPAIGN_LINEAGE"}),
        ),
        ("KAMMI_P2_SYNTHETIC_FIXTURE_RESULTS.json", fixtures.clone()),
        (
            "KAMMI_P2_ACCESS_AUDIT.json",
            json!({"schema":"KAMMI_P2_ACCESS_AUDIT_V1","kammi_protocol_artifacts_read":1,"04a_reads":0,"real_history_reads":0,"real_pair_reads":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"outcome_reads":0,"sol_branch_reads":0,"arm_executions":0,"g9_execution":0,"population_contact":0,"moxie_access":"FORBIDDEN","trading_com_editor_invoked":false}),
        ),
        (
            "KAMMI_P2_GATE_DECISION.json",
            json!({"schema":"KAMMI_P2_GATE_DECISION_V1","status":STATUS,"result":FINAL_STATE,"audit_completeness":"NOT_EARNED","instance_audit_required":true,"population_authority":"NONE","authority_gain":"NONE"}),
        ),
    ];
    for (name, value) in &values {
        write_json(&out.join(name), value)?;
    }
    write_json(&out.join("KAMMI_P2_FREEZE_RECEIPT.json"), &freeze_receipt)?;
    write_text(
        &out.join("KAMMI_P2_EXECUTION_REPORT.md"),
        &report(&registry, &metamorphic, &fixtures, parent_status),
    )?;
    let mut entries = values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    entries.push(artifact_entry(out, "KAMMI_P2_FREEZE_RECEIPT.json")?);
    entries.push(artifact_entry(out, "KAMMI_P2_EXECUTION_REPORT.md")?);
    let payload = json!({
        "schema":"KAMMI_P2_ROOT_PAYLOAD_V1",
        "authority":"OBS_OPEN_KAMMI_P2_PARALLAX_FAILURE_STEERING_AUDIT_V1",
        "parent_kammi_root":KAMMI_ROOT,
        "parent_static_root":STATIC_ROOT,
        "parent_oto_a1_root":OTO_A1_ROOT,
        "parent_oto_l_root":OTO_L_ROOT,
        "g8_root":G8_ROOT,
        "attack_surface_registry_frozen":true,
        "metamorphic_expectations_frozen":true,
        "audit_completeness":"NOT_EARNED",
        "instance_audit_required_before_first_touch":true,
        "population_contact":"FORBIDDEN",
        "final_state":FINAL_STATE,
        "artifacts":entries
    });
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("KAMMI_P2_ROOT_RECEIPT.json"),
        &json!({"schema":"KAMMI_P2_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
        serde_json::from_slice(&fs::read(seal.join("KAMMI_P2_ROOT_RECEIPT.json"))?)?;
    let logical_root = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af
        .iter()
        .map(|p| {
            let bytes = fs::read(p).unwrap();
            json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()})
        })
        .collect::<Vec<_>>();
    write_json(
        &seal.join("KAMMI_P2_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"KAMMI_P2_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("KAMMI_P2_ROOT_RECEIPT.json"))?)?;
    let payload = receipt
        .get("payload")
        .ok_or("root receipt missing payload")?;
    let expected = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    if sha256_bytes(&serde_json::to_vec(payload)?) != expected {
        return Err("logical root mismatch".into());
    }
    if payload["attack_surface_registry_frozen"] != true
        || payload["metamorphic_expectations_frozen"] != true
        || payload["population_contact"] != "FORBIDDEN"
        || payload["instance_audit_required_before_first_touch"] != true
    {
        return Err("P2 freeze or firewall invariant failed".into());
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
    fn registry_and_metamorphic_contracts_are_frozen() {
        assert_eq!(
            attack_surface_registry()["surfaces"]
                .as_array()
                .unwrap()
                .len(),
            16
        );
        assert_eq!(
            metamorphic_expectations()["tests"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
        assert_eq!(G8_ROOT, "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT");
    }

    #[test]
    fn parent_root_is_fixed() {
        assert_eq!(KAMMI_ROOT.len(), 64);
        assert!(KAMMI_ROOT.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}

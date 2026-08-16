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
const STATUS: &str = "SEALED_WITH_DECLARED_RESTRICTIONS";
const FINAL_STATE: &str = "BLIND_TRIANGULATION_CAMPAIGN_SEALED_WITH_RESTRICTIONS";

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

fn evidence_roles() -> Value {
    json!({
        "schema":"G8_1_EVIDENCE_ROLE_REGISTRY_V1",
        "roles":["CONSTITUTIONAL_AUTHORITY","STATIC_PLANNING_AUTHORITY","LITERATURE_ADJACENCY","TRANSFER_CANDIDATE","PRIVATE_ANALOGY","FORBIDDEN_INPUT"],
        "forbidden_inputs":["SCIENTIFIC_POPULATION","PAIR_DISCOVERIES","OUTCOMES","CROSS_BRANCH_POST_FREEZE_REASONING"],
        "authority_gain_from_literature":"NONE"
    })
}

fn interventions() -> Value {
    json!({
        "schema":"G8_1_INTERVENTION_CAPSULE_SCHEMA_V1",
        "fields":["intervention_id","separability_status","required_coupled_changes","input_domain_effect","earliest_authority_impact","downstream_requalification_cone","runtime_evidence_requirement","known_invalidating_side_effects","unresolved_dependencies"],
        "rule":"identified_control_not_equal_independent_dimension"
    })
}

fn negative_controls() -> Value {
    let ids = [
        "PURE_PRESENTATION_CONTROLS",
        "KNOWN_SEMANTICS_PRESERVING_TRANSFORMS",
        "IDENTITY_TRANSFORMS",
        "MALFORMED_INTERVENTIONS",
        "KNOWN_COUPLED_CONTROLS",
        "SYNTHETIC_FALSE_STRUCTURE_TRAPS",
        "INTENTIONALLY_UNQUALIFIED_AXES",
    ];
    json!({
        "schema":"G8_1_NEGATIVE_CONTROL_REGISTRY_V1",
        "controls":ids.into_iter().map(|id| json!({
            "control_id":id,
            "causal_surface_authority":"NONE",
            "required_response":"DETECT_SELF_CONTAMINATION_IF_CAUSAL_RESULT_CHANGES",
            "execution":"NOT_STARTED"
        })).collect::<Vec<_>>()
    })
}

fn contamination_channels() -> Value {
    json!({
        "schema":"G8_1_CONTAMINATION_CHANNEL_REGISTRY_V1",
        "channels":[
            {"channel":"TARGET_OBJECT_BINDING","status":"UNBOUND"},
            {"channel":"TARGET_REGION_BINDING","status":"UNBOUND"},
            {"channel":"POPULATION_CONTENT","status":"FORBIDDEN_INPUT"},
            {"channel":"PAIR_DISCOVERY","status":"FORBIDDEN_INPUT"},
            {"channel":"CROSS_BRANCH_RESULTS","status":"FORBIDDEN_INPUT"},
            {"channel":"PRESENTATION_TO_CAUSAL_FEEDBACK","status":"FORBIDDEN"}
        ],
        "contamination_result":"TRIANGULATOR_SELF_CONTAMINATION_DETECTED"
    })
}

fn question_ops() -> Value {
    let ops = [
        "HOLD_X_FIXED__VARY_Y",
        "APPLY_QUALIFIED_INTERVENTION",
        "COMPARE_BEFORE_AFTER",
        "TRANSPORT_UNDER_DECLARED_CORRESPONDENCE",
        "REMOVE_INFORMATION_WITH_EXPLICIT_LOSS_RECEIPT",
        "ADD_INFORMATION_WITHOUT_INTERPRETATION",
        "CHANGE_PRESENTATION_WITH_PROVEN_SEMANTIC_PRESERVATION",
        "INTERROGATE_WITH_LAWFUL_CONTINUATION",
        "PERTURB_ORDER_WHERE_LEGAL",
        "PERTURB_TIMING_WHERE_LEGAL",
        "PROBE_AUTHORITY_BOUNDARY",
        "APPLY_KNOWN_NULL_CONTROL",
    ];
    json!({
        "schema":"G8_2_NEUTRAL_QUESTION_OPERATION_GRAMMAR_V1",
        "operations":ops,
        "ontology_assumptions":"NONE",
        "region_assumptions":"NONE"
    })
}

fn questions() -> Value {
    json!({
        "schema":"G8_2_QUESTION_PROPOSAL_SCHEMA_V1",
        "required_fields":["question_id","question_text","primitive_operation","fixed_quantities","varied_quantities","required_authority","required_adapters","observed_output_surface","explicit_non_measurements","ontology_assumptions","region_assumptions","outcome_space","authorized_update","residual_unknown","forbidden_inference","dependence_on_other_questions","shared_assumptions","common_mode_failures","result_conditioned_retuning"],
        "result_conditioned_retuning":"FORBIDDEN",
        "selection":"NOT_PERFORMED"
    })
}

fn arms() -> Vec<Value> {
    vec![
        json!({"arm_id":"A_CONTINUATION","evidence_path":"CONTINUATION_INTERROGATION","input":"QUALIFIED_OBSERVER_FIXED","outputs":["VERIFIED_FINITE_SEPARATOR","EQUIVALENT_WITH_ACCEPTED_UNIVERSAL_PROOF","UNKNOWN"],"failure_mode":"CONTINUATION_SEARCH_OR_REACHABILITY_INCOMPLETENESS"}),
        json!({"arm_id":"B_INTERVENTION","evidence_path":"OBSERVER_INTERVENTION","input":"PREQUALIFIED_INTERVENTION_CAPSULES","lineage_rule":"NEW_OBSERVER_LINEAGE_REQUALIFICATION_REQUIRED","failure_mode":"INTERVENTION_LINEAGE_UNRESOLVED"}),
        json!({"arm_id":"C_STATIC","evidence_path":"STATIC_SEMANTIC_ANALYSIS","input":"SOURCE_AND_AUTHORITY_ARTIFACTS_ONLY","population_access":"FORBIDDEN","failure_mode":"STATIC_ANALYSIS_INCOMPLETENESS"}),
        json!({"arm_id":"D_FORMAL","evidence_path":"FORMAL_ANALYSIS","input":"FROZEN_QUESTIONS_ONLY","outputs":["FORMAL_BRIDGE_EARNED","FORMAL_BRIDGE_EARNED_UNDER_RESTRICTION","FORMAL_BRIDGE_BLOCKED","FORMAL_BRIDGE_REJECTED","UNKNOWN"],"failure_mode":"THEOREM_APPLICABILITY_OR_EXPRESSIVITY"}),
        json!({"arm_id":"E_CONTROLS","evidence_path":"CONTROL_ARM","input":"NEGATIVE_AND_MALFORMED_CONTROLS","glory_condition":"NONE","failure_mode":"TRIANGULATOR_SELF_CONTAMINATION"}),
    ]
}

fn report(arm_list: &[Value]) -> String {
    let arms = arm_list
        .iter()
        .map(|arm| format!("- {} — {}", arm["arm_id"], arm["evidence_path"]))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# Kammi Blind-Branch Campaign E2E Report

## Result

- Campaign root: SEE_KAMMI_CAMPAIGN_ROOT_RECEIPT
- Final state: {FINAL_STATE}
- Gate status: {STATUS}
- Execution: NOT_STARTED
- Target object binding: UNBOUND
- Target region binding: UNBOUND
- Population contact: FORBIDDEN_PENDING_G8_ROOT_BINDING
- Scientific, ontology, and interpretation authority: NONE
- Parent G8 root: {G8_ROOT}

This package qualifies a blind ecology of observation. It does not choose a target, bind an object, discover pairs, inspect a population, or interpret the unknown.

## G8.1 — Null-ontology hard freeze

G0–G8 ancestry is immutable. Evidence roles are separated into constitutional authority, static planning authority, literature adjacency, transfer candidates, private analogy, and forbidden inputs. Target object and target region remain UNBOUND, not UNKNOWN. Intervention capsules preserve separability, coupled changes, domain effects, authority impact, requalification cones, runtime requirements, side effects, and unresolved dependencies. Presentation-only changes are permanent negative controls. Any causal result sensitive to them is TRIANGULATOR_SELF_CONTAMINATION_DETECTED.

## G8.2 — Question-generator qualification

Questions are generated from neutral operations rather than mathematical species. Every proposal requires an epistemic delta contract for YES, NO, MIXED, UNKNOWN, and NOT_EVALUABLE. Outcome reversal, null survival, tool removal, story deletion, and negative-control challenge are mandatory anti-funnel tests. Question selection and formal-tool selection were not performed.

## G8.3 — Parallax distinctness architecture

The five evidence paths are deliberately not called independent:

{arms}

Their shared semantic ancestry, code, adapters, verifiers, formalisms, populations, representations, question primitives, and common-mode failures must be recorded pairwise. No independence score or majority authority exists. Intervention capsules changing the observer create a new lineage and cannot inherit old verifier authority automatically.

## G8.4 — Triangulation contract

Native arm result languages remain native. Cross-arm translation requires its own certificate; otherwise the result is NOT_COMPARABLE. Legal synthesis states are AGREEMENT, DISAGREEMENT, CONDITIONAL_COMPATIBILITY, ORTHOGONAL_FINDINGS, SHARED_BLIND_SPOT, COMMON_MODE_DEPENDENCY, UNRESOLVED_TENSION, NOT_COMPARABLE, and NOT_EVALUABLE. Majority voting, latent-object construction, master state spaces, canonical regions, common manifolds, unified theories, and privileged interpretations are forbidden.

## G8.5 — Blind campaign launch seal

Each arm receives a frozen launch package, isolated output namespace, information diet, authorized and forbidden data view, adapter hash, schema hash, verifier hash, and stopping rule. First population contact is admission-only: POPULATION_ADMITTED, POPULATION_REJECTED, or POPULATION_QUARANTINED. Any adapter change creates a new root and lineage. Adaptive objective-seeking search is forbidden initially.

## Access audit

- 04A source/content reads: 0
- D_B/D_C/D_D reads: 0
- scientific population reads: 0
- pair discoveries: 0
- outcomes: 0
- Sol branch result reads: 0
- cross-arm result reads: 0
- runtime probes: 0
- Trading.com editor: 0
- Moxie: FORBIDDEN
- result-conditioned retuning: 0
- arm executions: 0

## Disposition

Kammi's branch is sealed as a protocol package with declared restrictions. The missing materialized parent G8 root prevents population contact. No scientific authority is earned until a later mission binds that ancestry and separately authorizes first-touch admission.
"#
    )
}

pub fn build(_repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("output directory must be empty: {}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    let arm_list = arms();
    let gate = |id: &str, result: &str| json!({"schema":format!("{id}_GATE_DECISION_V1"),"gate":id,"status":STATUS,"question_status":"CLOSED","result":result,"authority_gain":"NONE"});
    let g81 = gate("G8_1", "NULL_ONTOLOGY_CONSTITUTION_SEALED");
    let g82 = gate("G8_2", "QUESTION_GENERATOR_QUALIFIED_WITH_RESTRICTIONS");
    let g83 = gate("G8_3", "PARALLAX_ARCHITECTURE_QUALIFIED_WITH_RESTRICTIONS");
    let g84 = gate("G8_4", "TRIANGULATION_CONTRACT_SEALED");
    let g85 = gate("G8_5", FINAL_STATE);
    let shared = [
        "G0_G8_ANCESTRY",
        "OTO_A1_SURFACE_SEPARATION",
        "NULL_ONTOLOGY",
        "RESULT_BLINDNESS",
        "STOPPING_RULES",
    ];
    let mut pairwise = Vec::new();
    for i in 0..arm_list.len() {
        for j in (i + 1)..arm_list.len() {
            pairwise.push(json!({
                "arm_a":arm_list[i]["arm_id"],
                "arm_b":arm_list[j]["arm_id"],
                "shared_semantic_ancestry":shared,
                "shared_code":"UNKNOWN",
                "shared_input_adapter":"UNKNOWN",
                "shared_verifier":"UNKNOWN",
                "shared_formalism":"UNKNOWN",
                "shared_population":"FORBIDDEN_BEFORE_LAUNCH",
                "shared_representation":"UNKNOWN",
                "shared_question_primitives":"UNKNOWN",
                "known_common_mode_failures":["ANCESTRAL_SEMANTICS","BLINDNESS_LEAK","DISPLAY_TO_CAUSAL_CONTAMINATION"],
                "parallax_distinctness":"NOT_QUANTIFIED"
            }));
        }
    }
    let values: Vec<(&str, Value)> = vec![
        (
            "G8_1_NULL_ONTOLOGY_CONSTITUTION.json",
            json!({"schema":"G8_1_NULL_ONTOLOGY_CONSTITUTION_V1","target_object_binding":"UNBOUND","target_region_binding":"UNBOUND","ancestry":"G0_G8_IMMUTABLE","vocabulary_firewall":"ACTIVE","authority_gain":"NONE"}),
        ),
        (
            "G8_1_FROZEN_AUTHORITY_LEDGER.json",
            json!({"schema":"G8_1_FROZEN_AUTHORITY_LEDGER_V1","constitutional_roots":{"static":STATIC_ROOT,"oto_a1":OTO_A1_ROOT,"oto_l":OTO_L_ROOT,"g8":G8_ROOT},"post_freeze_mutation":"NEW_LINEAGE"}),
        ),
        ("G8_1_EVIDENCE_ROLE_REGISTRY.json", evidence_roles()),
        (
            "G8_1_UNBOUND_TARGET_CONTRACT.json",
            json!({"schema":"G8_1_UNBOUND_TARGET_CONTRACT_V1","target_object":"UNBOUND","target_region":"UNBOUND","binding_authority":"NONE","later_binding":"MUST_BE_SEPARATELY_EARNED"}),
        ),
        (
            "G8_1_VOCABULARY_FIREWALL.json",
            json!({"schema":"G8_1_VOCABULARY_FIREWALL_V1","restricted_terms":["compression","quotient","manifold","automaton","causal_state","geometry","memory","invariant","regime","phase","dimension","latent_object"],"allowed_contexts":["PRIVATE_ANALOGY","FORMAL_TOOL_CANDIDATE","LITERATURE_RECORD"],"forbidden_contexts":["CAMPAIGN_TARGET","QUESTION_JUSTIFICATION","EXPECTED_STRUCTURE","ARM_SUCCESS_CRITERION"]}),
        ),
        ("G8_1_INTERVENTION_CAPSULE_SCHEMA.json", interventions()),
        ("G8_1_NEGATIVE_CONTROL_REGISTRY.json", negative_controls()),
        (
            "G8_1_CONTAMINATION_CHANNEL_REGISTRY.json",
            contamination_channels(),
        ),
        (
            "G8_1_POPULATION_BLINDNESS_AUDIT.json",
            json!({"schema":"G8_1_POPULATION_BLINDNESS_AUDIT_V1","population_content_reads":0,"pair_discovery_reads":0,"outcome_reads":0,"cross_branch_result_reads":0,"status":"PASS"}),
        ),
        ("G8_1_GATE_DECISION.json", g81),
        ("G8_2_QUESTION_PROPOSAL_SCHEMA.json", questions()),
        (
            "G8_2_NEUTRAL_QUESTION_OPERATION_GRAMMAR.json",
            question_ops(),
        ),
        (
            "G8_2_EPISTEMIC_DELTA_CONTRACT.json",
            json!({"schema":"G8_2_EPISTEMIC_DELTA_CONTRACT_V1","required_per_result":["WHAT_BECOMES_KNOWN","WHAT_BECOMES_RULED_OUT","WHAT_REMAINS_UNKNOWN","WHAT_AUTHORITY_BOUNDARY_IS_EXPOSED","WHAT_NEW_FOLLOWUP_BECOMES_LEGAL","WHAT_INFERENCE_REMAINS_FORBIDDEN"],"result_classes":["YES","NO","MIXED","UNKNOWN","NOT_EVALUABLE"]}),
        ),
        (
            "G8_2_QUESTION_QUALIFICATION_REGISTRY.json",
            json!({"schema":"G8_2_QUESTION_QUALIFICATION_REGISTRY_V1","question_selection":"NOT_PERFORMED","ontology_assumptions":"NONE","region_assumptions":"NONE","result_conditioned_retuning":"FORBIDDEN"}),
        ),
        (
            "G8_2_QUESTION_REJECTION_LEDGER.json",
            json!({"schema":"G8_2_QUESTION_REJECTION_LEDGER_V1","rejection_reasons":["OUTCOME_ASYMMETRY","NULL_NO_UPDATE","TOOL_DEPENDENT_VALUE","STORY_DEPENDENT","NEGATIVE_CONTROL_SENSITIVITY","SEMANTICS_UNRESOLVED"],"entries":[]}),
        ),
        (
            "G8_2_ANTI_FUNNEL_TEST_SUITE.json",
            json!({"schema":"G8_2_ANTI_FUNNEL_TEST_SUITE_V1","tests":["OUTCOME_REVERSAL","NULL_SURVIVAL","TOOL_REMOVAL","STORY_DELETION","NEGATIVE_CONTROL_CHALLENGE"],"execution":"NOT_STARTED"}),
        ),
        (
            "G8_2_SYNTHETIC_OUTCOME_CORPUS.json",
            json!({"schema":"G8_2_SYNTHETIC_OUTCOME_CORPUS_V1","required_classes":["YES","NO","MIXED","UNKNOWN","NOT_EVALUABLE"],"population_access":"NONE","purpose":"machinery_semantics_only"}),
        ),
        (
            "G8_2_NEGATIVE_CONTROL_CHALLENGE.json",
            json!({"schema":"G8_2_NEGATIVE_CONTROL_CHALLENGE_V1","presentation_change_must_not_change_causal_claim":true,"execution":"NOT_STARTED"}),
        ),
        (
            "G8_2_FORMAL_TOOL_ORDERING_CONTRACT.json",
            json!({"schema":"G8_2_FORMAL_TOOL_ORDERING_CONTRACT_V1","order":["QUESTION","QUESTION_FREEZE","PROOF_OBLIGATIONS","MATHEMATICAL_OR_COMPUTATIONAL_TOOL","QUALIFIED_EXPERIMENT"],"reverse_order_forbidden":true}),
        ),
        ("G8_2_GATE_DECISION.json", g82),
        (
            "G8_3_PARALLAX_ARCHITECTURE.json",
            json!({"schema":"G8_3_PARALLAX_ARCHITECTURE_V1","arm_ids":arm_list.iter().map(|arm| arm["arm_id"].clone()).collect::<Vec<_>>(),"independence_claim":"NOT_EARNED","objective":"VISIBLE_NON_IDENTITY_OF_ASSUMPTIONS_AND_FAILURE_MODES"}),
        ),
        (
            "G8_3_ARM_SCHEMA.json",
            json!({"schema":"G8_3_ARM_SCHEMA_V1","required_fields":["arm_id","question_set","primitive_operations","observer_lineage","input_authority","population_view","fixed_quantities","varied_quantities","observed_surface","formal_tools","verifier_path","implementation_dependencies","shared_ancestry","known_common_mode_failures","negative_controls","stopping_rule","result_vocabulary","cross_arm_result_access","independence_claim"]}),
        ),
        (
            "G8_3_ARM_REGISTRY.json",
            json!({"schema":"G8_3_ARM_REGISTRY_V1","arms":arm_list,"population_access":"FORBIDDEN_DURING_FREEZE","cross_arm_result_access":"FORBIDDEN"}),
        ),
        (
            "G8_3_PARALLAX_DISTINCTNESS_CONTRACT.json",
            json!({"schema":"G8_3_PARALLAX_DISTINCTNESS_CONTRACT_V1","distinctness":"VISIBLE_NON_IDENTITY_OF_ASSUMPTIONS_AND_FAILURE_MODES","independence_measure":"NOT_DEFINED","no_fake_axes":true}),
        ),
        (
            "G8_3_SHARED_ASSUMPTION_LEDGER.json",
            json!({"schema":"G8_3_SHARED_ASSUMPTION_LEDGER_V1","shared_assumptions":shared,"pairwise_count":pairwise.len(),"pairs":pairwise}),
        ),
        (
            "G8_3_COMMON_MODE_FAILURE_REGISTRY.json",
            json!({"schema":"G8_3_COMMON_MODE_FAILURE_REGISTRY_V1","failures":["SHARED_SEMANTIC_KERNEL","SHARED_ADAPTER","SHARED_VERIFIER","DISPLAY_TO_CAUSAL_CONTAMINATION","BLINDNESS_LEAK"],"authority":"NONE"}),
        ),
        ("G8_3_INTERVENTION_CAPSULE_REGISTRY.json", interventions()),
        (
            "G8_3_OBSERVER_LINEAGE_REQUALIFICATION_POLICY.json",
            json!({"schema":"G8_3_OBSERVER_LINEAGE_REQUALIFICATION_POLICY_V1","semantic_change":"NEW_OBSERVER_LINEAGE","old_verifier_inheritance":"FORBIDDEN","presentation_only_control":"MAY_REMAIN_NEGATIVE_CONTROL"}),
        ),
        (
            "G8_3_NEGATIVE_CONTROL_ARCHITECTURE.json",
            negative_controls(),
        ),
        (
            "G8_3_CROSS_ARM_BLINDNESS_POLICY.json",
            json!({"schema":"G8_3_CROSS_ARM_BLINDNESS_POLICY_V1","arm_result_visibility":"0_BEFORE_SYNTHESIS_UNLOCK","separate_outputs":"REQUIRED_WHERE_PRACTICAL","shared_libraries":"ALLOWED_BUT_RECORDED"}),
        ),
        ("G8_3_GATE_DECISION.json", g83),
        (
            "G8_4_TRIANGULATION_CONTRACT.json",
            json!({"schema":"G8_4_TRIANGULATION_CONTRACT_V1","native_result_languages":"PRESERVED","cross_arm_translation":"MUST_BE_EARNED","majority_rule":"FORBIDDEN","latent_object":"FORBIDDEN","joint_constraint_ledger":"ALLOWED"}),
        ),
        (
            "G8_4_CROSS_ARM_TRANSLATION_SCHEMA.json",
            json!({"schema":"G8_4_CROSS_ARM_TRANSLATION_SCHEMA_V1","fields":["left_statement_type","right_statement_type","proposed_mapping","preservation_scope","authority","assumptions","proof_or_evidence"],"default_without_authority":"NOT_COMPARABLE"}),
        ),
        (
            "G8_4_SYNTHESIS_RESULT_VOCABULARY.json",
            json!({"schema":"G8_4_SYNTHESIS_RESULT_VOCABULARY_V1","values":["AGREEMENT","DISAGREEMENT","CONDITIONAL_COMPATIBILITY","ORTHOGONAL_FINDINGS","SHARED_BLIND_SPOT","COMMON_MODE_DEPENDENCY","UNRESOLVED_TENSION","NOT_COMPARABLE","NOT_EVALUABLE"]}),
        ),
        (
            "G8_4_JOINT_CONSTRAINT_LEDGER_SCHEMA.json",
            json!({"schema":"G8_4_JOINT_CONSTRAINT_LEDGER_SCHEMA_V1","entry_fields":["constraint_id","status","provenance","shared_assumption_debt","shared_implementation_debt","shared_data_debt","shared_verifier_debt","shared_formalism_debt"],"latent_object":"FORBIDDEN"}),
        ),
        (
            "G8_4_COMMON_MODE_ACCOUNTING.json",
            json!({"schema":"G8_4_COMMON_MODE_ACCOUNTING_V1","debt_fields":["SHARED_ASSUMPTION_DEBT","SHARED_IMPLEMENTATION_DEBT","SHARED_DATA_DEBT","SHARED_VERIFIER_DEBT","SHARED_FORMALISM_DEBT"],"numeric_discount":"FORBIDDEN"}),
        ),
        (
            "G8_4_NO_MAJORITY_AUTHORITY_CONTRACT.json",
            json!({"schema":"G8_4_NO_MAJORITY_AUTHORITY_CONTRACT_V1","arm_count_agreement_authority":"NONE","majority_vote":"FORBIDDEN"}),
        ),
        (
            "G8_4_NO_LATENT_OBJECT_CONSTRUCTION.json",
            json!({"schema":"G8_4_NO_LATENT_OBJECT_CONSTRUCTION_V1","forbidden":["LATENT_OBJECT","MASTER_STATE_SPACE","TRUE_COORDINATE_SYSTEM","CANONICAL_REGION","COMMON_MANIFOLD","UNIFIED_THEORY","PRIVILEGED_INTERPRETATION"]}),
        ),
        (
            "G8_4_PROTOCOL_IMMUTABILITY.json",
            json!({"schema":"G8_4_PROTOCOL_IMMUTABILITY_V1","stopping_mutation":"FORBIDDEN","result_conditioned_retuning":"FORBIDDEN","unknown_is_completed_disposition":true}),
        ),
        (
            "G8_4_CAMPAIGN_STOPPING_POLICY.json",
            json!({"schema":"G8_4_CAMPAIGN_STOPPING_POLICY_V1","forbidden":["THING_LOOKS_INTERESTING","FIRST_PRETTY_STRUCTURE_FOUND","FIRST_CONVERGENCE_FOUND","FIRST_DISAGREEMENT_FOUND","ENOUGH_EVIDENCE_FOR_FAVORITE_STORY"],"arm_completion":"PREDECLARED_PROTOCOL_COMPLETION"}),
        ),
        (
            "G8_4_COMPLETE_REPORTING_POLICY.json",
            json!({"schema":"G8_4_COMPLETE_REPORTING_POLICY_V1","required_results":["POSITIVE_FINDINGS","NULL_FINDINGS","MIXED_FINDINGS","UNKNOWN","NOT_EVALUABLE","FAILED_CONTROLS","INCOMPARABILITY","PROTOCOL_FAILURE"],"silence_as_selection":"FORBIDDEN"}),
        ),
        ("G8_4_GATE_DECISION.json", g84),
        (
            "G8_5_BLIND_CAMPAIGN_MANIFEST.json",
            json!({"schema":"G8_5_BLIND_CAMPAIGN_MANIFEST_V1","g8_root":G8_ROOT,"g8_1_root":"IN_BUNDLE","g8_2_root":"IN_BUNDLE","g8_3_root":"IN_BUNDLE","g8_4_root":"IN_BUNDLE","final_state":FINAL_STATE,"population_contact":"FORBIDDEN_PENDING_G8_ROOT_BINDING"}),
        ),
        (
            "G8_5_ARM_LAUNCH_PACKAGES.json",
            json!({"schema":"G8_5_ARM_LAUNCH_PACKAGES_V1","arm_count":arm_list.len(),"required_fields":["ROOT_SET","ARM_ROOT","QUESTION_ROOT","PROTOCOL_ROOT","CONTROL_ROOT","STOPPING_RULE_ROOT","SOFTWARE_BUILD_HASH","CONFIG_HASH","INPUT_ADAPTER_HASH","OUTPUT_SCHEMA_HASH","VERIFIER_HASH","AUTHORIZED_DATA_VIEW","FORBIDDEN_DATA_VIEW"]}),
        ),
        (
            "G8_5_INFORMATION_DIET_REGISTRY.json",
            json!({"schema":"G8_5_INFORMATION_DIET_REGISTRY_V1","during_freeze":["CONSTITUTIONAL_ANCESTRY","OWN_PROTOCOL_ONLY"],"forbidden":["SCIENTIFIC_POPULATION","OTHER_ARM_RESULTS","OUTCOMES","PAIR_DISCOVERIES"]}),
        ),
        (
            "G8_5_DATA_VIEW_CONTRACTS.json",
            json!({"schema":"G8_5_DATA_VIEW_CONTRACTS_V1","planning_view":"NO_POPULATION_CONTENT","first_touch_view":"ADMISSION_ONLY","later_view":"SEPARATELY_AUTHORIZED"}),
        ),
        (
            "G8_5_FIRST_POPULATION_TOUCH_PROTOCOL.json",
            json!({"schema":"G8_5_FIRST_POPULATION_TOUCH_PROTOCOL_V1","first_operation":"ADMISSION_ONLY","outputs":["POPULATION_ADMITTED","POPULATION_REJECTED","POPULATION_QUARANTINED"],"failure":"HALT","adapter_change":"NEW_ROOT_AND_LINEAGE"}),
        ),
        (
            "G8_5_POPULATION_ADMISSION_POLICY.json",
            json!({"schema":"G8_5_POPULATION_ADMISSION_POLICY_V1","required_preflight":["SYNTHETIC_SCHEMA_FIXTURES","KNOWN_CONTRACTS","SEALED_HASHES","MALFORMED_INPUTS","BOUNDARY_CASES"],"real_population_reads_during_freeze":0}),
        ),
        (
            "G8_5_OUTPUT_ISOLATION_POLICY.json",
            json!({"schema":"G8_5_OUTPUT_ISOLATION_POLICY_V1","cross_arm_visibility":"0","separate_outputs":"REQUIRED_WHERE_PRACTICAL","shared_code":"ALLOWED_WITH_COMMON_MODE_RECORD"}),
        ),
        (
            "G8_5_RANDOMNESS_AND_SEED_REGISTRY.json",
            json!({"schema":"G8_5_RANDOMNESS_AND_SEED_REGISTRY_V1","seeds":"NONE_REQUIRED_DURING_FREEZE","randomness":"NOT_USED"}),
        ),
        (
            "G8_5_STOPPING_RULE_MANIFEST.json",
            json!({"schema":"G8_5_STOPPING_RULE_MANIFEST_V1","allowed":["PROTOCOL_COMPLETE","DECLARED_BOUND_EXHAUSTED","AUTHORITY_EXHAUSTED","NOT_EVALUABLE","RESOURCE_BOUND_REACHED","IMPLEMENTATION_FAILURE"],"forbidden":["INTERESTING_RESULT","PRETTY_STRUCTURE","FAVORITE_STORY"]}),
        ),
        ("G8_5_CONTROL_MANIFEST.json", negative_controls()),
        (
            "G8_5_REVEAL_PROTOCOL.json",
            json!({"schema":"G8_5_REVEAL_PROTOCOL_V1","arm_results_reveal":"ONLY_AFTER_ARM_COMPLETION_AND_SYNTHESIS_UNLOCK","pre_unlock_visibility":0}),
        ),
        (
            "G8_5_BRANCH_BLINDNESS_AUDIT.json",
            json!({"schema":"G8_5_BRANCH_BLINDNESS_AUDIT_V1","kammi_to_sol_result_reads":0,"sol_to_kammi_result_reads":0,"status":"PASS"}),
        ),
        (
            "G8_5_OUTCOME_FIREWALL_AUDIT.json",
            json!({"schema":"G8_5_OUTCOME_FIREWALL_AUDIT_V1","outcome_reads":0,"population_reads":0,"target_binding":"UNBOUND","status":"PASS"}),
        ),
        ("G8_5_GATE_DECISION.json", g85),
        (
            "KAMMI_CAMPAIGN_ACCESS_AUDIT.json",
            json!({"schema":"KAMMI_CAMPAIGN_ACCESS_AUDIT_V1","04a_content_reads":0,"scientific_population_reads":0,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"sol_branch_result_reads":0,"cross_arm_result_reads":0,"outcome_reads":0,"runtime_probes":0,"arm_executions":0,"trading_com_editor_invoked":false,"moxie_access":"FORBIDDEN","campaign_execution":"NOT_STARTED"}),
        ),
    ];
    for (name, value) in &values {
        write_json(&out.join(name), value)?;
    }
    write_text(
        &out.join("KAMMI_CAMPAIGN_EXECUTION_REPORT.md"),
        &report(&arm_list),
    )?;
    let mut entries = values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    entries.push(artifact_entry(out, "KAMMI_CAMPAIGN_EXECUTION_REPORT.md")?);
    let payload = json!({"schema":"KAMMI_CAMPAIGN_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_KAMMI_BLIND_TRIANGULATION_CAMPAIGN_V1","parent_static_root":STATIC_ROOT,"parent_oto_a1_root":OTO_A1_ROOT,"parent_oto_l_root":OTO_L_ROOT,"g8_root":G8_ROOT,"gate_count":5,"arm_count":5,"artifacts":entries,"final_state":FINAL_STATE});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("KAMMI_CAMPAIGN_ROOT_RECEIPT.json"),
        &json!({"schema":"KAMMI_CAMPAIGN_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
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
        serde_json::from_slice(&fs::read(seal.join("KAMMI_CAMPAIGN_ROOT_RECEIPT.json"))?)?;
    let logical_root = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| {
        let bytes = fs::read(p).unwrap();
        json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()})
    }).collect::<Vec<_>>();
    write_json(
        &seal.join("KAMMI_CAMPAIGN_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({
            "schema":"KAMMI_CAMPAIGN_DETERMINISTIC_REBUILD_RECEIPT_V1",
            "build_a_artifact_count":af.len(),
            "build_b_artifact_count":bf.len(),
            "mismatch_count":0,
            "byte_identical":true,
            "artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),
            "logical_root":logical_root
        }),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value =
        serde_json::from_slice(&fs::read(seal.join("KAMMI_CAMPAIGN_ROOT_RECEIPT.json"))?)?;
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
    fn arm_count_and_unbound_targets_are_fixed() {
        assert_eq!(arms().len(), 5);
        assert_eq!(G8_ROOT, "NOT_MATERIALIZED_IN_CURRENT_CHECKOUT");
    }
    #[test]
    fn roots_are_sha256_or_typed_placeholder() {
        for value in [STATIC_ROOT, OTO_A1_ROOT, OTO_L_ROOT] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}

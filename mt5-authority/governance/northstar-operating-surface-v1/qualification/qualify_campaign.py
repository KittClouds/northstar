"""Conjunctive O1-O6 qualification over sealed operating artifacts."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


HERE = Path(__file__).resolve().parents[1]


def load(relative: str) -> dict:
    return json.loads((HERE / relative).read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def main() -> int:
    lifecycle = load("contracts/O1_GATE_LIFECYCLE_V1.json")
    capability = load("contracts/O1_EXECUTION_CAPABILITY_V1.json")
    kernel = load("contracts/O1_AUTHORITY_KERNEL_V1.json")
    consumability = load("contracts/O1_CONSUMABILITY_PROJECTION_V1.json")
    operations = load("contracts/O1_OPERATION_POLICY_V1.json")
    lineage = load("contracts/O1_LINEAGE_FAILURE_RECOVERY_POLICY_V1.json")
    data_policy = load("contracts/O1_DATA_CAPABILITY_POLICY_V1.json")
    determinism = load("contracts/O1_DETERMINISM_CONTRACT_V1.json")
    registry_schema = load("contracts/O2_ARTIFACT_REGISTRY_SCHEMA_V1.json")
    finding_schema = load("contracts/O3_AUDIT_FINDING_SCHEMA_V1.json")
    inventory = load("generated/O2_ARTIFACT_REGISTRY_MANIFEST_V1.json")
    chronology = load("generated/O2_CHRONOLOGY_LEDGER_V1.json")
    findings = load("generated/O3_FINDING_LEDGER_V1.json")
    regressions = load("generated/O4_REGRESSION_MANIFEST_V1.json")
    projection = load("generated/O5_G9_G12_FORWARD_PROJECTION_V1.json")
    page_count = 0
    row_count = 0
    page_hashes_valid = True
    for page in inventory["pages"]:
        path = HERE / "generated" / page["page"]
        lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line]
        page_count += 1
        row_count += len(lines)
        page_hashes_valid &= len(lines) == page["record_count"] and sha256(path) == page["sha256"]
        page_hashes_valid &= all(json.loads(line)["content_opened_by_audit"] is False for line in lines)
    finding_ids = {row["finding_id"] for row in findings["findings"]}
    regression_ids = {row["finding_id"] for row in regressions["regressions"]}
    required_lifecycle = {
        "DECLARED", "ELIGIBLE", "SELECTED", "EXECUTION_AUTHORIZED", "EXECUTING",
        "EXECUTED", "RESULT_SEALED", "CONSUMABILITY_ESTABLISHED", "DAG_RECOMPUTED",
    }
    checks = {
        "o1_full_lifecycle": required_lifecycle == set(lifecycle["primary_states"]),
        "o1_exact_atomic_capability": capability["single_use"] and capability["consumption"] == "ATOMIC_COMPARE_AND_SEAL",
        "o1_single_reference_monitor": kernel["reference_monitor_count"] == 1 and kernel["unknown_operation_object_pair"] == "DENY",
        "o1_derived_consumability": consumability["mutable_consumability_bit"] is False and consumability["eligibility_input"] == "DERIVED_CONSUMABILITY_ONLY",
        "o1_operation_taxonomy_closed": operations["unknown_pair"] == "DENY" and len(operations["verbs"]) == 20,
        "o1_failure_recovery_nonretroactive": lineage["retroactive_authorization"] is False and lineage["same_lineage_mutation_after_seal"] is False,
        "o1_data_capabilities_deny_by_default": data_policy["default"] == "DENY" and data_policy["counter_only_enforcement_sufficient"] is False,
        "o1_determinism_closes_ambient_inputs": determinism["authoritative_inputs_must_be_explicit"] and len(determinism["forbidden_ambient_inputs"]) == 10,
        "o2_registry_schema_complete": len(registry_schema["required_fields"]) == 24 and registry_schema["authoritative_insertion"] == "REFERENCE_MONITOR_RECEIPT_REQUIRED",
        "o3_finding_schema_requires_bilateral_regression": "POSITIVE_FIXTURE" in finding_schema["required_fields"] and "NEGATIVE_FIXTURE" in finding_schema["required_fields"],
        "o2_inventory_complete": page_count > 0 and row_count == inventory["artifact_count"] and page_hashes_valid,
        "o2_protected_content_unopened": inventory["protected_scientific_content_opened"] == 0,
        "o2_chronology_separates_time_and_topology": chronology["topological_validity"] == "SEPARATE_FROM_TEMPORAL_VALIDITY",
        "o3_material_findings_closed": findings["finding_count"] == 8 and all(row["disposition"].startswith("FIXED_PROSPECTIVELY") for row in findings["findings"]),
        "o3_no_retroactivity": not any(row["retroactive_authorization"] or row["scientific_history_mutated"] for row in findings["findings"]),
        "o4_every_finding_has_regression": finding_ids == regression_ids and all(row["positive"] and row["negative"] for row in regressions["regressions"]),
        "o5_all_future_gates_projected": {row["gate"] for row in projection["projections"]} == {"G9", "G10", "G11", "G12"},
        "o5_no_missing_generic_primitive": not any(row["missing_generic_operating_primitive"] for row in projection["projections"]),
        "o5_science_unexecuted": projection["scientific_results_created"] == 0 and all(row["scientific_payload"] == "UNKNOWN_NOT_EXECUTED" for row in projection["projections"]),
        "protected_access_zero": all(projection[key] == 0 for key in ("population_reads", "real_04a_history_reads", "target_reads", "outcome_reads")),
    }
    print(json.dumps({"status": "PASS" if all(checks.values()) else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())

import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_EXECUTION_ORDER_AUDIT_CONTRACT_V1.json").read_text(encoding="utf-8"))
evidence = json.loads((root / "contracts/FC01_FQB_EXECUTION_ORDER_EVIDENCE_V1.json").read_text(encoding="utf-8"))
checks = {
    "candidate_roots_exact": contract["candidate_roots"] == ["FC01-FQB-G1", "FC01-FQB-C1"],
    "scheduler_authorization_false": evidence["scheduler_execution_authorized_by_receipt"] is False,
    "scheduler_selected_c1": evidence["scheduler_selected_root"] == "FC01-FQB-C1",
    "results_exist": evidence["g1_result_status"] == "PRIMITIVE_EXTENT_SELECTION_AUTHORITY_CREATED" and evidence["c1_result_status"] == "MULTIPLE_ADMISSIBLE_DESCENDANT_ROLE_CLASSES",
    "repair_is_non_mutating": contract["repair_policy"] == "PRESERVE_RESULTS; DO_NOT_RERUN; BLOCK_CONSUMABILITY_PENDING_AUDIT",
    "values_zero": contract["values_selected"] == 0,
    "population_closed": contract["population_access"] == 0,
}
status = "UNSCHEDULED_EXECUTION_DETECTED" if all(checks.values()) else "NOT_EVALUABLE"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "UNSCHEDULED_EXECUTION_DETECTED" else 1)

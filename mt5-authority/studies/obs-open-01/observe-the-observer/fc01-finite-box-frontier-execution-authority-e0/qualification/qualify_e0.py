import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_E0_CONTRACT_V1.json").read_text(encoding="utf-8"))
records = json.loads((root / "contracts/FC01_FQB_E0_GATE_AUDIT_RECORDS_V1.json").read_text(encoding="utf-8"))
by_gate = {r["gate_id"]: r for r in records["records"]}
checks = {
    "both_gates_present": set(by_gate) == {"FC01-FQB-G1", "FC01-FQB-C1"},
    "scheduler_exclusive": contract["scheduler_prerequisite_model"] == "SCHEDULER_EXCLUSIVE" and contract["alternative_execution_authority_permitted"] is False,
    "g1_not_selected": by_gate["FC01-FQB-G1"]["scheduler_selected"] is False,
    "c1_selected_without_execution_authorization": by_gate["FC01-FQB-C1"]["scheduler_selected"] is True and by_gate["FC01-FQB-C1"]["scheduler_execution_authorization"] is False,
    "no_independent_roots": all(r["independent_execution_authority_root"] == "NONE" for r in records["records"]),
    "no_exact_authority_binding": all(not r["authority_bound_to_exact_gate_id"] and not r["authority_bound_to_exact_input_roots"] for r in records["records"]),
    "both_not_authorized": all(r["historical_execution_status"] == "NOT_AUTHORIZED" for r in records["records"]),
    "fossils_preserved": contract["repair_policy"] == "PRESERVE_FOSSILS; NO_RERUN; NO_RETROACTIVE_AUTHORIZATION",
    "no_reruns": records["reruns"] == 0,
    "population_closed": contract["population_access"] == 0,
}
status = "HISTORICAL_EXECUTION_AUTHORITY_NOT_ESTABLISHED" if all(checks.values()) else "NOT_EVALUABLE"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "HISTORICAL_EXECUTION_AUTHORITY_NOT_ESTABLISHED" else 1)

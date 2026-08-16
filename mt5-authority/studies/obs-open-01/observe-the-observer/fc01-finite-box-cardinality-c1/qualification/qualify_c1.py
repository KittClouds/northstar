import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_C1_CONTRACT_V1.json").read_text(encoding="utf-8"))
records = json.loads((root / "contracts/FC01_FQB_C1_ROLE_COMPATIBILITY_RECORDS_V1.json").read_text(encoding="utf-8"))
by_role = {r["candidate_role"]: r for r in records["records"]}
checks = {
    "candidate_scope_exact": set(contract["candidate_role_classes"]) == set(by_role),
    "historical_recovery_forbidden": contract["historical_role_recovery"] == "FORBIDDEN" and records["historical_role"] == "NOT_IDENTIFIED",
    "admissible_set_consistent": set(records["admissible_descendant_role_classes"]) == {"PRIMITIVE_EXTENT", "DERIVED_RECEIPT_METADATA"},
    "noninterference_firewall_present": len(contract["required_firewalls"]) == 9,
    "no_role_selected": records["role_selected"] == "NONE" and contract["role_selected"] == "NONE",
    "no_values": records["values_selected"] == 0 and records["values_derived"] == 0 and contract["values_selected"] == 0 and contract["values_derived"] == 0,
    "no_instance": records["finite_box_instance"] == "NONE" and contract["finite_box_instance"] == "NONE",
    "population_closed": contract["population_access"] == 0,
    "requires_authority_for_unbound_roles": by_role["CONTRACT_AUTHORIZED_MECHANICAL_DERIVATION"]["result"] == "SEMANTIC_AUTHORITY_REQUIRED" and by_role["CONTRACT_FIXED_VALUE"]["result"] == "SEMANTIC_AUTHORITY_REQUIRED",
}
status = "PASS" if all(checks.values()) else "FAIL"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "PASS" else 1)

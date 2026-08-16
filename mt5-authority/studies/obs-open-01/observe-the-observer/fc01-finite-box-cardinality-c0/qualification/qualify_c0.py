import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_C0_CONTRACT_V1.json").read_text(encoding="utf-8"))
record = json.loads((root / "contracts/FC01_FQB_C0_ROLE_RECORD_V1.json").read_text(encoding="utf-8"))
checks = {
    "scope_exactly_one": contract["scope"] == ["BOX_CARDINALITY"] and record["parameter_class"] == "BOX_CARDINALITY",
    "role_unresolved": record["concrete_value_role"] == "ROLE_AUTHORITY_NOT_IDENTIFIED" and record["role_status"] == "ROLE_AUTHORITY_NOT_IDENTIFIED",
    "no_derivation_authority": record["value_derivation_authority"] == "NOT_IDENTIFIED" and record["authorized_operation"] == "NONE_ESTABLISHED_BY_SEALED_CONTRACTS",
    "no_governance_selectability": record["governance_selectability"] == "NOT_ESTABLISHED",
    "no_values": record["values_selected"] == 0 and record["values_derived"] == 0 and record["values_inherited"] == 0,
    "no_instance": record["finite_box_instance"] == "NONE" and contract["finite_box_instance"] == "NONE",
    "population_closed": contract["population_access"] == 0,
}
status = "PASS" if all(checks.values()) else "FAIL"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "PASS" else 1)

import json
from pathlib import Path

EXPECTED = {
    "TOKEN_LENGTH_BOUND",
    "MAX_TOKEN_WORD_LENGTH",
    "HORIZON_BOUND",
    "INTEGER_BOUND",
}

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_G0_CONTRACT_V1.json").read_text(encoding="utf-8"))
records = json.loads((root / "contracts/FC01_FQB_G0_JURISDICTION_RECORDS_V1.json").read_text(encoding="utf-8"))
ids = {r["parameter_class"] for r in records["records"]}
checks = {
    "scope_exactly_four": ids == EXPECTED and set(contract["scope"]) == EXPECTED,
    "all_compatible": all(r["compatibility_result"] == "GOVERNANCE_EXTENT_SELECTION_COMPATIBLE" for r in records["records"]),
    "selection_grants_zero": all(r["selection_authority_grant"] == "NOT_ISSUED" for r in records["records"]),
    "semantic_noninterference_declared": all(len(r["semantic_noninterference"]) == 5 for r in records["records"]),
    "no_values_selected": records["values_selected"] == 0 and contract["values_selected"] == 0,
    "no_box_instance": records["finite_box_instance"] == "NONE" and contract["finite_box_instance"] == "NONE",
    "population_closed": contract["population_access"] == 0,
}
status = "PASS" if all(checks.values()) else "FAIL"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "PASS" else 1)

import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
contract = json.loads((root / "contracts/FC01_FQB_G1_CONTRACT_V1.json").read_text(encoding="utf-8"))
grant = json.loads((root / "contracts/FINITE_QUESTION_BOX_PRIMITIVE_EXTENT_SELECTION_AUTHORITY_V1.json").read_text(encoding="utf-8"))
expected = ["TOKEN_LENGTH_BOUND", "MAX_TOKEN_WORD_LENGTH", "HORIZON_BOUND", "INTEGER_BOUND"]
checks = {
    "exact_scope": contract["scope"] == expected and grant["authorized_parameter_classes"] == expected,
    "g0_parent_bound": contract["parent_g0_root"] == grant["compatibility_parent"],
    "ns_gov_parent_bound": contract["ns_gov_root"] == grant["authority_parent"],
    "semantic_prohibitions": all(not grant[k] for k in ("may_change_domain", "may_change_parameter_semantics", "may_declare_sufficiency", "may_declare_optimality", "may_declare_exhaustiveness", "may_declare_minimality", "may_read_population", "may_read_results")),
    "no_values": contract["values_selected"] == 0 and grant["values_selected"] == 0 and contract["design_vector"] == "NONE" and grant["design_vector"] == "NONE",
    "population_closed": contract["population_access"] == 0 and grant["may_read_population"] is False,
    "downstream_closed": all(x not in grant["scope"] for x in []),
}
status = "PASS" if all(checks.values()) else "FAIL"
print(json.dumps({"checks": checks, "status": status}, sort_keys=True))
raise SystemExit(0 if status == "PASS" else 1)

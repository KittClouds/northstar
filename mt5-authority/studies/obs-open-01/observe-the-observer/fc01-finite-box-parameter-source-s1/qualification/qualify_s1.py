"""Read-only validator for S1 unresolved extent source authority."""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
EXPECTED = {"TOKEN_LENGTH_BOUND", "MAX_TOKEN_WORD_LENGTH", "HORIZON_BOUND", "INTEGER_BOUND", "BOX_CARDINALITY"}


def main() -> int:
    contract = json.loads((HERE / "contracts" / "FC01_FQB_S1_CONTRACT_V1.json").read_text())
    records = json.loads((HERE / "contracts" / "FC01_FQB_S1_PARAMETER_SOURCE_RECORDS_V1.json").read_text())["records"]
    ids = {r["parameter_class"] for r in records}
    checks = {
        "scope_exactly_five": set(contract["scope"]) == EXPECTED and ids == EXPECTED and len(records) == 5,
        "roles_closed": all(r["concrete_value_role"] in {"CONTRACT_FIXED", "PRIMITIVE_EXTENT", "DERIVED_VALUE", "UNKNOWN"} for r in records),
        "source_status_closed": all(r["source_authority_status"] in {"IDENTIFIED", "NOT_IDENTIFIED", "NOT_EVALUABLE"} for r in records),
        "governance_status_closed": all(r["governance_selectability"] in {"EXPLICITLY_AUTHORIZED", "EXPLICITLY_FORBIDDEN", "NOT_ESTABLISHED"} for r in records),
        "no_governance_selectability_promoted": not any(r["governance_selectability"] == "EXPLICITLY_AUTHORIZED" for r in records),
        "no_values_selected": contract["values_selected"] == 0,
        "no_values_derived": contract["values_derived"] == 0,
        "no_values_inherited": contract["values_inherited"] == 0,
        "no_bounds_bound": contract["actual_bounds_bound"] == 0,
        "no_instance": contract["finite_box_instance"] == "NONE",
    }
    print(json.dumps({"status": "PASS" if all(checks.values()) else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())

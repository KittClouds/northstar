"""Read-only validator for finite-box parameter source classification."""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
PARAMETERS = {
    "TOKEN_LENGTH_BOUND", "MAX_TOKEN_WORD_LENGTH", "HORIZON_BOUND", "INTEGER_BOUND",
    "ENUM_DOMAINS", "RANK_SHELL_CONTRACT", "CANONICAL_TOKEN_ORDER", "INTEGER_DOMAIN",
    "BOX_CARDINALITY", "PROOF_BOX_IS_FINITE",
}
TAXONOMY = {
    "SEALED_CONTRACT_EXACT_VALUE", "SEALED_CONTRACT_DOMAIN",
    "CONTRACT_AUTHORIZED_MECHANICAL_DERIVATION",
    "GOVERNANCE_SELECTABLE_EXPERIMENTAL_EXTENT", "NOT_AVAILABLE", "NOT_EVALUABLE",
}
OUTCOMES = {
    "VALUE_SOURCE_IDENTIFIED", "GOVERNANCE_SELECTION_REQUIRED",
    "SOURCE_AUTHORITY_NOT_IDENTIFIED", "NOT_EVALUABLE",
}


def main() -> int:
    contract = json.loads((HERE / "contracts" / "FC01_FINITE_BOX_PARAMETER_SOURCE_S0_CONTRACT_V1.json").read_text())
    records = json.loads((HERE / "contracts" / "FC01_FINITE_BOX_PARAMETER_SOURCE_RECORDS_V1.json").read_text())["records"]
    ids = {r["parameter_class"] for r in records}
    checks = {
        "all_ten_parameters_present": ids == PARAMETERS and len(records) == 10,
        "closed_source_taxonomy": set(contract["source_taxonomy"]) == TAXONOMY,
        "closed_outcome_taxonomy": set(contract["per_parameter_outcomes"]) == OUTCOMES,
        "one_source_class_each": all(r["value_source_class"] in TAXONOMY for r in records),
        "one_current_status_each": all(r["current_status"] in {"VALUE_SOURCE_IDENTIFIED_DOMAIN_UNMATERIALIZED", "VALUE_SOURCE_IDENTIFIED_VALUE_UNMATERIALIZED", "VALUE_SOURCE_IDENTIFIED_PREDECESSORS_UNBOUND", "SOURCE_AUTHORITY_NOT_IDENTIFIED"} for r in records),
        "no_values_selected": contract["values_selected"] == 0,
        "no_values_derived": contract["values_derived"] == 0,
        "no_values_inherited": contract["values_inherited"] == 0,
        "no_bounds_bound": contract["actual_bounds_bound"] == 0,
        "no_instance": contract["finite_box_instance"] == "NONE",
        "no_governance_selection_promoted": not any(r["value_source_class"] == "GOVERNANCE_SELECTABLE_EXPERIMENTAL_EXTENT" for r in records),
    }
    print(json.dumps({"status": "PASS" if all(checks.values()) else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())

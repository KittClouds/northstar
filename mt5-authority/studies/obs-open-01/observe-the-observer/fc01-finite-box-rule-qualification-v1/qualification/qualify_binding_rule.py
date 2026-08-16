"""Qualification runner for the finite-box binding rule only."""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE / "implementation"))
from binding_rule import AUTHORIZED_INPUT_IDS, build_rule_descriptor  # noqa: E402


FORBIDDEN = {
    "POPULATION_VALUES",
    "REALIZED_TOKENS",
    "PAIR_IDS",
    "SEPARATOR_INCIDENCE",
    "COVERAGE_SUCCESS",
    "EXPLORER_RESULTS",
    "G8_VERDICTS",
    "TARGET_INFORMATION",
    "OUTCOME_INFORMATION",
    "SCIENTIFIC_ATTRACTIVENESS",
    "POSTHOC_TUNING",
    "ACTUAL_BINDING_CONFIGURATION",
}


def main() -> int:
    inputs = json.loads((HERE / "fixtures" / "synthetic_rule_inputs.json").read_text())
    registry = json.loads((HERE / "contracts" / "FINITE_BOX_PARAMETER_REGISTRY_V1.json").read_text())
    allowed = json.loads((HERE / "contracts" / "FINITE_BOX_ALLOWED_RULE_INPUTS_V1.json").read_text())
    first = build_rule_descriptor(inputs)
    second = build_rule_descriptor(dict(inputs))
    registered = {p["parameter_id"] for p in registry["parameters"]}
    checks = {
        "authorized_input_set_exact": tuple(inputs.keys()) == AUTHORIZED_INPUT_IDS,
        "dependency_closure": set(first["authorized_input_ids"]).issubset(set(AUTHORIZED_INPUT_IDS)),
        "forbidden_inputs_disjoint": not (FORBIDDEN & set(first["authorized_input_ids"])),
        "all_required_parameters_registered": set(first["parameter_ids"]) == registered,
        "deterministic_replay": first == second,
        "actual_bounds_zero": first["actual_bounds_bound"] == 0,
        "box_instance_zero": first["box_instance_created"] == 0,
        "token_realization_zero": first["token_realization"] == 0,
        "population_reads_zero": first["population_reads"] == 0,
        "fail_closed_declared": first["fail_closed"] is True,
    }
    status = "PASS" if all(checks.values()) else "FAILED"
    print(json.dumps({"status": status, "checks": checks, "rule_id": first["rule_id"]}, sort_keys=True))
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())

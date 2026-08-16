"""Qualification runner for the one-shot finite-box binding execution."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("binding_instance", HERE / "implementation" / "bind_instance.py")
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)


def main() -> int:
    vector = json.loads((HERE / "inputs" / "FINITE_BOX_INSTANCE_BINDING_INPUT_VECTOR_V1.json").read_text())
    result = module.bind_once(vector)
    checks = {
        "rule_application_pass": result["rule_application"] == "PASS",
        "schema_failure_exposed": result["finite_box_schema_validation"] == "FAIL",
        "authority_binding_not_evaluable": result["finite_box_authority_binding"] == "NOT_EVALUABLE",
        "no_instance_created": result["finite_box_instance"] is None,
        "exactly_one_execution": result["execution_count"] == 1,
        "no_retry": result["retries"] == 0,
        "no_fallback": result["fallbacks"] == 0,
        "no_override": result["manual_overrides"] == 0,
        "all_ten_unbound_are_reported": len(result["unbound_parameter_ids"]) == 10,
        "actual_bounds_zero": result["actual_bounds_bound"] == 0,
        "downstream_zero": all(result[k] == 0 for k in (
            "tokens_realized", "pairs", "fibers", "comparisons", "certificates",
            "coverage_receipts", "g8_verifier_invocations", "population_reads")),
    }
    print(json.dumps({"status": "PASS" if all(checks.values()) else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())

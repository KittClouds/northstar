"""One-shot finite-box instance binding.

The qualified rule is invoked exactly once. This module never supplies a
fallback value, default, clamp, retry, or manual override.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE.parent / "fc01-finite-box-rule-qualification-v1" / "implementation"))
from binding_rule import build_rule_descriptor  # noqa: E402

EXPECTED_RULE_ROOT = "823A7198C62563C1C7A76E58BBECF18DE662E06F72A1D2FBA76F00E9DD8C52F0"
EXPECTED_RULE_AUTHORITY_ROOT = "9F463DBF90F1481D7282CD51151C747F62CE8643BFA60C1C920DF0ACE33EA1BF"
EXPECTED_V2_ROOT = "2B04972F9345A37F249E28584F52F678DF109DE7D0DBEED30724434EE076072F"
PARAMETER_IDS = (
    "TOKEN_LENGTH_BOUND", "MAX_TOKEN_WORD_LENGTH", "HORIZON_BOUND",
    "INTEGER_BOUND", "ENUM_DOMAINS", "RANK_SHELL_CONTRACT",
    "CANONICAL_TOKEN_ORDER", "INTEGER_DOMAIN", "BOX_CARDINALITY",
    "PROOF_BOX_IS_FINITE",
)


def bind_once(envelope: dict[str, Any]) -> dict[str, Any]:
    if envelope["qualified_rule_root"] != EXPECTED_RULE_ROOT:
        raise RuntimeError("RULE_ROOT_MISMATCH")
    if envelope["rule_qualification_authority_root"] != EXPECTED_RULE_AUTHORITY_ROOT:
        raise RuntimeError("RULE_AUTHORITY_ROOT_MISMATCH")
    if envelope["v2_binding_authority_root"] != EXPECTED_V2_ROOT:
        raise RuntimeError("V2_AUTHORITY_ROOT_MISMATCH")

    # Exactly one invocation. The descriptor is allowed to report that values
    # remain unbound; that is a result, never an invitation to repair it.
    rule_output = build_rule_descriptor(envelope["authorized_rule_input_vector"])
    missing = list(PARAMETER_IDS) if rule_output["actual_bounds_bound"] == 0 else []
    schema_pass = (
        tuple(rule_output["parameter_ids"]) == PARAMETER_IDS
        and rule_output["actual_bounds_bound"] == 1
        and not missing
    )
    return {
        "schema": "FC01_FINITE_BOX_INSTANCE_BINDING_EXECUTION_RESULT_V1",
        "rule_application": "PASS",
        "finite_box_schema_validation": "PASS" if schema_pass else "FAIL",
        "finite_box_authority_binding": "PASS" if schema_pass else "NOT_EVALUABLE",
        "overall": "FINITE_BOX_INSTANCE_BOUND_WITH_RESTRICTIONS" if schema_pass else "NOT_EVALUABLE",
        "rule_output": rule_output,
        "unbound_parameter_ids": missing,
        "finite_box_instance": None,
        "execution_count": 1,
        "retries": 0,
        "fallbacks": 0,
        "manual_overrides": 0,
        "actual_bounds_bound": 0,
        "tokens_realized": 0,
        "pairs": 0,
        "fibers": 0,
        "comparisons": 0,
        "certificates": 0,
        "coverage_receipts": 0,
        "g8_verifier_invocations": 0,
        "population_reads": 0,
    }


def main() -> int:
    vector = json.loads((HERE / "inputs" / "FINITE_BOX_INSTANCE_BINDING_INPUT_VECTOR_V1.json").read_text())
    result = bind_once(vector)
    print(json.dumps(result, sort_keys=True, separators=(",", ":"), ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

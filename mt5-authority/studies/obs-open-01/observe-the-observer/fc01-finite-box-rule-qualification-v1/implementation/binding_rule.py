"""Deterministic finite-box binding-rule descriptor.

This module qualifies the ruler, not the measurements. It never selects a
bound, creates a box, reads a population, or realizes a token.
"""

from __future__ import annotations

from typing import Any, Mapping

PARAMETER_IDS = (
    "TOKEN_LENGTH_BOUND",
    "MAX_TOKEN_WORD_LENGTH",
    "HORIZON_BOUND",
    "INTEGER_BOUND",
    "ENUM_DOMAINS",
    "RANK_SHELL_CONTRACT",
    "CANONICAL_TOKEN_ORDER",
    "INTEGER_DOMAIN",
    "BOX_CARDINALITY",
    "PROOF_BOX_IS_FINITE",
)

AUTHORIZED_INPUT_IDS = (
    "FINITE_QUESTION_BOX_SPEC_ROOT",
    "TOKEN_REALIZATION_CONTRACT_ROOT",
    "G5_AUTHORITY_ROOT",
    "CANONICALIZATION_CONTRACT_ID",
    "RULE_PARAMETER_REGISTRY_ID",
)


def build_rule_descriptor(inputs: Mapping[str, Any]) -> dict[str, Any]:
    """Return only a canonical rule descriptor; all values remain unbound."""
    if tuple(inputs.keys()) != AUTHORIZED_INPUT_IDS:
        raise ValueError("AUTHORIZED_INPUT_SET_MISMATCH")
    return {
        "rule_id": "FINITE_QUESTION_BOX_BINDING_RULE_V1",
        "parameter_ids": list(PARAMETER_IDS),
        "authorized_input_ids": list(AUTHORIZED_INPUT_IDS),
        "binding_mode": "PRECOMMITTED_CONTENT_INDEPENDENT_RULE",
        "dependency_scope": "SEALED_CONTRACTS_AND_DECLARED_TYPE_DOMAINS_ONLY",
        "output_status": "RULE_QUALIFIED_CANDIDATE_ACTUAL_VALUES_UNBOUND",
        "fail_closed": True,
        "actual_bounds_bound": 0,
        "box_instance_created": 0,
        "token_realization": 0,
        "population_reads": 0,
    }

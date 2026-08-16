"""Variant-aware doorway repair.

Applied records delegate to the immutable A1 projector. Rejected records use
an explicit mechanical rejection view and never receive fabricated state.
"""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from typing import Any, Mapping

from a1_projector import FC01AProjector


REJECTED_REQUIRED = (
    "g1_authority_root",
    "semantic_lineage_root",
    "context_identity",
    "input_identity",
    "transition_status",
    "rejection_status",
)
REJECTED_FORBIDDEN = (
    "state_projection",
    "ordered_emissions",
    "knowledge_time_projection",
    "derived_reason",
    "threshold_distance",
    "raw_input_payload",
)


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


def root(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest().upper()


def project_applied(payload: Mapping[str, Any]) -> dict[str, Any]:
    result = FC01AProjector().project(payload).as_dict()
    if result["doorway_disposition"] != "ACCEPTED":
        raise RuntimeError("A2R_APPLIED_ROUTE_REJECTED")
    return {"return_variant": "APPLIED", "a1_projection": result, "view_root": root(result)}


def project_rejected(payload: Mapping[str, Any]) -> dict[str, Any]:
    if any(field not in payload for field in REJECTED_REQUIRED):
        raise RuntimeError("A2R_REJECTED_REQUIRED_FIELD_MISSING")
    if any(field in payload for field in REJECTED_FORBIDDEN):
        raise RuntimeError("A2R_REJECTED_FORBIDDEN_FIELD_PRESENT")
    if payload["transition_status"].get("status") != "REJECTED":
        raise RuntimeError("A2R_REJECTED_VARIANT_TAG_MISMATCH")
    view = {field: deepcopy(payload[field]) for field in REJECTED_REQUIRED}
    return {"return_variant": "REJECTED", "rejection_view": view, "view_root": root(view)}

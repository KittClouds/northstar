"""FC-01A A1 projection-only doorway.

This module deliberately contains no G1 lifecycle, genealogy, rejection, or
temporal semantics.  It accepts a contract-valid G1 semantic product and
copies only the named fields into independent consumer views.
"""

from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass
import hashlib
import json
from typing import Any, Callable, Mapping, Optional, Protocol


G1_REQUIRED_FIELDS = (
    "g1_authority_root",
    "context_identity",
    "input_identity",
    "transition_status",
    "state_projection",
    "ordered_emissions",
    "knowledge_time_projection",
    "rejection_status",
    "semantic_lineage_root",
)

VIEW_ALLOWLISTS = {
    "G5_PRESENTABILITY_VIEW": (
        "context_identity",
        "input_identity",
        "transition_status",
        "knowledge_time_projection",
        "rejection_status",
    ),
    "G6_CORRESPONDENCE_PRIMITIVE_VIEW": (
        "context_identity",
        "state_projection",
        "ordered_emissions",
        "semantic_lineage_root",
    ),
    "G7_CERTIFICATE_LINEAGE_VIEW": (
        "g1_authority_root",
        "semantic_lineage_root",
        "context_identity",
        "input_identity",
    ),
    "G8_VERIFIER_INPUT_VIEW": (
        "g1_authority_root",
        "semantic_lineage_root",
        "transition_status",
        "state_projection",
        "ordered_emissions",
        "rejection_status",
    ),
}


class QualifiedG1Invoker(Protocol):
    """The only accepted end-to-end dependency.

    A concrete implementation must be supplied by the sealed G1 authority.
    FC-01A never manufactures one.
    """

    qualified_g1_root: str

    def __call__(self, canonical_input: Mapping[str, Any], context: Mapping[str, Any]) -> Mapping[str, Any]:
        ...


class _DoorwayError(Exception):
    def __init__(self, code: str) -> None:
        self.code = code


def canonical_bytes(value: Any) -> bytes:
    """Canonical JSON bytes; source sequence order is preserved."""

    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def sha256_hex(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest().upper()


def _json_value(value: Any) -> bool:
    if value is None or isinstance(value, (str, int, bool)):
        return True
    if isinstance(value, list):
        return all(_json_value(item) for item in value)
    if isinstance(value, dict):
        return all(isinstance(k, str) and _json_value(v) for k, v in value.items())
    return False


def _validate_g1_output(value: Mapping[str, Any]) -> None:
    if not isinstance(value, Mapping):
        raise _DoorwayError("PROJECTION_CONTRACT_VIOLATION")
    for field in G1_REQUIRED_FIELDS:
        if field not in value:
            raise _DoorwayError("G1_REQUIRED_PRIMITIVE_UNAVAILABLE")
        if not _json_value(value[field]):
            raise _DoorwayError("PROJECTION_CONTRACT_VIOLATION")
    if not isinstance(value["ordered_emissions"], list):
        raise _DoorwayError("PROJECTION_CONTRACT_VIOLATION")


def _copy_fields(source: Mapping[str, Any], fields: tuple[str, ...]) -> dict[str, Any]:
    return {field: deepcopy(source[field]) for field in fields}


@dataclass(frozen=True)
class ProjectionResult:
    doorway_disposition: str
    g1_rejection_status: Any
    core: Optional[dict[str, Any]]
    views: dict[str, dict[str, Any]]
    view_roots: dict[str, str]
    core_root: Optional[str]

    def as_dict(self) -> dict[str, Any]:
        return {
            "doorway_disposition": self.doorway_disposition,
            "g1_rejection_status": deepcopy(self.g1_rejection_status),
            "core": deepcopy(self.core),
            "views": deepcopy(self.views),
            "view_roots": dict(self.view_roots),
            "core_root": self.core_root,
        }


class FC01AProjector:
    """Stateless A0-authorized projection machine."""

    def project(self, g1_output: Mapping[str, Any]) -> ProjectionResult:
        try:
            _validate_g1_output(g1_output)
        except _DoorwayError as error:
            return ProjectionResult(error.code, None, None, {}, {}, None)

        core = _copy_fields(g1_output, G1_REQUIRED_FIELDS)
        views = {
            view_id: _copy_fields(g1_output, fields)
            for view_id, fields in VIEW_ALLOWLISTS.items()
        }
        return ProjectionResult(
            doorway_disposition="ACCEPTED",
            g1_rejection_status=deepcopy(g1_output["rejection_status"]),
            core=core,
            views=views,
            view_roots={view_id: sha256_hex(view) for view_id, view in views.items()},
            core_root=sha256_hex(core),
        )

    def invoke_exact_g1(
        self,
        canonical_input: Mapping[str, Any],
        context: Mapping[str, Any],
        invoker: Optional[QualifiedG1Invoker],
        expected_g1_root: str,
    ) -> ProjectionResult:
        """Run the end-to-end path, or fail closed when G1 is unavailable."""

        if invoker is None or getattr(invoker, "qualified_g1_root", None) != expected_g1_root:
            return ProjectionResult("G1_REQUIRED_PRIMITIVE_UNAVAILABLE", None, None, {}, {}, None)
        try:
            g1_output = invoker(canonical_input, context)
        except Exception:
            return ProjectionResult("NOT_EVALUABLE", None, None, {}, {}, None)
        return self.project(g1_output)


def synthetic_g1_output(tag: str = "A") -> dict[str, Any]:
    """Contract-valid nonpopulation fixture; it is not a G1 implementation."""

    return {
        "g1_authority_root": "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd",
        "context_identity": {"fixture": "SYNTHETIC_NONPOPULATION", "tag": tag},
        "input_identity": {"canonical_id": "SYNTHETIC_INPUT_" + tag},
        "transition_status": {"status": "APPLIED", "ordinal": 1},
        "state_projection": {"state": "VALID_SYNTHETIC_STATE", "tag": tag},
        "ordered_emissions": [{"event": "SYNTHETIC_EVENT", "ordinal": 1}],
        "knowledge_time_projection": {"knowledge_ordinal": 1},
        "rejection_status": {"status": "ACCEPTED"},
        "semantic_lineage_root": "SYNTHETIC_G1_LINEAGE_" + tag,
    }

"""Synthetic-only G8_04A adapter descendant.

The module invokes the sealed A1 projector and performs only direct field
selection plus structural packaging. It contains no lifecycle, genealogy,
time, token, fiber, certificate, or verifier semantics.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any, Mapping


PROJECTOR_SOURCE = (
    Path(__file__).resolve().parents[2]
    / "fc01a-semantic-doorway"
    / "a1-implementation-qualification"
    / "implementation"
    / "fc01a_projector.py"
)
EXPECTED_G1_ROOT = "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd"


def _load_projector() -> Any:
    spec = importlib.util.spec_from_file_location("sealed_a1_projector", PROJECTOR_SOURCE)
    if spec is None or spec.loader is None:
        raise RuntimeError("A1_PROJECTOR_UNAVAILABLE")
    module = importlib.util.module_from_spec(spec)
    sys.modules["sealed_a1_projector"] = module
    spec.loader.exec_module(module)
    return module


def construct_adapter_instance(g1_output: Mapping[str, Any]) -> dict[str, Any]:
    projector = _load_projector()
    projected = projector.FC01AProjector().project(g1_output).as_dict()
    if projected["doorway_disposition"] != "ACCEPTED":
        raise RuntimeError("A1_PROJECTION_FAILED")
    core = projected["core"]
    views = projected["views"]
    if core["g1_authority_root"] != EXPECTED_G1_ROOT:
        raise RuntimeError("G1_LINEAGE_MISMATCH")

    # Every payload below is a direct selection from the already-qualified
    # G1/A1 object. No new semantic value is calculated.
    return {
        "adapter_instance_id": "G8_04A_ADAPTER_INSTANCE_V1",
        "authority_scope": "DECLARED_SYNTHETIC_AND_FORMALLY_UNDERSTOOD_SYSTEMS_ONLY",
        "historical_adapter_identity": "NOT_CLAIMED",
        "qualified_history_view": {
            "source": "A1_CORE_DIRECT_SELECTION",
            "payload": core,
        },
        "g4_observable_projection": {
            "source": "A1_CORE_DIRECT_SELECTION",
            "payload": {
                "state_projection": core["state_projection"],
                "ordered_emissions": core["ordered_emissions"],
                "knowledge_time_projection": core["knowledge_time_projection"],
            },
        },
        "g5_presentability_receipt": {
            "source": "G5_PRESENTABILITY_VIEW",
            "payload": views["G5_PRESENTABILITY_VIEW"],
        },
        "g6_fiber_certificate_input": {
            "source": "G6_CORRESPONDENCE_PRIMITIVE_VIEW",
            "payload": views["G6_CORRESPONDENCE_PRIMITIVE_VIEW"],
            "authority_status": "INPUT_ONLY_NOT_FIBER_AUTHORITY",
        },
        "g8_verifier_input": {
            "source": "A1_VIEW_REGISTRY",
            "payload": views["G8_VERIFIER_INPUT_VIEW"],
            "authority_status": "TRANSPORT_CANDIDATE_ONLY_NO_G8_INVOCATION",
        },
        "projection_roots": projected["view_roots"],
        "core_root": projected["core_root"],
    }


def main() -> None:
    fixture = Path(__file__).resolve().parents[1] / "fixtures" / "synthetic_g1_output.json"
    g1_output = json.loads(fixture.read_text(encoding="utf-8"))
    result = construct_adapter_instance(g1_output)
    print(json.dumps(result, sort_keys=True, separators=(",", ":"), ensure_ascii=False))


if __name__ == "__main__":
    main()

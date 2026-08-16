"""A1 projection-only qualification corpus.

The corpus is synthetic and contract-only.  It never reads protected
population material and it does not pretend to qualify the missing G1 call.
"""

from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import sys

_PROJECTOR_PATH = Path(__file__).with_name("fc01a_projector.py")
_PROJECTOR_SPEC = importlib.util.spec_from_file_location("fc01a_projector", _PROJECTOR_PATH)
if _PROJECTOR_SPEC is None or _PROJECTOR_SPEC.loader is None:
    raise RuntimeError("projection implementation unavailable")
_PROJECTOR_MODULE = importlib.util.module_from_spec(_PROJECTOR_SPEC)
sys.modules["fc01a_projector"] = _PROJECTOR_MODULE
_PROJECTOR_SPEC.loader.exec_module(_PROJECTOR_MODULE)
FC01AProjector = _PROJECTOR_MODULE.FC01AProjector
synthetic_g1_output = _PROJECTOR_MODULE.synthetic_g1_output


def _assert(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run_projection_fixtures() -> dict[str, object]:
    projector = FC01AProjector()
    base = synthetic_g1_output("A")
    accepted = projector.project(base)
    _assert(accepted.doorway_disposition == "ACCEPTED", "base fixture rejected")

    # Same valid semantic products, different projector invocation order.
    products = {tag: synthetic_g1_output(tag) for tag in ("A", "B", "C")}
    orders = (("A", "B", "C"), ("C", "A", "B"), ("B", "C", "A"))
    order_roots: dict[str, dict[str, str]] = {}
    for order in orders:
        for tag in order:
            result = projector.project(products[tag])
            order_roots.setdefault(tag, result.view_roots)
            _assert(result.view_roots == order_roots[tag], "projection depends on prior projector call")

    # Authorized variation may alter only views that name the changed field.
    authorized = deepcopy(base)
    authorized["state_projection"]["state"] = "OTHER_VALID_STATE"
    changed = projector.project(authorized)
    _assert(
        changed.view_roots["G6_CORRESPONDENCE_PRIMITIVE_VIEW"]
        != accepted.view_roots["G6_CORRESPONDENCE_PRIMITIVE_VIEW"],
        "authorized G1 field did not reach its named view",
    )
    _assert(
        changed.view_roots["G5_PRESENTABILITY_VIEW"] == accepted.view_roots["G5_PRESENTABILITY_VIEW"],
        "projection leaked state_projection into G5",
    )

    # Other-consumer-only/unknown fields cannot alter any authorized view.
    irrelevant = deepcopy(base)
    irrelevant["unlisted_fixture_field"] = {"must_not_be_read": "x"}
    irrelevant_result = projector.project(irrelevant)
    _assert(irrelevant_result.view_roots == accepted.view_roots, "unlisted field leaked into a view")

    # G1 rejection is copied without renaming or enrichment.
    rejected = deepcopy(base)
    rejected["rejection_status"] = {"status": "REJECTED", "code": "G1_CONTRACT_REJECTION"}
    rejected_result = projector.project(rejected)
    _assert(rejected_result.g1_rejection_status == rejected["rejection_status"], "G1 rejection was not passed through")
    _assert(rejected_result.doorway_disposition == "ACCEPTED", "G1 rejection was reinterpreted as doorway failure")

    # Missing G1 primitive fails closed; no substitute is derived.
    missing = deepcopy(base)
    del missing["state_projection"]
    missing_result = projector.project(missing)
    _assert(missing_result.doorway_disposition == "G1_REQUIRED_PRIMITIVE_UNAVAILABLE", "missing G1 field was repaired")
    _assert(missing_result.core is None and not missing_result.views, "failed projection exposed partial semantics")

    # End-to-end path cannot claim G1 invocation without the sealed callable.
    end_to_end = projector.invoke_exact_g1({}, {}, None, base["g1_authority_root"])
    _assert(end_to_end.doorway_disposition == "G1_REQUIRED_PRIMITIVE_UNAVAILABLE", "unbound G1 path was accepted")

    return {
        "projection_only_path": "PASS",
        "pointwise_order_independence": "PASS",
        "authorized_field_closure": "PASS",
        "irrelevant_field_noninterference": "PASS",
        "g1_rejection_passthrough": "PASS",
        "missing_primitive_fail_closed": "PASS",
        "end_to_end_g1_path": "NOT_EVALUABLE",
        "population_reads": 0,
        "population_derived_content_reads": 0,
        "population_derived_metadata_reads": 0,
    }


if __name__ == "__main__":
    result = run_projection_fixtures()
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))

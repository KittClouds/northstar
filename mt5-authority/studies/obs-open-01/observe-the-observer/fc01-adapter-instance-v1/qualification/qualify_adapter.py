"""Qualification runner for one synthetic G8_04A adapter instance.

This runner is intentionally read-only: it invokes the sealed A1 projector
through the adapter implementation, checks only the V2 grant's allowed
representation-preserving operations, and emits a deterministic receipt.
"""

from __future__ import annotations

import json
import hashlib
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE / "implementation"))

from adapter_instance import construct_adapter_instance  # noqa: E402


EXPECTED_GRANT_ROOT = (
    "2B04972F9345A37F249E28584F52F678DF109DE7D0DBEED30724434EE076072F"
)
EXPECTED_PROJECTOR_ROOT = (
    "53D4F850C8D3943A0EA4F56CB3FF533434A6BB6C50CEF82720DA6A84AE976E54"
)


def main() -> int:
    contract = json.loads((HERE / "contracts" / "FC01_ADAPTER_INSTANCE_QUALIFICATION_CONTRACT_V1.json").read_text())
    fixture = json.loads((HERE / "fixtures" / "synthetic_g1_output.json").read_text())
    result = construct_adapter_instance(fixture)

    projector_source = (HERE / "implementation" / "adapter_instance.py").resolve().parents[2] / "fc01a-semantic-doorway" / "a1-implementation-qualification" / "implementation" / "fc01a_projector.py"
    projector_hash = hashlib.sha256(projector_source.read_bytes()).hexdigest().upper()

    checks = {
        "grant_root_bound": contract["producer_authority_root"] == EXPECTED_GRANT_ROOT,
        "projector_root_bound": projector_hash == EXPECTED_PROJECTOR_ROOT,
        "synthetic_nonpopulation_fixture": fixture["context_identity"]["fixture"] == "SYNTHETIC_NONPOPULATION",
        "a1_projector_invoked": result["core_root"] == "4A6C66E7D3C5AA815C93BA71C973F3AFD000802F75D0A00A8790202EF48518F1",
        "direct_field_mapping": result["g4_observable_projection"]["source"] == "A1_CORE_DIRECT_SELECTION",
        "sequence_order_preserved": result["g4_observable_projection"]["payload"]["ordered_emissions"] == [{"event": "SYNTHETIC_EVENT", "ordinal": 1}],
        "variant_preserved": result["g5_presentability_receipt"]["payload"]["rejection_status"]["status"] == "ACCEPTED",
        "g6_input_not_fiber_authority": result["g6_fiber_certificate_input"]["authority_status"] == "INPUT_ONLY_NOT_FIBER_AUTHORITY",
        "g8_not_invoked": result["g8_verifier_input"]["authority_status"] == "TRANSPORT_CANDIDATE_ONLY_NO_G8_INVOCATION",
    }
    if not all(checks.values()):
        print(json.dumps({"status": "FAILED", "checks": checks}, sort_keys=True))
        return 1
    print(json.dumps({"status": "PASS", "checks": checks, "instance_id": result["adapter_instance_id"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

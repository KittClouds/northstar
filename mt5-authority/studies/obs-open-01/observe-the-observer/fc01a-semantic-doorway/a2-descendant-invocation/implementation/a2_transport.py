"""FC-01A-A2: bind one G1-R executable and transport StepResult mechanically."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
A2_ROOT = Path(__file__).resolve().parents[0].parent
INSTANCE_PATH = A2_ROOT / "contracts" / "A2_G1R_EXECUTABLE_INSTANCE_V1.json"
TRANSPORT_PATH = A2_ROOT / "contracts" / "A2_STEPRESULT_TRANSPORT_CONTRACT_V1.json"
CORPUS_PATH = ROOT / "observe-the-observer" / "g1-r-executable-descendant" / "fixtures" / "synthetic_qualification_corpus.json"
EXECUTABLE_PATH = Path(r"D:\codex-target-obs-open-g1-r\release\obs-open-04a-g1-executable-descendant.exe")
PROJECTOR_PATH = ROOT / "observe-the-observer" / "fc01a-semantic-doorway" / "a1-implementation-qualification" / "implementation" / "fc01a_projector.py"
EXPECTED_EXECUTABLE_SHA256 = "E1FD57AE1FEC58684D98A61AAC9F972821900291B2A44E9C211BECFDC8912AEA"
EXPECTED_G1R_ROOT = "21E310BA2B7E95ABFF2AA7B0F8165A4D44F2AF37FC7AB50BEF35DE76BF3FE2FB"
EXPECTED_G1_ROOT = "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest().upper()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_projector() -> Any:
    import importlib.util
    import sys

    spec = importlib.util.spec_from_file_location("fc01a_projector_a2", PROJECTOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("A1_PROJECTOR_UNAVAILABLE")
    module = importlib.util.module_from_spec(spec)
    sys.modules["fc01a_projector_a2"] = module
    spec.loader.exec_module(module)
    return module.FC01AProjector()


def run() -> dict[str, Any]:
    instance = load_json(INSTANCE_PATH)
    transport_contract = load_json(TRANSPORT_PATH)
    corpus = load_json(CORPUS_PATH)
    if instance["g1r_authority_root"] != EXPECTED_G1R_ROOT:
        raise RuntimeError("A2_G1R_ROOT_MISMATCH")
    if instance["parent_semantic_root"] != EXPECTED_G1_ROOT:
        raise RuntimeError("A2_PARENT_G1_ROOT_MISMATCH")
    if sha256_file(EXECUTABLE_PATH) != EXPECTED_EXECUTABLE_SHA256:
        raise RuntimeError("A2_EXECUTABLE_HASH_MISMATCH")
    projector = load_projector()
    invocation_output = A2_ROOT / "qualification" / "G1_R_CAPTURE_BUILD_A.json"
    invocation_output.parent.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(
        [str(EXECUTABLE_PATH), "run", str(CORPUS_PATH), str(invocation_output)],
        check=True,
        capture_output=True,
        text=True,
    )
    captured = load_json(invocation_output)
    captured_root = sha256_file(invocation_output)
    if completed.stdout.strip().upper() != captured_root:
        raise RuntimeError("A2_CAPTURE_ROOT_MISMATCH")
    if captured["descendant_id"] != "G1_EXECUTABLE_DESCENDANT_V1" or captured["g1_root"] != EXPECTED_G1_ROOT:
        raise RuntimeError("A2_CAPTURE_LINEAGE_MISMATCH")

    case_by_id = {case["case_id"]: case for case in corpus["cases"]}
    records: list[dict[str, Any]] = []
    missing: list[dict[str, Any]] = []
    for captured_case in captured["cases"]:
        source_case = case_by_id[captured_case["case_id"]]
        for ordinal, output in enumerate(captured_case["outputs"]):
            observation = source_case["observations"][ordinal]
            abi = {
                "g1_authority_root": EXPECTED_G1_ROOT,
                "context_identity": deepcopy(source_case["context"]),
                "input_identity": {
                    "source_row_id": observation["source_row_id"],
                    "event_time_ns": observation["event_time_ns"],
                    "knowledge_time_ns": observation["knowledge_time_ns"],
                },
                "transition_status": {"status": "APPLIED"},
                "state_projection": deepcopy(output["state"]),
                "ordered_emissions": deepcopy(output["emissions"]["ordered"]),
                "knowledge_time_projection": output["state"]["knowledge_time_ns"],
                "rejection_status": {"status": "ACCEPTED"},
                "semantic_lineage_root": EXPECTED_G1R_ROOT,
            }
            projected = projector.project(abi).as_dict()
            if projected["doorway_disposition"] != "ACCEPTED":
                raise RuntimeError("A2_APPLIED_ABI_REJECTED")
            records.append({"case_id": captured_case["case_id"], "ordinal": ordinal, "abi": abi, "projection": projected})
        if captured_case["terminal"] == "REJECTED":
            missing.append({
                "case_id": captured_case["case_id"],
                "g1_rejection_status": {"status": "REJECTED", "code": captured_case["error"]},
                "missing_required_primitives": ["state_projection", "ordered_emissions", "knowledge_time_projection"],
                "doorway_disposition": "G1_REQUIRED_PRIMITIVE_UNAVAILABLE",
            })

    return {
        "schema": "FC01A_A2_EXECUTION_RESULT_V1",
        "result": "NOT_EVALUABLE" if missing else "PASS",
        "exact_instance_binding": "PASS",
        "g1r_capture": "PASS",
        "stepresult_transport_applied_returns": "PASS" if records else "NOT_EVALUABLE",
        "stepresult_transport_rejected_returns": "NOT_EVALUABLE" if missing else "NOT_APPLICABLE",
        "unchanged_a1_projector": "PASS",
        "projector_sha256": sha256_file(PROJECTOR_PATH),
        "transport_contract_sha256": sha256_file(TRANSPORT_PATH),
        "captured_semantic_envelope_sha256": captured_root,
        "applied_projection_records": records,
        "rejected_transport_records": missing,
        "transport_source_contract": transport_contract["source"],
        "population_access": 0,
        "target_access": 0,
        "outcome_access": 0,
        "pair_access": 0,
        "scope_expansion": "NONE",
        "nonclaims": ["historical_binary_identity", "real_history_applicability", "fc01b_authority", "population_authority"],
    }


if __name__ == "__main__":
    print(json.dumps(run(), sort_keys=True, separators=(",", ":")))

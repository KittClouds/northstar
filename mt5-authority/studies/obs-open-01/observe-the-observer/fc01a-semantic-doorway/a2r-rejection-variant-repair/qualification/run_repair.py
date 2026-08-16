from __future__ import annotations

from copy import deepcopy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


REPAIR_ROOT = Path(__file__).resolve().parents[1]
OBSERVE_ROOT = Path(__file__).resolve().parents[3]
INSTANCE_PATH = OBSERVE_ROOT / "fc01a-semantic-doorway" / "a2-descendant-invocation" / "contracts" / "A2_G1R_EXECUTABLE_INSTANCE_V1.json"
CORPUS_PATH = OBSERVE_ROOT / "g1-r-executable-descendant" / "fixtures" / "synthetic_qualification_corpus.json"
PROJECTOR_PATH = OBSERVE_ROOT / "fc01a-semantic-doorway" / "a1-implementation-qualification" / "implementation" / "fc01a_projector.py"
EXECUTABLE_PATH = Path(r"D:\codex-target-obs-open-g1-r\release\obs-open-04a-g1-executable-descendant.exe")
EXPECTED_EXECUTABLE_SHA256 = "E1FD57AE1FEC58684D98A61AAC9F972821900291B2A44E9C211BECFDC8912AEA"
EXPECTED_G1R_ROOT = "21E310BA2B7E95ABFF2AA7B0F8165A4D44F2AF37FC7AB50BEF35DE76BF3FE2FB"
EXPECTED_G1_ROOT = "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd"
EXPECTED_PROJECTOR_SHA256 = "53D4F850C8D3943A0EA4F56CB3FF533434A6BB6C50CEF82720DA6A84AE976E54"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest().upper()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def load_variant_module():
    sys.path.insert(0, str(REPAIR_ROOT / "implementation"))
    a1_spec = importlib.util.spec_from_file_location("a1_projector", PROJECTOR_PATH)
    if a1_spec is None or a1_spec.loader is None:
        raise RuntimeError("A1_PROJECTOR_UNAVAILABLE")
    a1_module = importlib.util.module_from_spec(a1_spec)
    sys.modules["a1_projector"] = a1_module
    a1_spec.loader.exec_module(a1_module)
    variant_spec = importlib.util.spec_from_file_location("variant_projector", REPAIR_ROOT / "implementation" / "variant_projector.py")
    if variant_spec is None or variant_spec.loader is None:
        raise RuntimeError("VARIANT_PROJECTOR_UNAVAILABLE")
    variant_module = importlib.util.module_from_spec(variant_spec)
    sys.modules["variant_projector"] = variant_module
    variant_spec.loader.exec_module(variant_module)
    return variant_module


def run() -> dict:
    instance = load_json(INSTANCE_PATH)
    corpus = load_json(CORPUS_PATH)
    if instance["g1r_authority_root"] != EXPECTED_G1R_ROOT or instance["parent_semantic_root"] != EXPECTED_G1_ROOT:
        raise RuntimeError("A2R_LINEAGE_MISMATCH")
    if sha256_file(EXECUTABLE_PATH) != EXPECTED_EXECUTABLE_SHA256:
        raise RuntimeError("A2R_EXECUTABLE_HASH_MISMATCH")
    if sha256_file(PROJECTOR_PATH) != EXPECTED_PROJECTOR_SHA256:
        raise RuntimeError("A2R_A1_PROJECTOR_MUTATED")
    variant = load_variant_module()
    output_path = REPAIR_ROOT / "qualification" / "G1_R_CAPTURE_BUILD_A.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run([str(EXECUTABLE_PATH), "run", str(CORPUS_PATH), str(output_path)], check=True, capture_output=True, text=True)
    captured = load_json(output_path)
    capture_root = sha256_file(output_path)
    if completed.stdout.strip().upper() != capture_root:
        raise RuntimeError("A2R_CAPTURE_ROOT_MISMATCH")
    case_by_id = {case["case_id"]: case for case in corpus["cases"]}
    applied = []
    rejected = []
    for captured_case in captured["cases"]:
        source_case = case_by_id[captured_case["case_id"]]
        for ordinal, output in enumerate(captured_case["outputs"]):
            observation = source_case["observations"][ordinal]
            payload = {
                "g1_authority_root": EXPECTED_G1_ROOT,
                "context_identity": deepcopy(source_case["context"]),
                "input_identity": {"source_row_id": observation["source_row_id"], "event_time_ns": observation["event_time_ns"], "knowledge_time_ns": observation["knowledge_time_ns"]},
                "transition_status": {"status": "APPLIED"},
                "state_projection": deepcopy(output["state"]),
                "ordered_emissions": deepcopy(output["emissions"]["ordered"]),
                "knowledge_time_projection": output["state"]["knowledge_time_ns"],
                "rejection_status": {"status": "ACCEPTED"},
                "semantic_lineage_root": EXPECTED_G1R_ROOT,
            }
            applied.append({"case_id": captured_case["case_id"], "ordinal": ordinal, "projection": variant.project_applied(payload)})
        if captured_case["terminal"] == "REJECTED":
            failed_observation = source_case["observations"][len(captured_case["outputs"])]
            payload = {
                "g1_authority_root": EXPECTED_G1_ROOT,
                "semantic_lineage_root": EXPECTED_G1R_ROOT,
                "context_identity": deepcopy(source_case["context"]),
                "input_identity": {"source_row_id": failed_observation["source_row_id"], "event_time_ns": failed_observation["event_time_ns"], "knowledge_time_ns": failed_observation["knowledge_time_ns"]},
                "transition_status": {"status": "REJECTED"},
                "rejection_status": {"status": "REJECTED", "code": captured_case["error"]},
            }
            rejected.append({"case_id": captured_case["case_id"], "projection": variant.project_rejected(payload)})
    return {
        "schema": "FC01A_A2R_REPAIR_RESULT_V1",
        "result": "REPAIR_QUALIFIED_WITH_RESTRICTIONS",
        "parent_a2_result": "NOT_EVALUABLE",
        "selected_executable_sha256": EXPECTED_EXECUTABLE_SHA256,
        "g1r_root": EXPECTED_G1R_ROOT,
        "g1_root": EXPECTED_G1_ROOT,
        "applied_route": "PASS_UNCHANGED_A1_PROJECTOR",
        "rejected_route": "PASS_EXPLICIT_REJECTION_VIEW",
        "applied_count": len(applied),
        "rejected_count": len(rejected),
        "applied": applied,
        "rejected": rejected,
        "capture_root": capture_root,
        "a1_projector_sha256": sha256_file(PROJECTOR_PATH),
        "scope_expansion": "NONE",
        "population_access": 0,
        "target_access": 0,
        "outcome_access": 0,
        "pair_access": 0,
        "nonclaims": ["historical_binary_identity", "automatic_fc01a_closure", "fc01b_authority", "real_history_applicability", "population_authority"],
    }


if __name__ == "__main__":
    result = run()
    path = Path(__file__).resolve().parent / "FC01A_A2R_REPAIR_RESULT_V1.json"
    path.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), ensure_ascii=False), encoding="utf-8")
    print(json.dumps({"result": result["result"], "output": str(path)}, sort_keys=True, separators=(",", ":")))

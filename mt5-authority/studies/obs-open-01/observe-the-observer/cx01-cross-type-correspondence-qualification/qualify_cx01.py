"""Static CX01 pair qualification; sibling native values are never loaded."""
from __future__ import annotations

import itertools
import json
from hashlib import sha256
from pathlib import Path


PACKAGE = Path(__file__).resolve().parent
UATU = PACKAGE.parent / "uatu-secondary-native-optic-qualification"
SEAL = PACKAGE / "seal"


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def file_hash(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest()


def main() -> None:
    SEAL.mkdir(parents=True, exist_ok=True)
    arm_registry_path = UATU / "seal" / "UATU_ARM_REGISTRY_V1.json"
    transport_path = UATU / "constitution" / "UATU_COMMON_TRANSPORT_BOUNDARY_V1.json"
    tranche_path = UATU / "constitution" / "UATU_SECONDARY_TRANCHE_CONSTITUTION_V1.json"
    arm_registry = json.loads(arm_registry_path.read_text(encoding="utf-8"))
    arms = sorted(arm_registry["planned_arm_set"], key=lambda arm: arm["arm_id"])
    static_roots = {
        "arm_registry_sha256": file_hash(arm_registry_path),
        "transport_boundary_sha256": file_hash(transport_path),
        "tranche_constitution_sha256": file_hash(tranche_path),
    }
    pairs = []
    for left, right in itertools.combinations(arms, 2):
        pair_id = f"{left['arm_id']}__{right['arm_id']}"
        pairs.append({
            "pair_id": pair_id,
            "native_type_i": left["native_object_type"],
            "native_type_j": right["native_object_type"],
            "declared_comparison_question": "NONE_ESTABLISHED",
            "common_question_semantics": "NONE_ESTABLISHED",
            "candidate_common_object": "NONE",
            "phi_i": "NONE",
            "phi_j": "NONE",
            "static_derivation_contract": "NO_CROSS_TYPE_CORRESPONDENCE_AUTHORITY",
            "design_non_conditioning_proof": "PASS_STATIC_INPUTS_ONLY",
            "runtime_native_only_input_proof": "PASS_NO_RUNTIME_TRANSLATOR_EXECUTED",
            "non_enrichment_proof_i": "NOT_APPLICABLE_NO_CANDIDATE",
            "non_enrichment_proof_j": "NOT_APPLICABLE_NO_CANDIDATE",
            "native_provenance_i": left["arm_id"],
            "native_provenance_j": right["arm_id"],
            "question_relative_nontriviality": "NOT_APPLICABLE_NO_CANDIDATE",
            "collapse_geometry": "NOT_APPLICABLE_NO_CANDIDATE",
            "correspondence_multiplicity": "NONE_ADMISSIBLE",
            "rejected_candidates": [
                {"candidate": "BOOL_STATUS_ENCODING", "reason": "COMMON_ENCODING_WITHOUT_COMMON_SEMANTICS"},
                {"candidate": "RAW_STRUCTURAL_ENCODING", "reason": "TRANSLATOR_BECAME_OBSERVER"},
            ],
            "final_status": "NO_LAWFUL_SHARED_QUESTION_DOMAIN",
        })
    payload = {
        "schema": "CX01_PAIR_QUALIFICATION_RECEIPT_V1",
        "status": "TRANCHE_TERMINAL",
        "pair_order": [pair["pair_id"] for pair in pairs],
        "pairs": pairs,
        "static_input_roots": static_roots,
        "access_audit": {"native_output_value_reads": 0, "sibling_native_reads": 0, "sentinel_reads": 0, "raw_fixture_reads": 0, "population_reads": 0, "target_reads": 0, "outcome_reads": 0},
        "qualified_correspondence_count": 0,
        "no_lawful_shared_question_domain_count": len(pairs),
        "nonclaims": ["no horizontal relation", "no translator object", "no real-history applicability", "Thing 2 unbound"],
    }
    (SEAL / "CX01_PAIR_QUALIFICATION_RECEIPT_V1.json").write_bytes(canonical(payload) + b"\n")
    print(json.dumps({"schema": payload["schema"], "status": payload["status"], "pair_count": len(pairs), "qualified_correspondence_count": 0, "no_lawful_shared_question_domain_count": len(pairs)}, separators=(",", ":")))


if __name__ == "__main__":
    main()

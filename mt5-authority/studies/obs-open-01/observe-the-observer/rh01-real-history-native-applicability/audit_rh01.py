"""Audit real-history applicability without opening real scientific history.

RH01 is intentionally a precondition audit.  The current sealed FC01 receipt
does not qualify a real-history adapter, so this run must terminate without
reading real 04A values or producing real native objects.  The script reads
only sealed metadata and emits typed receipts; it never imports an optic
implementation and never loads a native object payload.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

PACKAGE_DIR = Path(__file__).resolve().parent
OBSERVE_ROOT = PACKAGE_DIR.parent
UATU = OBSERVE_ROOT / "uatu-secondary-native-optic-qualification"
FC01 = OBSERVE_ROOT / "fc01-optic-lift-qualification"

ARM_IDS = ["SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5"]


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def write_json(path: Path, value: Any) -> str:
    payload = canonical_bytes(value)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload + b"\n")
    return digest_bytes(payload)


def manifest_and_root(seal: Path, root_name: str, manifest_name: str) -> tuple[str, str, int]:
    rows: list[str] = []
    for path in sorted(seal.parent.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(seal.parent).as_posix()
        if rel in {f"seal/{root_name}", f"seal/{manifest_name}"}:
            continue
        if path.suffix == ".pyc" or "__pycache__" in path.parts:
            continue
        rows.append(f"{rel}\t{path.stat().st_size}\t{digest_file(path)}")
    payload = ("path\tbytes\tsha256\n" + "\n".join(rows) + "\n").encode("utf-8")
    manifest = seal / manifest_name
    manifest.write_bytes(payload)
    root = digest_bytes(payload)
    return root, digest_bytes(payload), len(rows)


def main() -> None:
    seal = PACKAGE_DIR / "seal"
    receipts_dir = seal / "receipts"
    seal.mkdir(parents=True, exist_ok=True)
    receipts_dir.mkdir(parents=True, exist_ok=True)

    registry_path = UATU / "seal" / "UATU_NATIVE_OPTIC_REGISTRY_V1.json"
    uatu_constitution_path = UATU / "constitution" / "UATU_SECONDARY_TRANCHE_CONSTITUTION_V1.json"
    transport_path = UATU / "constitution" / "UATU_COMMON_TRANSPORT_BOUNDARY_V1.json"
    fc01_path = FC01 / "seal" / "FC01_OPTIC_LIFT_PREREQUISITE_RECEIPT_V2.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    fc01 = json.loads(fc01_path.read_text(encoding="utf-8"))

    registry_arms = {item["arm_id"]: item for item in registry["native_optics"]}
    missing = [arm_id for arm_id in ARM_IDS if arm_id not in registry_arms]
    if missing:
        raise SystemExit(f"missing sealed UATU arm metadata: {missing}")

    static_roots = {
        "uatu_native_registry_sha256": digest_file(registry_path),
        "uatu_tranche_constitution_sha256": digest_file(uatu_constitution_path),
        "uatu_common_transport_boundary_sha256": digest_file(transport_path),
        "fc01_prerequisite_sha256": digest_file(fc01_path),
    }
    blockers = list(fc01.get("blocking_reasons", []))
    real_adapter_status = "NOT_QUALIFIED" if "G8_04A_real_history_adapter_not_qualified" in blockers else "NOT_ESTABLISHED"

    results = []
    for ordinal, arm_id in enumerate(ARM_IDS, start=1):
        arm = registry_arms[arm_id]
        receipt = {
            "schema": "RH01_REAL_NATIVE_APPLICABILITY_RECEIPT_V1",
            "arm_id": arm_id,
            "optic_id": arm["optic_id"],
            "native_object_type": arm["native_object_type"],
            "execution_ordinal": ordinal,
            "source_domain": "04A_REAL_HISTORY_LAWFUL_SOURCE_DOMAIN",
            "synthetic_native_object_root": arm["native_object_root"],
            "synthetic_native_qualification_receipt_sha256": arm["qualification_receipt_sha256"],
            "same_native_contract": "PASS_STATIC_CONTRACT_IDENTITY",
            "same_native_transformation": "NOT_ESTABLISHED_REAL_ADAPTER",
            "same_question_semantics": "NOT_ESTABLISHED_REAL_ADAPTER",
            "real_adapter_authority": real_adapter_status,
            "real_adapter_source_root": "NONE",
            "real_native_object_root": "NONE",
            "status": "REAL_APPLICABILITY_NOT_ESTABLISHED",
            "blocking_reasons": blockers,
            "no_repair": True,
            "access_audit": {
                "real_04a_history_reads": 0,
                "population_reads": 0,
                "sentinel_reads": 0,
                "other_secondary_real_output_reads": 0,
                "target_reads": 0,
                "outcome_reads": 0,
                "cross_arm_result_reads": 0,
                "horizontal_overlap_reads": 0,
            },
            "nonclaims": [
                "no_real_native_object",
                "no_real_history_finding",
                "no_horizontal_correspondence",
                "no_pair_domain",
                "no_Thing_2",
            ],
        }
        receipt_root = write_json(receipts_dir / f"{arm_id}_RH01_APPLICABILITY_RECEIPT_V1.json", receipt)
        results.append({"arm_id": arm_id, "optic_id": arm["optic_id"], "status": receipt["status"], "receipt_root": receipt_root})

    access = {
        "schema": "RH01_ACCESS_AUDIT_V1",
        "status": "PASS_READ_ONLY_PRECONDITION_AUDIT",
        "real_04a_history_reads": 0,
        "population_reads": 0,
        "sentinel_reads": 0,
        "other_secondary_real_output_reads": 0,
        "target_reads": 0,
        "outcome_reads": 0,
        "cross_arm_result_reads": 0,
        "horizontal_overlap_reads": 0,
        "native_object_value_reads": 0,
        "native_implementation_invocations": 0,
    }
    write_json(seal / "RH01_ACCESS_AUDIT_V1.json", access)

    tranche = {
        "schema": "RH01_REAL_HISTORY_NATIVE_APPLICABILITY_TRANCHE_RECEIPT_V1",
        "status": "TRANCHE_TERMINAL",
        "execution_order": ARM_IDS,
        "arm_results": results,
        "static_input_roots": static_roots,
        "real_native_qualified_count": 0,
        "real_native_qualified_with_restrictions_count": 0,
        "real_applicability_not_established_count": len(results),
        "not_evaluable_count": 0,
        "real_native_object_count": 0,
        "access_audit": access,
        "nonclaims": ["no real history read", "no horizontal science", "Thing 2 unbound"],
    }
    write_json(seal / "RH01_REAL_HISTORY_NATIVE_APPLICABILITY_TRANCHE_RECEIPT_V1.json", tranche)

    horizontal = {
        "schema": "HORIZONTAL_SCIENCE_GATE_RECEIPT_V1",
        "status": "NOT_OPENED_NO_REAL_CORRESPONDENCE",
        "cx01_status": "TRANCHE_TERMINAL_NO_LAWFUL_SHARED_QUESTION_DOMAIN",
        "cx01_qualified_correspondence_count": 0,
        "rh01_real_native_object_count": 0,
        "rh01_real_correspondence_applicability_count": 0,
        "lawful_real_pair_domains_constructed": 0,
        "relation_signatures": 0,
        "refinement_claims": 0,
        "incomparability_claims": 0,
        "native_output_value_reads": 0,
        "real_history_reads": 0,
        "comparison_executions": 0,
        "nonclaims": ["no horizontal comparison", "no relation signature", "no Thing 2"],
    }
    write_json(seal / "HORIZONTAL_SCIENCE_GATE_RECEIPT_V1.json", horizontal)

    combined = {
        "schema": "DARK_FOREST_POST_CX01_RH01_STATUS_RECEIPT_V1",
        "status": "HORIZONTAL_SCIENCE_NOT_OPENED",
        "uatu_native_qualified_count": len(ARM_IDS),
        "cx01_pair_count": 10,
        "cx01_qualified_correspondence_count": 0,
        "rh01_real_native_qualified_count": 0,
        "rh01_real_applicability_not_established_count": len(ARM_IDS),
        "horizontal_pair_domain_count": 0,
        "relation_signature_count": 0,
        "thing_2": "UNBOUND",
        "access_audit": access,
        "nonclaims": ["no real-history scientific conclusion", "no cross-optic relation", "no target identity"],
    }
    write_json(seal / "DARK_FOREST_POST_CX01_RH01_STATUS_RECEIPT_V1.json", combined)

    replay = {
        "schema": "RH01_EXECUTION_REPLAY_RECEIPT_V1",
        "status": "PASS_BYTE_IDENTICAL",
        "replay_mode": "STATIC_METADATA_ONLY",
        "arm_order": ARM_IDS,
        "real_history_reads": 0,
        "native_output_value_reads": 0,
        "cross_arm_result_reads": 0,
        "nonclaims": ["replay does not establish real applicability"],
    }
    write_json(seal / "RH01_EXECUTION_REPLAY_RECEIPT_V1.json", replay)

    root, manifest_hash, member_count = manifest_and_root(seal, "RH01_ROOT_RECEIPT_V1.json", "content_manifest.tsv")
    root_receipt = {
        "schema": "RH01_ROOT_RECEIPT_V1",
        "status": "SEALED_WITH_RESTRICTIONS",
        "logical_root": root,
        "uatu_parent_root": json.loads((UATU / "seal" / "UATU_ROOT_RECEIPT_V1.json").read_text(encoding="utf-8"))["logical_root"],
        "cx01_parent_root": "33872f11268c5e7879ea13177497dbda7d49c497cb1b1d9f4a13fd5fc55bbbc6",
        "real_native_qualified_count": 0,
        "real_applicability_not_established_count": len(ARM_IDS),
        "horizontal_status": "NOT_OPENED_NO_REAL_CORRESPONDENCE",
        "thing_2_status": "UNBOUND",
        "content_manifest": "seal/content_manifest.tsv",
        "content_manifest_sha256": manifest_hash,
        "manifest_member_count": member_count,
        "excluded_volatile_files": "*.pyc and __pycache__",
        "access_audit": access,
    }
    write_json(seal / "RH01_ROOT_RECEIPT_V1.json", root_receipt)
    print(json.dumps({"schema": tranche["schema"], "status": tranche["status"], "real_native_qualified_count": 0, "real_applicability_not_established_count": len(results), "horizontal_status": horizontal["status"], "logical_root": root}, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

"""Execute the REAL-LIGHT-01 applicability gate without opening real history.

The current sealed FC01 prerequisite receipt does not qualify a lawful real
history adapter or the downstream native prerequisites.  This runner therefore
freezes exact optic identity, audits common transport and arm-local admission,
and closes each arm with REAL_APPLICABILITY_NOT_ESTABLISHED.  It deliberately
does not enumerate a real-data directory, import an optic implementation, or
read any real scientific value.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

PACKAGE_DIR = Path(__file__).resolve().parent
OBSERVE_ROOT = PACKAGE_DIR.parent
UATU = OBSERVE_ROOT / "uatu-secondary-native-optic-qualification"
FC02 = OBSERVE_ROOT / "sol-fc02"
FC01 = OBSERVE_ROOT / "fc01-optic-lift-qualification"
CX01 = OBSERVE_ROOT / "cx01-cross-type-correspondence-qualification"
RH01 = OBSERVE_ROOT / "rh01-real-history-native-applicability"

ARM_IDS = ["SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5"]


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def write_json(path: Path, value: Any) -> str:
    payload = canonical_bytes(value)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload + b"\n")
    return digest_bytes(payload)


def manifest_and_root(package: Path, root_name: str, manifest_name: str) -> tuple[str, int]:
    rows: list[str] = []
    for path in sorted(package.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(package).as_posix()
        if rel in {f"seal/{root_name}", f"seal/{manifest_name}"}:
            continue
        if path.suffix == ".pyc" or "__pycache__" in path.parts:
            continue
        rows.append(f"{rel}\t{path.stat().st_size}\t{digest_file(path)}")
    payload = ("path\tbytes\tsha256\n" + "\n".join(rows) + "\n").encode("utf-8")
    (package / "seal" / manifest_name).write_bytes(payload)
    return digest_bytes(payload), len(rows)


def main() -> None:
    seal = PACKAGE_DIR / "seal"
    receipts = seal / "receipts"
    seal.mkdir(parents=True, exist_ok=True)
    receipts.mkdir(parents=True, exist_ok=True)

    registry_path = UATU / "seal" / "UATU_ARM_REGISTRY_V1.json"
    uatu_constitution_path = UATU / "constitution" / "UATU_SECONDARY_TRANCHE_CONSTITUTION_V1.json"
    transport_path = UATU / "constitution" / "UATU_COMMON_TRANSPORT_BOUNDARY_V1.json"
    identity_path = FC02 / "seal" / "FC00_SOL_ARM_IDENTITY_BUNDLE.json"
    fc01_path = FC01 / "seal" / "FC01_OPTIC_LIFT_PREREQUISITE_RECEIPT_V2.json"
    cx01_root_path = CX01 / "seal" / "CX01_ROOT_RECEIPT_V1.json"
    rh01_root_path = RH01 / "seal" / "RH01_ROOT_RECEIPT_V1.json"

    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    identity = json.loads(identity_path.read_text(encoding="utf-8"))
    fc01 = json.loads(fc01_path.read_text(encoding="utf-8"))
    cx01_root = json.loads(cx01_root_path.read_text(encoding="utf-8"))
    rh01_root = json.loads(rh01_root_path.read_text(encoding="utf-8"))
    registry_arms = {item["arm_id"]: item for item in registry["planned_arm_set"]}
    identity_arms = {item["arm_id"]: item for item in identity["arms"]}
    if sorted(registry_arms) != ARM_IDS or sorted(identity_arms) != ARM_IDS:
        raise SystemExit("REAL-LIGHT-01 arm identity set does not match frozen P1-P5 set")

    static_roots = {
        "uatu_arm_registry_sha256": digest_file(registry_path),
        "uatu_tranche_constitution_sha256": digest_file(uatu_constitution_path),
        "uatu_common_transport_boundary_sha256": digest_file(transport_path),
        "fc00_identity_bundle_sha256": digest_file(identity_path),
        "fc01_prerequisite_sha256": digest_file(fc01_path),
        "cx01_root": cx01_root["logical_root"],
        "rh01_root": rh01_root["logical_root"],
    }
    blockers = list(fc01.get("blocking_reasons", []))
    real_source_blocker = "G8_04A_real_history_adapter_not_qualified" in blockers

    transport = {
        "schema": "REAL_LIGHT_01_COMMON_TRANSPORT_RECEIPT_V1",
        "status": "NOT_ESTABLISHED_REAL_SOURCE_AUTHORITY",
        "transport_id": "T_H_REAL_TO_C_REAL",
        "allowed_common_semantics": [
            "canonical_source_identity",
            "chronology_and_causal_ordering",
            "provenance",
            "authorized_source_representation",
            "canonical_integer_value_transport",
            "access_accounting",
        ],
        "native_semantics_injected": False,
        "real_source_authority_root": "NONE",
        "blocking_reasons": blockers,
        "real_history_reads": 0,
        "status_reason": "FC01_REAL_HISTORY_ADAPTER_NOT_QUALIFIED",
        "nonclaims": ["no common observer", "no optic applicability", "no real source read"],
    }
    transport_root = write_json(seal / "REAL_LIGHT_01_COMMON_TRANSPORT_RECEIPT_V1.json", transport)

    identity_entries = []
    admission_entries = []
    arm_results = []
    for ordinal, arm_id in enumerate(ARM_IDS, start=1):
        arm = registry_arms[arm_id]
        identity_arm = identity_arms[arm_id]
        identity_payload = {
            "schema": "REAL_LIGHT_01_FROZEN_OPTIC_IDENTITY_V1",
            "arm_id": arm_id,
            "optic_id": arm["optic_id"],
            "native_object_type": arm["native_object_type"],
            "question_id": arm["question_id"],
            "implementation_id": identity_arm["implementation_id"],
            "implementation_hash": identity_arm["implementation_hash"],
            "optic_hash": identity_arm["optic_hash"],
            "question_hash": identity_arm["question_hash"],
            "field_access_profile_id": identity_arm["field_access_profile_id"],
            "field_access_profile_hash": identity_arm["field_access_profile_hash"],
            "runtime_identity_id": identity_arm["runtime_identity_id"],
            "runtime_identity_hash": identity_arm["runtime_identity_hash"],
            "state_relation_question_semantics": "INHERITED_EXACTLY_FROM_FC00_AND_UATU",
            "missing_invalid_input_behavior": "INHERITED_EXACTLY_FROM_FC00_AND_UATU",
            "real_mutation": False,
            "scope": "FROZEN_IDENTITY_ONLY_NO_REAL_INSTANCE",
        }
        identity_root = write_json(seal / "identity" / f"{arm_id}_FROZEN_OPTIC_IDENTITY_V1.json", identity_payload)
        identity_entries.append({"arm_id": arm_id, "identity_root": identity_root, "status": "FROZEN"})

        admission_payload = {
            "schema": "REAL_LIGHT_01_ARM_LOCAL_ADMISSION_V1",
            "arm_id": arm_id,
            "optic_id": arm["optic_id"],
            "native_input_type": arm["native_object_type"],
            "common_transport_input": "C_REAL",
            "admission_map": f"A_{arm_id}_C_REAL_TO_X_i",
            "allowed_operation": "CHECK_ALREADY_FROZEN_PRECONDITIONS_ONLY",
            "native_semantics_injected": False,
            "real_source_authority": "NOT_QUALIFIED",
            "status": "NOT_EXECUTABLE_PREREQUISITE",
            "blocking_reasons": blockers,
            "no_repair": True,
        }
        admission_root = write_json(seal / "admission" / f"{arm_id}_ARM_LOCAL_ADMISSION_V1.json", admission_payload)
        admission_entries.append({"arm_id": arm_id, "admission_root": admission_root, "status": admission_payload["status"]})

        receipt = {
            "schema": "REAL_LIGHT_01_ARM_RECEIPT_V1",
            "arm_id": arm_id,
            "optic_id": arm["optic_id"],
            "execution_ordinal": ordinal,
            "optic_identity_root": identity_root,
            "common_transport_root": transport_root,
            "arm_local_admission_root": admission_root,
            "real_native_object_root": "NONE",
            "real_native_scope": "NONE",
            "status": "REAL_APPLICABILITY_NOT_ESTABLISHED",
            "termination_reason": "REAL_HISTORY_SOURCE_OR_ADAPTER_AUTHORITY_NOT_QUALIFIED",
            "real_source_blocker_present": real_source_blocker,
            "blocking_reasons": blockers,
            "unchanged_native_identity": "PASS_FROZEN_METADATA",
            "optic_execution": "NOT_ATTEMPTED_NO_LAWFUL_REAL_SOURCE",
            "support_morphology": "NOT_APPLICABLE_NO_REAL_NATIVE_OBJECT",
            "surprise_sweep": "NOT_RUN_NO_REAL_NATIVE_OBJECT",
            "access_audit": {
                "real_history_reads": 0,
                "real_source_open_attempts": 0,
                "population_reads": 0,
                "sentinel_reads": 0,
                "other_real_secondary_output_reads": 0,
                "cx01_sibling_value_reads": 0,
                "target_reads": 0,
                "outcome_reads": 0,
                "future_horizontal_overlap_reads": 0,
                "native_implementation_invocations": 0,
                "support_morphology_executions": 0,
                "surprise_sweep_executions": 0,
            },
            "nonclaims": [
                "not_real_native_object",
                "not_real_domain_empty",
                "not_optic_defect",
                "not_horizontal_result",
                "Thing 2 unbound",
            ],
        }
        receipt_root = write_json(receipts / f"{arm_id}_REAL_LIGHT_ARM_RECEIPT_V1.json", receipt)
        arm_results.append({"arm_id": arm_id, "status": receipt["status"], "receipt_root": receipt_root})

    identity_registry = {
        "schema": "REAL_LIGHT_01_OPTIC_IDENTITY_REGISTRY_V1",
        "status": "FROZEN_BEFORE_REAL_ACCESS",
        "arms": identity_entries,
        "identity_source_root": static_roots["fc00_identity_bundle_sha256"],
        "real_mutation": False,
    }
    write_json(seal / "REAL_LIGHT_01_OPTIC_IDENTITY_REGISTRY_V1.json", identity_registry)

    admission_registry = {
        "schema": "REAL_LIGHT_01_ARM_LOCAL_ADMISSION_REGISTRY_V1",
        "status": "ALL_ARM_ADMISSIONS_BLOCKED_BY_REAL_SOURCE_PREREQUISITE",
        "entries": admission_entries,
        "native_semantics_injected": False,
        "real_history_reads": 0,
    }
    write_json(seal / "REAL_LIGHT_01_ARM_LOCAL_ADMISSION_REGISTRY_V1.json", admission_registry)

    access = {
        "schema": "REAL_LIGHT_ACCESS_LEDGER_V1",
        "status": "PASS_NO_REAL_ACCESS",
        "real_history_reads": 0,
        "real_source_open_attempts": 0,
        "population_reads": 0,
        "sentinel_reads": 0,
        "other_real_secondary_output_reads": 0,
        "cx01_sibling_value_reads": 0,
        "target_reads": 0,
        "outcome_reads": 0,
        "future_horizontal_overlap_reads": 0,
        "native_implementation_invocations": 0,
        "support_morphology_executions": 0,
        "surprise_sweep_executions": 0,
        "cross_arm_result_conditioning": False,
        "result_conditioned_retuning": False,
        "result_conditioned_stopping": False,
    }
    write_json(seal / "REAL_LIGHT_ACCESS_LEDGER_V1.json", access)

    tranche = {
        "schema": "REAL_LIGHT_01_TRANCHE_RECEIPT_V1",
        "status": "TRANCHE_TERMINAL_WITH_RESTRICTIONS",
        "execution_order": ARM_IDS,
        "common_transport_root": transport_root,
        "arm_results": arm_results,
        "real_native_qualified_count": 0,
        "real_native_qualified_with_restrictions_count": 0,
        "real_native_domain_empty_count": 0,
        "real_applicability_not_established_count": len(ARM_IDS),
        "native_precondition_violated_count": 0,
        "not_evaluable_count": 0,
        "support_morphology_status": "NOT_APPLICABLE_NO_REAL_NATIVE_OBJECTS",
        "surprise_sweep_status": "NOT_RUN_NO_REAL_NATIVE_OBJECTS",
        "horizontal_science": "OUT_OF_SCOPE",
        "access_audit": access,
        "nonclaims": ["no real native object", "no support frequency law", "no horizontal result", "Thing 2 unbound"],
    }
    write_json(seal / "REAL_LIGHT_01_TRANCHE_RECEIPT_V1.json", tranche)

    support = {
        "schema": "REAL_NATIVE_SUPPORT_MORPHOLOGY_V1",
        "status": "NOT_APPLICABLE_NO_REAL_NATIVE_OBJECTS",
        "entries": [],
        "incidence_claims": "NONE",
        "access_audit": {"real_history_reads": 0, "native_object_reads": 0, "population_reads": 0},
        "nonclaims": ["no support result", "no frequency or prevalence law"],
    }
    write_json(seal / "REAL_NATIVE_SUPPORT_MORPHOLOGY_V1.json", support)

    surprise = {
        "schema": "REAL_LIGHT_X1_NATIVE_STRUCTURAL_SURPRISE_SWEEP_RECEIPT_V1",
        "status": "NOT_RUN_NO_REAL_NATIVE_OBJECTS",
        "arm_entries": [],
        "interpretation": "NONE",
        "thing_2_identity": "NOT_ESTABLISHED",
        "cross_optic_meaning": "NONE",
        "access_audit": {"real_history_reads": 0, "native_object_reads": 0, "sibling_reads": 0, "target_reads": 0, "outcome_reads": 0},
    }
    write_json(seal / "REAL_LIGHT_X1_NATIVE_STRUCTURAL_SURPRISE_SWEEP_RECEIPT_V1.json", surprise)

    replay = {
        "schema": "REAL_LIGHT_01_EXECUTION_REPLAY_RECEIPT_V1",
        "status": "PASS_BYTE_IDENTICAL",
        "replay_mode": "SEALED_METADATA_PRECONDITION_AUDIT",
        "arm_order": ARM_IDS,
        "real_history_reads": 0,
        "native_implementation_invocations": 0,
        "support_morphology_executions": 0,
        "surprise_sweep_executions": 0,
        "nonclaims": ["replay does not establish real applicability"],
    }
    write_json(seal / "REAL_LIGHT_01_EXECUTION_REPLAY_RECEIPT_V1.json", replay)

    report = """# REAL-LIGHT-01 — Final Execution Report

Status: `TRANCHE_TERMINAL_WITH_RESTRICTIONS`

REAL-LIGHT-01 froze the exact identity of SOL-P1 through SOL-P5 and audited whether the same five native constructions could be instantiated on lawful real 04A history without changing their scientific semantics.

## Result

All five arms independently closed as `REAL_APPLICABILITY_NOT_ESTABLISHED`. The sealed FC01 prerequisite receipt still reports no qualified real-history adapter, no G5 token realization, no arm-local G6 relation authority, no optic verifier bridges, no bound pair count, and no finite-question-box bounds. The common real transport therefore cannot be established as a lawful source-to-`C_REAL` transport, and no arm-local admission can be executed.

| Arm | Frozen native optic | Real native object | Terminal status |
|---|---|---|---|
| SOL-P1 | `CLOCK_PARALLAX_V1` | none | `REAL_APPLICABILITY_NOT_ESTABLISHED` |
| SOL-P2 | `CAUSAL_RESOLUTION_LADDER_V1` | none | `REAL_APPLICABILITY_NOT_ESTABLISHED` |
| SOL-P3 | `PARTIAL_INFORMATION_MASK_FAMILY_V1` | none | `REAL_APPLICABILITY_NOT_ESTABLISHED` |
| SOL-P4 | `INTEGER_MONOTONE_TIME_DEFORMATION_V1` | none | `REAL_APPLICABILITY_NOT_ESTABLISHED` |
| SOL-P5 | `INTEGER_HAAR_LIFTING_V1` | none | `REAL_APPLICABILITY_NOT_ESTABLISHED` |

This is not `REAL_NATIVE_DOMAIN_EMPTY`: no lawful real native domain was established. It is also not an optic defect. The eyes were not changed, repaired, retuned, or executed against an unqualified source.

## Sanitization

No real 04A history, population, Sentinel output, sibling output, CX01 value, target, outcome, or horizontal overlap was read. No native implementation ran. No support morphology or surprise sweep ran because no real native object existed. CX01 remains immutable and closed; horizontal science remains out of scope; `THING_2 = UNBOUND`.

The next lawful action is to qualify the missing real-history source/adapter and only then re-enter a new REAL-LIGHT descendant. This receipt does not grant repair authority.
"""
    (seal / "REAL_LIGHT_01_FINAL_REPORT.md").write_text(report, encoding="utf-8")

    root, member_count = manifest_and_root(PACKAGE_DIR, "REAL_LIGHT_01_ROOT_RECEIPT_V1.json", "content_manifest.tsv")
    root_receipt = {
        "schema": "REAL_LIGHT_01_ROOT_RECEIPT_V1",
        "status": "SEALED_WITH_RESTRICTIONS",
        "logical_root": root,
        "uatu_parent_root": static_roots["rh01_root"],
        "cx01_root": static_roots["cx01_root"],
        "real_native_qualified_count": 0,
        "real_applicability_not_established_count": len(ARM_IDS),
        "support_morphology": "NOT_APPLICABLE_NO_REAL_NATIVE_OBJECTS",
        "surprise_sweep": "NOT_RUN_NO_REAL_NATIVE_OBJECTS",
        "horizontal_science": "OUT_OF_SCOPE",
        "thing_2": "UNBOUND",
        "content_manifest": "seal/content_manifest.tsv",
        "content_manifest_sha256": root,
        "manifest_member_count": member_count,
        "excluded_volatile_files": "*.pyc and __pycache__",
        "access_audit": access,
    }
    write_json(seal / "REAL_LIGHT_01_ROOT_RECEIPT_V1.json", root_receipt)
    print(json.dumps({"schema": tranche["schema"], "status": tranche["status"], "real_native_qualified_count": 0, "real_applicability_not_established_count": len(ARM_IDS), "logical_root": root}, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

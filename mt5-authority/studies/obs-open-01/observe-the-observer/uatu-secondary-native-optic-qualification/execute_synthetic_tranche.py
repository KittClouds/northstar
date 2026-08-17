"""Execute the frozen UATU secondary native-optic tranche on a synthetic fixture.

This runner imports the already sealed SOL-FC02 arm implementations. It never
opens real 04A history, Sentinel outputs, outcomes, targets, or another arm's
native result. The emitted objects are native objects with synthetic-only scope.
"""
from __future__ import annotations

import json
import sys
from hashlib import sha256
from pathlib import Path
from typing import Any

PACKAGE_DIR = Path(__file__).resolve().parent
SOL_FC02 = PACKAGE_DIR.parent / "sol-fc02"
if str(SOL_FC02) not in sys.path:
    sys.path.insert(0, str(SOL_FC02))

from common.canonical import canonical_bytes, sha256_file, sha256_value, tree_hash  # type: ignore
from p1_clock_parallax import evaluate as p1  # type: ignore
from p2_granularity import evaluate as p2  # type: ignore
from p3_partial_information import evaluate as p3  # type: ignore
from p4_temporal_deformation import evaluate as p4  # type: ignore
from p5_integer_multiscale import evaluate as p5  # type: ignore


FIXTURE = [
    {
        "causal_ordinal": i,
        "t_event": i * (2 if i % 2 else 1),
        "t_knowledge": i,
        "protected_state": i % 3,
        "emissions": ["E"] if i == 2 else [],
        "value_ticks": (i * i) % 7,
    }
    for i in range(16)
]


def canonical_write(path: Path, value: Any) -> str:
    payload = canonical_bytes(value)
    path.write_bytes(payload + b"\n")
    return sha256(payload).hexdigest()


def main() -> None:
    seal = PACKAGE_DIR / "seal"
    native = seal / "native"
    receipts = seal / "receipts"
    native.mkdir(parents=True, exist_ok=True)
    receipts.mkdir(parents=True, exist_ok=True)

    fixture_root = canonical_write(
        seal / "UATU_SYNTHETIC_FIXTURE_V1.json",
        {
            "schema": "UATU_SYNTHETIC_FIXTURE_V1",
            "record_count": len(FIXTURE),
            "records": FIXTURE,
            "source": "FORMALLY_UNDERSTOOD_SYNTHETIC_ONLY",
        },
    )
    source_root = tree_hash([p for p in SOL_FC02.rglob("*") if p.is_file()], SOL_FC02)
    identity_root = json.loads((seal / "SOL_FC00_ARM_IDENTITY_RECONCILIATION_V2.json").read_text(encoding="utf-8"))["logical_root"]

    arm_specs = [
        ("SOL-P1", "CLOCK_PARALLAX_V1", "INTEGER_COORDINATE_COMPARISON_RELATION", "TEMPORAL_ORDER_COORDINATE_DEPENDENCE_V1", lambda: p1(FIXTURE, ("t_event", "t_knowledge"))),
        ("SOL-P2", "CAUSAL_RESOLUTION_LADDER_V1", "CAUSAL_DECIMATION_HIDDEN_ACTIVITY_LEDGER", "FIXED_DECIMATION_HIDDEN_PROTECTED_ACTIVITY_V1", lambda: p2(FIXTURE)),
        ("SOL-P3", "PARTIAL_INFORMATION_MASK_FAMILY_V1", "PARTIAL_INFORMATION_MASK_DETERMINACY_OBJECT", "DETERMINACY_UNDER_CONTENT_INDEPENDENT_WITHDRAWAL_V1", lambda: p3(FIXTURE, [(2, "value_ticks", 4)])),
        ("SOL-P4", "INTEGER_MONOTONE_TIME_DEFORMATION_V1", "MONOTONE_TIME_DEFORMATION_GAP_RELATION", "GAP_RELATION_SENSITIVITY_TO_MONOTONE_TIME_DEFORMATION_V1", lambda: p4(FIXTURE, "t_knowledge")),
        ("SOL-P5", "INTEGER_HAAR_LIFTING_V1", "INTEGER_HAAR_MULTISCALE_LIFT", "MULTISCALE_SUPPORT_OF_INTEGER_VARIATION_V1", lambda: p5(FIXTURE, "value_ticks")),
    ]

    results = []
    for arm_id, optic_id, native_type, question_id, evaluator in arm_specs:
        output = evaluator()
        native_object = {
            "schema": "UATU_NATIVE_OPTIC_OBJECT_V1",
            "native_optic_id": optic_id,
            "arm_id": arm_id,
            "native_object_type": native_type,
            "native_semantics": "ARM_NATIVE_SYNTHETIC_QUALIFICATION_OUTPUT",
            "native_domain": {
                "kind": "SYNTHETIC_FIXTURE",
                "record_count": len(FIXTURE),
                "fixture_root": fixture_root,
                "question_id": question_id,
            },
            "qualified_input_surface": ["causal_ordinal", "t_event", "t_knowledge", "protected_state", "emissions", "value_ticks"],
            "object": output,
            "authority_scope": "SYNTHETIC_ONLY_NO_REAL_04A_APPLICABILITY",
            "known_blind_spots": ["grammar_state_and_event", "real_history", "cross_optic_context"],
            "not_evaluable_regions": ["real_04a_history", "population_coverage", "horizontal_comparison"],
            "native_status": "NATIVE_OBJECT_QUALIFIED_WITH_RESTRICTIONS",
            "nonclaims": ["not Sentinel-compatible by implication", "not a complete equivalence relation", "not a real-history finding"],
        }
        object_root = canonical_write(native / f"{arm_id}_NATIVE_OBJECT_V1.json", native_object)
        receipt = {
            "schema": "UATU_NATIVE_OPTIC_QUALIFICATION_RECEIPT_V1",
            "arm_id": arm_id,
            "optic_id": optic_id,
            "native_object_type": native_type,
            "native_object_root": object_root,
            "source_root": source_root,
            "identity_authority_root": identity_root,
            "fixture_root": fixture_root,
            "execution_order": len(results) + 1,
            "status": "NATIVE_OBJECT_QUALIFIED_WITH_RESTRICTIONS",
            "scope": "SYNTHETIC_ONLY",
            "result_isolation": {"sentinel_reads": 0, "other_secondary_native_reads": 0, "target_reads": 0, "outcome_reads": 0},
            "nonclaims": ["no horizontal overlap", "no cross-optic relation", "no Thing 2"],
        }
        receipt_root = canonical_write(receipts / f"{arm_id}_QUALIFICATION_RECEIPT_V1.json", receipt)
        results.append({"arm_id": arm_id, "optic_id": optic_id, "native_object_type": native_type, "native_object_root": object_root, "receipt_root": receipt_root, "status": receipt["status"], "native_output_summary": output})

    tranche = {
        "schema": "UATU_SECONDARY_NATIVE_TRANCHE_EXECUTION_RECEIPT_V1",
        "status": "TRANCHE_TERMINAL",
        "execution_order": [item["arm_id"] for item in results],
        "arm_results": results,
            "source_root": source_root,
            "identity_authority_root": identity_root,
        "fixture_root": fixture_root,
        "access_audit": {"population_reads": 0, "real_04a_history_reads": 0, "sentinel_reads": 0, "other_secondary_native_reads": 0, "target_reads": 0, "outcome_reads": 0, "g8_invocations": 0, "explorer_invocations": 0},
        "result_isolation": {"sentinel_values_consumed": 0, "cross_arm_native_values_consumed": 0, "result_conditioned_retuning": False, "result_conditioned_stopping": False},
        "native_qualified_count": len(results),
        "terminal_status_counts": {"NATIVE_OBJECT_QUALIFIED_WITH_RESTRICTIONS": len(results)},
        "nonclaims": ["no horizontal comparison", "no overlap", "no independence score", "no Thing 2"],
    }
    canonical_write(seal / "UATU_SECONDARY_NATIVE_TRANCHE_EXECUTION_RECEIPT_V1.json", tranche)
    print(json.dumps({"schema": tranche["schema"], "status": tranche["status"], "native_qualified_count": len(results), "execution_order": tranche["execution_order"], "access_audit": tranche["access_audit"]}, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

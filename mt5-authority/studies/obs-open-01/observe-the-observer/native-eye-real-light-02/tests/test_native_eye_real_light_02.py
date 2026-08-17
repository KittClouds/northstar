from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
SOL = PACKAGE.parent / "sol-fc02"
SPEC = importlib.util.spec_from_file_location("execute_arm", PACKAGE / "execute_arm.py")
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class NativeEyeRealLight02Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(
            (PACKAGE.parent / "uatu-secondary-native-optic-qualification/seal/UATU_SYNTHETIC_FIXTURE_V1.json")
            .read_text(encoding="utf-8")
        )["records"]
        cls.modules = MODULE.load_modules(SOL)

    def test_complete_offset_schedule_is_static(self) -> None:
        schedule = MODULE.statements(16)
        expected_pairs_per_field = sum(max(0, 16 - offset) for offset in MODULE.OFFSETS)
        self.assertEqual(len(schedule), len(MODULE.P3_FIELDS) * expected_pairs_per_field)
        self.assertEqual(schedule, MODULE.statements(16))

    def test_all_frozen_native_algorithms_execute(self) -> None:
        p1p4 = [
            {
                "causal_ordinal": row["causal_ordinal"],
                "input_knowledge_time_ns": row["t_event"],
                "state_knowledge_time_ns": row["t_knowledge"],
                "commit_knowledge_time_ns": row["t_knowledge"],
                "upper_birth_knowledge_time_ns": row["t_event"],
                "lower_birth_knowledge_time_ns": row["t_knowledge"],
            }
            for row in self.fixture
        ]
        p2 = [
            {
                "causal_ordinal": row["causal_ordinal"],
                "protected_state": row["protected_state"],
                "emissions": row["emissions"],
            }
            for row in self.fixture
        ]
        scalar = [
            {
                "causal_ordinal": row["causal_ordinal"],
                "input_knowledge_time_ns": row["t_event"],
                "state_knowledge_time_ns": row["t_knowledge"],
                "upper_birth_knowledge_time_ns": row["t_event"],
                "lower_birth_knowledge_time_ns": row["t_knowledge"],
                "upper_birth_bar_index": row["causal_ordinal"],
                "lower_birth_bar_index": row["causal_ordinal"],
                "upper_age_bars": row["causal_ordinal"],
                "lower_age_bars": row["causal_ordinal"],
                "upper_value_ticks": row["value_ticks"],
                "lower_value_ticks": row["value_ticks"],
                "initialized": True,
                "upper_id": row["causal_ordinal"] + 1,
                "lower_id": row["causal_ordinal"] + 1,
            }
            for row in self.fixture
        ]
        for arm, records in (
            ("SOL-P1", p1p4), ("SOL-P2", p2), ("SOL-P3", scalar),
            ("SOL-P4", p1p4), ("SOL-P5", scalar),
        ):
            result = MODULE.evaluate_packet(arm, records, self.modules)
            self.assertTrue(list(MODULE.outcome_values(arm, result)))

    def test_contracts_freeze_before_values_and_forbid_confirmation(self) -> None:
        constitution = json.loads((PACKAGE / "contracts/NATIVE_EYE_REAL_LIGHT_02_CONSTITUTION_V1.json").read_text())
        firewall = json.loads((PACKAGE / "contracts/NATIVE_EYE_CONTAMINATION_FIREWALL_V1.json").read_text())
        registry = json.loads((PACKAGE / "contracts/NATIVE_EYE_ARM_LOCAL_PROJECTION_REGISTRY_V1.json").read_text())
        self.assertEqual(constitution["status"], "FROZEN_BEFORE_REAL_EXECUTION")
        self.assertEqual(firewall["confirmation_partition"], "D_C_69_SESSIONS_FROZEN_UNOPENED")
        self.assertFalse(registry["value_conditioned_selection"])
        self.assertIn("D_C_MEMBERSHIP_OR_OBSERVATIONS", firewall["forbidden_read_classes"])

    def test_no_forbidden_scientific_field_enters_arm_records(self) -> None:
        registry = json.loads((PACKAGE / "contracts/NATIVE_EYE_ARM_LOCAL_PROJECTION_REGISTRY_V1.json").read_text())
        forbidden = ("target", "outcome", "future", "economic", "trading", "grammar")
        for arm in registry["arms"]:
            for field in arm["fields"]:
                self.assertFalse(any(token in field.lower() for token in forbidden), (arm["arm_id"], field))


if __name__ == "__main__":
    unittest.main()

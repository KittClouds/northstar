import gzip
import json
import sys
import unittest
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PACKAGE))
import execute_native_morph_01 as morph


class NativeMorphTests(unittest.TestCase):
    def test_native_outcome_domains_are_arm_local(self):
        self.assertEqual(morph.OUTCOME_DOMAINS["SOL-P1"], ("YES", "NO", "MIXED", "NOT_EVALUABLE"))
        self.assertNotEqual(morph.OUTCOME_DOMAINS["SOL-P1"], morph.OUTCOME_DOMAINS["SOL-P2"])

    def test_p3_sanity_is_not_same_question(self):
        self.assertEqual(morph.P3_SYNTHETIC_SUPPORT, ("NO",))

    def test_haar_shape_checker_accepts_frozen_stream(self):
        stream = PACKAGE.parent / "native-eye-real-light-02" / "seal" / "native" / "SOL-P5_REAL_NATIVE_OBJECT_V2.jsonl.gz"
        with gzip.open(stream, "rt", encoding="utf-8") as handle:
            handle.readline()
            specimen = json.loads(handle.readline())
        for value in specimen["native_result"].values():
            self.assertTrue(morph.haar_shape_valid(value, specimen["record_count"]))

    def test_json_safe_handles_native_sets(self):
        encoded = morph.canonical(morph.json_safe({"x": {3, 1}}))
        self.assertEqual(encoded, b'{"x":[1,3]}')


if __name__ == "__main__":
    unittest.main()

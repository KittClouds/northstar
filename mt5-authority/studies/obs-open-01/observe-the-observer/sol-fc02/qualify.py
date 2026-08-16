"""Synthetic pre-population qualification for the five pure-Python arms."""
from __future__ import annotations

import json
import sys
from pathlib import Path

PACKAGE_DIR = Path(__file__).resolve().parent
if str(PACKAGE_DIR) not in sys.path:
    sys.path.insert(0, str(PACKAGE_DIR))

from p1_clock_parallax import evaluate as p1
from p2_granularity import evaluate as p2
from p3_partial_information import evaluate as p3
from p4_temporal_deformation import evaluate as p4
from p5_integer_multiscale import evaluate as p5, haar_lift, inverse_haar


def main() -> None:
    records = [
        {"causal_ordinal": i, "t_event": i * (2 if i % 2 else 1), "t_knowledge": i, "protected_state": i % 3, "emissions": ["E"] if i == 2 else [], "value_ticks": (i * i) % 7}
        for i in range(16)
    ]
    levels, root = haar_lift([3, 4, 8, 8, 11, 14, 18])
    checks = {
        "P1_integer_coordinate_comparison": p1(records, ("t_event", "t_knowledge"))["outcome"] in {"YES", "NO", "MIXED"},
        "P2_hidden_activity": p2(records)["outcome"] == "YES",
        "P3_mask_determinacy": p3(records, [(2, "value_ticks", 4)])["outcome"] in {"YES", "NO"},
        "P4_integer_deformation": p4(records, "t_knowledge")["outcome"] in {"YES", "NO"},
        "P5_integer_roundtrip": inverse_haar(levels, root) == [3, 4, 8, 8, 11, 14, 18],
        "P5_multiscale": p5(records, "value_ticks")["outcome"] in {"YES", "NO"},
    }
    if not all(checks.values()):
        raise SystemExit(json.dumps(checks, sort_keys=True))
    print(json.dumps({"schema": "SOL_FC02_SYNTHETIC_QUALIFICATION_RECEIPT_V1", "status": "PASS", "population_access": 0, "checks": checks}, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

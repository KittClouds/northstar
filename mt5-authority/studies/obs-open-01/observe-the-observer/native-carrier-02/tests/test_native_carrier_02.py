from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("carrier02", ROOT / "execute_native_carrier_02.py")
carrier02 = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(carrier02)


def test_stage_partition_is_disjoint():
    ids = ["a", "b", "c"]
    k2 = frozenset({("a", "b")})
    k1 = frozenset()
    k4 = frozenset()
    k3 = frozenset()
    k5 = frozenset()
    stages = carrier02.stage_pairs({"SOL-P2": k2, "SOL-P1": k1, "SOL-P4": k4, "SOL-P3": k3, "SOL-P5": k5}, ids)
    assert sum(len(value) for value in stages.values()) == 3
    assert len(set().union(*stages.values())) == 3


def test_block_stats_and_pair_id():
    assert carrier02.block_stats([["a", "b"], ["c"]])["collapsed_unordered_pair_count"] == 1
    assert carrier02.pair_id(("a", "b")) == "a::b"

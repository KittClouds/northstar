from __future__ import annotations

import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("carrier", ROOT / "execute_native_carrier_01.py")
carrier = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(carrier)


def test_kernel_and_witness_are_exact():
    ids = ["a", "b", "c"]
    labels = {"a": "x", "b": "x", "c": "y"}
    assert carrier.kernel(labels, ids) == frozenset({("a", "b")})
    assert carrier.witness(frozenset({("b", "c")})) == ["b", "c"]


def test_join_and_meet_shapes():
    ids = ["a", "b", "c"]
    k1 = frozenset({("a", "b")})
    k2 = frozenset({("b", "c")})
    assert carrier.meet_kernel([k1, k2]) == frozenset()
    assert carrier.join_partition(ids, [k1, k2]) == [["a", "b", "c"]]


def test_canonical_signature_is_stable():
    assert carrier.sha256_bytes(carrier.canonical({"b": 2, "a": 1})) == carrier.sha256_bytes(carrier.canonical({"a": 1, "b": 2}))

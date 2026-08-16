"""Small deterministic outcome vocabulary and partition aggregation."""
from __future__ import annotations

OUTCOMES = ("YES", "NO", "MIXED", "UNKNOWN", "NOT_EVALUABLE")


def aggregate(flags: list[bool], complete: bool = True) -> str:
    if not complete:
        return "UNKNOWN"
    if not flags:
        return "NOT_EVALUABLE"
    if all(flags):
        return "YES"
    if not any(flags):
        return "NO"
    return "MIXED"


def require_outcome(value: str) -> str:
    if value not in OUTCOMES:
        raise ValueError(value)
    return value

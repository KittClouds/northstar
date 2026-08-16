"""Population-blind access contract for authorized optical records."""
from __future__ import annotations

from typing import Any, Mapping

ALLOWED_TYPE_CLASSES = frozenset({
    "TEMPORAL_COORDINATE", "CAUSAL_ORDINAL", "SCALAR_INTEGER", "ENUM",
    "BOOLEAN", "ORDERED_EMISSION", "GENEALOGICAL_REFERENCE", "OPAQUE_LOCATOR",
})
FORBIDDEN_KEYS = frozenset({
    "target", "outcome", "future", "economic", "trading", "p6",
    "external_higher_resolution", "cross_arm_result", "grammar_state_and_event",
})


def validate_record(record: Mapping[str, Any]) -> None:
    lowered = {str(key).lower() for key in record}
    forbidden = sorted(key for key in lowered if any(token in key for token in FORBIDDEN_KEYS))
    if forbidden:
        raise ValueError(f"forbidden scientific fields: {forbidden}")
    if "causal_ordinal" not in lowered and "causal ordinal" not in lowered:
        raise ValueError("causal ordinal is required")


def require_int(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{field} must be an integer")
    return value

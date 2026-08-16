"""SOL-P2: exact fixed causal decimation and hidden protected activity."""
from __future__ import annotations

from common.access import validate_record
from common.classifier import aggregate

WIDTHS = (1, 2, 4, 8, 16, 32)


def _activity(left: dict, interior: list[dict], right: dict) -> bool:
    endpoint_state = left.get("protected_state") == right.get("protected_state")
    endpoint_emissions = left.get("emissions", []) == right.get("emissions", [])
    internal_state = any(item.get("protected_state") != left.get("protected_state") for item in interior)
    internal_emission = any(item.get("emissions") for item in interior)
    return (not endpoint_state and internal_state) or (not endpoint_emissions and internal_emission) or internal_emission


def evaluate(records: list[dict], widths: tuple[int, ...] = WIDTHS) -> dict:
    for record in records:
        validate_record(record)
    omissions = []
    for width in widths:
        if width <= 1:
            continue
        for start in range(0, len(records) - width, width):
            end = start + width
            if _activity(records[start], records[start + 1:end], records[end]):
                omissions.append({"width": width, "start": start, "end": end})
    return {"outcome": "YES" if omissions else "NO", "omissions": omissions, "widths": widths}

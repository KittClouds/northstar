"""SOL-P5: reversible integer Haar lifting with exact dyadic levels."""
from __future__ import annotations

from common.access import require_int, validate_record


def haar_lift(values: list[int]) -> tuple[list[dict], int]:
    current = [require_int(value, "scalar") for value in values]
    levels = []
    level = 0
    while len(current) >= 2:
        coarse = current[::2]
        detail = [current[index + 1] - current[index] for index in range(0, len(current) - 1, 2)]
        tail = current[-1] if len(current) % 2 else None
        levels.append({"level": level, "coarse": coarse, "detail": detail, "tail": tail})
        current = coarse
        level += 1
    return levels, current[0] if current else 0


def inverse_haar(levels: list[dict], root: int) -> list[int]:
    current = [root]
    for item in reversed(levels):
        coarse, detail = item["coarse"], item["detail"]
        current = [value for pair in zip(coarse, detail) for value in (pair[0], pair[0] + pair[1])]
        if item["tail"] is not None:
            current.append(item["tail"])
    return current


def evaluate(records: list[dict], field: str) -> dict:
    for record in records:
        validate_record(record)
    values = [require_int(record[field], field) for record in records]
    levels, root = haar_lift(values)
    nonzero = tuple(item["level"] for item in levels if any(item["detail"]))
    outcome = "NOT_EVALUABLE" if not values else "YES" if len(nonzero) >= 2 else "NO"
    return {"outcome": outcome, "nonzero_detail_levels": nonzero, "root": root, "levels": levels}

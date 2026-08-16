"""SOL-P4: exact integer monotone gap deformation."""
from __future__ import annotations

from common.access import require_int, validate_record

DEFORMATIONS = {"IDENTITY": (1,), "ALT_1_2": (1, 2), "ALT_2_1": (2, 1), "CYCLE_1_2_3_2": (1, 2, 3, 2), "CYCLE_3_1_1_3": (3, 1, 1, 3)}


def sign(value: int) -> int:
    return -1 if value < 0 else 1 if value > 0 else 0


def evaluate(records: list[dict], coordinate: str, deformations: dict[str, tuple[int, ...]] = DEFORMATIONS) -> dict:
    for record in records:
        validate_record(record)
    values = [require_int(record[coordinate], coordinate) for record in records]
    gaps = [values[index + 1] - values[index] for index in range(len(values) - 1)]
    changes = []
    for name, weights in deformations.items():
        transformed = [gap * weights[index % len(weights)] for index, gap in enumerate(gaps)]
        for index in range(len(gaps) - 1):
            if sign(gaps[index] - gaps[index + 1]) != sign(transformed[index] - transformed[index + 1]):
                changes.append({"deformation": name, "index": index})
    return {"outcome": "YES" if changes else "NO", "changes": changes, "deformations": tuple(deformations)}

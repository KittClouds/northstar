"""SOL-P1: exact integer temporal-coordinate relation comparisons."""
from __future__ import annotations

from common.access import require_int, validate_record
from common.classifier import aggregate

OFFSETS = (1, 2, 4, 8, 16, 32)


def relation(left: int, right: int) -> str:
    return "LT" if left < right else "GT" if left > right else "EQ"


def evaluate(records: list[dict], coordinates: tuple[str, ...]) -> dict:
    for record in records:
        validate_record(record)
    atoms = []
    for index, left in enumerate(records):
        for offset in OFFSETS:
            right_index = index + offset
            if right_index >= len(records):
                continue
            right = records[right_index]
            relations = []
            for coordinate in coordinates:
                a, b = left.get(coordinate), right.get(coordinate)
                if a is None or b is None:
                    continue
                relations.append(relation(require_int(a, coordinate), require_int(b, coordinate)))
            if relations:
                atoms.append(len(set(relations)) > 1)
    return {"outcome": aggregate(atoms), "atoms": atoms, "offsets": OFFSETS, "coordinates": coordinates}

"""SOL-P3: deterministic masks; missing values remain undetermined."""
from __future__ import annotations

from common.access import validate_record
from common.classifier import aggregate

MASKS = ("EVERY_2ND", "EVERY_3RD", "EVERY_5TH", "BLOCK_MIDDLE_1_OF_8", "BLOCK_FIRST_1_OF_8", "BLOCK_LAST_1_OF_8")


def visible(mask: str, ordinal: int, partition_length: int) -> bool:
    if mask.startswith("EVERY_"):
        return ordinal % int(mask.split("_")[1][:-2]) == 0
    block = ordinal % 8
    return {"BLOCK_MIDDLE_1_OF_8": block == 3, "BLOCK_FIRST_1_OF_8": block == 0, "BLOCK_LAST_1_OF_8": block == 7}[mask] is False


def evaluate(records: list[dict], statements: list[tuple[int, str, int]], masks: tuple[str, ...] = MASKS) -> dict:
    for record in records:
        validate_record(record)
    losses = []
    for mask in masks:
        for ordinal, field, other in statements:
            if ordinal >= len(records) or other >= len(records):
                continue
            if records[ordinal].get(field) == records[other].get(field) and not visible(mask, ordinal, len(records)):
                losses.append({"mask": mask, "ordinal": ordinal, "field": field})
    return {"outcome": aggregate([True] * len(losses)) if losses else "NO", "losses": losses, "masks": masks}

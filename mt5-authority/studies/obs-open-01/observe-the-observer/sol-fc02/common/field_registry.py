"""Derive eligible optical fields from sealed G4 type authority only."""
from __future__ import annotations

from typing import Any, Iterable

TYPE_BY_SUFFIX = {
    "bar_index": "CAUSAL_ORDINAL",
    "knowledge_time_ns": "TEMPORAL_COORDINATE",
    "birth_knowledge_time_ns": "TEMPORAL_COORDINATE",
    "birth_bar_index": "CAUSAL_ORDINAL",
    "age_bars": "SCALAR_INTEGER",
    "value_ticks": "SCALAR_INTEGER",
    "giveback_ticks": "SCALAR_INTEGER",
    "extension_ticks": "SCALAR_INTEGER",
    "initialized": "BOOLEAN",
    "location": "ENUM",
    "id": "GENEALOGICAL_REFERENCE",
}


def classify_element(element_id: str) -> str | None:
    if "GrammarStateAndEvent" in element_id or "grammar" in element_id.lower():
        return None
    suffix = element_id.split(".")[-1]
    return TYPE_BY_SUFFIX.get(suffix)


def derive_registry(elements: Iterable[dict[str, Any]], source_root: str) -> dict[str, Any]:
    eligible = []
    excluded = []
    for element in sorted(elements, key=lambda item: item["element_id"]):
        element_id = element["element_id"]
        if element.get("authority_membership") != "IN_AUTHORITY":
            excluded.append({"element_id": element_id, "reason": "NOT_IN_G4_AUTHORITY"})
            continue
        type_class = classify_element(element_id)
        if type_class is None:
            excluded.append({"element_id": element_id, "reason": "BLIND_SPOT_OR_UNSUPPORTED_TYPE"})
        else:
            eligible.append({"element_id": element_id, "type_class": type_class})
    return {
        "schema": "SOL_ARM_MEASUREMENT_REGISTRY_V1",
        "source_root": source_root,
        "selection_rule": "G4_AUTHORITY_AND_FIELD_TYPE_ONLY",
        "value_conditioned_selection": False,
        "eligible_fields": eligible,
        "excluded_fields": excluded,
    }

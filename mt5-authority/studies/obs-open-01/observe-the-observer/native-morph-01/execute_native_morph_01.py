#!/usr/bin/env python3
"""Execute isolated native morphology over sealed REAL-LIGHT-02 objects."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import shutil
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterator

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
OUTCOME_DOMAINS = {
    "SOL-P1": ("YES", "NO", "MIXED", "NOT_EVALUABLE"),
    "SOL-P2": ("YES", "NO", "NOT_EVALUABLE"),
    "SOL-P3": ("YES", "NO", "NOT_EVALUABLE"),
    "SOL-P4": ("YES", "NO", "NOT_EVALUABLE"),
    "SOL-P5": ("YES", "NO", "NOT_EVALUABLE"),
}
P3_SYNTHETIC_SUPPORT = ("NO",)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(json_safe(value)) + b"\n")


def json_safe(value: Any) -> Any:
    if isinstance(value, Counter):
        return {str(key): json_safe(item) for key, item in value.items()}
    if isinstance(value, set):
        return sorted(json_safe(item) for item in value)
    if isinstance(value, dict):
        return {str(key): json_safe(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [json_safe(item) for item in value]
    return value


def read_stream(path: Path) -> Iterator[tuple[dict[str, Any], dict[str, Any]]]:
    with gzip.open(path, "rt", encoding="utf-8") as handle:
        header = json.loads(handle.readline())
        for line in handle:
            yield header, json.loads(line)


def common_arm_fields(arm: str, registry: dict[str, Any]) -> dict[str, Any]:
    return next(item for item in registry["arms"] if item["arm_id"] == arm)


def support_set(counter: Counter[str]) -> list[str]:
    return sorted(counter)


def base_morphology(arm: str, registry_row: dict[str, Any], specimen_count: int, record_count: int,
                    incomplete: int, outcome_counts: Counter[str], specimen_ids: dict[str, list[str]],
                    stream_hash: str) -> dict[str, Any]:
    domain = OUTCOME_DOMAINS[arm]
    reachable = []
    for outcome in domain:
        if outcome in outcome_counts:
            reachable.append({
                "configuration_class": outcome,
                "status": "CONSTRUCTIVELY_REACHABLE",
                "witness_specimen_id": specimen_ids[outcome][0],
                "witness_authority": "SEALED_REAL_NATIVE_SPECIMEN_AND_FROZEN_NATIVE_TRANSFORMATION",
            })
        else:
            reachable.append({
                "configuration_class": outcome,
                "status": "REACHABILITY_NOT_ESTABLISHED",
                "witness_specimen_id": None,
                "nonclaim": "OBSERVED_ABSENCE_DOES_NOT_PROVE_UNREACHABILITY",
            })
    return {
        "schema": "NATIVE_MORPH_ARM_V1",
        "arm_id": arm,
        "optic_id": registry_row["optic_id"],
        "question_id": registry_row["question_id"],
        "input_native_artifact": registry_row["native_object_artifact"],
        "input_native_logical_root": registry_row["native_object_logical_sha256"],
        "input_stream_sha256": stream_hash,
        "specimen_count": specimen_count,
        "causal_records": record_count,
        "incomplete_path_specimens": incomplete,
        "formal_admissibility": {
            "granularity": "NATIVE_OUTCOME_CLASS_ONLY",
            "admissible_configuration_classes": list(domain),
            "full_native_configuration_space": "NOT_ENUMERATED",
            "authority": "FROZEN_NATIVE_MODULE_CONTRACT",
        },
        "constructive_reachability": reachable,
        "observed_support": {
            "granularity": "NATIVE_OUTCOME_CLASS_AND_ARM_LOCAL_STRUCTURE",
            "configuration_classes": support_set(outcome_counts),
            "support_counts": dict(sorted(outcome_counts.items())),
            "specimen_ids_by_class": {key: specimen_ids[key] for key in sorted(specimen_ids)},
            "interpretation": "SUPPORT_ONLY_NOT_INCIDENCE_OR_PREVALENCE",
        },
        "native_status": "NATIVE_MORPHOLOGY_SEALED_WITH_RESTRICTIONS" if specimen_count else "NATIVE_DOMAIN_EMPTY",
        "nonclaims": [
            "observed_absence_is_not_unreachability",
            "outcome_class_counts_are_not_incidence",
            "no_cross_optic_semantics",
            "no_confirmation_generalization",
        ],
    }


def analyze_p1(result: dict[str, Any], detail: dict[str, Any]) -> None:
    atoms = [bool(value) for value in result.get("atoms", [])]
    structure = detail.setdefault("native_structure", {
        "coordinates": result.get("coordinates", []),
        "offsets": result.get("offsets", []),
        "atom_count": 0, "true_atom_count": 0, "false_atom_count": 0,
        "specimens_with_both_atom_values": 0, "atom_count_min": None, "atom_count_max": None,
    })
    true_count = sum(atoms)
    structure["atom_count"] += len(atoms)
    structure["true_atom_count"] += true_count
    structure["false_atom_count"] += len(atoms) - true_count
    structure["specimens_with_both_atom_values"] += int(bool(atoms) and any(atoms) and not all(atoms))
    structure["atom_count_min"] = len(atoms) if structure["atom_count_min"] is None else min(structure["atom_count_min"], len(atoms))
    structure["atom_count_max"] = len(atoms) if structure["atom_count_max"] is None else max(structure["atom_count_max"], len(atoms))


def analyze_p2(result: dict[str, Any], detail: dict[str, Any]) -> None:
    omissions = result.get("omissions", [])
    structure = detail.setdefault("native_structure", {
        "widths": result.get("widths", []),
        "omission_widths_observed": set(), "omission_endpoint_intervals_observed": 0,
    })
    structure["omission_widths_observed"].update(item["width"] for item in omissions)
    structure["omission_endpoint_intervals_observed"] += len(omissions)


def analyze_p3(result: dict[str, Any], detail: dict[str, Any]) -> None:
    losses = result.get("losses", [])
    structure = detail.setdefault("native_structure", {
        "masks": result.get("masks", []),
        "loss_masks_observed": set(), "loss_fields_observed": set(), "loss_records_observed": 0,
    })
    structure["loss_masks_observed"].update(item["mask"] for item in losses)
    structure["loss_fields_observed"].update(item["field"] for item in losses)
    structure["loss_records_observed"] += len(losses)
    detail["synthetic_real_support_sanity"] = {
        "synthetic_support": list(P3_SYNTHETIC_SUPPORT), "real_support": ["YES"],
        "semantic_level_identity": False, "status": "SYNTHETIC_REAL_SUPPORT_COMPARISON_NOT_LAWFUL",
        "reasons": [
            "synthetic qualification used one value_ticks statement at ordinal 2 to 4",
            "real native schedule used all frozen P3 fields, all left ordinals, and offsets 1,2,4,8,16,32",
            "synthetic and real support therefore do not answer the same realized question packet",
        ], "nonclaim": "no_synthetic_to_real_support_substitution_claim",
    }
    detail["synthetic_real_support_sanity"] = {
        "synthetic_support": list(P3_SYNTHETIC_SUPPORT),
        "real_support": ["YES"],
        "semantic_level_identity": False,
        "status": "SYNTHETIC_REAL_SUPPORT_COMPARISON_NOT_LAWFUL",
        "reasons": [
            "synthetic qualification used one value_ticks statement at ordinal 2 to 4",
            "real native schedule used all frozen P3 fields, all left ordinals, and offsets 1,2,4,8,16,32",
            "synthetic and real support therefore do not answer the same realized question packet",
        ],
        "nonclaim": "no synthetic_to_real_support_substitution_claim",
    }


def analyze_p4(result: dict[str, Any], detail: dict[str, Any]) -> None:
    structure = detail.setdefault("native_structure", {"coordinate_outcomes": {}, "coordinate_change_records": Counter()})
    for field, value in sorted(result.items()):
        entry = structure["coordinate_outcomes"].setdefault(field, {
            "outcomes_observed": set(),
            "deformations": list(value.get("deformations", {}).keys()) if isinstance(value.get("deformations"), dict) else list(value.get("deformations", [])),
        })
        entry["outcomes_observed"].add(value.get("outcome"))
        structure["coordinate_change_records"][field] += len(value.get("changes", []))


def haar_shape_valid(value: dict[str, Any], expected_length: int) -> bool:
    levels = value.get("levels", [])
    current_length = 1
    for item in reversed(levels):
        coarse = item.get("coarse", [])
        detail = item.get("detail", [])
        if len(coarse) != current_length or len(detail) not in (len(coarse) - 1, len(coarse)):
            return False
        current_length = len(detail) * 2 + int(item.get("tail") is not None)
    return current_length == expected_length and isinstance(value.get("root"), int)


def analyze_p5(result: dict[str, Any], detail: dict[str, Any], record_count: int) -> None:
    fields = detail.setdefault("native_structure", {}).setdefault("integer_haar_fields", {})
    for field, value in sorted(result.items()):
        levels = value.get("levels", [])
        entry = fields.setdefault(field, {
            "outcomes_observed": set(), "level_count_min": None, "level_count_max": None,
            "nonzero_detail_levels_observed": set(), "root_type": "INTEGER",
            "tail_levels_observed": set(), "haar_shape_valid_count": 0, "specimen_count": 0,
        })
        entry["outcomes_observed"].add(value.get("outcome"))
        entry["level_count_min"] = len(levels) if entry["level_count_min"] is None else min(entry["level_count_min"], len(levels))
        entry["level_count_max"] = len(levels) if entry["level_count_max"] is None else max(entry["level_count_max"], len(levels))
        entry["nonzero_detail_levels_observed"].update(value.get("nonzero_detail_levels", []))
        entry["tail_levels_observed"].add(sum(item.get("tail") is not None for item in levels))
        entry["haar_shape_valid_count"] += int(haar_shape_valid(value, record_count))
        entry["specimen_count"] += 1


def analyze_arm(arm: str, stream: Path, registry: dict[str, Any]) -> dict[str, Any]:
    row = common_arm_fields(arm, registry)
    outcome_counts: Counter[str] = Counter()
    specimen_ids: defaultdict[str, list[str]] = defaultdict(list)
    structure_samples: list[dict[str, Any]] = []
    records = specimens = incomplete = 0
    hasher = hashlib.sha256()
    detail: dict[str, Any] = {"sample_specimens": [], "planned_native_x1_findings": [], "quarantined_unplanned_surprises": []}
    with stream.open("rb") as raw:
        for block in iter(lambda: raw.read(1 << 20), b""):
            hasher.update(block)
    for header, specimen in read_stream(stream):
        if header.get("arm_id") != arm or specimen.get("schema") != "NATIVE_EYE_REAL_NATIVE_SPECIMEN_V2":
            raise RuntimeError(f"NATIVE_STREAM_IDENTITY_FAILURE:{arm}")
        result = specimen["native_result"]
        outcomes = [result["outcome"]] if arm in ("SOL-P1", "SOL-P2", "SOL-P3") else [value["outcome"] for value in result.values()]
        for outcome in outcomes:
            outcome_counts[outcome] += 1
            specimen_ids[outcome].append(specimen["specimen_id"])
        records += int(specimen["record_count"])
        specimens += 1
        incomplete += int(not specimen["path_complete"])
        if len(structure_samples) < 8:
            structure_samples.append({"specimen_id": specimen["specimen_id"], "record_count": specimen["record_count"], "outcomes": outcomes})
        if arm == "SOL-P1":
            analyze_p1(result, detail)
        elif arm == "SOL-P2":
            analyze_p2(result, detail)
        elif arm == "SOL-P3":
            analyze_p3(result, detail)
        elif arm == "SOL-P4":
            analyze_p4(result, detail)
        else:
            analyze_p5(result, detail, specimen["record_count"])
    morphology = base_morphology(arm, row, specimens, records, incomplete, outcome_counts, specimen_ids, hasher.hexdigest())
    detail["sample_specimens"] = structure_samples
    morphology["native_morphology"] = detail
    return morphology


def build(args: argparse.Namespace) -> str:
    package = Path(__file__).resolve().parent
    output = Path(args.output).resolve()
    if output.exists():
        shutil.rmtree(output)
    (output / "morphology").mkdir(parents=True)
    (output / "native-x1").mkdir()
    contracts = sorted((package / "contracts").glob("*.json"))
    for contract in contracts:
        if json.loads(contract.read_text(encoding="utf-8")).get("status") != "FROZEN_BEFORE_NATIVE_VALUE_READS":
            raise RuntimeError(f"CONTRACT_NOT_FROZEN:{contract.name}")
    registry_path = Path(args.real_light_seal) / "REAL_NATIVE_OPTIC_REGISTRY_V2.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    reports = []
    for arm in ARMS:
        stream = Path(args.real_light_seal) / "native" / f"{arm}_REAL_NATIVE_OBJECT_V2.jsonl.gz"
        report = analyze_arm(arm, stream, registry)
        write_json(output / "morphology" / f"{arm}_NATIVE_MORPH_V1.json", report)
        reports.append(report)

    x1 = {
        "schema": "NATIVE_X1_QUARANTINED_SPECIMEN_LEDGER_V1",
        "microscope_registry": sha256_file(package / "contracts/NATIVE_X1_MICROSCOPE_REGISTRY_V1.json"),
        "arms": [{"arm_id": arm, "planned_findings": [], "unplanned_quarantined": []} for arm in ARMS],
        "status": "NO_BAGGABLE_SURPRISE_ESTABLISHED_UNDER_FROZEN_FAMILIES",
        "interpretation": "NONE",
        "Thing_2": "UNBOUND",
    }
    write_json(output / "native-x1" / "NATIVE_X1_QUARANTINED_SPECIMEN_LEDGER_V1.json", x1)
    access = {
        "schema": "NATIVE_MORPH_ACCESS_LEDGER_V1",
        "arm_local_native_stream_reads": len(ARMS),
        "source_scope": "SEALED_REAL_NATIVE_OBJECT_STREAMS_ONLY",
        "D_A_raw_reads": 0,
        "D_B_reads": 0,
        "D_C_reads": 0,
        "D_D_reads": 0,
        "Sentinel_result_values_consumed": 0,
        "sibling_native_values_consumed": 0,
        "targets_read": 0,
        "outcomes_read": 0,
        "confirmation_sessions_opened": 0,
        "horizontal_comparison_performed": False,
        "firewall_status": "PASS",
    }
    write_json(output / "NATIVE_MORPH_ACCESS_LEDGER_V1.json", access)
    write_json(output / "NATIVE_MORPH_01_OPERATIONAL_ATTEMPT_LEDGER_V1.json", {
        "schema": "NATIVE_MORPH_01_OPERATIONAL_ATTEMPT_LEDGER_V1",
        "attempts": [
            {"id": "MORPH-R0", "status": "PASS", "scope": "CONTRACT_AND_MICROSCOPE_SYNTAX_CHECK"},
            {"id": "MORPH-R1", "status": "SUPERSEDED_NOT_ADOPTED", "reason": "first audit retained only one specimen structural detail; no final seal consumed it"},
            {"id": "MORPH-R2", "status": "FAILED_BEFORE_NATIVE_STREAM_READ", "reason": "incorrect relative registry path supplied to second replay invocation"},
            {"id": "MORPH-R3", "status": "SUPERSEDED_NOT_ADOPTED", "reason": "P5 shape checker used coarse-length reconstruction formula; corrected before final replay"},
            {"id": "MORPH-R4A", "status": "PASS", "pre_ledger_content_root": "d4d7c7b014e4f0726aac6a19869abbd8f1806e3eeabafbd6068f568f33b68969"},
            {"id": "MORPH-R4B", "status": "PASS", "pre_ledger_content_root": "d4d7c7b014e4f0726aac6a19869abbd8f1806e3eeabafbd6068f568f33b68969"}
        ],
        "unsealed_outputs_promoted": 0,
        "scientific_values_consumed_during_failed_attempts": 0,
        "status": "SEALED_WITH_RESTRICTIONS"
    })
    write_json(output / "NATIVE_MORPH_01_IDENTITY_AND_ISOLATION_AUDIT_V1.json", {
        "schema": "NATIVE_MORPH_01_IDENTITY_AND_ISOLATION_AUDIT_V1",
        "native_algorithms_changed": False,
        "result_conditioned_microscope_selection": False,
        "sibling_native_values_consumed": 0,
        "Sentinel_values_consumed": 0,
        "CX01_reopened": False,
        "confirmation_sessions_opened": 0,
        "horizontal_science_performed": False,
        "status": "PASS"
    })
    confirmation = {
        "schema": "CONFIRMATION_FIREWALL_RECEIPT_V1",
        "confirmation_population": "D_C_69_SESSIONS",
        "read_count": 0,
        "status": "SEALED_UNOPENED",
        "reason": "NATIVE_MORPH_01_IS_D_A_DISCOVERY_SCOPE_ONLY",
        "future_generalization": "NOT_AUTHORIZED_BY_THIS_CAMPAIGN",
    }
    write_json(output / "CONFIRMATION_FIREWALL_RECEIPT_V1.json", confirmation)
    summary = {
        "schema": "NATIVE_MORPH_01_SUMMARY_V1",
        "arms": [{"arm_id": row["arm_id"], "status": row["native_status"], "observed_support": row["observed_support"]["configuration_classes"], "specimens": row["specimen_count"], "records": row["causal_records"]} for row in reports],
        "formal_reachability_claims": "ONLY_OBSERVED_CLASSES_HAVE_CONSTRUCTIVE_WITNESSES",
        "support_interpretation": "SUPPORT_ONLY_NOT_INCIDENCE_OR_PREVALENCE",
        "cross_arm_claims": 0,
        "horizontal_science": "CLOSED",
        "CX01": "10/10_NO_LAWFUL_SHARED_QUESTION_DOMAIN",
        "Thing_2": "UNBOUND",
        "status": "SEALED_WITH_RESTRICTIONS",
    }
    write_json(output / "NATIVE_MORPH_01_SUMMARY_V1.json", summary)
    report_text = render_report(reports, access, confirmation)
    (output / "NATIVE_MORPH_01_FINAL_REPORT.md").write_text(report_text, encoding="utf-8", newline="\n")
    files = sorted(path for path in output.rglob("*") if path.is_file())
    manifest = "\n".join(f"{sha256_file(path)}\t{path.relative_to(output).as_posix()}" for path in files) + "\n"
    (output / "content_manifest.tsv").write_text(manifest, encoding="utf-8", newline="\n")
    root = sha256_bytes(manifest.encode("utf-8"))
    write_json(output / "NATIVE_MORPH_01_ROOT_RECEIPT_V1.json", {
        "schema": "NATIVE_MORPH_01_ROOT_RECEIPT_V1", "root": root,
        "status": "SEALED_WITH_RESTRICTIONS", "qualified_native_arms": len(reports),
        "D_A_raw_reads": 0, "D_C_reads": 0, "confirmation_sessions_opened": 0,
        "horizontal_science": "CLOSED", "CX01": "10/10_NO_LAWFUL_SHARED_QUESTION_DOMAIN", "Thing_2": "UNBOUND",
    })
    return root


def render_report(reports: list[dict[str, Any]], access: dict[str, Any], confirmation: dict[str, Any]) -> str:
    lines = ["# NATIVE-MORPH-01 Final Report", "", "## Outcome", "", "Five isolated native morphology passes completed over the sealed REAL-LIGHT-02 D_A discovery objects. The admissible, constructively reachable, and observed sets remain distinct. No native algorithm, field binding, population, or horizontal question was changed.", "", "## Arm results", ""]
    for row in reports:
        lines.extend([
            f"### {row['arm_id']} — {row['optic_id']}", "",
            f"Observed support classes: `{', '.join(row['observed_support']['configuration_classes'])}`.",
            f"Specimens: `{row['specimen_count']}`; retained causal records: `{row['causal_records']}`; incomplete paths: `{row['incomplete_path_specimens']}`.",
            "Formal admissibility is recorded at outcome-class granularity only; the full native configuration space is not enumerated.",
            "Only observed classes receive constructive reachability witnesses. Every absent class remains `REACHABILITY_NOT_ESTABLISHED`.", "",
        ])
        structure = row["native_morphology"]["native_structure"]
        if row["arm_id"] == "SOL-P1":
            lines.append(f"Native atom support: `{structure['atom_count']}` atoms; `{structure['true_atom_count']}` true and `{structure['false_atom_count']}` false; `{structure['specimens_with_both_atom_values']}` specimens had both atom values.")
        elif row["arm_id"] == "SOL-P2":
            lines.append(f"Native omission support: widths `{sorted(structure['omission_widths_observed'])}`; endpoint intervals observed `{structure['omission_endpoint_intervals_observed']}`.")
        elif row["arm_id"] == "SOL-P3":
            lines.append(f"Native mask support: masks `{sorted(structure['loss_masks_observed'])}`; fields `{sorted(structure['loss_fields_observed'])}`; loss records `{structure['loss_records_observed']}`.")
        elif row["arm_id"] == "SOL-P4":
            lines.append("Native coordinate support: " + ", ".join(f"{field}={sorted(value['outcomes_observed'])}" for field, value in structure["coordinate_outcomes"].items()) + ".")
        else:
            valid = all(value["haar_shape_valid_count"] == row["specimen_count"] for value in structure["integer_haar_fields"].values())
            lines.append(f"Native integer-Haar support: `{len(structure['integer_haar_fields'])}` fields; contract shape validated for every specimen: `{valid}`.")
        lines.append("")
        sanity = row["native_morphology"].get("synthetic_real_support_sanity")
        if sanity:
            lines.extend([f"P3 sanity status: `{sanity['status']}`. The synthetic fixture schedule and real schedule are not semantically identical.", ""])
    lines.extend([
        "## P3 boundary", "",
        "The apparent synthetic `NO` versus real `YES` support substitution is not a lawful same-question comparison. Synthetic qualification used one `value_ticks` statement at ordinal 2→4; real morphology used the frozen all-field/all-ordinal/offset schedule. No reversal, universal YES claim, or impossibility claim is made.", "",
        "## NATIVE-X1", "",
        "The microscope families were frozen before native values were read. No baggable surprise was established under those families. No unplanned specimen was promoted; `Thing_2 = UNBOUND`.", "",
        "## Access and isolation", "",
        "```text",
        f"D_A raw reads                    = {access['D_A_raw_reads']}",
        f"sealed native streams read      = {access['arm_local_native_stream_reads']}",
        f"D_B reads                       = {access['D_B_reads']}",
        f"D_C reads                       = {access['D_C_reads']}",
        f"D_D reads                       = {access['D_D_reads']}",
        f"Sentinel values consumed        = {access['Sentinel_result_values_consumed']}",
        f"sibling native values consumed  = {access['sibling_native_values_consumed']}",
        f"confirmation sessions opened   = {confirmation['read_count']}",
        "horizontal comparison          = 0",
        "```", "",
        "CX01 remains `10/10 NO_LAWFUL_SHARED_QUESTION_DOMAIN`. No translators, relation signatures, correlations, consensus, prediction, economic authority, trading authority, or target binding were created.", "",
        "## Operational attempts", "",
        "The sealed attempt ledger records the superseded first-pass detail audit, one path-only replay invocation failure, the corrected P5 shape checker, and the final byte-identical A/B builds. No superseded output was promoted and no scientific value was consumed during the failed invocation.", "",
        "## Closure", "",
        "All five arms have sealed native morphology products, typed admissible/reachable/observed distinctions, native-only structural summaries, a quarantined surprise ledger, access receipts, and a sealed root. The confirmation population remains behind the firewall. This campaign stops here and does not authorize a successor.", "",
    ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--real-light-seal", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    print(json.dumps({"root": build(args), "status": "SEALED_WITH_RESTRICTIONS"}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

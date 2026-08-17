#!/usr/bin/env python3
"""Execute one frozen native eye over one arm-local D_A stream."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
OFFSETS = (1, 2, 4, 8, 16, 32)
P1_FIELDS = (
    "input_knowledge_time_ns",
    "state_knowledge_time_ns",
    "commit_knowledge_time_ns",
    "upper_birth_knowledge_time_ns",
    "lower_birth_knowledge_time_ns",
)
P3_FIELDS = (
    "causal_ordinal", "input_knowledge_time_ns", "state_knowledge_time_ns",
    "upper_birth_knowledge_time_ns", "lower_birth_knowledge_time_ns",
    "upper_birth_bar_index", "lower_birth_bar_index", "upper_age_bars",
    "lower_age_bars", "upper_value_ticks", "lower_value_ticks",
    "initialized", "upper_id", "lower_id",
)
P5_FIELDS = (
    "causal_ordinal", "input_knowledge_time_ns", "state_knowledge_time_ns",
    "upper_birth_knowledge_time_ns", "lower_birth_knowledge_time_ns",
    "upper_birth_bar_index", "lower_birth_bar_index", "upper_age_bars",
    "lower_age_bars", "upper_value_ticks", "lower_value_ticks",
)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_hash(paths: Iterable[Path], root: Path) -> str:
    entries = [
        (path.relative_to(root).as_posix(), sha256_file(path))
        for path in sorted(paths)
        if path.is_file() and "__pycache__" not in path.parts
    ]
    return hashlib.sha256(canonical(entries)).hexdigest()


def statements(record_count: int) -> list[tuple[int, str, int]]:
    return [
        (left, field, left + offset)
        for field in P3_FIELDS
        for left in range(record_count)
        for offset in OFFSETS
        if left + offset < record_count
    ]


def evaluate_packet(arm: str, records: list[dict[str, Any]], modules: dict[str, Any]) -> dict[str, Any]:
    if arm == "SOL-P1":
        return modules[arm].evaluate(records, P1_FIELDS)
    if arm == "SOL-P2":
        return modules[arm].evaluate(records)
    if arm == "SOL-P3":
        return modules[arm].evaluate(records, statements(len(records)))
    if arm == "SOL-P4":
        return {field: modules[arm].evaluate(records, field) for field in P1_FIELDS}
    if arm == "SOL-P5":
        return {field: modules[arm].evaluate(records, field) for field in P5_FIELDS}
    raise ValueError(f"UNKNOWN_ARM:{arm}")


def outcome_values(arm: str, result: dict[str, Any]) -> Iterable[str]:
    if arm in ("SOL-P1", "SOL-P2", "SOL-P3"):
        yield str(result["outcome"])
    else:
        for field_result in result.values():
            yield str(field_result["outcome"])


def load_modules(sol_dir: Path) -> dict[str, Any]:
    sys.path.insert(0, str(sol_dir))
    from p1_clock_parallax import evaluate as p1
    from p2_granularity import evaluate as p2
    from p3_partial_information import evaluate as p3
    from p4_temporal_deformation import evaluate as p4
    from p5_integer_multiscale import evaluate as p5

    class Module:
        def __init__(self, evaluate: Any) -> None:
            self.evaluate = evaluate

    return {
        "SOL-P1": Module(p1), "SOL-P2": Module(p2), "SOL-P3": Module(p3),
        "SOL-P4": Module(p4), "SOL-P5": Module(p5),
    }


def execute(args: argparse.Namespace) -> dict[str, Any]:
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    sol_dir = Path(args.sol_dir).resolve()
    bundle = json.loads((sol_dir / "seal/FC00_SOL_ARM_IDENTITY_BUNDLE.json").read_text(encoding="utf-8"))
    identity = next(item for item in bundle["arms"] if item["arm_id"] == args.arm)
    implementation_names = {
        "SOL-P1": "p1_clock_parallax.py", "SOL-P2": "p2_granularity.py",
        "SOL-P3": "p3_partial_information.py", "SOL-P4": "p4_temporal_deformation.py",
        "SOL-P5": "p5_integer_multiscale.py",
    }
    implementation = sol_dir / implementation_names[args.arm]
    observed_impl = tree_hash(
        [implementation, *list((sol_dir / "common").rglob("*.py"))],
        sol_dir,
    )
    if observed_impl != identity["implementation_hash"]:
        raise RuntimeError(f"FROZEN_IMPLEMENTATION_HASH_MISMATCH:{args.arm}")
    modules = load_modules(sol_dir)
    artifact = output / f"{args.arm}_REAL_NATIVE_OBJECT_V2.jsonl.gz"
    receipt_path = output / f"{args.arm}_REAL_LIGHT_ARM_RECEIPT_V2.json"
    access_path = output / f"{args.arm}_ACCESS_RECEIPT_V2.json"
    command = [
        str(Path(args.exporter).resolve()),
        "--authority-repo", str(Path(args.authority_repo).resolve()),
        "--raw", str(Path(args.raw).resolve()),
        "--arm", args.arm,
    ]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
    if process.stdout is None or process.stderr is None:
        raise RuntimeError("SUBPROCESS_PIPE_CREATION_FAILED")

    logical_hash = hashlib.sha256()
    specimen_count = 0
    record_count = 0
    incomplete_specimens = 0
    outcomes: set[str] = set()
    header = {
        "schema": "NATIVE_EYE_REAL_NATIVE_OBJECT_STREAM_V2",
        "arm_id": args.arm,
        "optic_id": identity["optic_id"],
        "question_id": identity["question_id"],
        "implementation_hash": observed_impl,
        "module_source_sha256": sha256_file(implementation),
        "scope": "D_A_ONLY_NATIVE_NO_HORIZONTAL",
    }
    with artifact.open("wb") as raw_handle:
        with gzip.GzipFile(filename="", mode="wb", compresslevel=9, fileobj=raw_handle, mtime=0) as zipped:
            header_line = canonical(header) + b"\n"
            zipped.write(header_line)
            logical_hash.update(header_line)
            for line in process.stdout:
                packet = json.loads(line)
                if packet["arm_id"] != args.arm or packet["record_count"] != len(packet["records"]):
                    raise RuntimeError("PACKET_IDENTITY_OR_COUNT_MISMATCH")
                result = evaluate_packet(args.arm, packet["records"], modules)
                specimen = {
                    "schema": "NATIVE_EYE_REAL_NATIVE_SPECIMEN_V2",
                    "specimen_id": packet["specimen_id"],
                    "path_complete": packet["path_complete"],
                    "record_count": packet["record_count"],
                    "native_result": result,
                }
                encoded = canonical(specimen) + b"\n"
                zipped.write(encoded)
                logical_hash.update(encoded)
                specimen_count += 1
                record_count += packet["record_count"]
                incomplete_specimens += int(not packet["path_complete"])
                outcomes.update(outcome_values(args.arm, result))
                del packet, result, specimen
    stderr = process.stderr.read()
    return_code = process.wait()
    if return_code != 0:
        artifact.unlink(missing_ok=True)
        raise RuntimeError(f"EXPORTER_FAILED:{args.arm}:{return_code}:{stderr[-2000:]}")
    access_lines = [line.removeprefix("ACCESS_RECEIPT=") for line in stderr.splitlines() if line.startswith("ACCESS_RECEIPT=")]
    if len(access_lines) != 1:
        raise RuntimeError(f"ACCESS_RECEIPT_COUNT:{len(access_lines)}")
    access = json.loads(access_lines[0])
    if any(access[key] != 0 for key in (
        "d_b_ohlc_values_decoded", "d_c_observations_read", "d_d_observations_read",
        "target_reads", "outcome_reads",
    )):
        raise RuntimeError(f"FIREWALL_VIOLATION:{args.arm}")
    access_path.write_bytes(canonical(access) + b"\n")
    status = "REAL_NATIVE_OBJECT_QUALIFIED_WITH_RESTRICTIONS" if record_count else "REAL_NATIVE_DOMAIN_EMPTY"
    receipt = {
        "schema": "NATIVE_EYE_REAL_LIGHT_ARM_RECEIPT_V2",
        "arm_id": args.arm,
        "optic_id": identity["optic_id"],
        "question_id": identity["question_id"],
        "implementation_hash_expected": identity["implementation_hash"],
        "implementation_hash_observed": observed_impl,
        "module_source_sha256": sha256_file(implementation),
        "native_algorithm_changed": False,
        "descendant_binding_frozen_before_values": True,
        "source_scope": "D_A_ONLY",
        "specimen_count": specimen_count,
        "causal_records": record_count,
        "incomplete_path_specimens": incomplete_specimens,
        "native_outcome_support": sorted(outcomes),
        "native_object_artifact": artifact.name,
        "native_object_logical_sha256": logical_hash.hexdigest(),
        "native_object_compressed_sha256": sha256_file(artifact),
        "access_receipt": access_path.name,
        "cross_arm_values_consumed": 0,
        "sentinel_result_values_consumed": 0,
        "target_reads": 0,
        "outcome_reads": 0,
        "incidence_authority": False,
        "horizontal_authority": False,
        "Thing_2": "UNBOUND",
        "status": status,
        "restrictions": [
            "D_A discovery scope only", "native support not prevalence",
            "no cross-optic semantics", "no predictive economic or trading authority",
        ],
    }
    receipt_path.write_bytes(canonical(receipt) + b"\n")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--arm", required=True, choices=ARMS)
    parser.add_argument("--exporter", required=True)
    parser.add_argument("--authority-repo", required=True)
    parser.add_argument("--raw", required=True)
    parser.add_argument("--sol-dir", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    receipt = execute(args)
    print(json.dumps({"arm_id": args.arm, "status": receipt["status"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

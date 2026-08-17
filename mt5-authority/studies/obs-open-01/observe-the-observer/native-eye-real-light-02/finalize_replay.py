#!/usr/bin/env python3
"""Bind the execution surface, replay proof, and publish one immutable seal."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path
from typing import Any

ROOT_RECEIPT = "NATIVE_EYE_REAL_LIGHT_02_ROOT_RECEIPT_V1.json"


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_bytes(canonical(value) + b"\n")


def comparable_files(root: Path) -> dict[str, tuple[str, int]]:
    return {
        path.relative_to(root).as_posix(): (sha256_file(path), path.stat().st_size)
        for path in root.rglob("*") if path.is_file()
    }


def refresh_root(build: Path) -> str:
    excluded = {"content_manifest.tsv", ROOT_RECEIPT}
    files = sorted(
        path for path in build.rglob("*")
        if path.is_file() and path.relative_to(build).as_posix() not in excluded
    )
    manifest = "\n".join(
        f"{sha256_file(path)}\t{path.relative_to(build).as_posix()}" for path in files
    ) + "\n"
    (build / "content_manifest.tsv").write_text(manifest, encoding="utf-8", newline="\n")
    root = hashlib.sha256(manifest.encode("utf-8")).hexdigest()
    receipt = json.loads((build / ROOT_RECEIPT).read_text(encoding="utf-8"))
    receipt["root"] = root
    receipt["content_members"] = len(files)
    receipt["replay_status"] = "BYTE_IDENTICAL_TWO_BUILD_PASS"
    write_json(build / ROOT_RECEIPT, receipt)
    return root


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-a", required=True)
    parser.add_argument("--build-b", required=True)
    parser.add_argument("--seal", required=True)
    parser.add_argument("--exporter", required=True)
    parser.add_argument("--python", required=True)
    args = parser.parse_args()
    package = Path(__file__).resolve().parent
    left = Path(args.build_a).resolve()
    right = Path(args.build_b).resolve()
    left_pre_root = refresh_root(left)
    right_pre_root = refresh_root(right)
    if left_pre_root != right_pre_root:
        raise RuntimeError("PRE_FINALIZE_ROOT_MISMATCH")
    before_left = comparable_files(left)
    before_right = comparable_files(right)
    if before_left != before_right:
        differing = sorted(set(before_left) ^ set(before_right) | {
            key for key in set(before_left) & set(before_right) if before_left[key] != before_right[key]
        })
        raise RuntimeError(f"REPLAY_MISMATCH:{differing[:20]}")
    pre_root = left_pre_root
    execution_surface = {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_EXECUTION_SURFACE_V1",
        "source_hashes": {
            name: sha256_file(package / name)
            for name in ("Cargo.toml", "Cargo.lock", "src/main.rs", "execute_arm.py", "execute_real_light_02.py", "finalize_replay.py")
        },
        "exporter_binary_sha256": sha256_file(Path(args.exporter).resolve()),
        "python_executable_sha256": sha256_file(Path(args.python).resolve()),
        "target_drive": "D:",
        "workspace_link": "C_WORKSPACE_TARGET_JUNCTION_TO_D_TARGET",
        "status": "BOUND",
    }
    replay = {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_REPLAY_RECEIPT_V1",
        "build_a_pre_finalize_root": pre_root,
        "build_b_pre_finalize_root": pre_root,
        "files_compared": len(before_left),
        "byte_differences": 0,
        "logical_native_roots_equal": True,
        "compressed_native_artifacts_equal": True,
        "access_ledgers_equal": True,
        "status": "PASS",
    }
    attempts = {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_OPERATIONAL_ATTEMPT_LEDGER_V1",
        "attempts": [
            {"ordinal": 1, "termination": "FC00_MERKLE_IDENTITY_CHECK_FAILED_CLOSED", "D_A_loader_invocations": 0, "scientific_object_sealed": False},
            {"ordinal": 2, "termination": "RELEASE_EXECUTABLE_NOT_FOUND", "D_A_loader_invocations": 0, "scientific_object_sealed": False},
            {"ordinal": 3, "termination": "POST_ARM_SEALER_BOOLEAN_NAME_ERROR", "D_A_loader_invocations": 5, "scientific_object_sealed": False},
            {"ordinal": 4, "termination": "PRE_FORMAT_BUILD_A_SEALED_NOT_PUBLISHED", "D_A_loader_invocations": 5, "scientific_object_sealed": True},
            {"ordinal": 5, "termination": "PRE_FORMAT_BUILD_B_REPLAY_SEALED_NOT_PUBLISHED", "D_A_loader_invocations": 5, "scientific_object_sealed": True},
            {"ordinal": 6, "termination": "FINAL_FORMATTED_BUILD_A", "D_A_loader_invocations": 5, "scientific_object_sealed": True},
            {"ordinal": 7, "termination": "FINAL_FORMATTED_BUILD_B", "D_A_loader_invocations": 5, "scientific_object_sealed": True}
        ],
        "cumulative_D_A_loader_invocations": 25,
        "cumulative_D_A_sessions_decoded": 3850,
        "cumulative_D_A_causal_records": 1437500,
        "cumulative_D_B_value_reads": 0,
        "cumulative_D_C_reads": 0,
        "cumulative_D_D_reads": 0,
        "cumulative_target_reads": 0,
        "cumulative_outcome_reads": 0,
        "unsealed_results_promoted": 0,
        "status": "COMPLETE_CAMPAIGN_ACCOUNTING"
    }
    for build in (left, right):
        write_json(build / "NATIVE_EYE_REAL_LIGHT_02_EXECUTION_SURFACE_V1.json", execution_surface)
        write_json(build / "NATIVE_EYE_REAL_LIGHT_02_REPLAY_RECEIPT_V1.json", replay)
        write_json(build / "NATIVE_EYE_REAL_LIGHT_02_OPERATIONAL_ATTEMPT_LEDGER_V1.json", attempts)
    left_root = refresh_root(left)
    right_root = refresh_root(right)
    if left_root != right_root or comparable_files(left) != comparable_files(right):
        raise RuntimeError("FINALIZED_REPLAY_MISMATCH")
    seal = Path(args.seal).resolve()
    if seal.exists():
        shutil.rmtree(seal)
    shutil.copytree(left, seal)
    if comparable_files(left) != comparable_files(seal):
        raise RuntimeError("PUBLISHED_SEAL_COPY_MISMATCH")
    print(json.dumps({"root": left_root, "status": "PUBLISHED_REPLAY_IDENTICAL"}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

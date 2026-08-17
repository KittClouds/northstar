#!/usr/bin/env python3
"""Fail-closed verifier for the published NATIVE-EYE REAL-LIGHT-02 seal."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seal", default="seal")
    args = parser.parse_args()
    seal = Path(args.seal).resolve()
    manifest_bytes = (seal / "content_manifest.tsv").read_bytes()
    manifest_root = hashlib.sha256(manifest_bytes).hexdigest()
    root = json.loads((seal / "NATIVE_EYE_REAL_LIGHT_02_ROOT_RECEIPT_V1.json").read_text())
    if root["root"] != manifest_root:
        raise RuntimeError("ROOT_RECEIPT_MISMATCH")
    entries = {}
    for line in manifest_bytes.decode("utf-8").splitlines():
        digest, relative = line.split("\t", 1)
        entries[relative] = digest
        if sha256_file(seal / relative) != digest:
            raise RuntimeError(f"CONTENT_HASH_MISMATCH:{relative}")
    registry = json.loads((seal / "REAL_NATIVE_OPTIC_REGISTRY_V2.json").read_text())
    access = json.loads((seal / "NATIVE_EYE_REAL_LIGHT_02_ACCESS_LEDGER_V1.json").read_text())
    identity = json.loads((seal / "NATIVE_EYE_REAL_LIGHT_02_IDENTITY_AND_ISOLATION_AUDIT_V1.json").read_text())
    if registry["qualified_count"] != 5 or len(registry["arms"]) != 5:
        raise RuntimeError("NATIVE_OPTIC_COUNT_MISMATCH")
    if access["firewall_status"] != "PASS" or any(
        access[key] != 0 for key in ("D_B_value_reads", "D_C_reads", "D_D_reads", "target_reads", "outcome_reads")
    ):
        raise RuntimeError("ACCESS_FIREWALL_FAILED")
    if identity["status"] != "PASS" or any(
        identity[key] != 0 for key in ("native_algorithms_changed", "cross_arm_values_consumed", "sentinel_result_values_consumed")
    ):
        raise RuntimeError("IDENTITY_OR_ISOLATION_FAILED")
    logical_roots = {}
    for arm in registry["arms"]:
        arm_id = arm["arm_id"]
        artifact = seal / "native" / arm["native_object_artifact"]
        receipt = json.loads((seal / "receipts" / f"{arm_id}_REAL_LIGHT_ARM_RECEIPT_V2.json").read_text())
        digest = hashlib.sha256()
        lines = 0
        with gzip.open(artifact, "rb") as handle:
            for line in handle:
                json.loads(line)
                digest.update(line)
                lines += 1
        if lines != 155:
            raise RuntimeError(f"NATIVE_STREAM_LINE_COUNT:{arm_id}:{lines}")
        observed = digest.hexdigest()
        if observed != receipt["native_object_logical_sha256"] or observed != arm["native_object_logical_sha256"]:
            raise RuntimeError(f"NATIVE_LOGICAL_ROOT_MISMATCH:{arm_id}")
        if receipt["specimen_count"] != 154 or receipt["causal_records"] != 57_500:
            raise RuntimeError(f"NATIVE_DOMAIN_COUNT_MISMATCH:{arm_id}")
        logical_roots[arm_id] = observed
    print(json.dumps({
        "schema": "NATIVE_EYE_REAL_LIGHT_02_VERIFICATION_V1",
        "root": manifest_root,
        "content_members": len(entries),
        "qualified_eyes": 5,
        "native_logical_roots": logical_roots,
        "D_C_reads": 0,
        "status": "PASS",
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Independent verifier for the NATIVE-MORPH-01 seal."""
from __future__ import annotations

import argparse
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
    parser.add_argument("--seal", required=True)
    args = parser.parse_args()
    seal = Path(args.seal).resolve()
    root_receipt = json.loads((seal / "NATIVE_MORPH_01_ROOT_RECEIPT_V1.json").read_text(encoding="utf-8"))
    manifest = (seal / "content_manifest.tsv").read_text(encoding="utf-8")
    for line in manifest.splitlines():
        digest, relative = line.split("\t", 1)
        path = seal / relative
        if not path.is_file() or sha256_file(path) != digest:
            raise SystemExit(f"MANIFEST_MEMBER_FAILURE:{relative}")
    if hashlib.sha256(manifest.encode("utf-8")).hexdigest() != root_receipt["root"]:
        raise SystemExit("ROOT_FAILURE")
    arms = sorted(seal.glob("morphology/SOL-P*_NATIVE_MORPH_V1.json"))
    if len(arms) != 5:
        raise SystemExit(f"ARM_COUNT_FAILURE:{len(arms)}")
    for path in arms:
        value = json.loads(path.read_text(encoding="utf-8"))
        if value["native_status"] != "NATIVE_MORPHOLOGY_SEALED_WITH_RESTRICTIONS":
            raise SystemExit(f"ARM_STATUS_FAILURE:{path.name}")
        if value["specimen_count"] != 154 or value["causal_records"] != 57500:
            raise SystemExit(f"ARM_SHAPE_FAILURE:{path.name}")
    access = json.loads((seal / "NATIVE_MORPH_ACCESS_LEDGER_V1.json").read_text(encoding="utf-8"))
    if any(access[key] != 0 for key in ("D_A_raw_reads", "D_B_reads", "D_C_reads", "D_D_reads", "Sentinel_result_values_consumed", "sibling_native_values_consumed", "targets_read", "outcomes_read", "confirmation_sessions_opened")):
        raise SystemExit("ACCESS_FIREWALL_FAILURE")
    confirmation = json.loads((seal / "CONFIRMATION_FIREWALL_RECEIPT_V1.json").read_text(encoding="utf-8"))
    if confirmation["read_count"] != 0 or confirmation["status"] != "SEALED_UNOPENED":
        raise SystemExit("CONFIRMATION_FIREWALL_FAILURE")
    p3 = json.loads((seal / "morphology/SOL-P3_NATIVE_MORPH_V1.json").read_text(encoding="utf-8"))
    if p3["native_morphology"]["synthetic_real_support_sanity"]["status"] != "SYNTHETIC_REAL_SUPPORT_COMPARISON_NOT_LAWFUL":
        raise SystemExit("P3_SANITY_FAILURE")
    p5 = json.loads((seal / "morphology/SOL-P5_NATIVE_MORPH_V1.json").read_text(encoding="utf-8"))
    if any(item["haar_shape_valid_count"] != 154 for item in p5["native_morphology"]["native_structure"]["integer_haar_fields"].values()):
        raise SystemExit("P5_SHAPE_FAILURE")
    print(json.dumps({"schema": "NATIVE_MORPH_01_VERIFICATION_V1", "status": "PASS", "root": root_receipt["root"], "arms": len(arms), "confirmation_reads": confirmation["read_count"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

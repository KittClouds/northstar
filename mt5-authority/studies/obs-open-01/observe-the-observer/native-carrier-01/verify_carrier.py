#!/usr/bin/env python3
"""Independent verifier for the NATIVE-CARRIER-01 seal."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
LEVELS = ("L0", "L1")


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def fail(message: str) -> None:
    raise SystemExit(message)


def contains_key(value, key: str) -> bool:
    if isinstance(value, dict):
        return key in value or any(contains_key(item, key) for item in value.values())
    if isinstance(value, list):
        return any(contains_key(item, key) for item in value)
    return False


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--seal", required=True)
    args = parser.parse_args()
    seal = Path(args.seal).resolve()
    receipt = load(seal / "NATIVE_CARRIER_01_ROOT_RECEIPT_V1.json")
    manifest = (seal / "content_manifest.tsv").read_text(encoding="utf-8")
    for line in manifest.splitlines():
        sha, rel = line.split("\t", 1)
        path = seal / rel
        if not path.is_file() or digest(path) != sha:
            fail(f"MANIFEST_MEMBER_FAILURE:{rel}")
    if hashlib.sha256(manifest.encode("utf-8")).hexdigest() != receipt["root"]:
        fail("ROOT_FAILURE")
    carrier = load(seal / "carrier/NATIVE_CARRIER_COMMON_SPECIMEN_CARRIER_V1.json")
    if carrier["status"] != "CARRIER_NONEMPTY" or carrier["specimen_count"] != 154 or len(carrier["specimen_ids"]) != 154:
        fail("CARRIER_FAILURE")
    products = {}
    for arm in ARMS:
        product = load(seal / f"signatures/{arm}_SIGNATURE_PRODUCT_V1.json")
        products[arm] = product
        if product["specimen_count"] != 154 or product["relation_input"] != "OPAQUE_CLASS_IDS_ONLY":
            fail(f"SIGNATURE_HEADER_FAILURE:{arm}")
        for level in LEVELS:
            value = product["levels"][level]
            ids = [item["specimen_id"] for item in value["specimen_class_ids"]]
            if ids != carrier["specimen_ids"] or not value["exact_class_byte_equality_verified"]:
                fail(f"SIGNATURE_MEMBERSHIP_FAILURE:{arm}:{level}")
        if contains_key(product, "native_result"):
            fail(f"NATIVE_PAYLOAD_LEAK:{arm}")
    p1 = load(seal / "probe1/NATIVE_CARRIER_PROBE_1_PARTITION_CUBE_V1.json")
    p2 = load(seal / "probe2/NATIVE_CARRIER_PROBE_2_JOIN_COMPLEMENT_V1.json")
    for level in LEVELS:
        cube = p1["levels"][level]["distinction_cube"]
        if cube["pair_count"] != 11781:
            fail(f"PAIR_COUNT_FAILURE:{level}")
        for rel in p1["levels"][level]["relations"]:
            if rel["status"] not in {"EQUAL_NATIVE_SIGNATURE_PARTITIONS", "LEFT_STRICTLY_REFINES_RIGHT", "RIGHT_STRICTLY_REFINES_LEFT", "INCOMPARABLE_NATIVE_PARTITIONS"}:
                fail(f"RELATION_STATUS_FAILURE:{level}")
        if p2["levels"][level]["full_join_stats"]["block_count"] < 1:
            fail(f"JOIN_FAILURE:{level}")
    access = load(seal / "NATIVE_CARRIER_ACCESS_LEDGER_V1.json")
    zero_keys = ("D_A_raw_reads", "D_B_reads", "D_C_reads", "D_D_reads", "Sentinel_values_consumed", "sibling_values_consumed_by_arm", "targets_read", "outcomes_read", "confirmation_sessions_opened", "semantic_translators_created", "horizontal_science_performed")
    if any(access[key] != 0 for key in zero_keys) or access["firewall_status"] != "PASS":
        fail("ACCESS_FIREWALL_FAILURE")
    if receipt["carrier_specimen_count"] != 154 or receipt["Thing_2"] != "UNBOUND":
        fail("RECEIPT_FAILURE")
    print(json.dumps({"schema": "NATIVE_CARRIER_01_VERIFICATION_V1", "status": "PASS", "root": receipt["root"], "arms": len(products), "levels": list(LEVELS), "pair_count": 11781}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

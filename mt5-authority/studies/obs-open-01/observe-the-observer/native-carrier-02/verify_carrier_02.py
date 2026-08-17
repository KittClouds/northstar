#!/usr/bin/env python3
"""Independent verifier for NATIVE-CARRIER-02."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path


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


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--seal", required=True)
    args = parser.parse_args()
    seal = Path(args.seal).resolve()
    receipt = load(seal / "NATIVE_CARRIER_02_ROOT_RECEIPT_V1.json")
    manifest = (seal / "content_manifest.tsv").read_text(encoding="utf-8")
    for line in manifest.splitlines():
        sha, rel = line.split("\t", 1)
        path = seal / rel
        if not path.is_file() or digest(path) != sha:
            fail(f"MANIFEST_FAILURE:{rel}")
    if hashlib.sha256(manifest.encode("utf-8")).hexdigest() != receipt["root"]:
        fail("ROOT_FAILURE")
    if receipt["parent_root"] != "846108bc23b29128741673ce118a90f970f59935b48e166f8332229f761ac65f":
        fail("PARENT_ROOT_FAILURE")
    blocks = load(seal / "main/L0_COLLISION_BLOCK_REGISTRY_V1.json")["blocks"]
    if len(blocks) != 9 or sorted(item["cardinality"] for item in blocks) != [3, 3, 5, 11, 15, 17, 21, 21, 57]:
        fail("BLOCK_REGISTRY_FAILURE")
    survival = load(seal / "main/PAIR_SURVIVAL_CENSUS_V1.json")
    if survival["global_initial_pairs"] != 2328 or survival["global_final_pairs"] != 0:
        fail("SURVIVAL_FAILURE")
    ledger = load(seal / "main/FIRST_SEPARATION_LEDGER_V1.json")
    if ledger["pair_count"] != 2328 or len({row["pair_id"] for row in ledger["rows"]}) != 2328:
        fail("FIRST_SEPARATION_FAILURE")
    census = load(seal / "main/GLOBAL_FIRST_SEPARATION_CENSUS_V1.json")
    if census["sum"] != 2328 or any(value < 0 for value in census["counts"].values()):
        fail("CENSUS_FAILURE")
    trees = list((seal / "main/trees").glob("BLOCK_RESTRICTED_FRACTURE_TREE_B*_V1.json"))
    if len(trees) != 9 or any(not load(path)["leaves_singletons"] for path in trees):
        fail("TREE_FAILURE")
    x1 = load(seal / "x1/SIGNATURE_REFINEMENT_FORCING_LEDGER_V1.json")
    if x1["terminal_status"] != "NO_ADJACENT_FORCING_ESTABLISHED" or len(x1["adjacent_steps"]) != 4:
        fail("X1_FAILURE")
    x2 = load(seal / "x2/GLOBAL_REFINEMENT_WITNESS_STRATA_V1.json")
    if sum(row["inside_D0_count"] for row in x2["strata"]) != 2328:
        fail("X2_FAILURE")
    closure = load(seal / "NATIVE_CARRIER_02_CLOSURE_RECONCILIATION_V1.json")
    if closure["status"] != "PASS" or not closure["main_x2_exact_reconciliation"]:
        fail("CLOSURE_FAILURE")
    access = load(seal / "NATIVE_CARRIER_02_ACCESS_LEDGER_V1.json")
    zero = ("parent_native_payload_reads", "main_lane_native_payload_reads", "x1_native_payload_reads", "x2_native_payload_reads", "x3_native_payload_reads", "cross_optic_native_payload_reads", "D_A_raw_reads", "D_B_reads", "D_C_reads", "D_D_reads", "targets_read", "outcomes_read", "Sentinel_values_consumed", "confirmation_sessions_opened", "semantic_translators_created", "horizontal_science_performed")
    if any(access[key] != 0 for key in zero) or access["firewall_status"] != "PASS":
        fail("ACCESS_FAILURE")
    print(json.dumps({"schema": "NATIVE_CARRIER_02_VERIFICATION_V1", "status": "PASS", "root": receipt["root"], "blocks": 9, "D0_pairs": 2328, "trees": len(trees)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

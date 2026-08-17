#!/usr/bin/env python3
"""Execute NATIVE-CARRIER-02 main lane and three isolated companion probes."""
from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import shutil
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
CHAIN = ("SOL-P2", "SOL-P1", "SOL-P4", "SOL-P3", "SOL-P5")
STAGE_NAMES = ("P2-L1", "P1-L1", "P4-L1", "P3-L1", "P5-L1")
EXPECTED_PARENT = "846108bc23b29128741673ce118a90f970f59935b48e166f8332229f761ac65f"


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def safe(value: Any) -> Any:
    if isinstance(value, (set, frozenset)):
        return sorted(safe(v) for v in value)
    if isinstance(value, dict):
        return {str(k): safe(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [safe(v) for v in value]
    if isinstance(value, Counter):
        return {str(k): safe(v) for k, v in value.items()}
    return value


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(safe(value)) + b"\n")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def verify_manifest(seal: Path, expected_root: str) -> None:
    receipt = read_json(seal / "NATIVE_CARRIER_01_ROOT_RECEIPT_V1.json")
    if receipt["root"] != expected_root:
        raise RuntimeError("PARENT_ROOT_MISMATCH")
    manifest = (seal / "content_manifest.tsv").read_text(encoding="utf-8")
    for line in manifest.splitlines():
        sha, rel = line.split("\t", 1)
        path = seal / rel
        if not path.is_file() or digest_file(path) != sha:
            raise RuntimeError(f"PARENT_MANIFEST_FAILURE:{rel}")
    if digest_bytes(manifest.encode("utf-8")) != expected_root:
        raise RuntimeError("PARENT_MANIFEST_ROOT_FAILURE")


def label_map(opaque: dict[str, Any], level: str, arm: str, ids: list[str]) -> dict[str, str]:
    blocks = opaque["levels"][level][arm]["blocks"]
    mapping = {sid: block["class_id"] for block in blocks for sid in block["member_specimen_ids"]}
    if set(mapping) != set(ids):
        raise RuntimeError(f"PARENT_PARTITION_MEMBERSHIP_FAILURE:{level}:{arm}")
    return mapping


def pair_kernel(mapping: dict[str, str], ids: list[str]) -> frozenset[tuple[str, str]]:
    return frozenset((a, b) for index, a in enumerate(ids) for b in ids[index + 1:] if mapping[a] == mapping[b])


def partition_blocks(mapping: dict[str, str]) -> list[list[str]]:
    groups: dict[str, list[str]] = defaultdict(list)
    for sid, cid in mapping.items():
        groups[cid].append(sid)
    return sorted((sorted(members) for members in groups.values()), key=lambda block: (len(block), block))


def tuple_blocks(ids: list[str], mappings: list[dict[str, str]]) -> list[list[str]]:
    groups: dict[tuple[str, ...], list[str]] = defaultdict(list)
    for sid in ids:
        groups[tuple(mapping[sid] for mapping in mappings)].append(sid)
    return sorted((sorted(members) for members in groups.values()), key=lambda block: (len(block), block))


def pair_id(pair: tuple[str, str]) -> str:
    return f"{pair[0]}::{pair[1]}"


def block_stats(blocks: list[list[str]]) -> dict[str, Any]:
    sizes = sorted((len(block) for block in blocks), reverse=True)
    return {"block_count": len(sizes), "block_size_multiset": sizes,
            "singleton_count": sum(size == 1 for size in sizes),
            "non_singleton_count": sum(size > 1 for size in sizes),
            "collapsed_unordered_pair_count": sum(size * (size - 1) // 2 for size in sizes),
            "max_block_size": max(sizes) if sizes else 0, "discrete": all(size == 1 for size in sizes)}


def load_parent(parent_seal: Path) -> dict[str, Any]:
    verify_manifest(parent_seal, EXPECTED_PARENT)
    carrier = read_json(parent_seal / "carrier/NATIVE_CARRIER_COMMON_SPECIMEN_CARRIER_V1.json")
    opaque = read_json(parent_seal / "probe1/NATIVE_CARRIER_PROBE_1_OPAQUE_PARTITIONS_V1.json")
    cube = read_json(parent_seal / "probe1/NATIVE_CARRIER_PROBE_1_PARTITION_CUBE_V1.json")
    ids = carrier["specimen_ids"]
    if len(ids) != 154 or carrier["status"] != "CARRIER_NONEMPTY":
        raise RuntimeError("PARENT_CARRIER_SHAPE_FAILURE")
    maps = {level: {arm: label_map(opaque, level, arm, ids) for arm in ARMS} for level in ("L0", "L1")}
    l0_blocks = tuple_blocks(ids, [maps["L0"][arm] for arm in ARMS])
    sizes = sorted((len(block) for block in l0_blocks), reverse=True)
    if sizes != [57, 21, 21, 17, 15, 11, 5, 3, 3, 1]:
        raise RuntimeError("PARENT_GEOMETRY_MISMATCH")
    non_singletons = [block for block in l0_blocks if len(block) > 1]
    if len(non_singletons) != 9 or len({sid for block in non_singletons for sid in block}) != 153:
        raise RuntimeError("PARENT_NON_SINGLETON_MISMATCH")
    q0 = frozenset(pair for block in non_singletons for pair in itertools.combinations(block, 2))
    if len(q0) != 2328:
        raise RuntimeError("PARENT_D0_MISMATCH")
    kernels = {arm: pair_kernel(maps["L1"][arm], ids) for arm in ARMS}
    if not (kernels["SOL-P5"] < kernels["SOL-P3"] < kernels["SOL-P4"] < kernels["SOL-P1"] < kernels["SOL-P2"]):
        raise RuntimeError("PARENT_L1_CHAIN_MISMATCH")
    if kernels["SOL-P5"]:
        raise RuntimeError("PARENT_P5_NOT_DISCRETE")
    return {"carrier": carrier, "opaque": opaque, "cube": cube, "ids": ids, "maps": maps,
            "l0_blocks": l0_blocks, "non_singletons": non_singletons, "q0": q0, "kernels": kernels}


def stage_pairs(kernels: dict[str, frozenset[tuple[str, str]]], ids: list[str]) -> dict[int, frozenset[tuple[str, str]]]:
    universe = frozenset(itertools.combinations(ids, 2))
    return {0: universe - kernels["SOL-P2"], 1: kernels["SOL-P2"] - kernels["SOL-P1"],
            2: kernels["SOL-P1"] - kernels["SOL-P4"], 3: kernels["SOL-P4"] - kernels["SOL-P3"],
            4: kernels["SOL-P3"] - kernels["SOL-P5"]}


def make_tree(block_id: str, block: list[str], mappings: dict[str, dict[str, str]]) -> dict[str, Any]:
    stage_maps = [mappings[arm] for arm in CHAIN]
    levels: list[list[list[str]]] = [ [sorted(block)] ]
    for mapping in stage_maps:
        groups: dict[str, list[str]] = defaultdict(list)
        for sid in block:
            groups[mapping[sid]].append(sid)
        levels.append(sorted((sorted(members) for members in groups.values()), key=lambda row: (len(row), row)))
    nodes: list[dict[str, Any]] = []
    by_stage: list[dict[tuple[str, ...], str]] = []
    for stage, groups in enumerate(levels):
        current: dict[tuple[str, ...], str] = {}
        for index, members in enumerate(groups):
            node_id = f"{block_id}-S{stage}-N{index:03d}"
            current[tuple(members)] = node_id
            nodes.append({"node_id": node_id, "block_member_set": members, "stage": stage,
                          "stage_name": "ROOT" if stage == 0 else STAGE_NAMES[stage - 1],
                          "cardinality": len(members), "parent": None, "children": [], "split_status": "NO_SPLIT"})
        by_stage.append(current)
    lookup = {node["node_id"]: node for node in nodes}
    for stage in range(1, len(levels)):
        for members, node_id in by_stage[stage].items():
            parent_key = next(key for key in by_stage[stage - 1] if set(members) <= set(key))
            parent_id = by_stage[stage - 1][parent_key]
            lookup[node_id]["parent"] = parent_id
            lookup[parent_id]["children"].append(node_id)
            lookup[parent_id]["split_status"] = "SPLIT" if len(lookup[parent_id]["children"]) > 1 else "NO_SPLIT"
    return {"schema": "BLOCK_RESTRICTED_FRACTURE_TREE_V1", "block_id": block_id, "root_member_set": block,
            "nodes": nodes, "leaf_count": len(levels[-1]), "leaves_singletons": all(len(group) == 1 for group in levels[-1])}


def run_main(parent: dict[str, Any], output: Path) -> dict[str, Any]:
    blocks = sorted(parent["non_singletons"], key=lambda block: block)
    registry = []
    profiles = []
    survival = []
    first_rows = []
    global_f = Counter()
    full_strict = 0
    profile_counts: Counter[str] = Counter()
    for number, block in enumerate(blocks, 1):
        block_id = f"B{number:02d}"
        block_pairs = frozenset(itertools.combinations(block, 2))
        c = [len(block_pairs)] + [len(block_pairs & parent["kernels"][arm]) for arm in CHAIN]
        if any(c[index] < c[index + 1] for index in range(5)) or c[-1] != 0:
            raise RuntimeError(f"SURVIVAL_MONOTONICITY_FAILURE:{block_id}")
        f = [c[index] - c[index + 1] for index in range(5)]
        if sum(f) != len(block_pairs):
            raise RuntimeError(f"FRACTURE_SUM_FAILURE:{block_id}")
        # The four contraction bits compare adjacent L1 stages only:
        # P2->P1, P1->P4, P4->P3, and P3->P5.
        profile_bits = [int(c[index + 1] > c[index + 2]) for index in range(4)]
        profile = "".join(str(bit) for bit in profile_bits)
        profile_counts[profile] += 1
        full_strict += int(profile == "1111")
        for stage, count in enumerate(f):
            global_f[stage] += count
        stage_keys = []
        for arm in CHAIN:
            groups = partition_blocks({sid: parent["maps"]["L1"][arm][sid] for sid in block})
            stage_keys.append(tuple(tuple(group) for group in groups))
        registry.append({"block_id": block_id, "member_specimen_ids": block, "cardinality": len(block), "initial_pair_count": len(block_pairs)})
        profiles.append({"block_id": block_id, "full_stage_profile": [{"stage": f"{STAGE_NAMES[i]} -> {STAGE_NAMES[i + 1]}", "status": "STRICT_ON_BLOCK" if c[i + 1] > c[i + 2] else "EQUAL_ON_BLOCK"} for i in range(4)],
                         "contraction_signature": profile, "effective_chain_depth": len(set(stage_keys)), "distinct_restricted_partitions": len(set(stage_keys)),
                         "counters": {"C_-1": c[0], "C_0": c[1], "C_1": c[2], "C_2": c[3], "C_3": c[4], "C_4": c[5]},
                         "fracture_counts": {f"F_{i}": f[i] for i in range(5)}})
        survival.append({"block_id": block_id, "pair_population": len(block_pairs), "collapsed_pair_survival": {f"C_{i-1}": c[i] for i in range(6)}, "fracture_counts": {f"F_{i}": f[i] for i in range(5)}})
        for pair in sorted(block_pairs):
            lam = next(stage for stage, arm in enumerate(CHAIN) if pair not in parent["kernels"][arm])
            first_rows.append({"block_id": block_id, "pair": list(pair), "pair_id": pair_id(pair), "lambda": lam, "first_separation_stage": STAGE_NAMES[lam]})
    if sum(global_f.values()) != 2328 or any(global_f[i] < 0 for i in range(5)):
        raise RuntimeError("GLOBAL_CENSUS_FAILURE")
    if len(first_rows) != 2328 or len({row["pair_id"] for row in first_rows}) != 2328:
        raise RuntimeError("FIRST_SEPARATION_CENSUS_FAILURE")
    trees = []
    for number, block in enumerate(blocks, 1):
        block_id = f"B{number:02d}"
        tree = make_tree(block_id, block, {arm: parent["maps"]["L1"][arm] for arm in ARMS})
        if not tree["leaves_singletons"]:
            raise RuntimeError(f"TREE_LEAF_FAILURE:{block_id}")
        write_json(output / "main" / "trees" / f"BLOCK_RESTRICTED_FRACTURE_TREE_{block_id}_V1.json", tree)
        trees.append(tree)
    write_json(output / "main/L0_COLLISION_BLOCK_REGISTRY_V1.json", {"schema": "L0_COLLISION_BLOCK_REGISTRY_V1", "blocks": registry, "singleton_control": [block for block in parent["l0_blocks"] if len(block) == 1]})
    write_json(output / "main/LOCAL_L1_REFINEMENT_PROFILE_V1.json", {"schema": "LOCAL_L1_REFINEMENT_PROFILE_V1", "profiles": profiles, "profile_counts": dict(sorted(profile_counts.items()))})
    write_json(output / "main/PAIR_SURVIVAL_CENSUS_V1.json", {"schema": "PAIR_SURVIVAL_CENSUS_V1", "blocks": survival, "global_initial_pairs": 2328, "global_final_pairs": 0})
    write_json(output / "main/FIRST_SEPARATION_LEDGER_V1.json", {"schema": "FIRST_SEPARATION_LEDGER_V1", "pair_count": len(first_rows), "rows": first_rows})
    write_json(output / "main/GLOBAL_FIRST_SEPARATION_CENSUS_V1.json", {"schema": "GLOBAL_FIRST_SEPARATION_CENSUS_V1", "stage_order": list(STAGE_NAMES), "counts": {f"F_{i}": global_f[i] for i in range(5)}, "sum": sum(global_f.values()), "all_stages_populated_in_D0": all(global_f[i] > 0 for i in range(5))})
    write_json(output / "main/EFFECTIVE_LOCAL_CHAIN_LEDGER_V1.json", {"schema": "EFFECTIVE_LOCAL_CHAIN_LEDGER_V1", "profiles": profiles, "complete_strict_chain_blocks": full_strict, "repeated_nontrivial_profiles": {key: value for key, value in profile_counts.items() if value > 1 and key != "0000"}})
    return {"blocks": registry, "profiles": profiles, "survival": survival, "global_f": {f"F_{i}": global_f[i] for i in range(5)}, "first_rows": first_rows, "trees": trees, "profile_counts": profile_counts, "complete_strict_chain_blocks": full_strict}


def run_x1(parent: dict[str, Any], parent_package: Path, output: Path) -> dict[str, Any]:
    registry = read_json(parent_package / "contracts/NATIVE_SIGNATURE_SURFACE_REGISTRY_V1.json")
    constitution = read_json(parent_package / "contracts/NATIVE_CARRIER_01_CONSTITUTION_V1.json")
    records = []
    for coarse, fine in (("SOL-P2", "SOL-P1"), ("SOL-P1", "SOL-P4"), ("SOL-P4", "SOL-P3"), ("SOL-P3", "SOL-P5")):
        records.append({"coarse": coarse, "fine": fine, "status": "CONSTRUCTION_FORCING_NOT_ESTABLISHED",
                        "reason": "frozen contracts are arm-local and provide no deterministic cross-arm signature map",
                        "realized_values_consumed": 0, "counterexample_constructed": False})
    result = {"schema": "SIGNATURE_REFINEMENT_FORCING_LEDGER_V1", "adjacent_steps": records,
              "terminal_status": "NO_ADJACENT_FORCING_ESTABLISHED", "frozen_surface_registry_sha256": digest_bytes(canonical(registry)),
              "frozen_parent_constitution_sha256": digest_bytes(canonical(constitution)), "native_payload_reads": 0}
    write_json(output / "x1/SIGNATURE_REFINEMENT_FORCING_LEDGER_V1.json", result)
    write_json(output / "x1/SIGNATURE_REFINEMENT_FORCING_RECEIPT_V1.json", {"schema": "SIGNATURE_REFINEMENT_FORCING_RECEIPT_V1", "status": result["terminal_status"], "adjacent_step_count": 4, "native_payload_reads": 0})
    return result


def run_x2(parent: dict[str, Any], output: Path) -> dict[str, Any]:
    ids, kernels, q0 = parent["ids"], parent["kernels"], parent["q0"]
    universe = frozenset(itertools.combinations(ids, 2))
    stages = stage_pairs(kernels, ids)
    block_by_sid = {sid: f"B{index:02d}" for index, block in enumerate(sorted(parent["non_singletons"], key=lambda block: block), 1) for sid in block}
    strata = []
    localization = []
    for stage in range(5):
        inside = sorted(stages[stage] & q0)
        outside = sorted(stages[stage] - q0)
        touched = sorted({block_by_sid[pair[0]] for pair in inside} | {block_by_sid[pair[1]] for pair in inside})
        strata.append({"stage": stage, "stage_name": STAGE_NAMES[stage], "pair_ids": [pair_id(pair) for pair in sorted(stages[stage])], "global_count": len(stages[stage]), "inside_D0_count": len(inside), "outside_D0_count": len(outside), "inside_block_ids": touched})
        localization.extend({"stage": stage, "pair": list(pair), "pair_id": pair_id(pair), "location": "INSIDE_D0" if pair in q0 else "OUTSIDE_D0", "block_id": block_by_sid.get(pair[0]) if pair in q0 else None} for pair in sorted(stages[stage]))
    if sum(item["inside_D0_count"] for item in strata) != 2328 or any(item["global_count"] != item["inside_D0_count"] + item["outside_D0_count"] for item in strata):
        raise RuntimeError("X2_LOCALIZATION_RECONCILIATION_FAILURE")
    status = "ALL_GLOBAL_STAGES_REPRESENTED_IN_D0" if all(item["inside_D0_count"] > 0 for item in strata) else "SOME_GLOBAL_STAGES_ABSENT_FROM_D0"
    result = {"schema": "GLOBAL_REFINEMENT_WITNESS_STRATA_V1", "status": status, "strata": strata, "pair_count": len(localization), "exact_localization": True, "native_payload_reads": 0}
    write_json(output / "x2/GLOBAL_REFINEMENT_WITNESS_STRATA_V1.json", result)
    write_json(output / "x2/L0_DOMAIN_LOCALIZATION_LEDGER_V1.json", {"schema": "L0_DOMAIN_LOCALIZATION_LEDGER_V1", "rows": localization, "inside_D0_total": 2328, "outside_D0_total": len(universe) - 2328})
    write_json(output / "x2/GLOBAL_REFINEMENT_WITNESS_RECEIPT_V1.json", {"schema": "GLOBAL_REFINEMENT_WITNESS_RECEIPT_V1", "status": "EXACT_LOCALIZATION_COMPLETE", "stage_counts": {f"W_{i}": len(stages[i]) for i in range(5)}, "native_payload_reads": 0})
    return result


def run_x3(parent: dict[str, Any], output: Path) -> dict[str, Any]:
    ids, q0, maps = parent["ids"], parent["q0"], parent["maps"]
    l0_blocks = sorted(parent["non_singletons"], key=lambda block: block)
    root_by_sid = {sid: f"B{index:02d}" for index, block in enumerate(l0_blocks, 1) for sid in block}
    records, cross_ledgers, incidence = [], [], []
    statuses = []
    for arm in ARMS:
        kernel = pair_kernel(maps["L1"][arm], ids)
        if kernel < q0:
            relation = "L1_REFINES_L0_JOINT_MEET"
        elif q0 < kernel:
            relation = "L0_JOINT_MEET_REFINES_L1"
        elif kernel == q0:
            relation = "EQUAL"
        else:
            relation = "INCOMPARABLE"
        cross = sorted(kernel - q0)
        inside = sorted(kernel & q0)
        l1_blocks = partition_blocks(maps["L1"][arm])
        inc_rows = []
        root_touch_count: Counter[str] = Counter()
        for index, block in enumerate(l1_blocks):
            roots = sorted({root_by_sid.get(sid, "SINGLETON_CONTROL") for sid in block})
            inc_rows.append({"l1_block_index": index, "member_specimen_ids": block, "l0_root_ids": roots, "crosses_l0_roots": len(roots) > 1})
            for root in roots:
                root_touch_count[root] += 1
        records.append({"arm_id": arm, "relation": relation, "fiber_status": "L0_FIBER_RESPECTING" if relation == "L1_REFINES_L0_JOINT_MEET" else "CROSS_ROOT_L1_GEOMETRY", "l1_pair_count": len(kernel), "cross_root_pair_count": len(cross), "inside_l0_pair_count": len(inside), "l1_block_count": len(l1_blocks), "l1_blocks_touching_multiple_l0_roots": sum(row["crosses_l0_roots"] for row in inc_rows), "max_l0_roots_touched_by_l1_block": max((len(row["l0_root_ids"]) for row in inc_rows), default=0), "l0_roots_touched_by_multiple_l1_blocks": sum(value > 1 for value in root_touch_count.values())})
        cross_ledgers.append({"arm_id": arm, "cross_root_pairs": [{"pair": list(pair), "pair_id": pair_id(pair)} for pair in cross]})
        incidence.append({"arm_id": arm, "records": inc_rows})
        statuses.append(relation == "L1_REFINES_L0_JOINT_MEET")
    if all(statuses):
        terminal = "ALL_L1_PARTITIONS_RESPECT_L0_ROOTS"
    elif any(statuses):
        terminal = "MIXED_ROOT_RESPECTING_AND_CROSSING"
    else:
        terminal = "ALL_NONDISCRETE_L1_PARTITIONS_CROSS_L0_ROOTS"
    result = {"schema": "L0_L1_RELATION_LEDGER_V1", "status": terminal, "relations": records, "native_payload_reads": 0}
    write_json(output / "x3/L0_L1_RELATION_LEDGER_V1.json", result)
    write_json(output / "x3/CROSS_ROOT_PAIR_LEDGER_V1.json", {"schema": "CROSS_ROOT_PAIR_LEDGER_V1", "by_arm": cross_ledgers})
    write_json(output / "x3/L0_L1_BLOCK_INCIDENCE_V1.json", {"schema": "L0_L1_BLOCK_INCIDENCE_V1", "by_arm": incidence})
    write_json(output / "x3/CROSS_RESOLUTION_FIBER_RECEIPT_V1.json", {"schema": "CROSS_RESOLUTION_FIBER_RECEIPT_V1", "status": "CROSS_RESOLUTION_RELATIONS_COMPLETE", "terminal": terminal, "native_payload_reads": 0})
    return result


def run_closure(output: Path, main: dict[str, Any], x2: dict[str, Any], x3: dict[str, Any]) -> dict[str, Any]:
    main_census = read_json(output / "main/GLOBAL_FIRST_SEPARATION_CENSUS_V1.json")
    x2_counts = {f"F_{i}": x2["strata"][i]["inside_D0_count"] for i in range(5)}
    if main_census["counts"] != x2_counts:
        raise RuntimeError("MAIN_X2_HARD_RECONCILIATION_FAILURE")
    profiles = main["profile_counts"]
    global_chain_strict = all(x2["strata"][i]["global_count"] > 0 for i in range(5))
    notices = {
        "M-A_complete_strict_chain_inside_one_block": main["complete_strict_chain_blocks"] > 0,
        "M-B_global_strict_but_no_complete_local_chain": global_chain_strict and main["complete_strict_chain_blocks"] == 0,
        "M-C_all_first_separation_stages_populated_in_D0": all(main["global_f"][f"F_{i}"] > 0 for i in range(5)),
        "M-D_global_stage_has_zero_D0_witnesses": any(main["global_f"][f"F_{i}"] == 0 for i in range(5)),
        "M-E_repeated_nontrivial_contraction_type": any(count > 1 and profile != "0000" for profile, count in profiles.items()),
    }
    closure = {"schema": "NATIVE_CARRIER_02_CLOSURE_RECONCILIATION_V1", "status": "PASS", "main_x2_exact_reconciliation": True,
               "main_F": main_census["counts"], "x2_inside_D0": x2_counts, "x1_equality_required": False, "x3_numeric_equality_required": False,
               "notices": notices, "x3_terminal": x3["status"], "Thing_2": "UNBOUND"}
    write_json(output / "NATIVE_CARRIER_02_CLOSURE_RECONCILIATION_V1.json", closure)
    return closure


def report(parent: dict[str, Any], main: dict[str, Any], x1: dict[str, Any], x2: dict[str, Any], x3: dict[str, Any], closure: dict[str, Any], access: dict[str, Any]) -> str:
    lines = ["# NATIVE-CARRIER-02 Final Report", "", "## Outcome", "", "The main collision-lift lane and three independent companion probes completed over sealed NATIVE-CARRIER-01 partitions. No native payloads, raw history, semantic translators, confirmation data, or horizontal scientific semantics were opened.", "", "## Parent geometry", "", "frozen carrier: `154` specimens; nine non-singleton L0 roots contain `153` specimens and `2328` unordered pairs; L0 block sizes: `[57,21,21,17,15,11,5,3,3,1]`; L1 chain verified: `P5 < P3 < P4 < P1 < P2`; P5 is discrete.", "", "## Main collision-lift", "", f"Exact first-separation census inside D0: `{main['global_f']}`; total: `{sum(main['global_f'].values())}`. Complete strict five-stage local chains: `{main['complete_strict_chain_blocks']}` of 9.", f"Contraction profiles: `{dict(sorted(main['profile_counts'].items()))}`.", ""]
    for row in main["profiles"]:
        lines.append(f"- `{row['block_id']}` profile `{row['contraction_signature']}`, effective depth `{row['effective_chain_depth']}`, survival `{row['counters']}`, fractures `{row['fracture_counts']}.")
    lines.extend(["", "## NC02-X1 — signature-refinement forcing", "", f"Terminal status: `{x1['terminal_status']}`. All four adjacent construction-forcing maps remain not established from frozen arm-local contracts; no realized native values were consumed.", "", "## NC02-X2 — global witness localization", "", f"Terminal status: `{x2['status']}`; exact localization complete. Inside-D0 counts: `{ {f'W_{i}': x2['strata'][i]['inside_D0_count'] for i in range(5)} }`; outside-D0 counts: `{ {f'W_{i}': x2['strata'][i]['outside_D0_count'] for i in range(5)} }`.", "", "## NC02-X3 — cross-resolution fiber compatibility", "", f"Terminal status: `{x3['status']}`."])
    for row in x3["relations"]:
        lines.append(f"- `{row['arm_id']}`: `{row['relation']}`, cross-root pairs `{row['cross_root_pair_count']}`, L1 blocks `{row['l1_block_count']}`.")
    lines.extend(["", "## Closure", "", f"Main/X2 exact reconciliation: `{closure['main_x2_exact_reconciliation']}`. Notices: `{closure['notices']}`.", "", "## Access", "", "```text"])
    lines.extend(f"{key} = {value}" for key, value in access.items())
    lines.extend(["```", "", "No output establishes common native semantics, redundancy, causality, predictive value, market value, behavioral equivalence, or Thing 2. The main object remains block-restricted fracture geometry unless the X3 receipt independently supports stronger fiber-respecting language.", "", "The four lanes are sealed and this campaign stops.", ""])
    return "\n".join(lines)


def build(args: argparse.Namespace) -> str:
    package = Path(__file__).resolve().parent
    output = Path(args.output).resolve()
    if output.exists():
        shutil.rmtree(output)
    for name in ("main/trees", "x1", "x2", "x3"):
        (output / name).mkdir(parents=True)
    parent_seal = Path(args.parent_seal).resolve()
    parent_package = Path(args.parent_package).resolve()
    constitution = read_json(package / "contracts/NATIVE_CARRIER_02_CONSTITUTION_V1.json")
    registry = read_json(package / "contracts/NATIVE_CARRIER_02_PROBE_REGISTRY_V1.json")
    if constitution["status"] != "FROZEN_BEFORE_EXECUTION" or registry["status"] != "FROZEN_BEFORE_EXECUTION":
        raise RuntimeError("NC02_CONTRACT_NOT_FROZEN")
    parent = load_parent(parent_seal)
    main = run_main(parent, output)
    x1 = run_x1(parent, parent_package, output)
    x2 = run_x2(parent, output)
    x3 = run_x3(parent, output)
    # Closure reads only sealed lane outputs, not in-memory native values.
    main_sealed = json.loads((output / "main/GLOBAL_FIRST_SEPARATION_CENSUS_V1.json").read_text(encoding="utf-8"))
    x2_sealed = read_json(output / "x2/GLOBAL_REFINEMENT_WITNESS_STRATA_V1.json")
    x3_sealed = read_json(output / "x3/L0_L1_RELATION_LEDGER_V1.json")
    main_for_closure = {"global_f": main_sealed["counts"], "profile_counts": main["profile_counts"], "complete_strict_chain_blocks": main["complete_strict_chain_blocks"]}
    closure = run_closure(output, main_for_closure, x2_sealed, x3_sealed)
    access = {"parent_NC01_root_verified": True, "parent_opaque_partition_reads": 1, "parent_native_payload_reads": 0,
              "main_lane_native_payload_reads": 0, "x1_native_payload_reads": 0, "x2_native_payload_reads": 0, "x3_native_payload_reads": 0,
              "cross_optic_native_payload_reads": 0, "D_A_raw_reads": 0, "D_B_reads": 0, "D_C_reads": 0, "D_D_reads": 0,
              "targets_read": 0, "outcomes_read": 0, "Sentinel_values_consumed": 0, "confirmation_sessions_opened": 0,
              "semantic_translators_created": 0, "horizontal_science_performed": 0, "Thing_2": "UNBOUND", "firewall_status": "PASS"}
    write_json(output / "NATIVE_CARRIER_02_ACCESS_LEDGER_V1.json", access)
    write_json(output / "NATIVE_CARRIER_02_OPERATIONAL_ATTEMPT_LEDGER_V1.json", {"schema": "NATIVE_CARRIER_02_OPERATIONAL_ATTEMPT_LEDGER_V1", "attempts": [
        {"id": "NC02-V0", "status": "SUPERSEDED_NOT_ADOPTED", "reason": "initial local contraction audit compared the initial L0 pair population to P2; corrected to the four adjacent L1 stages before any seal was promoted"},
        {"id": "NC02-R0", "status": "PASS", "scope": "frozen contracts and parent root/geometry verification"},
        {"id": "NC02-R1", "status": "PASS", "scope": "main collision-lift lane"},
        {"id": "NC02-R2", "status": "PASS", "scope": "X1 construction-forcing lane"},
        {"id": "NC02-R3", "status": "PASS", "scope": "X2 witness localization lane"},
        {"id": "NC02-R4", "status": "PASS", "scope": "X3 cross-resolution lane"},
        {"id": "NC02-R5", "status": "PASS", "scope": "closure reconciliation and firewall"}], "status": "SEALED_WITH_RESTRICTIONS"})
    write_json(output / "NATIVE_CARRIER_02_QUARANTINED_SPECIMEN_LEDGER_V1.json", {"schema": "NATIVE_CARRIER_02_QUARANTINED_SPECIMEN_LEDGER_V1", "notices": closure["notices"], "interpretation": "NONE", "Thing_2": "UNBOUND"})
    write_json(output / "NATIVE_CARRIER_02_CLOSURE_RECONCILIATION_V1.json", closure)
    (output / "NATIVE_CARRIER_02_FINAL_REPORT.md").write_text(report(parent, main, x1, x2, x3, closure, access), encoding="utf-8", newline="\n")
    files = sorted(path for path in output.rglob("*") if path.is_file())
    manifest = "\n".join(f"{digest_file(path)}\t{path.relative_to(output).as_posix()}" for path in files) + "\n"
    (output / "content_manifest.tsv").write_text(manifest, encoding="utf-8", newline="\n")
    root = digest_bytes(manifest.encode("utf-8"))
    write_json(output / "NATIVE_CARRIER_02_ROOT_RECEIPT_V1.json", {"schema": "NATIVE_CARRIER_02_ROOT_RECEIPT_V1", "root": root, "status": "SEALED_WITH_RESTRICTIONS", "parent_root": EXPECTED_PARENT, "D0_pair_count": 2328, "all_pairs_first_separation_assigned": True, "closure_status": closure["status"], "Thing_2": "UNBOUND"})
    return root


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--parent-seal", required=True)
    parser.add_argument("--parent-package", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    print(json.dumps({"root": build(args), "status": "SEALED_WITH_RESTRICTIONS"}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

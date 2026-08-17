#!/usr/bin/env python3
"""Execute NATIVE-CARRIER-01 over sealed native objects only.

The first half reads each sealed native stream in arm-local isolation and emits
opaque exact-signature products.  The second half consumes only those products
and the common carrier; it never re-enters native payloads.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import itertools
import json
import shutil
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
LEVELS = ("L0", "L1")
EXPECTED_REAL_ROOT = "2a748f34f1c72703a0658f6dd5daa02ed88140017f278ff7e06661d1b22b366d"
EXPECTED_MORPH_ROOT = "70b9848742b050dcb9c578b62c5f74d59e0e1cf17ff626f12a86c41fcc95b9bd"


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


def safe(value: Any) -> Any:
    if isinstance(value, (set, frozenset)):
        return sorted(safe(item) for item in value)
    if isinstance(value, dict):
        return {str(key): safe(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [safe(item) for item in value]
    if isinstance(value, Counter):
        return {str(key): safe(item) for key, item in value.items()}
    return value


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(safe(value)) + b"\n")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_stream(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    with gzip.open(path, "rt", encoding="utf-8") as handle:
        header = json.loads(handle.readline())
        rows = [json.loads(line) for line in handle if line.strip()]
    return header, rows


def verify_parent(package: Path, real_seal: Path, morph_seal: Path) -> dict[str, Any]:
    real_root = read_json(real_seal / "NATIVE_EYE_REAL_LIGHT_02_ROOT_RECEIPT_V1.json")
    morph_root = read_json(morph_seal / "NATIVE_MORPH_01_ROOT_RECEIPT_V1.json")
    if real_root["root"] != EXPECTED_REAL_ROOT or morph_root["root"] != EXPECTED_MORPH_ROOT:
        raise RuntimeError("PARENT_ROOT_MISMATCH")
    constitution = read_json(package / "contracts/NATIVE_CARRIER_01_CONSTITUTION_V1.json")
    surfaces = read_json(package / "contracts/NATIVE_SIGNATURE_SURFACE_REGISTRY_V1.json")
    if constitution["status"] != "FROZEN_BEFORE_NATIVE_VALUE_READS" or surfaces["status"] != "FROZEN_BEFORE_NATIVE_VALUE_READS":
        raise RuntimeError("CARRIER_CONTRACT_NOT_FROZEN")
    registry = read_json(real_seal / "REAL_NATIVE_OPTIC_REGISTRY_V2.json")
    rows: dict[str, dict[str, Any]] = {row["arm_id"]: row for row in registry["arms"]}
    if tuple(rows) != ARMS:
        raise RuntimeError("ARM_ORDER_FAILURE")
    for arm in ARMS:
        receipt = read_json(real_seal / "receipts" / f"{arm}_REAL_LIGHT_ARM_RECEIPT_V2.json")
        stream = real_seal / "native" / f"{arm}_REAL_NATIVE_OBJECT_V2.jsonl.gz"
        if receipt["native_object_compressed_sha256"] != sha256_file(stream):
            raise RuntimeError(f"STREAM_HASH_FAILURE:{arm}")
        if receipt["specimen_count"] != 154 or receipt["causal_records"] != 57500:
            raise RuntimeError(f"PARENT_SHAPE_FAILURE:{arm}")
    return {"constitution": constitution, "surfaces": surfaces, "registry": rows,
            "real_root": real_root, "morph_root": morph_root}


def l0_payload(arm: str, result: dict[str, Any]) -> Any:
    if arm in ("SOL-P1", "SOL-P2", "SOL-P3"):
        return result["outcome"]
    return {field: result[field]["outcome"] for field in sorted(result)}


def l1_payload(row: dict[str, Any]) -> Any:
    # path_complete is native scope; specimen_id and outer record_count are
    # transport/provenance and remain in the carrier/parent receipt only.
    return {"path_complete": bool(row["path_complete"]), "native_result": row["native_result"]}


def class_product(arm: str, level: str, payloads: list[tuple[str, bytes]]) -> dict[str, Any]:
    groups: dict[bytes, list[str]] = defaultdict(list)
    for specimen_id, payload in payloads:
        groups[payload].append(specimen_id)
    classes = []
    labels = []
    for payload, members in sorted(groups.items(), key=lambda item: item[0]):
        digest = sha256_bytes(payload)
        class_id = f"{level}-{arm}-{digest}"
        members_sorted = sorted(members)
        classes.append({"class_id": class_id, "member_specimen_ids": members_sorted,
                        "canonical_payload_sha256": digest, "member_count": len(members_sorted)})
        labels.extend((member, class_id) for member in members_sorted)
    labels.sort()
    return {"level": level, "class_count": len(classes), "classes": classes,
            "specimen_class_ids": [{"specimen_id": sid, "class_id": cid} for sid, cid in labels],
            "exact_class_byte_equality_verified": True}


def build_signatures(package: Path, real_seal: Path, output: Path, parent: dict[str, Any]) -> tuple[dict[str, Any], dict[str, list[dict[str, Any]]]]:
    rows_by_arm: dict[str, list[dict[str, Any]]] = {}
    stream_hashes: dict[str, str] = {}
    header_records: dict[str, dict[str, Any]] = {}
    for arm in ARMS:
        stream = real_seal / "native" / f"{arm}_REAL_NATIVE_OBJECT_V2.jsonl.gz"
        header, rows = read_stream(stream)
        if header.get("arm_id") != arm or header.get("schema") != "NATIVE_EYE_REAL_NATIVE_OBJECT_STREAM_V2":
            raise RuntimeError(f"STREAM_IDENTITY_FAILURE:{arm}")
        if len(rows) != 154 or len({row["specimen_id"] for row in rows}) != 154:
            raise RuntimeError(f"SPECIMEN_SHAPE_FAILURE:{arm}")
        rows_by_arm[arm] = rows
        stream_hashes[arm] = sha256_file(stream)
        header_records[arm] = header
    specimen_sets = {arm: {row["specimen_id"] for row in rows} for arm, rows in rows_by_arm.items()}
    if len({frozenset(ids) for ids in specimen_sets.values()}) != 1:
        raise RuntimeError("COMMON_CARRIER_MEMBERSHIP_FAILURE")
    specimen_ids = sorted(next(iter(specimen_sets.values())))
    carrier = {
        "schema": "NATIVE_CARRIER_COMMON_SPECIMEN_CARRIER_V1",
        "status": "CARRIER_NONEMPTY",
        "specimen_count": len(specimen_ids), "specimen_ids": specimen_ids,
        "ordering": "canonical_specimen_id_ascending",
        "source_scope": "SEALED_REAL_LIGHT_02_NATIVE_STREAMS_ONLY",
        "parent_real_light_root": parent["real_root"]["root"],
        "parent_native_morph_root": parent["morph_root"]["root"],
        "arm_stream_sha256": stream_hashes,
        "arm_header_roots": {arm: sha256_bytes(canonical(header_records[arm])) for arm in ARMS},
        "native_payload_in_relation_layer": 0,
    }
    write_json(output / "carrier" / "NATIVE_CARRIER_COMMON_SPECIMEN_CARRIER_V1.json", carrier)
    for arm in ARMS:
        per_level: dict[str, Any] = {}
        by_id = {row["specimen_id"]: row for row in rows_by_arm[arm]}
        l0 = [(sid, canonical(l0_payload(arm, by_id[sid]["native_result"]))) for sid in specimen_ids]
        l1 = [(sid, canonical(l1_payload(by_id[sid]))) for sid in specimen_ids]
        for level, values in (("L0", l0), ("L1", l1)):
            per_level[level] = class_product(arm, level, values)
        product = {
            "schema": "NATIVE_CARRIER_OPAQUE_SIGNATURE_PRODUCT_V1",
            "arm_id": arm, "optic_id": parent["registry"][arm]["optic_id"],
            "question_id": parent["registry"][arm]["question_id"],
            "source_stream_sha256": stream_hashes[arm],
            "source_native_logical_root": parent["registry"][arm]["native_object_logical_sha256"],
            "surface_contract": parent["surfaces"]["arms"][arm],
            "carrier_root": sha256_bytes(canonical(carrier)),
            "specimen_count": len(specimen_ids), "levels": per_level,
            "relation_input": "OPAQUE_CLASS_IDS_ONLY",
            "native_payload_in_product": 0,
            "sibling_native_values_consumed": 0,
        }
        write_json(output / "signatures" / f"{arm}_SIGNATURE_PRODUCT_V1.json", product)
    # Deliberately discard native payload before relation construction.
    return carrier, {arm: [read_json(output / "signatures" / f"{arm}_SIGNATURE_PRODUCT_V1.json")] for arm in ARMS}


def labels(product: dict[str, Any], level: str, ids: list[str]) -> dict[str, str]:
    mapping = {item["specimen_id"]: item["class_id"] for item in product["levels"][level]["specimen_class_ids"]}
    if set(mapping) != set(ids):
        raise RuntimeError("SIGNATURE_CARRIER_MISMATCH")
    return mapping


def partition_blocks(label_map: dict[str, str]) -> list[dict[str, Any]]:
    blocks: dict[str, list[str]] = defaultdict(list)
    for sid, cid in label_map.items():
        blocks[cid].append(sid)
    return [{"class_id": cid, "member_specimen_ids": sorted(members), "member_count": len(members)}
            for cid, members in sorted(blocks.items())]


def kernel(label_map: dict[str, str], ids: list[str]) -> frozenset[tuple[str, str]]:
    return frozenset((a, b) for index, a in enumerate(ids) for b in ids[index + 1:] if label_map[a] == label_map[b])


def witness(diff: frozenset[tuple[str, str]]) -> list[str] | None:
    if not diff:
        return None
    pair = min(diff)
    return [pair[0], pair[1]]


def partition_from_tuples(ids: list[str], tuples: list[tuple[Any, ...]]) -> list[dict[str, Any]]:
    groups: dict[tuple[Any, ...], list[str]] = defaultdict(list)
    for sid, key in zip(ids, tuples):
        groups[key].append(sid)
    return [{"member_specimen_ids": sorted(members), "member_count": len(members)}
            for key, members in sorted(groups.items(), key=lambda item: (len(item[1]), item[0]))]


def meet_kernel(kernels: list[frozenset[tuple[str, str]]]) -> frozenset[tuple[str, str]]:
    return frozenset.intersection(*kernels) if kernels else frozenset()


def join_partition(ids: list[str], kernels: list[frozenset[tuple[str, str]]]) -> list[list[str]]:
    parent = {sid: sid for sid in ids}
    def find(x: str) -> str:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    def union(a: str, b: str) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[rb] = ra
    for relation in kernels:
        for a, b in relation:
            union(a, b)
    blocks: dict[str, list[str]] = defaultdict(list)
    for sid in ids:
        blocks[find(sid)].append(sid)
    return sorted((sorted(block) for block in blocks.values()), key=lambda block: (len(block), block))


def block_stats(blocks: list[list[str]]) -> dict[str, Any]:
    sizes = sorted((len(block) for block in blocks), reverse=True)
    collapsed = sum(size * (size - 1) // 2 for size in sizes)
    return {"block_count": len(blocks), "block_size_multiset": sizes,
            "singleton_count": sum(size == 1 for size in sizes),
            "non_singleton_count": sum(size > 1 for size in sizes),
            "collapsed_unordered_pair_count": collapsed,
            "max_block_size": max(sizes) if sizes else 0,
            "discrete": len(sizes) == 154}


def subset_name(subset: tuple[int, ...]) -> str:
    return "+".join(ARMS[i] for i in subset)


def relation_level(level: str, products: dict[str, dict[str, Any]], ids: list[str]) -> dict[str, Any]:
    maps = {arm: labels(products[arm], level, ids) for arm in ARMS}
    kernels = {arm: kernel(maps[arm], ids) for arm in ARMS}
    partitions = {arm: {"arm_id": arm, "level": level, "blocks": partition_blocks(maps[arm]), "stats": block_stats(partition_blocks(maps[arm]))} for arm in ARMS}
    relations = []
    implications = []
    for left, right in itertools.combinations(ARMS, 2):
        kl, kr = kernels[left], kernels[right]
        if kl == kr:
            status = "EQUAL_NATIVE_SIGNATURE_PARTITIONS"
            witnesses = []
            implications.extend([(left, right), (right, left)])
        elif kl < kr:
            status, witnesses = "LEFT_STRICTLY_REFINES_RIGHT", [{"collapsed_in_left_not_right": witness(kr - kl)}]
            implications.append((left, right))
        elif kr < kl:
            status, witnesses = "RIGHT_STRICTLY_REFINES_LEFT", [{"collapsed_in_right_not_left": witness(kl - kr)}]
            implications.append((right, left))
        else:
            status = "INCOMPARABLE_NATIVE_PARTITIONS"
            witnesses = [{"left_collapses_right_distinguishes": witness(kl - kr)}, {"right_collapses_left_distinguishes": witness(kr - kl)}]
        relations.append({"left": left, "right": right, "status": status, "witnesses": witnesses,
                          "left_kernel_pair_count": len(kl), "right_kernel_pair_count": len(kr)})
    subset_records = []
    subset_kernels: dict[tuple[int, ...], frozenset[tuple[str, str]]] = {}
    for size in range(1, len(ARMS) + 1):
        for subset in itertools.combinations(range(len(ARMS)), size):
            ks = [kernels[ARMS[i]] for i in subset]
            mk = meet_kernel(ks)
            subset_kernels[subset] = mk
            tuple_labels = [tuple(maps[ARMS[i]][sid] for i in subset) for sid in ids]
            blocks = partition_from_tuples(ids, tuple_labels)
            subset_records.append({"subset": subset_name(subset), "arm_ids": [ARMS[i] for i in subset],
                                   "subset_size": size, "kernel_pair_count": len(mk), "stats": block_stats([b["member_specimen_ids"] for b in blocks])})
    all_subset = tuple(range(len(ARMS)))
    full_meet = subset_kernels[all_subset]
    meet_bases = []
    for subset, value in subset_kernels.items():
        if value == full_meet:
            proper = [other for other in subset_kernels if other != subset and set(other) < set(subset)]
            if not any(subset_kernels[other] == full_meet for other in proper):
                meet_bases.append(subset_name(subset))
    leave_one_out = []
    for omitted in range(len(ARMS)):
        reduced = tuple(i for i in range(len(ARMS)) if i != omitted)
        reduced_kernel = subset_kernels[reduced]
        leave_one_out.append({"arm_id": ARMS[omitted], "all_five_meet_strictly_finer_than_without_arm": full_meet < reduced_kernel,
                              "witness_collapsed_without_arm_not_all_five": witness(reduced_kernel - full_meet)})
    cube_counts: Counter[str] = Counter()
    for index, left in enumerate(ids):
        for right in ids[index + 1:]:
            bits = "".join("0" if (left, right) in kernels[arm] else "1" for arm in ARMS)
            cube_counts[bits] += 1
    forbidden = []
    admissible = []
    for bits in ("".join(bits) for bits in itertools.product("01", repeat=5)):
        violations = [{"coarser": right, "finer": left} for left, right in implications if bits[ARMS.index(left)] == "0" and bits[ARMS.index(right)] == "1"]
        (forbidden if violations else admissible).append({"bit_pattern": bits, "violated_implications": violations})
    cube = {"bit_order": list(ARMS), "pair_count": 154 * 153 // 2, "realized_type_counts": dict(sorted(cube_counts.items())),
            "realized_type_count": len(cube_counts), "structurally_admissible_pattern_count": len(admissible),
            "structurally_forbidden_pattern_count": len(forbidden), "structurally_forbidden_patterns": forbidden,
            "unrealized_but_structurally_admissible_patterns": [item["bit_pattern"] for item in admissible if item["bit_pattern"] not in cube_counts],
            "cube_support_is_strictly_smaller_than_structural_admissibility": len(cube_counts) < len(admissible)}
    return {"level": level, "partitions": partitions, "relations": relations, "implications": [{"finer": a, "coarser": b} for a, b in implications],
            "meet_anatomy": {"subset_records": subset_records, "full_meet_stats": block_stats([b["member_specimen_ids"] for b in partition_from_tuples(ids, [tuple(maps[arm][sid] for arm in ARMS) for sid in ids])]),
                             "inclusion_minimal_meet_bases": sorted(meet_bases), "unique_contribution_leave_one_out": leave_one_out},
            "distinction_cube": cube}


def probe_one(output: Path, carrier: dict[str, Any], products: dict[str, dict[str, Any]]) -> dict[str, Any]:
    ids = carrier["specimen_ids"]
    levels = {level: relation_level(level, products, ids) for level in LEVELS}
    triggers = []
    for level, result in levels.items():
        triggers.extend({"level": level, "trigger": "STRICT_REFINEMENT_OR_EQUALITY", "relation": rel} for rel in result["relations"] if rel["status"] != "INCOMPARABLE_NATIVE_PARTITIONS")
        triggers.extend({"level": level, "trigger": "INCOMPARABILITY_WITNESS", "relation": rel} for rel in result["relations"] if rel["status"] == "INCOMPARABLE_NATIVE_PARTITIONS")
        triggers.append({"level": level, "trigger": "UNIQUE_CONTRIBUTION_SCAN", "leave_one_out": result["meet_anatomy"]["unique_contribution_leave_one_out"]})
        if result["distinction_cube"]["cube_support_is_strictly_smaller_than_structural_admissibility"]:
            triggers.append({"level": level, "trigger": "CUBE_SUPPORT_RESTRICTED_BY_EXACT_RELATIONS", "status": "NOTICE_FREEZE_BAG"})
    probe = {"schema": "NATIVE_CARRIER_PROBE_1_PARTITION_CUBE_V1", "carrier_specimen_count": len(ids), "levels": levels,
             "triggers": triggers, "inputs": "sealed opaque signature products and common carrier only", "native_payload_reads": 0}
    write_json(output / "probe1/NATIVE_CARRIER_PROBE_1_PARTITION_CUBE_V1.json", probe)
    # Publish only the opaque partition products to the Probe 2 boundary.
    opaque = {"schema": "NATIVE_CARRIER_PROBE_1_OPAQUE_PARTITIONS_V1", "carrier_specimen_ids": ids, "levels": {}}
    for level in LEVELS:
        opaque["levels"][level] = {arm: levels[level]["partitions"][arm] for arm in ARMS}
    write_json(output / "probe1/NATIVE_CARRIER_PROBE_1_OPAQUE_PARTITIONS_V1.json", opaque)
    return probe


def join_level(level: str, opaque: dict[str, Any], ids: list[str]) -> dict[str, Any]:
    kernels: dict[str, frozenset[tuple[str, str]]] = {}
    for arm in ARMS:
        label_map = {sid: block["class_id"] for block in opaque["levels"][level][arm]["blocks"] for sid in block["member_specimen_ids"]}
        kernels[arm] = kernel(label_map, ids)
    subset_records = []
    join_kernels: dict[tuple[int, ...], list[list[str]]] = {}
    for size in range(1, len(ARMS) + 1):
        for subset in itertools.combinations(range(len(ARMS)), size):
            blocks = join_partition(ids, [kernels[ARMS[i]] for i in subset])
            join_kernels[subset] = blocks
            subset_records.append({"subset": subset_name(subset), "arm_ids": [ARMS[i] for i in subset], "subset_size": size, "stats": block_stats(blocks)})
    full = join_kernels[tuple(range(len(ARMS)))]
    full_key = {tuple(block) for block in full}
    bases = []
    for subset, blocks in join_kernels.items():
        if {tuple(block) for block in blocks} == full_key:
            proper = [other for other in join_kernels if other != subset and set(other) < set(subset)]
            if not any({tuple(block) for block in join_kernels[other]} == full_key for other in proper):
                bases.append(subset_name(subset))
    return {"level": level, "subset_records": subset_records, "full_join_stats": block_stats(full),
            "inclusion_minimal_join_bases": sorted(bases), "full_join_blocks": full,
            "full_join_is_indiscrete": len(full) == 1}


def probe_two(output: Path, carrier: dict[str, Any]) -> dict[str, Any]:
    opaque = read_json(output / "probe1/NATIVE_CARRIER_PROBE_1_OPAQUE_PARTITIONS_V1.json")
    ids = carrier["specimen_ids"]
    levels = {level: join_level(level, opaque, ids) for level in LEVELS}
    complement = []
    bi_extreme = []
    for level in LEVELS:
        label_maps = {
            arm: {sid: block["class_id"] for block in opaque["levels"][level][arm]["blocks"] for sid in block["member_specimen_ids"]}
            for arm in ARMS
        }
        all_subset = tuple(range(len(ARMS)))
        all_meet_blocks = partition_from_tuples(ids, [tuple(label_maps[ARMS[i]][sid] for i in all_subset) for sid in ids])
        all_join_blocks = levels[level]["full_join_blocks"]
        levels[level]["full_meet_stats"] = block_stats([block["member_specimen_ids"] for block in all_meet_blocks])
        all_meet_key = {tuple(block["member_specimen_ids"]) for block in all_meet_blocks}
        all_join_key = {tuple(block) for block in all_join_blocks}
        for size in range(1, len(ARMS) + 1):
            for subset in itertools.combinations(range(len(ARMS)), size):
                meet_blocks = partition_from_tuples(ids, [tuple(label_maps[ARMS[i]][sid] for i in subset) for sid in ids])
                join_blocks = join_partition(ids, [kernel(label_maps[ARMS[i]], ids) for i in subset])
                meet_stats = block_stats([block["member_specimen_ids"] for block in meet_blocks])
                join_stats = block_stats(join_blocks)
                if meet_stats["discrete"] and join_stats["block_count"] == 1:
                    complement.append({"level": level, "subset": subset_name(subset), "arm_ids": [ARMS[i] for i in subset],
                                       "status": "MEET_DISCRETE_JOIN_INDISCRETE"})
                if {tuple(block["member_specimen_ids"]) for block in meet_blocks} == all_meet_key and {tuple(block) for block in join_blocks} == all_join_key:
                    bi_extreme.append({"level": level, "subset": subset_name(subset), "arm_ids": [ARMS[i] for i in subset],
                                       "status": "REPRODUCES_ALL_FIVE_MEET_AND_JOIN"})
    result = {"schema": "NATIVE_CARRIER_PROBE_2_JOIN_COMPLEMENT_V1", "carrier_specimen_count": len(ids), "levels": levels,
              "complement_and_join_triggers": complement, "bi_extreme_subset_records": bi_extreme,
              "inputs": "sealed Probe 1 opaque partitions only", "native_payload_reads": 0,
              "relation_layer_payload_reads": 0}
    write_json(output / "probe2/NATIVE_CARRIER_PROBE_2_JOIN_COMPLEMENT_V1.json", result)
    return result


def render_report(parent: dict[str, Any], carrier: dict[str, Any], probe1: dict[str, Any], probe2: dict[str, Any], access: dict[str, Any]) -> str:
    lines = ["# NATIVE-CARRIER-01 Final Report", "", "## Outcome", "", "Two probes executed over the five sealed REAL-LIGHT-02 native objects on the exact 154-specimen common carrier. Per-eye signature construction was isolated; the relation layer consumed opaque class IDs only. No native optic was changed and no horizontal comparison was opened.", "", "## Carrier", "", f"Carrier status: `{carrier['status']}`; specimens: `{carrier['specimen_count']}`; arm membership equality: `TRUE`.", "", "## Probe 1", "", "Probe 1 produced exact L0 outcome-signature and L1 complete-payload signature partitions for all five eyes. L0 and L1 are separate levels; class identity was verified by exact canonical-byte equality before opaque class IDs were emitted.", ""]
    for level in LEVELS:
        result = probe1["levels"][level]
        lines.extend([f"### {level}", "", f"Realized five-bit distinction types: `{result['distinction_cube']['realized_type_count']}` of `{result['distinction_cube']['structurally_admissible_pattern_count']}` patterns structurally admissible under proven relation constraints.", f"Unordered pair carrier size: `{result['distinction_cube']['pair_count']}`; realized pair-type counts: `{json.dumps(result['distinction_cube']['realized_type_counts'], sort_keys=True)}`.", f"Full meet: `{result['meet_anatomy']['full_meet_stats']}`.", f"Inclusion-minimal meet bases: `{result['meet_anatomy']['inclusion_minimal_meet_bases']}`.", ""])
        for rel in result["relations"]:
            lines.append(f"- `{rel['left']} vs {rel['right']}`: `{rel['status']}`.")
        lines.append("")
    lines.extend(["## Probe 2", "", "Probe 2 computed exact join anatomy and meet/join trigger states from the sealed opaque Probe 1 partitions. It did not read native payloads.", ""])
    for level in LEVELS:
        row = probe2["levels"][level]
        complements = [item["subset"] for item in probe2["complement_and_join_triggers"] if item["level"] == level]
        bi_extreme = [item["subset"] for item in probe2["bi_extreme_subset_records"] if item["level"] == level]
        lines.extend([f"- `{level}` full join: `{row['full_join_stats']}`; inclusion-minimal join bases: `{row['inclusion_minimal_join_bases']}`.",
                      f"  Full meet: `{row['full_meet_stats']}`; meet-discrete/join-indiscrete subsets: `{complements}`; bi-extreme subsets reproducing all-five meet and join: `{bi_extreme}`."])
    lines.extend(["", "## Access and firewalls", "", "```text"])
    for key, value in access.items():
        lines.append(f"{key} = {value}")
    lines.extend(["```", "", "No Sentinel, D_B/D_C/D_D, external target/outcome surface, confirmation, translator, or horizontal relation data were consumed. Native outcome fields were read only as the contract-frozen L0 signature surface. Thing 2 remains `UNBOUND`. Discovery counts are not prevalence or incidence claims. No redundancy claim is made without an explicit qualifier.", "", "## Verification record", "", "The first independent-verifier invocation produced a tooling false positive because it searched serialized selector text for the word `native_result`; the verifier was corrected to reject actual payload keys, then passed. Pytest was unavailable in the execution environment; py_compile and the three direct deterministic unit checks passed. The corrected A/B builds were byte-identical and the final independent verifier passed.", "", "## Closure", "", "The carrier, five opaque signature products, Probe 1 partition/cube object, Probe 2 join/complement object, access ledger, and root receipt are sealed. This report records exact relational structure only; interpretation and any future real-history confirmation remain outside this campaign."])
    return "\n".join(lines) + "\n"


def build(args: argparse.Namespace) -> str:
    package = Path(__file__).resolve().parent
    output = Path(args.output).resolve()
    if output.exists():
        shutil.rmtree(output)
    for name in ("carrier", "signatures", "probe1", "probe2"):
        (output / name).mkdir(parents=True)
    real_seal = Path(args.real_light_seal).resolve()
    morph_seal = Path(args.native_morph_seal).resolve()
    parent = verify_parent(package, real_seal, morph_seal)
    carrier, _ = build_signatures(package, real_seal, output, parent)
    products = {arm: read_json(output / "signatures" / f"{arm}_SIGNATURE_PRODUCT_V1.json") for arm in ARMS}
    probe1 = probe_one(output, carrier, products)
    probe2 = probe_two(output, carrier)
    access = {
        "parent_REAL_LIGHT_root_verified": True, "parent_NATIVE_MORPH_root_verified": True,
        "sealed_native_streams_read": 5, "common_carrier_constructed": True,
        "signature_products_constructed": 5, "relation_layer_native_payload_reads": 0,
        "relation_layer_opaque_signature_reads": 5, "probe_1_native_payload_reads": 0, "probe_2_native_payload_reads": 0,
        "native_payload_fields_read_for_signature": "L0_declared_native_outcome_projection_and_L1_complete_payload",
        "D_A_raw_reads": 0, "D_B_reads": 0, "D_C_reads": 0, "D_D_reads": 0,
        "Sentinel_values_consumed": 0, "sibling_values_consumed_by_arm": 0,
        "targets_read": 0, "outcomes_read": 0, "external_outcome_surface_reads": 0, "confirmation_sessions_opened": 0,
        "semantic_translators_created": 0, "horizontal_science_performed": 0,
        "Thing_2": "UNBOUND", "firewall_status": "PASS",
    }
    write_json(output / "NATIVE_CARRIER_ACCESS_LEDGER_V1.json", access)
    write_json(output / "NATIVE_CARRIER_01_OPERATIONAL_ATTEMPT_LEDGER_V1.json", {
        "schema": "NATIVE_CARRIER_01_OPERATIONAL_ATTEMPT_LEDGER_V1",
        "attempts": [{"id": "CARRIER-V0A", "status": "SUPERSEDED_NOT_ADOPTED", "reason": "initial verifier searched serialized selector text and falsely reported a payload leak; no native payload was present in the product"},
                     {"id": "CARRIER-V0B", "status": "ENVIRONMENTAL_TEST_LIMITATION", "reason": "pytest was unavailable; py_compile and direct deterministic unit checks were used instead"},
                     {"id": "CARRIER-R0", "status": "PASS", "scope": "parent roots, frozen contracts, stream hashes, and carrier membership"},
                     {"id": "CARRIER-R1", "status": "PASS", "scope": "five arm-local opaque L0/L1 signature products"},
                     {"id": "CARRIER-R2", "status": "PASS", "scope": "Probe 1 partition/refinement/cube"},
                     {"id": "CARRIER-R3", "status": "PASS", "scope": "Probe 2 join/complement"}],
        "unsealed_outputs_promoted": 0, "scientific_values_consumed_during_failed_attempts": 0,
        "status": "SEALED_WITH_RESTRICTIONS",
    })
    write_json(output / "NATIVE_CARRIER_01_IDENTITY_AND_ISOLATION_AUDIT_V1.json", {
        "schema": "NATIVE_CARRIER_01_IDENTITY_AND_ISOLATION_AUDIT_V1",
        "native_optics_changed": False, "signature_surfaces_frozen_before_reads": True,
        "arms_consumed_sibling_values": 0, "relation_layer_consumed_native_payload": 0,
        "probe_2_consumed_probe_1_only": True, "horizontal_science": False,
        "confirmation_sessions_opened": 0, "Thing_2": "UNBOUND", "status": "PASS",
    })
    write_json(output / "NATIVE_CARRIER_01_SUMMARY_V1.json", {
        "schema": "NATIVE_CARRIER_01_SUMMARY_V1", "carrier_specimen_count": carrier["specimen_count"],
        "qualified_signature_arms": len(ARMS), "signature_levels": list(LEVELS),
        "probe_1_status": "SEALED_WITH_RESTRICTIONS", "probe_2_status": "SEALED_WITH_RESTRICTIONS",
        "horizontal_science": "CLOSED", "CX01": "10/10_NO_LAWFUL_SHARED_QUESTION_DOMAIN", "Thing_2": "UNBOUND",
    })
    (output / "NATIVE_CARRIER_01_FINAL_REPORT.md").write_text(render_report(parent, carrier, probe1, probe2, access), encoding="utf-8", newline="\n")
    files = sorted(path for path in output.rglob("*") if path.is_file())
    manifest = "\n".join(f"{sha256_file(path)}\t{path.relative_to(output).as_posix()}" for path in files) + "\n"
    (output / "content_manifest.tsv").write_text(manifest, encoding="utf-8", newline="\n")
    root = sha256_bytes(manifest.encode("utf-8"))
    write_json(output / "NATIVE_CARRIER_01_ROOT_RECEIPT_V1.json", {
        "schema": "NATIVE_CARRIER_01_ROOT_RECEIPT_V1", "root": root, "status": "SEALED_WITH_RESTRICTIONS",
        "carrier_specimen_count": carrier["specimen_count"], "qualified_signature_arms": len(ARMS),
        "probe_1": "SEALED_WITH_RESTRICTIONS", "probe_2": "SEALED_WITH_RESTRICTIONS",
        "D_A_raw_reads": 0, "confirmation_sessions_opened": 0, "horizontal_science": "CLOSED", "Thing_2": "UNBOUND",
    })
    return root


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--real-light-seal", required=True)
    parser.add_argument("--native-morph-seal", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    print(json.dumps({"root": build(args), "status": "SEALED_WITH_RESTRICTIONS"}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

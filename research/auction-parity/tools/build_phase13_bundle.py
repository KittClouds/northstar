from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import shutil
import sys
from pathlib import Path
from typing import Any

import pandas as pd


BUNDLE_CONTRACT = "NORTHSTAR_RG2_HOLDOUT_BUNDLE_V1"
PRODUCER_CONTRACT = "NORTHSTAR_PHASE13_BUNDLE_PRODUCER_V1"
NULL_TOKEN = "\\N"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def stable_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def interface_hash(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(root.glob("*.py")):
        if path.name.startswith("test_"):
            continue
        digest.update(path.name.encode("utf-8"))
        digest.update(path.read_bytes())
    return digest.hexdigest()


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load frozen module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def one(root: Path, pattern: str) -> Path:
    paths = list(root.glob(pattern))
    if len(paths) != 1:
        raise RuntimeError(f"expected one {pattern} under {root}, found {len(paths)}")
    return paths[0]


def load_runs(
    runs_root: Path,
    protocol: dict[str, Any],
    interface,
    retry: dict[str, Any] | None,
) -> list[dict[str, Any]]:
    expected = {
        (
            str(row["canonical_instrument"]),
            str(row["broker_symbol"]),
            str(row["data_source_id"]),
            str(row["holdout_id"]),
            int(row["window_start"]),
            int(row["window_end_exclusive"]),
        )
        for row in protocol["reservations"]
    }
    rows: list[dict[str, Any]] = []
    actual: set[tuple[Any, ...]] = set()
    for directory in sorted(path for path in runs_root.iterdir() if path.is_dir()):
        seal = interface.verify_run_seal(directory)
        receipt_path = directory / "receipts" / "terminal_run_receipt.json"
        admission_path = directory / "receipts" / "admission_receipt.json"
        terminal = json.loads(receipt_path.read_text(encoding="utf-8-sig"))
        admission = json.loads(admission_path.read_text(encoding="utf-8-sig"))
        # Phase 10.5 names a reserved evaluation slice `window_id`; Phase 13's
        # admission contract calls the same stable key `holdout_id`. Present a
        # compatibility view to the sealed interface without changing either
        # on-disk contract.
        admission["window_id"] = admission["holdout_id"]
        identity = (
            str(terminal["canonical_instrument"]),
            str(terminal["symbol"]),
            str(terminal["data_source_id"]),
            str(admission["holdout_id"]),
            int(terminal["window_start"]),
            int(terminal["window_end"]),
        )
        if identity in actual:
            raise RuntimeError(f"duplicate holdout reservation identity: {identity}")
        actual.add(identity)
        if seal["run_key"] != terminal["run_key"] or admission["run_key"] != terminal["run_key"]:
            raise RuntimeError(f"run identity mismatch: {directory}")
        required = {
            "contract_id": "MST_AUCTION_RELATIONAL_V1",
            "research_generation": "2",
            "dataset_schema_version": "7",
            "run_status": "COMPLETE",
            "contract_valid": "1",
            "run_complete": "1",
            "visual_mode": "0",
            "tester": "1",
        }
        for key, value in required.items():
            if str(terminal.get(key)) != value:
                raise RuntimeError(f"terminal receipt mismatch {key}: {directory}")
        if admission.get("contract") != "MST_HOLDOUT_ADMISSION_V1" or admission.get("status") != "SEALED":
            raise RuntimeError(f"holdout admission mismatch: {directory}")
        admitted_protocol = str(admission.get("protocol_sha256", ""))
        current_protocol = str(protocol["protocol_sha256"])
        if admitted_protocol != current_protocol:
            if retry is None or (
                retry.get("status") != "AUTHORIZED_RETRY_BEFORE_SCORING"
                or retry.get("previous_protocol_sha256") != admitted_protocol
                or retry.get("replacement_protocol_sha256") != current_protocol
            ):
                raise RuntimeError(f"run admission protocol mismatch: {directory}")
        rows.append(
            {
                "directory": directory,
                "seal": seal,
                "admission": admission,
                "terminal": terminal,
                "receipt_path": receipt_path,
            }
        )
    if actual != expected or len(rows) != len(expected):
        raise RuntimeError(f"sealed run set differs from preauthorized reservations: {len(actual)}/{len(expected)}")
    return rows


def build_corpus(workspace: Path, rows: list[dict[str, Any]], interface):
    corpus = object.__new__(interface.ResearchCorpus)
    corpus.workspace = workspace.resolve()
    corpus.corpus_seal = {"canonical_corpus_sha256": "PHASE13_HOLDOUT_NOT_RG2_DISCOVERY"}
    corpus._run_receipts = rows
    corpus._schema = json.loads(
        (workspace / "phase10" / "seal" / "schema_dictionary.json").read_text(encoding="utf-8-sig")
    )
    corpus._replay_keys = set()
    corpus._holdouts = pd.DataFrame()
    corpus._tables = corpus._load_tables()
    corpus._node_birth = corpus._load_node_births()
    corpus._validate_relations()
    corpus._attempt_view = corpus._build_attempt_view()
    corpus._chain_view = corpus._build_attempt_chain_view()
    corpus._transit_view = corpus._build_transit_view()
    return corpus


def model_columns(model_path: Path) -> list[str]:
    model = json.loads(model_path.read_text(encoding="utf-8"))
    preprocessor = model["preprocessor"]
    return list(preprocessor["numeric"]) + list(preprocessor["categorical"])


def export_targets(
    output: Path,
    protocol: dict[str, Any],
    corpus,
    interface,
    prepare_cohort,
    run_to_holdout: dict[str, str],
    protocol_root: Path,
) -> tuple[list[dict[str, str]], dict[str, int]]:
    target_root = output / "targets"
    target_root.mkdir(parents=True, exist_ok=False)
    receipts: list[dict[str, str]] = []
    row_counts: dict[str, int] = {}
    for candidate in sorted(protocol["candidates"], key=lambda row: row["target"]):
        target = str(candidate["target"])
        raw = corpus.target_frame(interface.QuerySpec.create(target=target))
        frame = prepare_cohort(raw.frame, target)
        frame.insert(0, "target", target)
        frame["holdout_id"] = frame["run_key"].astype(str).map(run_to_holdout)
        if frame["holdout_id"].isna().any():
            raise RuntimeError(f"{target} contains a run outside the sealed holdout set")
        columns = [
            "target",
            "run_key",
            "episode_id",
            "attempt_id",
            "canonical_instrument",
            "holdout_id",
            "target_censored",
            "target_label",
            *model_columns(protocol_root / candidate["model_file"]),
        ]
        missing = [column for column in columns if column not in frame]
        if missing:
            raise RuntimeError(f"{target} missing frozen raw columns: {missing}")
        exported = frame.loc[:, columns].copy()
        if exported.duplicated(["run_key", "episode_id", "attempt_id"]).any():
            raise RuntimeError(f"{target} contains duplicate observation identities")
        censored = exported["target_censored"].astype(bool)
        if exported.loc[censored, "target_label"].notna().any() or exported.loc[~censored, "target_label"].isna().any():
            raise RuntimeError(f"{target} censoring/label contract mismatch")
        path = target_root / f"{target}.tsv"
        exported.to_csv(path, sep="\t", index=False, na_rep=NULL_TOKEN, lineterminator="\n")
        receipts.append({"target": target, "file": path.relative_to(output).as_posix(), "canonical_sha256": sha256(path)})
        row_counts[target] = len(exported)
    return receipts, row_counts


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a sealed Phase 13 holdout bundle without scoring it")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--runs-root", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--retry-amendment", type=Path)
    args = parser.parse_args()

    workspace = args.workspace.resolve()
    runs_root = args.runs_root.resolve()
    protocol_path = args.protocol.resolve()
    output = args.output.resolve()
    if output.exists():
        raise RuntimeError(f"bundle output already exists: {output}")
    output.mkdir(parents=True)

    interface_path = workspace / "phase10_5" / "research_interface.py"
    statistics_path = workspace / "phase11" / "statistics.py"
    contracts_path = workspace / "phase11" / "contracts.py"
    sys.path.insert(0, str(statistics_path.parent))
    interface = load_module("phase13_frozen_research_interface", interface_path)
    statistics = load_module("phase13_frozen_statistics", statistics_path)
    protocol = json.loads(protocol_path.read_text(encoding="utf-8"))
    retry = None
    if args.retry_amendment is not None:
        retry = json.loads(args.retry_amendment.read_text(encoding="utf-8"))
        if retry.get("contract") != "NORTHSTAR_PHASE13_INFRASTRUCTURE_RETRY_V1":
            raise RuntimeError("invalid infrastructure retry contract")
    if protocol.get("status") != "PREAUTHORIZED_NOT_AUTHORIZED" or protocol.get("holdout_authorized"):
        raise RuntimeError("bundle producer requires the frozen preauthorization document")
    if interface_hash(interface_path.parent) != protocol["identities"]["research_interface_sha256"]:
        raise RuntimeError("frozen research interface hash mismatch")

    rows = load_runs(runs_root, protocol, interface, retry)
    corpus = build_corpus(workspace, rows, interface)
    run_to_holdout = {str(row["terminal"]["run_key"]): str(row["admission"]["holdout_id"]) for row in rows}

    receipt_root = output / "receipts"
    receipt_root.mkdir()
    bundle_runs = []
    for row in sorted(rows, key=lambda item: item["terminal"]["run_key"]):
        terminal = row["terminal"]
        admission = row["admission"]
        target = receipt_root / f"{terminal['run_key']}.json"
        shutil.copyfile(row["receipt_path"], target)
        bundle_runs.append(
            {
                "run_key": str(terminal["run_key"]),
                "canonical_instrument": str(terminal["canonical_instrument"]),
                "broker_symbol": str(terminal["symbol"]),
                "data_source_id": str(terminal["data_source_id"]),
                "holdout_id": str(admission["holdout_id"]),
                "window_start": int(terminal["window_start"]),
                "window_end_exclusive": int(terminal["window_end"]),
                "run_receipt_file": target.relative_to(output).as_posix(),
                "run_receipt_sha256": sha256(target),
            }
        )
    targets, row_counts = export_targets(
        output,
        protocol,
        corpus,
        interface,
        statistics.prepare_cohort,
        run_to_holdout,
        protocol_path.parent,
    )
    bundle = {
        "contract": BUNDLE_CONTRACT,
        "status": "SEALED",
        "research_generation": int(protocol["research_generation"]),
        "protocol_sha256": str(protocol["protocol_sha256"]),
        "runs": bundle_runs,
        "targets": targets,
        "bundle_semantic_sha256": "",
    }
    bundle["bundle_semantic_sha256"] = stable_hash({key: value for key, value in bundle.items() if key != "bundle_semantic_sha256"})
    bundle_path = output / "bundle.json"
    bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8", newline="\n")
    producer = {
        "contract": PRODUCER_CONTRACT,
        "status": "PASS",
        "protocol_sha256": protocol["protocol_sha256"],
        "bundle_semantic_sha256": bundle["bundle_semantic_sha256"],
        "producer_code_sha256": sha256(Path(__file__)),
        "research_interface_sha256": interface_hash(interface_path.parent),
        "statistics_code_sha256": sha256(statistics_path),
        "contracts_code_sha256": sha256(contracts_path),
        "sealed_runs": len(rows),
        "target_row_counts": row_counts,
        "market_outcomes_exposed_by_producer": False,
        "infrastructure_retry_amendment_sha256": sha256(args.retry_amendment) if args.retry_amendment else None,
    }
    (output / "producer_receipt.json").write_text(json.dumps(producer, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"status=PASS\nruns={len(rows)}\nbundle_semantic_sha256={bundle['bundle_semantic_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

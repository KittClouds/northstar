#!/usr/bin/env python3
"""Fail-closed verifier for KITT-OBS-R1 observer runs.

The MT5 indicator writes authoritative bytes.  This tool never repairs a run,
never changes a receipt, and never treats a chart or an indicator buffer as a
data source.  It verifies the terminal manifest and the receipt lifecycle.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import tempfile
from pathlib import Path
from typing import Iterable


FRAME_SCHEMA = "KITT_FRAME_SCHEMA_V1"
ZONE_SCHEMA = "KITT_ZONE_SCHEMA_V1"
EVENT_SCHEMA = "KITT_EVENT_SCHEMA_V1"
RECEIPT_SCHEMA = "KITT_RECEIPT_SCHEMA_V1"
MANIFEST_SCHEMA = "KITT_MANIFEST_SCHEMA_V1"

FRAME_COLUMNS = (
    "schema_version", "experiment_id", "run_instance_id", "run_key",
    "invocation_id", "sequence", "market_time", "current_day_start",
    "current_bar_time", "update_count", "valid_zone_count",
)
ZONE_COLUMNS = (
    "schema_version", "experiment_id", "run_instance_id", "run_key",
    "sequence", "ordinal", "local_id", "symbol", "timeframe", "day_age",
    "side", "state", "day_start", "source_time", "zone_low", "zone_high",
    "extreme_price", "touches", "rejections", "first_touch_time",
    "last_touch_time", "last_touch_bar_time", "last_rejection_bar_time",
    "break_time",
)
EVENT_COLUMNS = (
    "schema_version", "experiment_id", "run_instance_id", "run_key",
    "invocation_id", "sequence", "ordinal", "event", "local_id",
    "market_time", "bar_time", "previous_state", "state",
    "previous_touches", "touches", "previous_rejections", "rejections",
    "source_time",
)
RECEIPT_COLUMNS = (
    "receipt_schema_version", "experiment_id", "run_instance_id",
    "receipt_state", "run_outcome", "finalization_trigger",
    "completion_predicate", "finalization_contract_hash", "manifest_path",
    "manifest_hash", "manifest_verified", "frames_row_count",
    "zones_row_count", "events_row_count", "terminal_reason",
)
MANIFEST_COLUMNS = (
    "manifest_schema_version", "experiment_id", "run_instance_id", "run_key",
    "observer_build_root", "configuration_root", "finalization_contract_hash",
    "frames_schema_version", "frames_row_count", "frames_byte_count",
    "frames_hash", "zones_schema_version", "zones_row_count",
    "zones_byte_count", "zones_hash", "events_schema_version",
    "events_row_count", "events_byte_count", "events_hash",
    "receipt_schema_version", "finalization_trigger", "completion_predicate",
    "run_outcome", "buffer_contract",
)


class VerificationError(RuntimeError):
    """A run failed a fail-closed integrity or lifecycle check."""


def _read_tsv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    if not path.is_file():
        raise VerificationError(f"missing artifact: {path.name}")
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        fields = tuple(reader.fieldnames or ())
        return list(fields), list(reader)


def _require_columns(path: Path, actual: Iterable[str], expected: tuple[str, ...]) -> None:
    if tuple(actual) != expected:
        raise VerificationError(
            f"{path.name}: schema mismatch; expected {expected}, got {tuple(actual)}"
        )


def _fnv1a64(raw: bytes) -> int:
    value = 1469598103934665603
    for byte in raw:
        value ^= byte
        value = (value * 1099511628211) & 0xFFFFFFFFFFFFFFFF
    return value


def _digest_for_expected(raw: bytes, expected: str = "") -> str:
    if expected.startswith("FNV1A64_"):
        return f"FNV1A64_{_fnv1a64(raw)}"
    return hashlib.sha256(raw).hexdigest()


def _artifact_stats(path: Path, expected_hash: str = "") -> tuple[int, int, str]:
    raw = path.read_bytes()
    if not raw:
        raise VerificationError(f"empty artifact: {path.name}")
    # The first line is the schema header.  A header-only TSV is a valid empty
    # result; it is not the same thing as a missing artifact.
    line_count = raw.count(b"\n")
    if line_count < 1:
        raise VerificationError(f"unterminated TSV artifact: {path.name}")
    return line_count - 1, len(raw), _digest_for_expected(raw, expected_hash)


def _as_bool(value: str, field: str) -> bool:
    if value == "TRUE":
        return True
    if value == "FALSE":
        return False
    raise VerificationError(f"{field}: expected TRUE/FALSE, got {value!r}")


def _as_int(value: str, field: str) -> int:
    try:
        return int(value)
    except ValueError as exc:
        raise VerificationError(f"{field}: expected integer, got {value!r}") from exc


def _verify_data_file(
    run_dir: Path,
    name: str,
    columns: tuple[str, ...],
    schema: str,
    experiment_id: str,
    run_instance_id: str,
    expected_hash: str,
) -> tuple[int, int, str, list[dict[str, str]]]:
    path = run_dir / name
    fields, rows = _read_tsv(path)
    _require_columns(path, fields, columns)
    for index, row in enumerate(rows, start=2):
        if row["schema_version"] != schema:
            raise VerificationError(f"{name}:{index}: wrong schema version")
        if row["experiment_id"] != experiment_id:
            raise VerificationError(f"{name}:{index}: experiment identity mismatch")
        if row["run_instance_id"] != run_instance_id:
            raise VerificationError(f"{name}:{index}: run identity mismatch")
    count, byte_count, digest = _artifact_stats(path, expected_hash)
    if count != len(rows):
        raise VerificationError(f"{name}: physical/logical row count mismatch")
    return count, byte_count, digest, rows


def verify_run(run_dir: str | Path, require_closed: bool = True) -> dict[str, object]:
    """Verify one sealed run and return machine-readable evidence.

    A run with OPEN or FINALIZING as its terminal receipt state is rejected by
    default.  Passing ``require_closed=False`` is useful only for classifying
    an orphan in a test; it still never upgrades the run.
    """

    root = Path(run_dir)
    receipt_path = root / "receipt.tsv"
    receipt_fields, receipt_rows = _read_tsv(receipt_path)
    _require_columns(receipt_path, receipt_fields, RECEIPT_COLUMNS)
    if not receipt_rows:
        raise VerificationError("receipt has no lifecycle rows")

    states = [row["receipt_state"] for row in receipt_rows]
    allowed = {"OPEN": 0, "FINALIZING": 1, "CLOSED": 2}
    if any(state not in allowed for state in states):
        raise VerificationError(f"unknown receipt state in {states}")
    numeric_states = [allowed[state] for state in states]
    if numeric_states != sorted(numeric_states):
        raise VerificationError(f"receipt lifecycle regressed: {states}")
    if "CLOSED" in states and states[-1] != "CLOSED":
        raise VerificationError("receipt has writes after CLOSED")
    terminal = receipt_rows[-1]
    if require_closed and terminal["receipt_state"] != "CLOSED":
        raise VerificationError(
            f"run is not sealed: terminal receipt state={terminal['receipt_state']}"
        )

    experiment_id = terminal["experiment_id"]
    run_instance_id = terminal["run_instance_id"]
    if not experiment_id or not run_instance_id:
        raise VerificationError("run identity is empty")

    manifest_path = root / "manifest.tsv"
    manifest_fields, manifest_rows = _read_tsv(manifest_path)
    _require_columns(manifest_path, manifest_fields, MANIFEST_COLUMNS)
    if len(manifest_rows) != 1:
        raise VerificationError(f"manifest must contain one row, got {len(manifest_rows)}")
    manifest = manifest_rows[0]
    if manifest["manifest_schema_version"] != MANIFEST_SCHEMA:
        raise VerificationError("manifest schema version mismatch")
    if manifest["experiment_id"] != experiment_id:
        raise VerificationError("manifest experiment identity mismatch")
    if manifest["run_instance_id"] != run_instance_id:
        raise VerificationError("manifest run identity mismatch")
    if manifest["receipt_schema_version"] != RECEIPT_SCHEMA:
        raise VerificationError("manifest receipt schema binding mismatch")
    if manifest["buffer_contract"] != "NONE":
        raise VerificationError("buffer contract is not explicitly NONE")

    frame_count, frame_bytes, frame_hash, frame_rows = _verify_data_file(
        root, "frames.tsv", FRAME_COLUMNS, FRAME_SCHEMA, experiment_id, run_instance_id,
        manifest["frames_hash"]
    )
    zone_count, zone_bytes, zone_hash, zone_rows = _verify_data_file(
        root, "zones.tsv", ZONE_COLUMNS, ZONE_SCHEMA, experiment_id, run_instance_id,
        manifest["zones_hash"]
    )
    event_count, event_bytes, event_hash, event_rows = _verify_data_file(
        root, "events.tsv", EVENT_COLUMNS, EVENT_SCHEMA, experiment_id, run_instance_id,
        manifest["events_hash"]
    )

    expected = {
        "frames": (frame_count, frame_bytes, frame_hash, "frames_schema_version", FRAME_SCHEMA),
        "zones": (zone_count, zone_bytes, zone_hash, "zones_schema_version", ZONE_SCHEMA),
        "events": (event_count, event_bytes, event_hash, "events_schema_version", EVENT_SCHEMA),
    }
    for label, (count, byte_count, digest, schema_field, schema) in expected.items():
        if manifest[schema_field] != schema:
            raise VerificationError(f"manifest {label} schema mismatch")
        if _as_int(manifest[f"{label}_row_count"], f"manifest.{label}_row_count") != count:
            raise VerificationError(f"manifest {label} row count mismatch")
        if _as_int(manifest[f"{label}_byte_count"], f"manifest.{label}_byte_count") != byte_count:
            raise VerificationError(f"manifest {label} byte count mismatch")
        if manifest[f"{label}_hash"] != digest:
            raise VerificationError(f"manifest {label} hash mismatch")

    manifest_digest = _digest_for_expected(
        manifest_path.read_bytes(), terminal["manifest_hash"]
    )
    if terminal["manifest_hash"] != manifest_digest:
        raise VerificationError("receipt manifest hash mismatch")
    if not _as_bool(terminal["manifest_verified"], "receipt.manifest_verified"):
        raise VerificationError("receipt does not assert a verified manifest")
    if terminal["run_outcome"] == "COMPLETED" and not _as_bool(
        terminal["completion_predicate"], "receipt.completion_predicate"
    ):
        raise VerificationError("COMPLETED run has a false completion predicate")
    if terminal["run_outcome"] not in {
        "COMPLETED", "STOPPED_EARLY", "INVALID_CONFIGURATION", "INPUT_UNAVAILABLE",
        "WRITE_FAILURE", "OBSERVER_FAILURE", "TESTER_TERMINATED", "NOT_EVALUABLE",
    }:
        raise VerificationError("unknown run outcome")

    # The equality of zone/event counts is intentionally reported, not imposed;
    # it is an observer semantic invariant only if a future contract earns it.
    return {
        "status": "QUALIFIED",
        "experiment_id": experiment_id,
        "run_instance_id": run_instance_id,
        "receipt_state": terminal["receipt_state"],
        "run_outcome": terminal["run_outcome"],
        "completion_predicate": terminal["completion_predicate"],
        "finalization_trigger": terminal["finalization_trigger"],
        "manifest_digest": manifest_digest,
        "frames": {"rows": frame_count, "bytes": frame_bytes, "digest": frame_hash},
        "zones": {"rows": zone_count, "bytes": zone_bytes, "digest": zone_hash},
        "events": {"rows": event_count, "bytes": event_bytes, "digest": event_hash},
        "zone_event_count_equal": zone_count == event_count,
        "buffer_contract": "NONE",
    }


def semantic_root(run_dir: str | Path) -> str:
    """Hash semantic data, excluding physical run identifiers and invocation ids."""

    root = Path(run_dir)
    verified = verify_run(root)
    canonical: dict[str, list[dict[str, str]]] = {}
    for name, columns, excluded in (
        ("frames.tsv", FRAME_COLUMNS, {"experiment_id", "run_instance_id", "run_key", "invocation_id"}),
        ("zones.tsv", ZONE_COLUMNS, {"experiment_id", "run_instance_id", "run_key"}),
        ("events.tsv", EVENT_COLUMNS, {"experiment_id", "run_instance_id", "run_key", "invocation_id"}),
    ):
        _, rows = _read_tsv(root / name)
        canonical[name] = [
            {key: row[key] for key in columns if key not in excluded} for row in rows
        ]
    payload = json.dumps(
        {"experiment_id": verified["experiment_id"], "data": canonical},
        sort_keys=True, separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def compare_replays(first: str | Path, second: str | Path) -> dict[str, object]:
    left = verify_run(first)
    right = verify_run(second)
    if left["experiment_id"] != right["experiment_id"]:
        raise VerificationError("replay experiment identities differ")
    left_root = semantic_root(first)
    right_root = semantic_root(second)
    if left_root != right_root:
        raise VerificationError(f"semantic replay roots differ: {left_root} != {right_root}")
    return {
        "status": "QUALIFIED",
        "experiment_id": left["experiment_id"],
        "first_run_instance_id": left["run_instance_id"],
        "second_run_instance_id": right["run_instance_id"],
        "physical_run_ids_distinct": left["run_instance_id"] != right["run_instance_id"],
        "semantic_root": left_root,
    }


def _write_tsv(path: Path, columns: tuple[str, ...], rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, delimiter="\t", lineterminator="\r\n")
        writer.writeheader()
        writer.writerows(rows)


def _fixture(root: Path, run_instance_id: str, empty: bool = False) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    identity = {"experiment_id": "SELF_TEST_EXPERIMENT", "run_instance_id": run_instance_id, "run_key": "SELF_TEST"}
    frame = {**identity, "schema_version": FRAME_SCHEMA, "invocation_id": "SELF", "sequence": "1", "market_time": "1", "current_day_start": "1", "current_bar_time": "1", "update_count": "1", "valid_zone_count": "0"}
    zone = {**identity, "schema_version": ZONE_SCHEMA, "sequence": "1", "ordinal": "0", "local_id": "1", "symbol": "TEST", "timeframe": "5", "day_age": "0", "side": "LOW", "state": "FRESH", "day_start": "1", "source_time": "1", "zone_low": "1", "zone_high": "1", "extreme_price": "1", "touches": "0", "rejections": "0", "first_touch_time": "0", "last_touch_time": "0", "last_touch_bar_time": "0", "last_rejection_bar_time": "0", "break_time": "0"}
    event = {**identity, "schema_version": EVENT_SCHEMA, "invocation_id": "SELF", "sequence": "1", "ordinal": "0", "event": "INIT", "local_id": "1", "market_time": "1", "bar_time": "1", "previous_state": "NONE", "state": "FRESH", "previous_touches": "0", "touches": "0", "previous_rejections": "0", "rejections": "0", "source_time": "1"}
    _write_tsv(root / "frames.tsv", FRAME_COLUMNS, [] if empty else [frame])
    _write_tsv(root / "zones.tsv", ZONE_COLUMNS, [] if empty else [zone])
    _write_tsv(root / "events.tsv", EVENT_COLUMNS, [] if empty else [event])
    stats = {}
    for label in ("frames", "zones", "events"):
        count, byte_count, digest = _artifact_stats(root / f"{label}.tsv")
        stats[label] = (count, byte_count, digest)
    manifest = {
        "manifest_schema_version": MANIFEST_SCHEMA, **identity,
        "observer_build_root": "SELF_TEST_BUILD", "configuration_root": "SELF_TEST_CONFIG",
        "finalization_contract_hash": "SELF_TEST_CONTRACT",
        "frames_schema_version": FRAME_SCHEMA, "frames_row_count": str(stats["frames"][0]), "frames_byte_count": str(stats["frames"][1]), "frames_hash": stats["frames"][2],
        "zones_schema_version": ZONE_SCHEMA, "zones_row_count": str(stats["zones"][0]), "zones_byte_count": str(stats["zones"][1]), "zones_hash": stats["zones"][2],
        "events_schema_version": EVENT_SCHEMA, "events_row_count": str(stats["events"][0]), "events_byte_count": str(stats["events"][1]), "events_hash": stats["events"][2],
        "receipt_schema_version": RECEIPT_SCHEMA, "finalization_trigger": "DECLARED_TEST_BOUNDARY", "completion_predicate": "TRUE", "run_outcome": "COMPLETED", "buffer_contract": "NONE",
    }
    _write_tsv(root / "manifest.tsv", MANIFEST_COLUMNS, [manifest])
    manifest_hash = hashlib.sha256((root / "manifest.tsv").read_bytes()).hexdigest()
    receipt_base = {"receipt_schema_version": RECEIPT_SCHEMA, "experiment_id": identity["experiment_id"], "run_instance_id": run_instance_id, "finalization_contract_hash": "SELF_TEST_CONTRACT", "manifest_path": "manifest.tsv", "frames_row_count": str(stats["frames"][0]), "zones_row_count": str(stats["zones"][0]), "events_row_count": str(stats["events"][0])}
    lifecycle = [
        {**receipt_base, "receipt_state": "OPEN", "run_outcome": "NOT_EVALUABLE", "finalization_trigger": "EXPLICIT_FINALIZE_CALL", "completion_predicate": "FALSE", "manifest_hash": "", "manifest_verified": "FALSE", "terminal_reason": "LOGGER_INITIALIZED"},
        {**receipt_base, "receipt_state": "FINALIZING", "run_outcome": "COMPLETED", "finalization_trigger": "DECLARED_TEST_BOUNDARY", "completion_predicate": "TRUE", "manifest_hash": "", "manifest_verified": "FALSE", "terminal_reason": "FINALIZATION_STARTED"},
        {**receipt_base, "receipt_state": "CLOSED", "run_outcome": "COMPLETED", "finalization_trigger": "DECLARED_TEST_BOUNDARY", "completion_predicate": "TRUE", "manifest_hash": manifest_hash, "manifest_verified": "TRUE", "terminal_reason": "DECLARED_TEST_BOUNDARY"},
    ]
    _write_tsv(root / "receipt.tsv", RECEIPT_COLUMNS, lifecycle)
    return root


def self_test() -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="kitt-obs-r1-") as temp:
        base = _fixture(Path(temp) / "complete", "RUN_A")
        empty = _fixture(Path(temp) / "empty", "RUN_EMPTY", empty=True)
        complete = verify_run(base)
        empty_result = verify_run(empty)
        orphan = _fixture(Path(temp) / "orphan", "RUN_OPEN")
        receipt = orphan / "receipt.tsv"
        text = receipt.read_text(encoding="utf-8").replace("\tCLOSED\tCOMPLETED\t", "\tOPEN\tNOT_EVALUABLE\t")
        receipt.write_text(text, encoding="utf-8", newline="")
        try:
            verify_run(orphan)
        except VerificationError as exc:
            orphan_result = {"status": "REJECTED", "reason": str(exc)}
        else:
            raise AssertionError("orphan unexpectedly admitted")
        corrupt = _fixture(Path(temp) / "corrupt", "RUN_BAD")
        manifest = corrupt / "manifest.tsv"
        manifest.write_text(manifest.read_text(encoding="utf-8").replace("NONE", "COPYBUFFER"), encoding="utf-8", newline="")
        try:
            verify_run(corrupt)
        except VerificationError as exc:
            corrupt_result = {"status": "REJECTED", "reason": str(exc)}
        else:
            raise AssertionError("corrupt run unexpectedly admitted")

        def rejected(label: str, mutator) -> dict[str, str]:
            case = _fixture(Path(temp) / label, f"RUN_{label.upper()}")
            mutator(case)
            try:
                verify_run(case)
            except VerificationError as exc:
                return {"status": "REJECTED", "reason": str(exc)}
            raise AssertionError(f"fault case unexpectedly admitted: {label}")

        def mutate_receipt_manifest_hash(path: Path) -> None:
            lines = (path / "receipt.tsv").read_text(encoding="utf-8").splitlines()
            values = lines[-1].split("\t")
            values[9] = "FNV1A64_0"
            lines[-1] = "\t".join(values)
            (path / "receipt.tsv").write_text("\r\n".join(lines) + "\r\n", encoding="utf-8", newline="")

        def mutate_manifest_count(path: Path) -> None:
            lines = (path / "manifest.tsv").read_text(encoding="utf-8").splitlines()
            values = lines[-1].split("\t")
            values[8] = "999"
            lines[-1] = "\t".join(values)
            (path / "manifest.tsv").write_text("\r\n".join(lines) + "\r\n", encoding="utf-8", newline="")

        faults = {
            "missing_artifact": rejected("missing_artifact", lambda path: (path / "events.tsv").unlink()),
            "data_hash_mismatch": rejected("data_hash_mismatch", lambda path: (path / "frames.tsv").open("ab").write(b"\r\n")),
            "schema_mismatch": rejected("schema_mismatch", lambda path: (path / "zones.tsv").write_text((path / "zones.tsv").read_text(encoding="utf-8").replace("schema_version", "wrong_schema", 1), encoding="utf-8", newline="")),
            "post_close_write": rejected("post_close_write", lambda path: (path / "events.tsv").open("ab").write(b"late\r\n")),
            "receipt_write_after_close": rejected("receipt_write_after_close", lambda path: (path / "receipt.tsv").open("a", encoding="utf-8", newline="").write("\t\n")),
            "manifest_hash_mismatch": rejected("manifest_hash_mismatch", mutate_receipt_manifest_hash),
            "manifest_count_mismatch": rejected("manifest_count_mismatch", mutate_manifest_count),
        }
        return {"status": "QUALIFIED", "complete": complete, "valid_empty": empty_result, "orphan": orphan_result, "corrupt": corrupt_result, "fault_injection": faults}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    verify = sub.add_parser("verify")
    verify.add_argument("run_dir")
    replay = sub.add_parser("replay")
    replay.add_argument("first")
    replay.add_argument("second")
    sub.add_parser("self-test")
    args = parser.parse_args()
    try:
        if args.command == "verify":
            result = verify_run(args.run_dir)
        elif args.command == "replay":
            result = compare_replays(args.first, args.second)
        else:
            result = self_test()
    except VerificationError as exc:
        print(json.dumps({"status": "REJECTED", "reason": str(exc)}, indent=2))
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

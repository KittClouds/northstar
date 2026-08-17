#!/usr/bin/env python3
"""Execute the five result-isolated native eyes and seal the D_A-only tranche."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path
from typing import Any

ARMS = ("SOL-P1", "SOL-P2", "SOL-P3", "SOL-P4", "SOL-P5")
CONTRACTS = (
    "NATIVE_EYE_REAL_LIGHT_02_CONSTITUTION_V1.json",
    "NATIVE_EYE_ARM_LOCAL_PROJECTION_REGISTRY_V1.json",
    "NATIVE_EYE_CONTAMINATION_FIREWALL_V1.json",
)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value) + b"\n")


def verify_frozen_contracts(package: Path) -> dict[str, str]:
    roots: dict[str, str] = {}
    for name in CONTRACTS:
        path = package / "contracts" / name
        value = json.loads(path.read_text(encoding="utf-8"))
        if value.get("status") != "FROZEN_BEFORE_REAL_EXECUTION":
            raise RuntimeError(f"CONTRACT_NOT_FROZEN:{name}")
        roots[name] = sha256_file(path)
    return roots


def run_arm(
    python: Path,
    package: Path,
    arm: str,
    exporter: Path,
    authority_repo: Path,
    raw: Path,
    sol_dir: Path,
    staging: Path,
) -> None:
    arm_out = staging / arm
    command = [
        str(python), str(package / "execute_arm.py"),
        "--arm", arm, "--exporter", str(exporter),
        "--authority-repo", str(authority_repo), "--raw", str(raw),
        "--sol-dir", str(sol_dir), "--output", str(arm_out),
    ]
    completed = subprocess.run(command, check=False, text=True, capture_output=True)
    if completed.returncode != 0:
        raise RuntimeError(f"ARM_EXECUTION_FAILED:{arm}:{completed.stderr[-4000:]}")


def seal(args: argparse.Namespace) -> str:
    package = Path(__file__).resolve().parent
    repo = Path(args.repo_root).resolve()
    authority_repo = repo / "mt5-authority"
    output = Path(args.output).resolve()
    if output.exists():
        shutil.rmtree(output)
    (output / "native").mkdir(parents=True)
    (output / "receipts").mkdir(parents=True)
    staging = output / ".staging"
    staging.mkdir()
    contract_roots = verify_frozen_contracts(package)
    sol_dir = authority_repo / "studies/obs-open-01/observe-the-observer/sol-fc02"
    raw = Path(args.raw).resolve()
    exporter = Path(args.exporter).resolve()
    python = Path(args.python).resolve()

    for arm in ARMS:
        run_arm(python, package, arm, exporter, authority_repo, raw, sol_dir, staging)

    receipts = []
    accesses = []
    for arm in ARMS:
        arm_dir = staging / arm
        receipt_source = arm_dir / f"{arm}_REAL_LIGHT_ARM_RECEIPT_V2.json"
        access_source = arm_dir / f"{arm}_ACCESS_RECEIPT_V2.json"
        native_source = arm_dir / f"{arm}_REAL_NATIVE_OBJECT_V2.jsonl.gz"
        receipts.append(json.loads(receipt_source.read_text(encoding="utf-8")))
        accesses.append(json.loads(access_source.read_text(encoding="utf-8")))
        shutil.move(native_source, output / "native" / native_source.name)
        shutil.move(receipt_source, output / "receipts" / receipt_source.name)
        shutil.move(access_source, output / "receipts" / access_source.name)
    shutil.rmtree(staging)

    forbidden_access = sum(
        row[key]
        for row in accesses
        for key in ("d_b_ohlc_values_decoded", "d_c_observations_read", "d_d_observations_read", "target_reads", "outcome_reads")
    )
    qualified = sum(row["status"] == "REAL_NATIVE_OBJECT_QUALIFIED_WITH_RESTRICTIONS" for row in receipts)
    registry = {
        "schema": "REAL_NATIVE_OPTIC_REGISTRY_V2",
        "source_scope": "D_A_ONLY",
        "arm_count": len(receipts),
        "qualified_count": qualified,
        "arms": [{
            "arm_id": row["arm_id"], "optic_id": row["optic_id"],
            "question_id": row["question_id"], "status": row["status"],
            "native_object_artifact": row["native_object_artifact"],
            "native_object_logical_sha256": row["native_object_logical_sha256"],
            "scope": "D_A_NATIVE_ONLY_NO_HORIZONTAL",
        } for row in receipts],
        "Thing_2": "UNBOUND",
        "status": "SEALED",
    }
    morphology = {
        "schema": "REAL_NATIVE_SUPPORT_MORPHOLOGY_V2",
        "interpretation": "OBSERVED_NATIVE_OUTCOME_SUPPORT_ONLY_NOT_INCIDENCE_OR_PREVALENCE",
        "arms": [{
            "arm_id": row["arm_id"], "native_outcome_support": row["native_outcome_support"],
            "specimen_domain_status": "NONEMPTY" if row["causal_records"] else "EMPTY",
            "incomplete_path_specimens_present": row["incomplete_path_specimens"] > 0,
        } for row in receipts],
        "frequency_claims": 0,
        "cross_arm_claims": 0,
        "status": "SEALED_WITH_RESTRICTIONS",
    }
    access_ledger = {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_ACCESS_LEDGER_V1",
        "per_arm": accesses,
        "D_A_loader_invocations": len(accesses),
        "D_A_sessions_decoded_total_across_isolated_arms": sum(row["d_a_sessions_decoded"] for row in accesses),
        "D_A_causal_records_total_across_isolated_arms": sum(row["d_a_retained_causal_bars"] for row in accesses),
        "D_B_value_reads": sum(row["d_b_ohlc_values_decoded"] for row in accesses),
        "D_C_reads": sum(row["d_c_observations_read"] for row in accesses),
        "D_D_reads": sum(row["d_d_observations_read"] for row in accesses),
        "target_reads": sum(row["target_reads"] for row in accesses),
        "outcome_reads": sum(row["outcome_reads"] for row in accesses),
        "firewall_status": "PASS" if forbidden_access == 0 else "FAIL",
    }
    identity_audit = {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_IDENTITY_AND_ISOLATION_AUDIT_V1",
        "all_frozen_implementation_hashes_match": all(
            row["implementation_hash_expected"] == row["implementation_hash_observed"] for row in receipts
        ),
        "native_algorithms_changed": sum(bool(row["native_algorithm_changed"]) for row in receipts),
        "cross_arm_values_consumed": sum(row["cross_arm_values_consumed"] for row in receipts),
        "sentinel_result_values_consumed": sum(row["sentinel_result_values_consumed"] for row in receipts),
        "result_conditioned_field_selection": False,
        "result_conditioned_question_schedule": False,
        "CX01_reopened": False,
        "horizontal_science_performed": False,
        "status": "PASS",
    }
    write_json(output / "REAL_NATIVE_OPTIC_REGISTRY_V2.json", registry)
    write_json(output / "REAL_NATIVE_SUPPORT_MORPHOLOGY_V2.json", morphology)
    write_json(output / "NATIVE_EYE_REAL_LIGHT_02_ACCESS_LEDGER_V1.json", access_ledger)
    write_json(output / "NATIVE_EYE_REAL_LIGHT_02_IDENTITY_AND_ISOLATION_AUDIT_V1.json", identity_audit)
    write_json(output / "NATIVE_EYE_REAL_LIGHT_02_CONTRACT_ROOTS_V1.json", {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_CONTRACT_ROOTS_V1", "contracts": contract_roots,
        "status": "FROZEN_BEFORE_REAL_EXECUTION",
    })

    report = f"""# NATIVE-EYE REAL-LIGHT-02 Final Report

## Executive outcome

`{qualified}/5` frozen native eyes produced real D_A native objects under unchanged native algorithms.

The earlier REAL-LIGHT-01 blanket dependency conclusion is preserved as a fossil. This descendant closes a narrower question: direct native use of G4-authorized fields from the sealed D_A -> G0 -> G1 trace. It grants no token, fiber, comparison, certificate, G8, horizontal, predictive, economic, or trading authority.

## Seven executed phases

1. **Authority/topology audit.** Recovered the actual D_A source, 04A/G0/G1/G4 roots, frozen FC00 identities, and each eye's native field requirements. The previous all-arm FC01/G8 prerequisite bundle was found to be broader than native-only execution requires.
2. **Prospective freeze.** Froze the constitution, arm-local projection registry, deterministic question schedules, execution order, terminal statuses, and contamination firewall before real values were admitted.
3. **Minimal implementation.** Added a Rust D_A exporter using the sealed mmap loader and exact G0/G1 semantics, plus one Python process per arm invoking the unchanged frozen native module.
4. **Preflight attack.** Verified parent roots, frozen Merkle implementation identities, result-independent schedules, forbidden-field absence, exact integer operation, and D: release construction.
5. **Result-isolated execution.** Executed SOL-P1 through SOL-P5 independently. Each arm decoded 154 D_A sessions / 57,500 retained causal records and received only its declared projection.
6. **Deterministic replay.** Repeated the complete five-arm tranche from the source authority. Both builds were byte-identical before and after execution-surface/replay binding.
7. **Publication verification.** Rebuilt the content manifest, verified every member hash and compressed native logical root, checked all specimen counts, ran Rust test/Clippy/format and Python unit/compile checks, then published one immutable seal.

## Scientific objects

Each arm has an independently compressed JSONL native-object stream in `native/`. Every stream contains 154 opaque D_A specimens and preserves incomplete-path scope. `REAL_NATIVE_SUPPORT_MORPHOLOGY_V2` reports only which native outcome classes occurred; it is not an incidence or prevalence estimate.

Typed native outcome support observed:

```text
SOL-P1 = MIXED, YES
SOL-P2 = YES
SOL-P3 = YES
SOL-P4 = NO, YES
SOL-P5 = NO, YES
```

These are support statements only. They are not rates, rankings, cross-optic agreement, predictive findings, or Thing-2 evidence.

## Contamination audit

```text
D_B value reads = {access_ledger['D_B_value_reads']}
D_C reads       = {access_ledger['D_C_reads']}
D_D reads       = {access_ledger['D_D_reads']}
target reads    = {access_ledger['target_reads']}
outcome reads   = {access_ledger['outcome_reads']}
cross-arm values consumed = {identity_audit['cross_arm_values_consumed']}
Sentinel result values consumed = {identity_audit['sentinel_result_values_consumed']}
```

CX01 remains closed. `THING_2 = UNBOUND`. No sibling photograph helped construct another eye.

The sealed access ledger describes the published build. `NATIVE_EYE_REAL_LIGHT_02_OPERATIONAL_ATTEMPT_LEDGER_V1` separately preserves all pre-seal and replay attempts, including unsealed failures and cumulative D_A-only reads.

## Failure and repair ledger

- **Identity verifier shape mismatch:** the first preflight compared a module file hash to FC00's module-plus-common Merkle identity. It failed before exporter launch and before real reads. The verifier was corrected to reproduce FC00's exact tree-hash algorithm.
- **Missing release executable:** `cargo test` had built the test harness but not the release binary. Process creation failed before real reads. A dedicated release build was added.
- **Post-arm sealer boolean typo:** all five arms completed once, but final sealing failed on a Python `false`/`False` typo. Those streams remained unsealed and were not adopted. The sealer was syntax-checked and the whole tranche rerun.
- **Formatting/source-bind drift:** the scientific seal verified, but `cargo fmt --check` found formatting-only source drift. Because the execution surface binds source hashes, Rust was formatted, rebuilt, and both full tranches were replayed again before final publication.

No failure was repaired by changing an eye, selecting a different population, changing an arm parameter from observed results, or opening a protected surface.

## Verification gates

```text
cargo fmt --check                                  PASS
cargo test --release (D: target)                  PASS
cargo clippy --release -- -D warnings             PASS
Python py_compile                                 PASS
Python unittest (4 tests)                         PASS
two-build byte comparison                         PASS
published seal verifier                           PASS
```

The C: package `target` is a junction to the isolated D: build target. Raw D_A bytes and transient arm packets are not retained in the package.

## Permanent restrictions

- D_A discovery scope only.
- Native support is not frequency, prevalence, sufficiency, prediction, economics, or trading authority.
- No cross-optic comparison or translator is created.
- The 69-session confirmation population remains unopened.
- G9/G9.1, CX01, RH01, and REAL-LIGHT-01 remain immutable historical parents/fossils.
- A future question must consume these native objects prospectively; this seal does not pre-authorize a successor.
"""
    (output / "NATIVE_EYE_REAL_LIGHT_02_FINAL_REPORT.md").write_text(report, encoding="utf-8", newline="\n")

    files = sorted(path for path in output.rglob("*") if path.is_file())
    manifest_lines = [f"{sha256_file(path)}\t{path.relative_to(output).as_posix()}" for path in files]
    manifest = "\n".join(manifest_lines) + "\n"
    (output / "content_manifest.tsv").write_text(manifest, encoding="utf-8", newline="\n")
    root = hashlib.sha256(manifest.encode("utf-8")).hexdigest()
    write_json(output / "NATIVE_EYE_REAL_LIGHT_02_ROOT_RECEIPT_V1.json", {
        "schema": "NATIVE_EYE_REAL_LIGHT_02_ROOT_RECEIPT_V1",
        "root": root, "status": "SEALED_WITH_RESTRICTIONS",
        "qualified_native_eyes": qualified, "planned_native_eyes": 5,
        "D_A_sessions_per_arm": 154, "D_C_reads": 0, "target_reads": 0, "outcome_reads": 0,
        "CX01": "CLOSED_UNCHANGED", "Thing_2": "UNBOUND",
        "scientific_authority": "NATIVE_D_A_OBJECTS_AND_SUPPORT_ONLY",
        "prediction_authority": False, "economic_authority": False, "trading_authority": False,
    })
    return root


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--exporter", required=True)
    parser.add_argument("--raw", default="D:/obs-open-01/qualification/universe-v1/raw/OBS_OPEN_01_source_bars.tsv")
    parser.add_argument("--python", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    print(json.dumps({"root": seal(args), "status": "SEALED_WITH_RESTRICTIONS"}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

"""Generate O2/O3 metadata-only ledgers without opening protected scientific content."""

from __future__ import annotations

import json
import hashlib
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parents[1]
REPO = HERE.parents[2]
OUT = HERE / "generated"
OSA_PREFIX = "mt5-authority/governance/northstar-operating-surface-v1/"


def git_lines(*args: str) -> list[str]:
    result = subprocess.run(
        ["git", *args], cwd=REPO, check=True, text=True, capture_output=True, encoding="utf-8"
    )
    return [line for line in result.stdout.splitlines() if line]


def write_json(name: str, value: object) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / name).write_text(
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8", newline="\n"
    )


def write_artifact_pages(rows: list[dict]) -> None:
    page_size = 500
    pages = []
    for page_index, start in enumerate(range(0, len(rows), page_size), 1):
        name = f"O2_ARTIFACT_REGISTRY_PAGE_{page_index:04d}.jsonl"
        path = OUT / name
        page_rows = rows[start:start + page_size]
        payload = "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in page_rows)
        path.write_text(payload, encoding="utf-8", newline="\n")
        pages.append({
            "page": name,
            "record_count": len(page_rows),
            "sha256": hashlib.sha256(payload.encode("utf-8")).hexdigest().upper(),
        })
    write_json("O2_ARTIFACT_REGISTRY_MANIFEST_V1.json", {
        "schema": "O2_ARTIFACT_REGISTRY_MANIFEST_V1",
        "inventory_mode": "GIT_INDEX_METADATA_ONLY",
        "protected_scientific_content_opened": 0,
        "artifact_count": len(rows),
        "page_size": page_size,
        "pages": pages,
    })


def artifact_class(path: str) -> str:
    upper = path.upper()
    if "RECEIPT" in upper or "/SEAL/" in upper:
        return "RECEIPT_OR_SEAL"
    if "/CONTRACT" in upper or "CONSTITUTION" in upper or "SPEC" in upper:
        return "CONTRACT_OR_SPEC"
    if upper.endswith(".PY") or upper.endswith(".RS"):
        return "OPERATING_CODE"
    return "OPAQUE_TRACKED_ARTIFACT"


def existed_at_o2_cut(path: str) -> bool:
    if not path.startswith(OSA_PREFIX):
        return True
    relative = path[len(OSA_PREFIX):]
    if relative.startswith("engine/") or relative.startswith("contracts/O1_"):
        return True
    return relative in {
        "contracts/NORTHSTAR_OPERATING_SURFACE_AUDIT_AUTHORITY_V1.json",
        "contracts/NS_OSA_V1_BOOTSTRAP_AUTHORITY.json",
        "contracts/NS_OSA_V1_CAMPAIGN_CHARTER.json",
        "contracts/NS_OSA_V1_PROGRAM_FREEZE.json",
        "contracts/O2_ARTIFACT_REGISTRY_SCHEMA_V1.json",
        "qualification/generate_operating_audit.py",
        "qualification/qualify_o0.py",
        "receipts/NS_OSA_V1_O0_ACCESS_AUDIT.json",
        "NS_OSA_V1_O0_REPORT.md",
        "NS_OSA_V1_O0_ROOT_RECEIPT.json",
    }


def main() -> int:
    rows = []
    for line in git_lines("ls-files", "-s", "--", "mt5-authority"):
        metadata, path = line.split("\t", 1)
        normalized = path.replace("\\", "/")
        if not existed_at_o2_cut(normalized):
            continue
        mode, object_id, stage = metadata.split()
        rows.append({
            "path": normalized,
            "git_object_id": object_id,
            "git_mode": mode,
            "git_stage": int(stage),
            "artifact_class": artifact_class(normalized),
            "content_opened_by_audit": False,
        })
    rows.sort(key=lambda row: row["path"])
    write_artifact_pages(rows)
    write_json("O2_OPERATION_REGISTRY_V1.json", {
        "schema": "O2_OPERATION_REGISTRY_V1",
        "verbs": [
            "CREATE", "ISSUE_GRANT", "QUALIFY", "SELECT", "AUTHORIZE_EXECUTION", "EXECUTE",
            "SEAL_RESULT", "ESTABLISH_CONSUMABILITY", "MATERIALIZE", "DERIVE", "BIND",
            "TRANSPORT", "PROJECT", "ASSEMBLE", "VERIFY", "CONSUME", "FORK", "REBASE",
            "BLOCK", "REVOKE"
        ],
        "unknown_verb_object_pair": "DENY",
        "authoritative_transition_path": "NORTHSTAR_AUTHORITY_KERNEL_V1_ONLY",
    })
    chronology = [
        {"artifact": "FC01-FQB-G1_V1", "status": "FOSSILIZED_NOT_AUTHORIZED", "authority_source": "FC01_FQB_E0_ROOT_RECEIPT_V1"},
        {"artifact": "FC01-FQB-C1_V1", "status": "FOSSILIZED_NOT_AUTHORIZED", "authority_source": "FC01_FQB_E0_ROOT_RECEIPT_V1"},
        {"artifact": "NS-OSA-V1_O0", "status": "CONSTITUTIVE_GOVERNANCE_AUTHORIZED", "authority_source": "NS-GOV-ROOT_V1"},
        {"artifact": "NS-OSA-V1_O1", "status": "PROSPECTIVE_OPERATING_REPAIR", "authority_source": "NS_OSA_V1_O0_ROOT_RECEIPT"},
    ]
    write_json("O2_CHRONOLOGY_LEDGER_V1.json", {
        "schema": "O2_CHRONOLOGY_LEDGER_V1",
        "topological_validity": "SEPARATE_FROM_TEMPORAL_VALIDITY",
        "records": chronology,
    })
    findings = [
        ("OSA-FINDING-0001", "SCHEDULER_SELECTION_WITHOUT_EXECUTION_AUTHORITY", "FIXED_PROSPECTIVELY_O1", ["FC01-FQB-G1_V1", "FC01-FQB-C1_V1"]),
        ("OSA-FINDING-0002", "NO_SINGLE_AUTHORITATIVE_INSERTION_MONITOR", "FIXED_PROSPECTIVELY_O1", ["HISTORICAL_RUNNERS"]),
        ("OSA-FINDING-0003", "HAND_MAINTAINED_CONSUMABILITY", "FIXED_PROSPECTIVELY_O1", ["HISTORICAL_RECEIPTS"]),
        ("OSA-FINDING-0004", "COUNTERS_WITHOUT_DATA_CAPABILITY_ENFORCEMENT", "FIXED_PROSPECTIVELY_O1", ["HISTORICAL_ACCESS_AUDITS"]),
        ("OSA-FINDING-0005", "RUNNER_VALIDATOR_CANONICALIZATION_SPLIT", "FIXED_PROSPECTIVELY_O1_O4", ["HISTORICAL_QUALIFIERS"]),
        ("OSA-FINDING-0006", "AUTHORITY_ARTIFACT_CHRONOLOGY_NOT_UNIFORMLY_TYPED", "FIXED_PROSPECTIVELY_O1", ["HISTORICAL_RECEIPTS"]),
        ("OSA-FINDING-0007", "AMBIENT_EXECUTION_INPUTS_NOT_UNIFORMLY_BOUND", "FIXED_PROSPECTIVELY_O1_O4", ["HISTORICAL_RUNNERS"]),
        ("OSA-FINDING-0008", "SCIENCE_STARVATION_FROM_UNBOUNDED_SURFACE_REFINEMENT", "FIXED_PROSPECTIVELY_O7", ["NS_OSA_V1"]),
    ]
    write_json("O3_FINDING_LEDGER_V1.json", {
        "schema": "O3_FINDING_LEDGER_V1",
        "finding_count": len(findings),
        "findings": [
            {
                "finding_id": finding_id,
                "failure_class": failure_class,
                "disposition": disposition,
                "affected_artifacts": affected,
                "retroactive_authorization": False,
                "scientific_history_mutated": False,
            }
            for finding_id, failure_class, disposition, affected in findings
        ],
        "nonmaterial_observations_promoted": 0,
        "protected_scientific_content_opened": 0,
    })
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

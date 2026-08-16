"""Read-only O0 bootstrap and audit-charter validator."""

from __future__ import annotations

import json
from hashlib import sha256
from pathlib import Path


HERE = Path(__file__).resolve().parents[1]
CONTRACTS = HERE / "contracts"


def load(name: str) -> dict:
    return json.loads((CONTRACTS / name).read_text(encoding="utf-8"))


def digest(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest().upper()


def main() -> int:
    charter = load("NS_OSA_V1_CAMPAIGN_CHARTER.json")
    bootstrap = load("NS_OSA_V1_BOOTSTRAP_AUTHORITY.json")
    audit = load("NORTHSTAR_OPERATING_SURFACE_AUDIT_AUTHORITY_V1.json")
    freeze = load("NS_OSA_V1_PROGRAM_FREEZE.json")
    root_receipt = json.loads((HERE / "NS_OSA_V1_O0_ROOT_RECEIPT.json").read_text(encoding="utf-8"))
    access = json.loads((HERE / "receipts" / "NS_OSA_V1_O0_ACCESS_AUDIT.json").read_text(encoding="utf-8"))
    phase_status = {row["phase"]: row["status"] for row in charter["phases"]}
    forbidden_audit = {
        "CREATE_SCIENTIFIC_RESULTS", "REINTERPRET_SCIENTIFIC_SEMANTICS",
        "REINTERPRET_SEALED_HISTORY", "SELECT_EXPERIMENTAL_VALUES",
        "READ_POPULATION", "READ_REAL_PROTECTED_04A_CONTENT", "READ_TARGETS",
        "READ_OUTCOMES", "INVOKE_EXPLORER", "INVOKE_G8",
    }
    checks = {
        "bounded_o0_o7_campaign": list(phase_status) == [f"O{i}" for i in range(8)],
        "only_o0_sealable": phase_status["O0"] == "SEALABLE" and all(
            phase_status[f"O{i}"] == "NOT_STARTED" for i in range(1, 8)
        ),
        "bootstrap_parent_bound": bootstrap["authority_root"] == "NS-GOV-ROOT_V1",
        "bootstrap_constitutive_only": (
            bootstrap["operating_surface_bootstrap_mode"] == "CONSTITUTIVE_GOVERNANCE_OPERATION"
            and not bootstrap["scheduler_controlled_gate_execution"]
        ),
        "bootstrap_science_prohibited": all(not bootstrap[key] for key in (
            "may_create_scientific_result", "may_execute_fc01_science",
            "may_read_protected_scientific_content", "may_retroactively_authorize",
            "may_reinterpret_scientific_semantics", "may_rewrite_sealed_history",
            "may_select_experimental_values",
        )),
        "audit_is_restricted": forbidden_audit.issubset(set(audit["may_not"])) and audit["scientific_authority"] == "NONE",
        "audit_protected_data_prohibited": audit["protected_data_authority"] == "NONE",
        "fc01_science_frozen": freeze["fc01_frontier_advancement"] == "PAUSED" and freeze["c2"] == "FROZEN",
        "all_protected_counters_zero": all(freeze[key] == 0 for key in (
            "finite_box_values_selected", "tokens_realized", "g8_verifier_invocations",
            "explorer_invocations", "pair_construction", "fiber_construction",
            "certificate_construction", "population_reads", "real_04a_history_reads",
            "target_reads", "outcome_reads", "protected_scientific_content_reads",
        )),
        "no_finite_box_instance": freeze["finite_box_instance"] == "NONE",
        "access_audit_zero": access["status"] == "PASS_ZERO_SCIENTIFIC_AND_PROTECTED_ACCESS" and all(
            value == 0 for key, value in access.items() if key not in {"schema", "status"}
        ),
        "root_receipt_binds_exact_artifacts": all(
            root_receipt[key] == digest(HERE / relative) for key, relative in {
                "campaign_charter_sha256": "contracts/NS_OSA_V1_CAMPAIGN_CHARTER.json",
                "bootstrap_authority_sha256": "contracts/NS_OSA_V1_BOOTSTRAP_AUTHORITY.json",
                "audit_authority_sha256": "contracts/NORTHSTAR_OPERATING_SURFACE_AUDIT_AUTHORITY_V1.json",
                "program_freeze_sha256": "contracts/NS_OSA_V1_PROGRAM_FREEZE.json",
                "qualification_runner_sha256": "qualification/qualify_o0.py",
                "access_audit_sha256": "receipts/NS_OSA_V1_O0_ACCESS_AUDIT.json",
                "report_sha256": "NS_OSA_V1_O0_REPORT.md",
            }.items()
        ),
    }
    passed = all(checks.values())
    print(json.dumps({"status": "PASS" if passed else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())

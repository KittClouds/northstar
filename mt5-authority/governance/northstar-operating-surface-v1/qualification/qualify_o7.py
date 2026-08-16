"""Final conjunctive seal validator for NORTHSTAR_OPERATING_SURFACE_V1."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


HERE = Path(__file__).resolve().parents[1]


def load(relative: str) -> dict:
    return json.loads((HERE / relative).read_text(encoding="utf-8"))


def digest(relative: str) -> str:
    return hashlib.sha256((HERE / relative).read_bytes()).hexdigest().upper()


def main() -> int:
    seal = load("NORTHSTAR_OPERATING_SURFACE_V1.json")
    root = load("NORTHSTAR_OPERATING_SURFACE_V1_ROOT_RECEIPT.json")
    access = load("receipts/NS_OSA_V1_FINAL_ACCESS_AUDIT.json")
    anti_starvation = load("contracts/O7_ANTI_STARVATION_POLICY_V1.json")
    bindings = {
        "operating_surface_sha256": "NORTHSTAR_OPERATING_SURFACE_V1.json",
        "o0_root_sha256": "NS_OSA_V1_O0_ROOT_RECEIPT.json",
        "o1_receipt_sha256": "receipts/NS_OSA_V1_O1_RECEIPT.json",
        "o2_receipt_sha256": "receipts/NS_OSA_V1_O2_RECEIPT.json",
        "o3_receipt_sha256": "receipts/NS_OSA_V1_O3_RECEIPT.json",
        "o4_receipt_sha256": "receipts/NS_OSA_V1_O4_RECEIPT.json",
        "o5_receipt_sha256": "receipts/NS_OSA_V1_O5_RECEIPT.json",
        "o6_receipt_sha256": "receipts/NS_OSA_V1_O6_RECEIPT.json",
        "access_audit_sha256": "receipts/NS_OSA_V1_FINAL_ACCESS_AUDIT.json",
        "anti_starvation_policy_sha256": "contracts/O7_ANTI_STARVATION_POLICY_V1.json",
        "final_report_sha256": "NS_OSA_V1_FINAL_REPORT.md",
        "campaign_qualification_runner_sha256": "qualification/qualify_campaign.py",
        "o7_qualification_runner_sha256": "qualification/qualify_o7.py",
    }
    checks = {
        "all_closure_conjuncts_pass": len(seal["closure"]) == 17 and set(seal["closure"].values()) == {"PASS"},
        "all_component_roots_bound": all(value != "PENDING" and len(value) == 64 for value in seal["roots"].values()),
        "root_receipt_exact": all(root[key] == digest(relative) for key, relative in bindings.items()),
        "zero_scientific_and_protected_access": access["status"] == "PASS_ZERO_SCIENTIFIC_AND_PROTECTED_ACCESS" and all(
            value == 0 for key, value in access.items() if key not in {"schema", "status"}
        ),
        "bootstrap_closed": seal["bootstrap_mode"] == "PERMANENTLY_CLOSED" and root["bootstrap_mode"] == "CLOSED",
        "anti_starvation_enforced": anti_starvation["conceptual_refinement_alone_may_reopen"] is False and len(anti_starvation["reopen_requires"]) == 5,
        "science_authority_not_claimed": seal["scientific_authority"] == "NONE" and seal["population_authority"] == "NONE",
        "historical_fossils_preserved": all("NONCONSUMABLE" in value for value in seal["historical_fossils"].values()),
        "no_pending_placeholders": "PENDING" not in (HERE / "NORTHSTAR_OPERATING_SURFACE_V1_ROOT_RECEIPT.json").read_text(encoding="utf-8"),
    }
    print(json.dumps({"status": "PASS" if all(checks.values()) else "FAILED", "checks": checks}, sort_keys=True))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


CONTRACT = "NORTHSTAR_PHASE13_INFRASTRUCTURE_RETRY_V1"


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def comparable(protocol: dict[str, Any]) -> dict[str, Any]:
    value = json.loads(json.dumps(protocol))
    value.pop("protocol_sha256", None)
    value["identities"].pop("evaluator_code_sha256", None)
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description="Authorize a pre-score infrastructure-only holdout retry")
    parser.add_argument("--previous-protocol", type=Path, required=True)
    parser.add_argument("--replacement-protocol", type=Path, required=True)
    parser.add_argument("--previous-authorization", type=Path, required=True)
    parser.add_argument("--previous-state", type=Path, required=True)
    parser.add_argument("--previous-report", type=Path, required=True)
    parser.add_argument("--reason", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    previous = json.loads(args.previous_protocol.read_text(encoding="utf-8-sig"))
    replacement = json.loads(args.replacement_protocol.read_text(encoding="utf-8-sig"))
    authorization = json.loads(args.previous_authorization.read_text(encoding="utf-8-sig"))
    if comparable(previous) != comparable(replacement):
        raise RuntimeError("replacement changes more than evaluator identity and protocol hash")
    if authorization.get("protocol_sha256") != previous.get("protocol_sha256"):
        raise RuntimeError("previous authorization does not bind previous protocol")
    if args.previous_state.exists() or args.previous_report.exists() or args.previous_report.with_suffix(".abort.json").exists():
        raise RuntimeError("retry forbidden after consumption or report publication")
    if args.output.exists():
        raise RuntimeError("retry amendment output already exists")

    receipt = {
        "contract": CONTRACT,
        "status": "AUTHORIZED_RETRY_BEFORE_SCORING",
        "reason": args.reason,
        "previous_protocol_sha256": previous["protocol_sha256"],
        "replacement_protocol_sha256": replacement["protocol_sha256"],
        "previous_authorization_token_sha256": authorization["authorization_token_sha256"],
        "collection_semantics_unchanged": True,
        "candidate_semantics_unchanged": True,
        "evaluation_policy_unchanged": True,
        "previous_consumption_absent": True,
        "previous_report_absent": True,
        "receipt_semantic_sha256": "",
    }
    receipt["receipt_semantic_sha256"] = digest(
        {key: value for key, value in receipt.items() if key != "receipt_semantic_sha256"}
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"status={receipt['status']}")
    print(f"replacement_protocol_sha256={receipt['replacement_protocol_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

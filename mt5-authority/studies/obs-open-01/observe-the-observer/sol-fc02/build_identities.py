"""Materialize deterministic FC-00 Sol identity data without population access."""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import sys
from pathlib import Path

# Isolated execution removes the script directory from sys.path. Re-add only
# this sealed package directory; no user site or external PYTHONPATH is used.
PACKAGE_DIR = Path(__file__).resolve().parent
if str(PACKAGE_DIR) not in sys.path:
    sys.path.insert(0, str(PACKAGE_DIR))

from common.canonical import load_json, sha256_file, sha256_value, tree_hash
from common.field_registry import derive_registry


def runtime_identity() -> dict:
    executable = Path(sys.executable)
    stdlib = Path(sysconfig_path())
    stdlib_files = list(stdlib.rglob("*.py")) if stdlib.exists() else []
    return {
        "schema": "PYTHON_RUNTIME_IDENTITY_V1",
        "implementation": "CPython",
        "executable": str(executable),
        "executable_sha256": sha256_file(executable),
        "version_string": sys.version,
        "platform_machine": platform.machine(),
        "platform_system": platform.system(),
        "stdlib_root": str(stdlib),
        "stdlib_merkle_root": tree_hash(stdlib_files, stdlib) if stdlib_files else "NOT_EVALUABLE",
    }


def sysconfig_path() -> str:
    import sysconfig
    return sysconfig.get_paths()["stdlib"]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--registry-only", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    package = root / "mt5-authority/studies/obs-open-01/observe-the-observer/sol-fc02"
    g4_elements = load_json(root / "mt5-authority/studies/obs-open-01/sentinel-behavioral-qualification-compound/g4-preservation-surface/seal/contracts/ELEMENT_PRESERVATION_CENSUS_V1.json")
    registry = derive_registry(g4_elements, "G4_ROOT_37ed98b4ebe447ef3c2152e550c99652d0157aea2c77b8a886379d1ba9e08e15")
    if args.registry_only:
        print(json.dumps(registry, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
        return
    contract_dir = package / "contracts"
    contracts = load_json(contract_dir / "arm_contracts.json")
    resource = load_json(contract_dir / "sol_common_resource_policy_v1.json")
    stopping = load_json(contract_dir / "sol_complete_frozen_domain_v1.json")
    retry = load_json(contract_dir / "sol_pre_read_retry_only_v1.json")
    randomness = load_json(contract_dir / "none_v1.json")
    environment = load_json(contract_dir / "sol_execution_environment_contract_v1.json")
    common_source = tree_hash((package / "common").rglob("*.py"), package)
    registry_hash = sha256_value(registry)
    runtime = runtime_identity()
    runtime_hash = sha256_value(runtime)
    arms = []
    for arm in contracts["arms"]:
        source = package / f"{arm['module']}.py"
        question = {"question_id": arm["question_id"], "arm_id": arm["arm_id"]}
        optic = {"optic_id": arm["optic_id"], "arm_id": arm["arm_id"]}
        reference = {"reference_view_id": arm["reference_view_id"], "arm_id": arm["arm_id"]}
        classifier = {"outcome_classifier_id": arm["outcome_classifier_id"], "arm_id": arm["arm_id"]}
        implementation_hash = tree_hash([source, *((package / "common").rglob("*.py"))], package)
        arms.append({
            "arm_id": arm["arm_id"],
            "question_id": arm["question_id"], "question_hash": sha256_value(question),
            "optic_id": arm["optic_id"], "optic_hash": sha256_value(optic),
            "reference_view_id": arm["reference_view_id"], "reference_view_hash": sha256_value(reference),
            "outcome_classifier_id": arm["outcome_classifier_id"], "outcome_classifier_hash": sha256_value(classifier),
            "field_access_profile_id": "SOL_AUTHORIZED_OPTICAL_INPUT_V1", "field_access_profile_hash": sha256_file(contract_dir / "sol_authorized_optical_input_v1.json"),
            "stopping_rule_id": "SOL_COMPLETE_FROZEN_DOMAIN_V1", "stopping_rule_hash": sha256_file(contract_dir / "sol_complete_frozen_domain_v1.json"),
            "resource_policy_id": "SOL_COMMON_RESOURCE_POLICY_V1", "resource_policy_hash": sha256_file(contract_dir / "sol_common_resource_policy_v1.json"),
            "retry_contract_id": "SOL_PRE_READ_RETRY_ONLY_V1", "retry_contract_hash": sha256_file(contract_dir / "sol_pre_read_retry_only_v1.json"),
            "randomness_contract_id": "RANDOMNESS_CONTRACT_NONE_V1", "randomness_contract_hash": sha256_file(contract_dir / "none_v1.json"),
            "implementation_id": arm["implementation_id"], "implementation_hash": implementation_hash,
            "runtime_identity_id": "PYTHON_RUNTIME_IDENTITY_V1", "runtime_identity_hash": runtime_hash,
            "execution_environment_contract_id": "SOL_EXECUTION_ENVIRONMENT_CONTRACT_V1", "execution_environment_contract_hash": sha256_file(contract_dir / "sol_execution_environment_contract_v1.json"),
            "language": "PYTHON_STDLIB_ONLY", "scientific_float_arithmetic": "NONE", "stochastic_search": False,
        })
    output = {
        "schema": "SOL_FC00_ARM_IDENTITY_BUNDLE_V1",
        "g8_root": "556cba86bf75a76d67c411db5a62229cb35ec84c92bd0bfbb6fd086971496ece",
        "question_packet_root": "0d24798eaa552913506fb548364f49c7b5dc418bd1977cfc1a1b05d2c63df6d0",
        "measurement_registry": registry,
        "runtime_identity": runtime,
        "common_source_root": common_source,
        "contracts_root": tree_hash(contract_dir.rglob("*.json"), package),
        "arm_count": len(arms),
        "arms": arms,
        "population_access": 0,
        "result_conditioned_retuning": False,
    }
    print(json.dumps(output, ensure_ascii=False, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

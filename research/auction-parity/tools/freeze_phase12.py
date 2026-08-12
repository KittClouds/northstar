from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

import numpy as np
import pandas as pd


TARGET_ID_DOMAIN = b"MST_RG2_TARGET_ID_SET_V1\0"
REFERENCE_MAGIC = b"NSTAR_MODELREF1\0"


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def id_digest(frame: pd.DataFrame, id_column: str, prefix: str) -> str:
    rows = sorted((str(run), int(identifier)) for run, identifier in zip(frame["run_key"], frame[id_column]))
    digest = hashlib.sha256(TARGET_ID_DOMAIN)
    for run, identifier in rows:
        digest.update(f"{run}\t{prefix}\t{identifier}\n".encode())
    return digest.hexdigest()


def target_registry(corpus, query_spec, targets: tuple[str, ...]) -> dict:
    rows = []
    for target in targets:
        frame = corpus.target_frame(query_spec.create(target)).frame
        id_column, prefix, identity = (
            ("episode_id", "E", "run_key+episode_id")
            if target == "transit_given_episode_acceptance"
            else ("attempt_id", "A", "run_key+attempt_id")
        )
        observed = frame[frame["target_observed"]]
        censored = frame[frame["target_censored"]]
        analyzable = frame[frame["target_analyzable"]]
        rows.append({
            "target": target,
            "identity_kind": identity,
            "eligible_count": len(frame),
            "observed_count": len(observed),
            "censored_count": len(censored),
            "analyzable_count": len(analyzable),
            "eligible_ids_sha256": id_digest(frame, id_column, prefix),
            "observed_ids_sha256": id_digest(observed, id_column, prefix),
            "censored_ids_sha256": id_digest(censored, id_column, prefix),
            "analyzable_ids_sha256": id_digest(analyzable, id_column, prefix),
        })
    return {
        "contract": "MST_RG2_TARGET_ID_REGISTRY_V1",
        "interface_version": "MST_RG2_RESEARCH_INTERFACE_V1_1",
        "source_corpus_sha256": corpus.corpus_hash,
        "targets": rows,
    }


def fhex(value: float) -> str:
    return f"{struct.unpack('<Q', struct.pack('<d', float(value)))[0]:016x}"


def freeze_preprocessor(prep) -> dict:
    return {
        "numeric": list(prep.numeric),
        "categorical": list(prep.categorical),
        "medians": {key: fhex(prep.medians[key]) for key in prep.numeric},
        "means": {key: fhex(prep.means[key]) for key in prep.numeric},
        "scales": {key: fhex(prep.scales[key]) for key in prep.numeric},
        "missing_indicators": sorted(prep.missing_indicators),
        "levels": {key: list(prep.levels[key]) for key in prep.categorical},
        "feature_order": list(prep.names),
        "minimum_category_count": 10,
        "unknown_category_policy": "OTHER_if_encoded_else_reference_all_zero",
    }


def scalar_sigmoid(value: float) -> float:
    if value >= 0:
        return 1.0 / (1.0 + np.exp(-value))
    exp = np.exp(value)
    return float(exp / (1.0 + exp))


def ridge_scores(matrix: np.ndarray, coefficients: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    scores = np.empty(len(matrix), dtype=np.float64)
    probabilities = np.empty(len(matrix), dtype=np.float64)
    for row_index, row in enumerate(matrix):
        score = float(coefficients[0])
        for value, coefficient in zip(row, coefficients[1:]):
            score += float(value) * float(coefficient)
        scores[row_index] = score
        probabilities[row_index] = scalar_sigmoid(score)
    return scores, probabilities


def stump_scores(matrix: np.ndarray, model) -> tuple[np.ndarray, np.ndarray]:
    scores = np.empty(len(matrix), dtype=np.float64)
    probabilities = np.empty(len(matrix), dtype=np.float64)
    for row_index, row in enumerate(matrix):
        score = float(model.base_score)
        for column, threshold, left, right in model.stumps or []:
            score += float(model.learning_rate) * (float(left) if row[column] <= threshold else float(right))
        scores[row_index] = score
        probabilities[row_index] = scalar_sigmoid(score)
    return scores, probabilities


def write_reference(path: Path, matrix: np.ndarray, scores: np.ndarray, probabilities: np.ndarray) -> None:
    with path.open("wb") as stream:
        stream.write(REFERENCE_MAGIC)
        stream.write(struct.pack("<II", len(matrix), matrix.shape[1]))
        for row, score, probability in zip(matrix, scores, probabilities):
            stream.write(struct.pack("<dd", float(score), float(probability)))
            stream.write(np.asarray(row, dtype="<f8").tobytes(order="C"))


def freeze_models(workspace: Path, output: Path, corpus, query_spec, targets, prepare_cohort, models) -> dict:
    protocol_path = workspace / "phase11" / "output" / "candidate_protocol.tsv"
    protocol = pd.read_csv(protocol_path, sep="\t", dtype=str)
    selected = protocol[protocol["status"].eq("FROZEN_RESEARCH_CANDIDATE")]
    feature_schema = {
        "numeric": list(models.NUMERIC_FEATURES) if hasattr(models, "NUMERIC_FEATURES") else [],
        "categorical": list(models.CATEGORICAL_FEATURES) if hasattr(models, "CATEGORICAL_FEATURES") else [],
        "preprocessor": "PHASE11_PREPROCESSOR_V1",
    }
    if not feature_schema["numeric"]:
        from contracts import NUMERIC_FEATURES, CATEGORICAL_FEATURES
        feature_schema["numeric"] = list(NUMERIC_FEATURES)
        feature_schema["categorical"] = list(CATEGORICAL_FEATURES)
    feature_schema_hash = digest_bytes(canonical_json(feature_schema))
    model_rows = []
    for candidate in selected.itertuples(index=False):
        target = str(candidate.target)
        raw = corpus.target_frame(query_spec.create(target))
        prepared = prepare_cohort(raw.frame, target)
        frame = prepared[prepared["target_analyzable"]].copy()
        frame["target_label"] = frame["target_label"].astype("int8")
        prep = models.Preprocessor.fit(frame)
        matrix = prep.transform(frame)
        labels = frame["target_label"].to_numpy(float)
        model_class = str(candidate.selected_model_class)
        if model_class == "RIDGE_LOGISTIC":
            penalty = models.select_penalty(frame)
            model = models.RidgeLogit(penalty).fit(matrix, labels)
            scores, probabilities = ridge_scores(matrix, model.coefficients)
            payload = {
                "kind": "RIDGE_LOGISTIC",
                "penalty": fhex(penalty),
                "coefficients": [fhex(value) for value in model.coefficients],
            }
        elif model_class == "BOOSTED_STUMPS":
            model = models.BoostedStumps().fit(matrix, labels)
            scores, probabilities = stump_scores(matrix, model)
            payload = {
                "kind": "BOOSTED_STUMPS",
                "iterations": model.iterations,
                "learning_rate": fhex(model.learning_rate),
                "leaf_penalty": fhex(model.leaf_penalty),
                "base_score": fhex(model.base_score),
                "stumps": [[int(column), fhex(threshold), fhex(left), fhex(right)] for column, threshold, left, right in model.stumps or []],
            }
        else:
            raise RuntimeError(f"unsupported candidate {model_class}")
        reference_name = f"{target}.reference.f64"
        reference_path = output / reference_name
        write_reference(reference_path, matrix, scores, probabilities)
        ordered_ids = "".join(f"{run}\t{int(attempt)}\n" for run, attempt in zip(frame["run_key"], frame["attempt_id"]))
        artifact = {
            "contract": "MST_PHASE11_FROZEN_MODEL_V1",
            "model_id": f"RG2_PHASE11_{target}_{model_class}_V1",
            "target": target,
            "candidate_protocol_sha256": digest_file(protocol_path),
            "training_corpus_sha256": corpus.corpus_hash,
            "feature_schema_sha256": feature_schema_hash,
            "interface_code_sha256": corpus.interface_code_hash,
            "cohort_query_sha256": raw.receipt["query_spec_sha256"],
            "ordered_observation_ids_sha256": digest_bytes(ordered_ids.encode()),
            "row_count": len(frame),
            "feature_count": matrix.shape[1],
            "preprocessor": freeze_preprocessor(prep),
            "model": payload,
            "calibration": {"kind": "NONE"},
            "output": {"raw": "f64_logit", "probability": "stable_sigmoid_f64"},
            "parity": {"score_absolute_tolerance": 1e-12, "probability_absolute_tolerance": 2e-15},
            "reference_file": reference_name,
            "reference_sha256": digest_file(reference_path),
        }
        artifact["artifact_semantic_sha256"] = digest_bytes(canonical_json(artifact))
        artifact_path = output / f"{target}.model.json"
        artifact_path.write_bytes(json.dumps(artifact, indent=2, sort_keys=True).encode() + b"\n")
        model_rows.append({
            "target": target,
            "model": artifact_path.name,
            "model_sha256": digest_file(artifact_path),
            "reference": reference_name,
            "reference_sha256": artifact["reference_sha256"],
            "rows": len(frame),
            "features": matrix.shape[1],
        })
    return {
        "contract": "MST_PHASE11_FROZEN_MODEL_REGISTRY_V1",
        "status": "PASS",
        "source_corpus_sha256": corpus.corpus_hash,
        "candidate_protocol_sha256": digest_file(protocol_path),
        "feature_schema_sha256": feature_schema_hash,
        "calibration": "NONE",
        "holdout_touched": False,
        "models": model_rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    workspace = args.workspace.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(workspace / "phase11"))
    sys.path.insert(0, str(workspace))
    from phase10_5 import QuerySpec, ResearchCorpus
    from phase11.contracts import TARGETS
    from phase11.statistics import prepare_cohort
    from phase11 import models

    corpus = ResearchCorpus(workspace)
    registry = target_registry(corpus, QuerySpec, TARGETS)
    target_path = output / "target_id_registry.json"
    target_path.write_bytes(json.dumps(registry, indent=2, sort_keys=True).encode() + b"\n")
    models_registry = freeze_models(workspace, output, corpus, QuerySpec, TARGETS, prepare_cohort, models)
    model_path = output / "frozen_model_registry.json"
    model_path.write_bytes(json.dumps(models_registry, indent=2, sort_keys=True).encode() + b"\n")
    receipt = {
        "contract": "MST_PHASE12_PYTHON_FREEZE_RECEIPT_V1",
        "status": "PASS",
        "source_corpus_sha256": corpus.corpus_hash,
        "target_registry_sha256": digest_file(target_path),
        "model_registry_sha256": digest_file(model_path),
        "model_count": len(models_registry["models"]),
        "holdout_touched": False,
    }
    (output / "freeze_receipt.json").write_bytes(json.dumps(receipt, indent=2, sort_keys=True).encode() + b"\n")
    print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()

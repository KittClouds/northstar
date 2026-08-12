//! Exact-model, tolerance-scored parity against frozen Phase 11 candidates.

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use memmap2::MmapOptions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const REFERENCE_MAGIC: &[u8; 16] = b"NSTAR_MODELREF1\0";

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("I/O failure at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("JSON failure at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("model parity contract failed: {0}")]
    Contract(String),
}
type Result<T> = std::result::Result<T, ModelError>;

#[derive(Debug, Deserialize)]
struct Registry {
    contract: String,
    status: String,
    holdout_touched: bool,
    source_corpus_sha256: String,
    models: Vec<RegistryModel>,
}

#[derive(Debug, Deserialize)]
struct RegistryModel {
    target: String,
    model: String,
    model_sha256: String,
    reference: String,
    reference_sha256: String,
    rows: usize,
    features: usize,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    contract: String,
    target: String,
    training_corpus_sha256: String,
    row_count: usize,
    feature_count: usize,
    model: FrozenModel,
    calibration: Calibration,
    parity: Tolerance,
}

#[derive(Debug, Deserialize)]
struct Calibration {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct Tolerance {
    score_absolute_tolerance: f64,
    probability_absolute_tolerance: f64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum FrozenModel {
    #[serde(rename = "RIDGE_LOGISTIC")]
    RidgeLogistic { coefficients: Vec<String> },
    #[serde(rename = "BOOSTED_STUMPS")]
    BoostedStumps {
        learning_rate: String,
        base_score: String,
        stumps: Vec<(usize, String, String, String)>,
    },
}

enum RuntimeModel {
    Ridge(Vec<f64>),
    Stumps {
        rate: f64,
        base: f64,
        trees: Vec<(usize, f64, f64, f64)>,
    },
}

impl RuntimeModel {
    fn score(&self, values: &[f64]) -> f64 {
        match self {
            Self::Ridge(beta) => {
                let mut score = beta[0];
                for (value, coefficient) in values.iter().zip(&beta[1..]) {
                    score += value * coefficient;
                }
                score
            }
            Self::Stumps { rate, base, trees } => {
                let mut score = *base;
                for &(column, threshold, left, right) in trees {
                    score += rate
                        * if values[column] <= threshold {
                            left
                        } else {
                            right
                        };
                }
                score
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ModelParity {
    pub target: String,
    pub model_class: String,
    pub rows: usize,
    pub features: usize,
    pub maximum_score_absolute_error: f64,
    pub maximum_probability_absolute_error: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelParityReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub source_corpus_sha256: String,
    pub holdout_touched: bool,
    pub models: Vec<ModelParity>,
}

pub fn verify_model_registry(path: &Path) -> Result<ModelParityReceipt> {
    verify_model_registry_inner(path)
}

pub fn verify_model_registry_with_sha(
    path: &Path,
    expected_sha256: &str,
) -> Result<ModelParityReceipt> {
    verify_sha(path, expected_sha256)?;
    verify_model_registry_inner(path)
}

fn verify_model_registry_inner(path: &Path) -> Result<ModelParityReceipt> {
    let root = path
        .parent()
        .ok_or_else(|| ModelError::Contract("registry has no parent".into()))?;
    let registry: Registry = read_json(path)?;
    if registry.contract != "MST_PHASE11_FROZEN_MODEL_REGISTRY_V1"
        || registry.status != "PASS"
        || registry.holdout_touched
    {
        return Err(ModelError::Contract(
            "model registry identity or holdout gate failed".into(),
        ));
    }
    let mut reports = Vec::with_capacity(registry.models.len());
    for row in &registry.models {
        let model_path = root.join(&row.model);
        let reference_path = root.join(&row.reference);
        verify_sha(&model_path, &row.model_sha256)?;
        verify_sha(&reference_path, &row.reference_sha256)?;
        let artifact: Artifact = read_json(&model_path)?;
        if artifact.contract != "MST_PHASE11_FROZEN_MODEL_V1"
            || artifact.target != row.target
            || artifact.training_corpus_sha256 != registry.source_corpus_sha256
            || artifact.row_count != row.rows
            || artifact.feature_count != row.features
            || artifact.calibration.kind != "NONE"
        {
            return Err(ModelError::Contract(format!(
                "{} model identity mismatch",
                row.target
            )));
        }
        reports.push(verify_reference(&reference_path, &artifact)?);
    }
    Ok(ModelParityReceipt {
        contract: "NORTHSTAR_PHASE11_MODEL_INFERENCE_PARITY_V1",
        status: "PASS",
        source_corpus_sha256: registry.source_corpus_sha256,
        holdout_touched: false,
        models: reports,
    })
}

fn verify_reference(path: &Path, artifact: &Artifact) -> Result<ModelParity> {
    let file = File::open(path).map_err(|source| ModelError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| ModelError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if mmap.len() < 24 || &mmap[..16] != REFERENCE_MAGIC {
        return Err(ModelError::Contract("reference header mismatch".into()));
    }
    let rows = read_u32(&mmap, 16)? as usize;
    let features = read_u32(&mmap, 20)? as usize;
    let stride = 16_usize
        .checked_add(
            features
                .checked_mul(8)
                .ok_or_else(|| ModelError::Contract("reference stride overflow".into()))?,
        )
        .ok_or_else(|| ModelError::Contract("reference stride overflow".into()))?;
    let expected_len = rows
        .checked_mul(stride)
        .and_then(|value| value.checked_add(24))
        .ok_or_else(|| ModelError::Contract("reference dimensions overflow".into()))?;
    if rows != artifact.row_count
        || features != artifact.feature_count
        || mmap.len() != expected_len
    {
        return Err(ModelError::Contract("reference dimensions mismatch".into()));
    }
    let (class, evaluator) = match &artifact.model {
        FrozenModel::RidgeLogistic { coefficients } => {
            let beta = coefficients
                .iter()
                .map(|value| parse_bits(value))
                .collect::<Result<Vec<_>>>()?;
            if beta.len() != features + 1 {
                return Err(ModelError::Contract(
                    "ridge coefficient width mismatch".into(),
                ));
            }
            ("RIDGE_LOGISTIC", RuntimeModel::Ridge(beta))
        }
        FrozenModel::BoostedStumps {
            learning_rate,
            base_score,
            stumps,
        } => {
            let rate = parse_bits(learning_rate)?;
            let base = parse_bits(base_score)?;
            let trees = stumps
                .iter()
                .map(|(column, threshold, left, right)| {
                    Ok((
                        *column,
                        parse_bits(threshold)?,
                        parse_bits(left)?,
                        parse_bits(right)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            if trees.iter().any(|tree| tree.0 >= features) {
                return Err(ModelError::Contract("stump feature out of range".into()));
            }
            ("BOOSTED_STUMPS", RuntimeModel::Stumps { rate, base, trees })
        }
    };
    let mut max_score = 0_f64;
    let mut max_probability = 0_f64;
    let mut values = vec![0_f64; features];
    for row in 0..rows {
        let base = 24 + row * stride;
        let expected_score = read_f64(&mmap, base)?;
        let expected_probability = read_f64(&mmap, base + 8)?;
        for (column, value) in values.iter_mut().enumerate() {
            *value = read_f64(&mmap, base + 16 + column * 8)?;
        }
        let score = evaluator.score(&values);
        let probability = sigmoid(score);
        if !expected_score.is_finite()
            || !expected_probability.is_finite()
            || !score.is_finite()
            || !probability.is_finite()
        {
            return Err(ModelError::Contract(format!(
                "{} contains non-finite inference values",
                artifact.target
            )));
        }
        max_score = max_score.max((score - expected_score).abs());
        max_probability = max_probability.max((probability - expected_probability).abs());
    }
    if max_score > artifact.parity.score_absolute_tolerance
        || max_probability > artifact.parity.probability_absolute_tolerance
    {
        return Err(ModelError::Contract(format!(
            "{} numerical drift score={max_score:e} probability={max_probability:e}",
            artifact.target
        )));
    }
    Ok(ModelParity {
        target: artifact.target.clone(),
        model_class: class.into(),
        rows,
        features,
        maximum_score_absolute_error: max_score,
        maximum_probability_absolute_error: max_probability,
    })
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}
fn parse_bits(value: &str) -> Result<f64> {
    u64::from_str_radix(value, 16)
        .map(f64::from_bits)
        .map_err(|_| ModelError::Contract("invalid f64 bit pattern".into()))
}
fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    bytes
        .get(offset..offset + 4)
        .and_then(|v| v.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| ModelError::Contract("reference truncated".into()))
}
fn read_f64(bytes: &[u8], offset: usize) -> Result<f64> {
    bytes
        .get(offset..offset + 8)
        .and_then(|v| v.try_into().ok())
        .map(f64::from_le_bytes)
        .ok_or_else(|| ModelError::Contract("reference truncated".into()))
}
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).map_err(|source| ModelError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| ModelError::Json {
        path: path.to_path_buf(),
        source,
    })
}
fn verify_sha(path: &Path, expected: &str) -> Result<()> {
    let bytes = fs::read(path).map_err(|source| ModelError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let actual = hex(&Sha256::digest(bytes));
    if actual != expected {
        return Err(ModelError::Contract(format!(
            "{} SHA-256 mismatch",
            path.display()
        )));
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(H[(b >> 4) as usize] as char);
        out.push(H[(b & 15) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_bit_patterns_and_stable_sigmoid_are_exact() {
        let value = -12.345678901234567_f64;
        let encoded = format!("{:016x}", value.to_bits());
        assert_eq!(parse_bits(&encoded).unwrap().to_bits(), value.to_bits());
        assert_eq!(sigmoid(0.0), 0.5);
        assert!(sigmoid(1_000.0).is_finite());
        assert!(sigmoid(-1_000.0).is_finite());
    }
}

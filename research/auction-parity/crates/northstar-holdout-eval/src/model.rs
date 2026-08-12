use std::{fs, path::Path};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, canonical};

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub contract: String,
    pub status: String,
    pub holdout_touched: bool,
    pub source_corpus_sha256: String,
    pub candidate_protocol_sha256: String,
    pub feature_schema_sha256: String,
    pub models: Vec<RegistryModel>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryModel {
    pub target: String,
    pub model: String,
    pub model_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Artifact {
    pub contract: String,
    pub target: String,
    pub artifact_semantic_sha256: String,
    pub feature_schema_sha256: String,
    pub interface_code_sha256: String,
    pub preprocessor: Preprocessor,
    pub model: FrozenModel,
    pub calibration: Calibration,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Calibration {
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preprocessor {
    pub numeric: Vec<String>,
    pub categorical: Vec<String>,
    pub feature_order: Vec<String>,
    pub levels: HashMap<String, Vec<String>>,
    pub medians: HashMap<String, String>,
    pub means: HashMap<String, String>,
    pub scales: HashMap<String, String>,
    pub missing_indicators: Vec<String>,
    pub unknown_category_policy: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum FrozenModel {
    #[serde(rename = "RIDGE_LOGISTIC")]
    Ridge { coefficients: Vec<String> },
    #[serde(rename = "BOOSTED_STUMPS")]
    Stumps {
        learning_rate: String,
        base_score: String,
        stumps: Vec<(usize, String, String, String)>,
    },
}

#[derive(Clone, Debug)]
pub struct RuntimeArtifact {
    pub artifact: Artifact,
    model: RuntimeModel,
}

#[derive(Clone, Debug)]
enum RuntimeModel {
    Ridge(Box<[f64]>),
    Stumps {
        rate: f64,
        base: f64,
        trees: Box<[(usize, f64, f64, f64)]>,
    },
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        let value: Self = serde_json::from_slice(&bytes)?;
        if value.contract != "MST_PHASE11_FROZEN_MODEL_REGISTRY_V1"
            || value.status != "PASS"
            || value.holdout_touched
        {
            return Err(Error::Contract(
                "frozen model registry is not admissible".into(),
            ));
        }
        Ok(value)
    }
}

impl RuntimeArtifact {
    pub fn load(root: &Path, row: &RegistryModel) -> Result<Self> {
        let path = root.join(&row.model);
        if !canonical::sealed_text_matches(&path, &row.model_sha256)? {
            return Err(Error::Contract(format!(
                "{} canonical model hash mismatch",
                row.target
            )));
        }
        let bytes = fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        let artifact: Artifact = serde_json::from_slice(&bytes)?;
        if artifact.contract != "MST_PHASE11_FROZEN_MODEL_V1"
            || artifact.target != row.target
            || artifact.calibration.kind != "NONE"
            || artifact.preprocessor.unknown_category_policy
                != "OTHER_if_encoded_else_reference_all_zero"
        {
            return Err(Error::Contract(format!(
                "{} artifact contract drift",
                row.target
            )));
        }
        if canonical::json_hash_without(&bytes, "artifact_semantic_sha256")?
            != artifact.artifact_semantic_sha256
        {
            return Err(Error::Contract(format!(
                "{} semantic model hash mismatch",
                row.target
            )));
        }
        let model = match &artifact.model {
            FrozenModel::Ridge { coefficients } => RuntimeModel::Ridge(
                coefficients
                    .iter()
                    .map(|v| parse_bits(v))
                    .collect::<Result<Vec<_>>>()?
                    .into_boxed_slice(),
            ),
            FrozenModel::Stumps {
                learning_rate,
                base_score,
                stumps,
            } => RuntimeModel::Stumps {
                rate: parse_bits(learning_rate)?,
                base: parse_bits(base_score)?,
                trees: stumps
                    .iter()
                    .map(|(c, t, l, r)| Ok((*c, parse_bits(t)?, parse_bits(l)?, parse_bits(r)?)))
                    .collect::<Result<Vec<_>>>()?
                    .into_boxed_slice(),
            },
        };
        let mut expected_order = Vec::with_capacity(artifact.preprocessor.feature_order.len());
        for name in &artifact.preprocessor.numeric {
            expected_order.push(name.clone());
            if artifact.preprocessor.missing_indicators.contains(name) {
                expected_order.push(format!("{name}__MISSING"));
            }
        }
        for name in &artifact.preprocessor.categorical {
            let levels =
                artifact.preprocessor.levels.get(name).ok_or_else(|| {
                    Error::Contract(format!("missing category levels for {name}"))
                })?;
            expected_order.extend(levels.iter().map(|level| format!("{name}=={level}")));
        }
        if expected_order != artifact.preprocessor.feature_order {
            return Err(Error::Contract(format!(
                "{} preprocessor feature ordering drift",
                row.target
            )));
        }
        let width = artifact.preprocessor.feature_order.len();
        match &model {
            RuntimeModel::Ridge(beta) if beta.len() != width + 1 => {
                return Err(Error::Contract("ridge width mismatch".into()));
            }
            RuntimeModel::Stumps { trees, .. } if trees.iter().any(|v| v.0 >= width) => {
                return Err(Error::Contract("stump width mismatch".into()));
            }
            _ => {}
        }
        Ok(Self { artifact, model })
    }

    pub fn raw_columns(&self) -> impl Iterator<Item = &str> {
        self.artifact
            .preprocessor
            .numeric
            .iter()
            .chain(&self.artifact.preprocessor.categorical)
            .map(String::as_str)
    }

    pub fn score(&self, raw: &HashMap<String, String>, scratch: &mut Vec<f64>) -> Result<f64> {
        let p = &self.artifact.preprocessor;
        scratch.clear();
        scratch.reserve(p.feature_order.len().saturating_sub(scratch.capacity()));
        for name in &p.numeric {
            let text = raw.get(name).map(String::as_str).unwrap_or("\\N");
            let missing = text.is_empty() || text == "\\N";
            let median = bits_for(&p.medians, name)?;
            let value = if missing {
                median
            } else {
                text.parse::<f64>()
                    .map_err(|_| Error::Input(format!("invalid numeric {name}={text}")))?
            };
            let mean = bits_for(&p.means, name)?;
            let scale = bits_for(&p.scales, name)?;
            if !value.is_finite() || !mean.is_finite() || !scale.is_finite() || scale <= 0.0 {
                return Err(Error::Input(format!(
                    "non-finite preprocessor value for {name}"
                )));
            }
            scratch.push((value - mean) / scale);
            if p.missing_indicators.contains(name) {
                scratch.push(f64::from(missing));
            }
        }
        for name in &p.categorical {
            let raw_value = raw.get(name).map(String::as_str).unwrap_or("\\N");
            let levels = p
                .levels
                .get(name)
                .ok_or_else(|| Error::Contract(format!("missing levels for {name}")))?;
            let encoded = if levels.iter().any(|v| v == raw_value) {
                raw_value
            } else if levels.iter().any(|v| v == "<OTHER>") {
                "<OTHER>"
            } else {
                ""
            };
            scratch.extend(levels.iter().map(|level| f64::from(level == encoded)));
        }
        if scratch.len() != p.feature_order.len() {
            return Err(Error::Contract(format!(
                "preprocessor width {} != {}",
                scratch.len(),
                p.feature_order.len()
            )));
        }
        let score = match &self.model {
            RuntimeModel::Ridge(beta) => {
                beta[0]
                    + scratch
                        .iter()
                        .zip(&beta[1..])
                        .map(|(x, b)| x * b)
                        .sum::<f64>()
            }
            RuntimeModel::Stumps { rate, base, trees } => {
                *base
                    + trees
                        .iter()
                        .map(|(c, t, l, r)| rate * if scratch[*c] <= *t { l } else { r })
                        .sum::<f64>()
            }
        };
        if !score.is_finite() {
            return Err(Error::Input("non-finite model score".into()));
        }
        Ok(sigmoid(score))
    }
}

fn bits_for(map: &HashMap<String, String>, key: &str) -> Result<f64> {
    parse_bits(
        map.get(key)
            .ok_or_else(|| Error::Contract(format!("missing preprocessor parameter {key}")))?,
    )
}

fn parse_bits(value: &str) -> Result<f64> {
    u64::from_str_radix(value, 16)
        .map(f64::from_bits)
        .map_err(|_| Error::Contract("invalid f64 bit pattern".into()))
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let e = value.exp();
        e / (1.0 + e)
    }
}

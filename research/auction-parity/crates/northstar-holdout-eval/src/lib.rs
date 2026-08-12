//! Fail-closed Phase 13 holdout preauthorization and one-shot evaluator.

mod canonical;
mod engine;
mod input;
mod manifest;
mod metrics;
mod model;

use std::path::PathBuf;

use thiserror::Error;

pub use engine::{
    AuthorizeArgs, EvaluateArgs, FreezeArgs, authorize_once, evaluate_once, freeze_protocol,
};
pub use manifest::{Authorization, Preauthorization};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O failure at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("JSON failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("contract failure: {0}")]
    Contract(String),
    #[error("input failure: {0}")]
    Input(String),
}

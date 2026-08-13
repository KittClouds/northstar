use std::{collections::BTreeSet, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{Error, Result, canonical};

pub const PREAUTH_CONTRACT: &str = "NORTHSTAR_RG2_HOLDOUT_PREAUTHORIZATION_V1";
pub const AUTH_CONTRACT: &str = "NORTHSTAR_RG2_HOLDOUT_AUTHORIZATION_V1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preauthorization {
    pub contract: String,
    pub status: String,
    pub holdout_authorized: bool,
    pub research_generation: u32,
    pub identities: Identities,
    pub candidates: Vec<Candidate>,
    pub reservations: Vec<Reservation>,
    pub evaluation: EvaluationPolicy,
    pub protocol_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Identities {
    pub source_corpus_sha256: String,
    pub packed_corpus_semantic_sha256: String,
    pub phase11_seal_sha256: String,
    pub candidate_protocol_sha256: String,
    pub model_registry_sha256: String,
    pub reservation_manifest_sha256: String,
    pub feature_schema_sha256: String,
    pub research_interface_sha256: String,
    pub evaluator_code_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Candidate {
    pub target: String,
    pub model_class: String,
    pub model_file: String,
    pub model_canonical_sha256: String,
    pub model_semantic_sha256: String,
    pub eligibility: String,
    pub development: DevelopmentReference,
    pub gates: CandidateGates,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DevelopmentReference {
    pub prevalence: f64,
    pub forward_auc: f64,
    pub forward_average_precision: f64,
    pub forward_brier: f64,
    pub forward_log_loss: f64,
    pub forward_ece: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CandidateGates {
    pub minimum_eligible: usize,
    pub minimum_analyzable: usize,
    pub minimum_positive: usize,
    pub minimum_negative: usize,
    pub minimum_runs: usize,
    pub maximum_censoring_rate: f64,
    pub minimum_auc: f64,
    pub minimum_auc_delta: f64,
    pub maximum_log_loss_delta: f64,
    pub maximum_brier_delta: f64,
    pub minimum_calibration_intercept: f64,
    pub maximum_calibration_intercept: f64,
    pub minimum_calibration_slope: f64,
    pub maximum_calibration_slope: f64,
    pub maximum_ece: f64,
    pub require_positive_primary_delta_each_calendar_block: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reservation {
    pub canonical_instrument: String,
    pub broker_symbol: String,
    pub data_source_id: String,
    pub holdout_id: String,
    pub window_start: i64,
    pub window_end_exclusive: i64,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EvaluationPolicy {
    pub primary_metric: String,
    pub baseline: String,
    pub censoring: String,
    pub probability_clip: f64,
    pub reliability_boundaries: Vec<f64>,
    pub bootstrap: BootstrapPolicy,
    pub multiplicity: MultiplicityPolicy,
    pub one_shot: OneShotPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BootstrapPolicy {
    pub unit: String,
    pub seed: u64,
    pub repetitions: usize,
    pub confidence_level: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiplicityPolicy {
    pub method: String,
    pub family_alpha: f64,
    pub candidate_family: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OneShotPolicy {
    pub expected_reservations: usize,
    pub infrastructure_failure_status: String,
    pub consumed_before_scoring: bool,
    pub atomic_report_publication: bool,
    pub partial_results_forbidden: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Authorization {
    pub contract: String,
    pub status: String,
    pub holdout_authorized: bool,
    pub protocol_sha256: String,
    pub authorization_token_sha256: String,
}

impl Preauthorization {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let value: Self = canonical::parse_json(&bytes)?;
        value.validate_semantics()?;
        let actual = canonical::json_hash_without(&bytes, "protocol_sha256")?;
        if actual != value.protocol_sha256 {
            return Err(Error::Contract(format!(
                "protocol hash mismatch expected={} actual={actual}",
                value.protocol_sha256
            )));
        }
        Ok(value)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_semantics()?;
        let actual = canonical::canonical_hash_without(self, "protocol_sha256")?;
        if actual != self.protocol_sha256 {
            return Err(Error::Contract(format!(
                "protocol hash mismatch expected={} actual={actual}",
                self.protocol_sha256
            )));
        }
        Ok(())
    }

    fn validate_semantics(&self) -> Result<()> {
        if self.contract != PREAUTH_CONTRACT
            || self.status != "PREAUTHORIZED_NOT_AUTHORIZED"
            || self.holdout_authorized
        {
            return Err(Error::Contract(
                "preauthorization state is not fail-closed".into(),
            ));
        }
        if self.candidates.is_empty()
            || self.reservations.len() != self.evaluation.one_shot.expected_reservations
        {
            return Err(Error::Contract(
                "candidate or reservation cardinality mismatch".into(),
            ));
        }
        if self.evaluation.primary_metric
            != "PAIRED_LOG_LOSS_IMPROVEMENT_VS_FROZEN_DEVELOPMENT_PREVALENCE"
            || self.evaluation.baseline != "FROZEN_DEVELOPMENT_PREVALENCE"
            || self.evaluation.censoring != "EXCLUDE_CENSORED_FROM_SCORE_RETAIN_IN_DENOMINATORS"
            || self.evaluation.bootstrap.unit != "RUN_CLUSTER"
            || self.evaluation.multiplicity.method != "HOLM_ONE_SIDED_PRIMARY_P"
        {
            return Err(Error::Contract("evaluation semantics drift".into()));
        }
        if self.evaluation.reliability_boundaries.first() != Some(&0.0)
            || self.evaluation.reliability_boundaries.last() != Some(&1.0)
            || self
                .evaluation
                .reliability_boundaries
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::Contract("invalid reliability boundaries".into()));
        }
        let targets = self
            .candidates
            .iter()
            .map(|row| row.target.as_str())
            .collect::<BTreeSet<_>>();
        let family = self
            .evaluation
            .multiplicity
            .candidate_family
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if targets != family {
            return Err(Error::Contract(
                "multiplicity family differs from candidate set".into(),
            ));
        }
        let reservations = self
            .reservations
            .iter()
            .map(|row| (&row.canonical_instrument, &row.holdout_id))
            .collect::<BTreeSet<_>>();
        if reservations.len() != self.reservations.len()
            || self.reservations.iter().any(|row| {
                row.status != "RESERVED_UNTOUCHED" || row.window_start >= row.window_end_exclusive
            })
        {
            return Err(Error::Contract("reservation allowlist is invalid".into()));
        }
        Ok(())
    }
}

impl Authorization {
    pub fn validate(&self, protocol: &Preauthorization) -> Result<()> {
        if self.contract != AUTH_CONTRACT
            || self.status != "AUTHORIZED_UNCONSUMED"
            || !self.holdout_authorized
            || self.protocol_sha256 != protocol.protocol_sha256
        {
            return Err(Error::Contract(
                "authorization does not bind the frozen protocol".into(),
            ));
        }
        if canonical::canonical_hash_without(self, "authorization_token_sha256")?
            != self.authorization_token_sha256
        {
            return Err(Error::Contract("authorization token hash mismatch".into()));
        }
        Ok(())
    }
}

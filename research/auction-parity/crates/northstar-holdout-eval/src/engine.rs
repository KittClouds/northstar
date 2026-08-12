use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use hashbrown::HashSet;
use serde::Serialize;
use serde_json::Value;

use crate::{
    Error, Result, canonical,
    input::{self, BundleManifest},
    manifest::{
        Authorization, BootstrapPolicy, Candidate, CandidateGates, DevelopmentReference,
        EvaluationPolicy, Identities, MultiplicityPolicy, OneShotPolicy, PREAUTH_CONTRACT,
        Preauthorization, Reservation,
    },
    metrics::{self, BootstrapResult, Breakdown, Metrics, ReliabilityBucket, ScoredRow},
    model::{Registry, RuntimeArtifact},
};

#[derive(Clone, Debug)]
pub struct FreezeArgs {
    pub model_registry: PathBuf,
    pub candidate_protocol: PathBuf,
    pub model_metrics: PathBuf,
    pub nonlinear_metrics: PathBuf,
    pub phase11_seal: PathBuf,
    pub reservations: PathBuf,
    pub packed_receipt: PathBuf,
    pub output_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct EvaluateArgs {
    pub protocol: PathBuf,
    pub authorization: PathBuf,
    pub bundle: PathBuf,
    pub state_dir: PathBuf,
    pub report: PathBuf,
}

#[derive(Clone, Debug)]
pub struct AuthorizeArgs {
    pub protocol: PathBuf,
    pub output: PathBuf,
    pub confirmation: String,
}

#[derive(Debug, Serialize)]
struct EvaluationReport {
    contract: &'static str,
    status: &'static str,
    protocol_sha256: String,
    authorization_token_sha256: String,
    bundle_semantic_sha256: String,
    candidates: Vec<CandidateReport>,
    report_semantic_sha256: String,
}

#[derive(Debug, Serialize)]
struct CandidateReport {
    target: String,
    outcome: String,
    failure_reasons: Vec<String>,
    eligible_n: usize,
    censored_n: usize,
    censoring_rate: f64,
    runs: usize,
    episodes: usize,
    metrics: Metrics,
    development_auc: f64,
    development_log_loss: f64,
    development_brier: f64,
    auc_delta: Option<f64>,
    log_loss_delta: f64,
    brier_delta: f64,
    bootstrap: BootstrapResult,
    holm_adjusted_primary_p: f64,
    reliability: Vec<ReliabilityBucket>,
    instruments: Vec<Breakdown>,
    temporal_blocks: Vec<Breakdown>,
}

#[derive(Debug, Serialize)]
struct AbortReceipt {
    contract: &'static str,
    status: &'static str,
    protocol_sha256: String,
    authorization_token_sha256: String,
    reason: String,
}

pub fn freeze_protocol(args: &FreezeArgs) -> Result<Preauthorization> {
    fs::create_dir_all(&args.output_dir).map_err(|source| Error::Io {
        path: args.output_dir.clone(),
        source,
    })?;
    let registry = Registry::load(&args.model_registry)?;
    let protocol_rows = read_tsv(&args.candidate_protocol)?;
    let linear = read_tsv(&args.model_metrics)?;
    let nonlinear = read_tsv(&args.nonlinear_metrics)?;
    let reservations = parse_reservations(&args.reservations)?;
    let model_root = args
        .model_registry
        .parent()
        .ok_or_else(|| Error::Contract("registry parent missing".into()))?;
    let model_output = args.output_dir.join("models");
    fs::create_dir_all(&model_output).map_err(|source| Error::Io {
        path: model_output.clone(),
        source,
    })?;
    let mut candidates = Vec::new();
    let mut interface_hash = None;
    for row in protocol_rows
        .iter()
        .filter(|r| get(r, "status") == "FROZEN_RESEARCH_CANDIDATE")
    {
        let target = get(row, "target");
        let class = get(row, "selected_model_class");
        let registry_row = registry
            .models
            .iter()
            .find(|r| r.target == target)
            .ok_or_else(|| Error::Contract(format!("registry lacks {target}")))?;
        let artifact = RuntimeArtifact::load(model_root, registry_row)?;
        if artifact.artifact.feature_schema_sha256 != registry.feature_schema_sha256 {
            return Err(Error::Contract("feature schema mismatch".into()));
        }
        if interface_hash.get_or_insert(artifact.artifact.interface_code_sha256.clone())
            != &artifact.artifact.interface_code_sha256
        {
            return Err(Error::Contract(
                "research interface differs across models".into(),
            ));
        }
        let metrics_source = if class == "BOOSTED_STUMPS" {
            &nonlinear
        } else {
            &linear
        };
        let metrics_row = metrics_source
            .iter()
            .find(|r| {
                get(r, "target") == target
                    && get(r, "split_type") == "FORWARD_TIME"
                    && get(r, "model") == class
            })
            .ok_or_else(|| Error::Contract(format!("forward metrics lack {target}/{class}")))?;
        let destination = model_output.join(&registry_row.model);
        fs::copy(model_root.join(&registry_row.model), &destination).map_err(|source| {
            Error::Io {
                path: destination.clone(),
                source,
            }
        })?;
        candidates.push(Candidate {
            target: target.into(),
            model_class: class.into(),
            model_file: format!("models/{}", registry_row.model),
            model_canonical_sha256: registry_row.model_sha256.clone(),
            model_semantic_sha256: artifact.artifact.artifact_semantic_sha256.clone(),
            eligibility: get(row, "evaluation_unit").into(),
            development: DevelopmentReference {
                prevalence: number(metrics_row, "prevalence")?,
                forward_auc: number(metrics_row, "auc")?,
                forward_average_precision: number(metrics_row, "average_precision")?,
                forward_brier: number(metrics_row, "brier")?,
                forward_log_loss: number(metrics_row, "log_loss")?,
                forward_ece: number(metrics_row, "ece_10")?,
            },
            gates: gates_for(target)?,
        });
    }
    candidates.sort_by(|a, b| a.target.cmp(&b.target));
    let packed: Value = serde_json::from_slice(&read(&args.packed_receipt)?)?;
    let packed_hash = packed
        .get("packed_semantic_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Contract("packed receipt lacks semantic hash".into()))?;
    let code_hash = evaluator_code_hash(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))?;
    let identities = Identities {
        source_corpus_sha256: registry.source_corpus_sha256.clone(),
        packed_corpus_semantic_sha256: packed_hash.into(),
        phase11_seal_sha256: canonical::canonical_text_sha256(&args.phase11_seal)?,
        candidate_protocol_sha256: registry.candidate_protocol_sha256.clone(),
        model_registry_sha256: canonical::canonical_text_sha256(&args.model_registry)?,
        reservation_manifest_sha256: canonical::canonical_text_sha256(&args.reservations)?,
        feature_schema_sha256: registry.feature_schema_sha256.clone(),
        research_interface_sha256: interface_hash
            .ok_or_else(|| Error::Contract("no frozen candidates".into()))?,
        evaluator_code_sha256: code_hash,
    };
    if !canonical::sealed_text_matches(
        &args.candidate_protocol,
        &registry.candidate_protocol_sha256,
    )? {
        return Err(Error::Contract(
            "candidate protocol content differs from registry seal".into(),
        ));
    }
    let family = candidates.iter().map(|c| c.target.clone()).collect();
    let mut protocol = Preauthorization {
        contract: PREAUTH_CONTRACT.into(),
        status: "PREAUTHORIZED_NOT_AUTHORIZED".into(),
        holdout_authorized: false,
        research_generation: 2,
        identities,
        candidates,
        reservations,
        evaluation: EvaluationPolicy {
            primary_metric: "PAIRED_LOG_LOSS_IMPROVEMENT_VS_FROZEN_DEVELOPMENT_PREVALENCE".into(),
            baseline: "FROZEN_DEVELOPMENT_PREVALENCE".into(),
            censoring: "EXCLUDE_CENSORED_FROM_SCORE_RETAIN_IN_DENOMINATORS".into(),
            probability_clip: 1e-15,
            reliability_boundaries: (0..=10).map(|v| v as f64 / 10.0).collect(),
            bootstrap: BootstrapPolicy {
                unit: "RUN_CLUSTER".into(),
                seed: 117_011,
                repetitions: 10_000,
                confidence_level: 0.95,
            },
            multiplicity: MultiplicityPolicy {
                method: "HOLM_ONE_SIDED_PRIMARY_P".into(),
                family_alpha: 0.05,
                candidate_family: family,
            },
            one_shot: OneShotPolicy {
                expected_reservations: 12,
                infrastructure_failure_status: "ABORT_NO_CANDIDATE_REPORT".into(),
                consumed_before_scoring: true,
                atomic_report_publication: true,
                partial_results_forbidden: true,
            },
        },
        protocol_sha256: String::new(),
    };
    protocol.protocol_sha256 = canonical::canonical_hash_without(&protocol, "protocol_sha256")?;
    protocol.validate()?;
    canonical::write_pretty_json(&args.output_dir.join("preauthorization.json"), &protocol)?;
    write_readme(&args.output_dir, &protocol)?;
    Ok(protocol)
}

pub fn authorize_once(args: &AuthorizeArgs) -> Result<Authorization> {
    if args.confirmation != "AUTHORIZE_ONE_SHOT_RG2_HOLDOUT" {
        return Err(Error::Contract(
            "explicit one-shot confirmation is absent".into(),
        ));
    }
    let protocol = Preauthorization::load(&args.protocol)?;
    let mut authorization = Authorization {
        contract: crate::manifest::AUTH_CONTRACT.into(),
        status: "AUTHORIZED_UNCONSUMED".into(),
        holdout_authorized: true,
        protocol_sha256: protocol.protocol_sha256.clone(),
        authorization_token_sha256: String::new(),
    };
    authorization.authorization_token_sha256 =
        canonical::canonical_hash_without(&authorization, "authorization_token_sha256")?;
    authorization.validate(&protocol)?;
    let mut bytes = serde_json::to_vec_pretty(&authorization)?;
    bytes.push(b'\n');
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    use std::io::Write;
    let mut file = options.open(&args.output).map_err(|source| Error::Io {
        path: args.output.clone(),
        source,
    })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| Error::Io {
            path: args.output.clone(),
            source,
        })?;
    Ok(authorization)
}

pub fn evaluate_once(args: &EvaluateArgs) -> Result<()> {
    let protocol = Preauthorization::load(&args.protocol)?;
    let running_code_hash = evaluator_code_hash(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))?;
    if running_code_hash != protocol.identities.evaluator_code_sha256 {
        return Err(Error::Contract(
            "running evaluator code differs from preauthorized evaluator".into(),
        ));
    }
    let auth: Authorization = serde_json::from_slice(&read(&args.authorization)?)?;
    auth.validate(&protocol)?;
    let bundle_root = args
        .bundle
        .parent()
        .ok_or_else(|| Error::Contract("bundle parent missing".into()))?;
    let bundle = BundleManifest::load_and_validate(&args.bundle, &protocol)?;
    consume(&args.state_dir, &protocol, &auth, &bundle)?;
    let protocol_root = args
        .protocol
        .parent()
        .ok_or_else(|| Error::Contract("protocol parent missing".into()))?;
    let result = (|| {
        verify_bundle_files(bundle_root, &bundle)?;
        evaluate_consumed(&protocol, &auth, &bundle, bundle_root, protocol_root)
    })();
    match result {
        Ok(mut report) => {
            report.report_semantic_sha256 =
                canonical::canonical_hash_without(&report, "report_semantic_sha256")?;
            atomic_json(&args.report, &report)
        }
        Err(error) => {
            let abort = AbortReceipt {
                contract: "NORTHSTAR_RG2_HOLDOUT_ABORT_V1",
                status: "ABORT",
                protocol_sha256: protocol.protocol_sha256,
                authorization_token_sha256: auth.authorization_token_sha256,
                reason: error.to_string(),
            };
            let abort_path = args.report.with_extension("abort.json");
            atomic_json(&abort_path, &abort)?;
            Err(error)
        }
    }
}

fn evaluate_consumed(
    protocol: &Preauthorization,
    auth: &Authorization,
    bundle: &BundleManifest,
    root: &Path,
    protocol_root: &Path,
) -> Result<EvaluationReport> {
    let mut pending = Vec::with_capacity(protocol.candidates.len());
    for candidate in &protocol.candidates {
        let model_path = protocol_root.join(&candidate.model_file);
        let registry_row = crate::model::RegistryModel {
            target: candidate.target.clone(),
            model: model_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            model_sha256: candidate.model_canonical_sha256.clone(),
        };
        let artifact = RuntimeArtifact::load(model_path.parent().unwrap(), &registry_row)?;
        let target = bundle
            .targets
            .iter()
            .find(|t| t.target == candidate.target)
            .ok_or_else(|| Error::Contract("target file absent".into()))?;
        let observations = input::load_observations(
            root,
            target,
            artifact.raw_columns().map(str::to_owned),
            &bundle.runs,
        )?;
        let eligible = observations.len();
        let censored = observations.iter().filter(|o| o.censored).count();
        let episodes = observations
            .iter()
            .map(|o| (&o.run_key, &o.episode_id))
            .collect::<HashSet<_>>()
            .len();
        let mut scratch = Vec::new();
        let mut scored = Vec::with_capacity(eligible - censored);
        for observation in observations.iter().filter(|o| !o.censored) {
            scored.push(ScoredRow {
                run_key: observation.run_key.clone(),
                instrument: observation.instrument.clone(),
                holdout_id: observation.holdout_id.clone(),
                label: observation.label.expect("uncensored labels validated"),
                probability: artifact.score(&observation.raw, &mut scratch)?,
            });
        }
        let run_count = scored
            .iter()
            .map(|r| r.run_key.as_str())
            .collect::<HashSet<_>>()
            .len();
        let (metrics, reliability, bootstrap, instruments, temporal_blocks) = if scored.is_empty() {
            (
                empty_metrics(),
                Vec::new(),
                empty_bootstrap(),
                Vec::new(),
                Vec::new(),
            )
        } else {
            let (calculated, reliability) = metrics::calculate(
                &scored,
                candidate.development.prevalence,
                &protocol.evaluation,
            )?;
            let bootstrap = if run_count >= 2 {
                metrics::bootstrap(
                    &scored,
                    candidate.development.prevalence,
                    &protocol.evaluation.bootstrap,
                    protocol.evaluation.probability_clip,
                )?
            } else {
                empty_bootstrap()
            };
            let instruments = metrics::breakdowns(
                &scored,
                candidate.development.prevalence,
                &protocol.evaluation,
                false,
            )?;
            let temporal_blocks = metrics::breakdowns(
                &scored,
                candidate.development.prevalence,
                &protocol.evaluation,
                true,
            )?;
            (
                calculated,
                reliability,
                bootstrap,
                instruments,
                temporal_blocks,
            )
        };
        pending.push((
            candidate,
            eligible,
            censored,
            episodes,
            run_count,
            metrics,
            reliability,
            bootstrap,
            instruments,
            temporal_blocks,
        ));
    }
    let adjusted = metrics::holm_adjust(
        &pending
            .iter()
            .map(|v| (v.0.target.clone(), v.7.one_sided_p))
            .collect::<Vec<_>>(),
    );
    let mut reports = Vec::with_capacity(pending.len());
    for (
        candidate,
        eligible,
        censored,
        episodes,
        runs,
        metric,
        reliability,
        bootstrap,
        instruments,
        blocks,
    ) in pending
    {
        let adjusted_p = adjusted[&candidate.target];
        let (outcome, failures) = metrics::candidate_gate(
            candidate,
            &metrics::GateEvidence {
                eligible,
                censored,
                run_count: runs,
                metrics: &metric,
                bootstrap: &bootstrap,
                blocks: &blocks,
                adjusted_p,
                alpha: protocol.evaluation.multiplicity.family_alpha,
            },
        );
        reports.push(CandidateReport {
            target: candidate.target.clone(),
            outcome: outcome.into(),
            failure_reasons: failures,
            eligible_n: eligible,
            censored_n: censored,
            censoring_rate: if eligible == 0 {
                1.0
            } else {
                censored as f64 / eligible as f64
            },
            runs,
            episodes,
            auc_delta: metric.auc.map(|v| v - candidate.development.forward_auc),
            log_loss_delta: metric.log_loss - candidate.development.forward_log_loss,
            brier_delta: metric.brier - candidate.development.forward_brier,
            development_auc: candidate.development.forward_auc,
            development_log_loss: candidate.development.forward_log_loss,
            development_brier: candidate.development.forward_brier,
            metrics: metric,
            bootstrap,
            holm_adjusted_primary_p: adjusted_p,
            reliability,
            instruments,
            temporal_blocks: blocks,
        });
    }
    Ok(EvaluationReport {
        contract: "NORTHSTAR_RG2_HOLDOUT_EVALUATION_V1",
        status: "COMPLETE",
        protocol_sha256: protocol.protocol_sha256.clone(),
        authorization_token_sha256: auth.authorization_token_sha256.clone(),
        bundle_semantic_sha256: bundle.bundle_semantic_sha256.clone(),
        candidates: reports,
        report_semantic_sha256: String::new(),
    })
}

fn empty_metrics() -> Metrics {
    Metrics {
        n: 0,
        positives: 0,
        negatives: 0,
        prevalence: 0.0,
        auc: None,
        average_precision: None,
        brier: 0.0,
        log_loss: 0.0,
        ece: 0.0,
        calibration_intercept: None,
        calibration_slope: None,
    }
}

fn empty_bootstrap() -> BootstrapResult {
    BootstrapResult {
        point_delta: 0.0,
        lower: 0.0,
        upper: 0.0,
        one_sided_p: 1.0,
    }
}

fn consume(
    state: &Path,
    protocol: &Preauthorization,
    auth: &Authorization,
    bundle: &BundleManifest,
) -> Result<()> {
    let parent = state
        .parent()
        .ok_or_else(|| Error::Contract("state parent missing".into()))?;
    fs::create_dir_all(parent).map_err(|source| Error::Io {
        path: parent.into(),
        source,
    })?;
    fs::create_dir(state).map_err(|source| Error::Io {
        path: state.into(),
        source,
    })?;
    let receipt = serde_json::json!({"contract":"NORTHSTAR_RG2_HOLDOUT_CONSUMPTION_V1","status":"CONSUMED_BEFORE_SCORING","protocol_sha256":protocol.protocol_sha256,"authorization_token_sha256":auth.authorization_token_sha256,"bundle_semantic_sha256":bundle.bundle_semantic_sha256});
    canonical::write_pretty_json(&state.join("consumed.json"), &receipt)
}

fn verify_bundle_files(root: &Path, bundle: &BundleManifest) -> Result<()> {
    for run in &bundle.runs {
        let path = input::safe_join(root, &run.run_receipt_file)?;
        if canonical::file_sha256(&path)? != run.run_receipt_sha256 {
            return Err(Error::Contract("run receipt hash mismatch".into()));
        }
        let receipt: serde_json::Value = serde_json::from_slice(&read(&path)?)?;
        if receipt.get("run_key").and_then(serde_json::Value::as_str) != Some(run.run_key.as_str())
        {
            return Err(Error::Contract(
                "run receipt does not bind the declared run key".into(),
            ));
        }
    }
    for target in &bundle.targets {
        let path = input::safe_join(root, &target.file)?;
        if canonical::canonical_text_sha256(&path)? != target.canonical_sha256 {
            return Err(Error::Contract("bundle file hash mismatch".into()));
        }
    }
    Ok(())
}

fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Contract("output parent missing".into()))?;
    fs::create_dir_all(parent).map_err(|source| Error::Io {
        path: parent.into(),
        source,
    })?;
    let temp = path.with_extension("tmp");
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    use std::io::Write;
    let mut file = options.open(&temp).map_err(|source| Error::Io {
        path: temp.clone(),
        source,
    })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| Error::Io {
            path: temp.clone(),
            source,
        })?;
    fs::rename(&temp, path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })
}

fn parse_reservations(path: &Path) -> Result<Vec<Reservation>> {
    let mut output = read_tsv(path)?
        .into_iter()
        .map(|r| {
            Ok(Reservation {
                canonical_instrument: get(&r, "canonical_instrument").into(),
                broker_symbol: get(&r, "broker_symbol").into(),
                data_source_id: get(&r, "data_source_id").into(),
                holdout_id: get(&r, "holdout_id").into(),
                window_start: integer(&r, "window_start")?,
                window_end_exclusive: integer(&r, "window_end_exclusive")?,
                status: get(&r, "status").into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    output.sort_by(|a, b| {
        (&a.canonical_instrument, &a.holdout_id).cmp(&(&b.canonical_instrument, &b.holdout_id))
    });
    Ok(output)
}

fn gates_for(target: &str) -> Result<CandidateGates> {
    let (eligible, analyzable, positive, negative, runs, censor, auc, auc_delta) = match target {
        "reclaim_given_break" => (150, 100, 25, 25, 8, 0.40, 0.52, -0.10),
        "initial_acceptance_given_initial_contact" => (300, 250, 30, 100, 8, 0.30, 0.65, -0.10),
        "rejection_given_initial_contact" => (300, 250, 100, 50, 8, 0.30, 0.75, -0.10),
        "retest_hold_given_retest_contact" => (100, 80, 20, 20, 8, 0.20, 0.70, -0.12),
        _ => {
            return Err(Error::Contract(format!(
                "no preauthorized gates for {target}"
            )));
        }
    };
    Ok(CandidateGates {
        minimum_eligible: eligible,
        minimum_analyzable: analyzable,
        minimum_positive: positive,
        minimum_negative: negative,
        minimum_runs: runs,
        maximum_censoring_rate: censor,
        minimum_auc: auc,
        minimum_auc_delta: auc_delta,
        maximum_log_loss_delta: 0.075,
        maximum_brier_delta: 0.04,
        minimum_calibration_intercept: -0.75,
        maximum_calibration_intercept: 0.75,
        minimum_calibration_slope: 0.5,
        maximum_calibration_slope: 1.5,
        maximum_ece: 0.15,
        require_positive_primary_delta_each_calendar_block: true,
    })
}

fn evaluator_code_hash(root: PathBuf) -> Result<String> {
    let mut files = fs::read_dir(&root)
        .map_err(|source| Error::Io {
            path: root.clone(),
            source,
        })?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|v| v == "rs"))
        .collect::<Vec<_>>();
    files.sort();
    let mut bytes = Vec::new();
    for path in files {
        bytes.extend_from_slice(path.file_name().unwrap().to_string_lossy().as_bytes());
        bytes.push(0);
        let raw = read(&path)?;
        bytes.extend(raw.into_iter().filter(|b| *b != b'\r'));
        bytes.push(0);
    }
    Ok(canonical::sha256(&bytes))
}

fn read_tsv(path: &Path) -> Result<Vec<BTreeMap<String, String>>> {
    let text = String::from_utf8(read(path)?)
        .map_err(|_| Error::Input(format!("{} is not UTF-8", path.display())))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| Error::Input("empty TSV".into()))?
        .split('\t')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines
        .filter(|line| !line.is_empty())
        .map(|line| {
            let values = line.split('\t').collect::<Vec<_>>();
            if values.len() != header.len() {
                return Err(Error::Input(format!("ragged TSV {}", path.display())));
            }
            Ok(header
                .iter()
                .cloned()
                .zip(values.into_iter().map(str::to_owned))
                .collect())
        })
        .collect()
}
fn get<'a>(row: &'a BTreeMap<String, String>, name: &str) -> &'a str {
    row.get(name).map(String::as_str).unwrap_or("")
}
fn number(row: &BTreeMap<String, String>, name: &str) -> Result<f64> {
    get(row, name)
        .parse()
        .map_err(|_| Error::Input(format!("invalid {name}")))
}
fn integer(row: &BTreeMap<String, String>, name: &str) -> Result<i64> {
    get(row, name)
        .parse()
        .map_err(|_| Error::Input(format!("invalid {name}")))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })
}

fn write_readme(root: &Path, protocol: &Preauthorization) -> Result<()> {
    let text = format!(
        "# Phase 13 preauthorization\n\nStatus: `PREAUTHORIZED_NOT_AUTHORIZED`\n\nProtocol: `{}`\n\nThis package freezes four candidates, untouched-window allowlists, metrics, uncertainty, multiplicity, and one-shot rules. It does not authorize or inspect holdout data.\n",
        protocol.protocol_sha256
    );
    fs::write(root.join("README.md"), text).map_err(|source| Error::Io {
        path: root.join("README.md"),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_is_explicit_unique_and_still_not_evaluation() {
        let root = std::env::temp_dir().join(format!(
            "northstar-holdout-auth-{}-{}",
            std::process::id(),
            "unique"
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut protocol = fixture_protocol();
        protocol.protocol_sha256 =
            canonical::canonical_hash_without(&protocol, "protocol_sha256").unwrap();
        canonical::write_pretty_json(&root.join("preauthorization.json"), &protocol).unwrap();
        let args = AuthorizeArgs {
            protocol: root.join("preauthorization.json"),
            output: root.join("authorization.json"),
            confirmation: "AUTHORIZE_ONE_SHOT_RG2_HOLDOUT".into(),
        };
        assert!(authorize_once(&args).is_ok());
        assert!(authorize_once(&args).is_err());
        assert!(!root.join("report.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn consumption_is_created_before_any_scoring_and_cannot_repeat() {
        let root = std::env::temp_dir().join(format!(
            "northstar-holdout-consume-{}-{}",
            std::process::id(),
            "consumed"
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut protocol = fixture_protocol();
        protocol.protocol_sha256 =
            canonical::canonical_hash_without(&protocol, "protocol_sha256").unwrap();
        let mut auth = Authorization {
            contract: crate::manifest::AUTH_CONTRACT.into(),
            status: "AUTHORIZED_UNCONSUMED".into(),
            holdout_authorized: true,
            protocol_sha256: protocol.protocol_sha256.clone(),
            authorization_token_sha256: String::new(),
        };
        auth.authorization_token_sha256 =
            canonical::canonical_hash_without(&auth, "authorization_token_sha256").unwrap();
        let bundle = BundleManifest {
            contract: input::BUNDLE_CONTRACT.into(),
            status: "SEALED".into(),
            research_generation: 2,
            protocol_sha256: protocol.protocol_sha256.clone(),
            runs: Vec::new(),
            targets: Vec::new(),
            bundle_semantic_sha256: "b".repeat(64),
        };
        let state = root.join("state");
        consume(&state, &protocol, &auth, &bundle).unwrap();
        assert!(state.join("consumed.json").exists());
        assert!(consume(&state, &protocol, &auth, &bundle).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synthetic_bundle_runs_through_frozen_models_once() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let package = crate_root.join("../../artifacts/phase13-preauthorization");
        if !package.join("preauthorization.json").exists() {
            return;
        }
        let root =
            std::env::temp_dir().join(format!("northstar-holdout-e2e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("receipts")).unwrap();
        let protocol_path = package.join("preauthorization.json");
        let protocol_bytes = fs::read(&protocol_path).unwrap();
        let protocol_raw: serde_json::Value = serde_json::from_slice(&protocol_bytes).unwrap();
        let protocol: Preauthorization = serde_json::from_slice(&protocol_bytes).unwrap();
        let typed_value = serde_json::to_value(&protocol).unwrap();
        if protocol_raw != typed_value {
            panic!(
                "protocol typed roundtrip differs: {}",
                first_json_difference(&protocol_raw, &typed_value, "$")
            );
        }
        let mut raw_unhashed = protocol_raw.clone();
        raw_unhashed
            .as_object_mut()
            .unwrap()
            .remove("protocol_sha256");
        let mut typed_unhashed = typed_value.clone();
        typed_unhashed
            .as_object_mut()
            .unwrap()
            .remove("protocol_sha256");
        let raw_bytes = canonical::canonical_json(&raw_unhashed).unwrap();
        let typed_bytes = canonical::canonical_json(&typed_unhashed).unwrap();
        if raw_bytes != typed_bytes {
            let index = raw_bytes
                .iter()
                .zip(&typed_bytes)
                .position(|(a, b)| a != b)
                .unwrap_or(raw_bytes.len().min(typed_bytes.len()));
            panic!(
                "canonical byte mismatch at {index}: raw={} typed={}",
                String::from_utf8_lossy(
                    &raw_bytes[index.saturating_sub(80)..(index + 80).min(raw_bytes.len())]
                ),
                String::from_utf8_lossy(
                    &typed_bytes[index.saturating_sub(80)..(index + 80).min(typed_bytes.len())]
                )
            );
        }
        protocol.validate().unwrap();
        let auth_path = root.join("authorization.json");
        authorize_once(&AuthorizeArgs {
            protocol: protocol_path.clone(),
            output: auth_path.clone(),
            confirmation: "AUTHORIZE_ONE_SHOT_RG2_HOLDOUT".into(),
        })
        .unwrap();
        let mut runs = Vec::new();
        for reservation in &protocol.reservations {
            let run_key = format!(
                "SYNTHETIC_{}_{}",
                reservation.canonical_instrument, reservation.holdout_id
            );
            let receipt_file = format!("receipts/{run_key}.json");
            let receipt_path = root.join(&receipt_file);
            fs::write(&receipt_path, format!("{{\"run_key\":\"{run_key}\"}}\n")).unwrap();
            runs.push(input::BundleRun {
                run_key,
                canonical_instrument: reservation.canonical_instrument.clone(),
                broker_symbol: reservation.broker_symbol.clone(),
                data_source_id: reservation.data_source_id.clone(),
                holdout_id: reservation.holdout_id.clone(),
                window_start: reservation.window_start,
                window_end_exclusive: reservation.window_end_exclusive,
                run_receipt_file: receipt_file,
                run_receipt_sha256: canonical::file_sha256(&receipt_path).unwrap(),
            });
        }
        let mut targets = Vec::new();
        for candidate in &protocol.candidates {
            let model_path = package.join(&candidate.model_file);
            let registry_row = crate::model::RegistryModel {
                target: candidate.target.clone(),
                model: model_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                model_sha256: candidate.model_canonical_sha256.clone(),
            };
            let artifact =
                RuntimeArtifact::load(model_path.parent().unwrap(), &registry_row).unwrap();
            let identity = [
                "target",
                "run_key",
                "episode_id",
                "attempt_id",
                "canonical_instrument",
                "holdout_id",
                "target_censored",
                "target_label",
            ];
            let extras = artifact
                .raw_columns()
                .filter(|name| !identity.contains(name))
                .collect::<Vec<_>>();
            let mut text = identity
                .iter()
                .copied()
                .chain(extras.iter().copied())
                .collect::<Vec<_>>()
                .join("\t");
            text.push('\n');
            for (index, run) in runs.iter().enumerate() {
                let mut row = vec![
                    candidate.target.clone(),
                    run.run_key.clone(),
                    format!("episode-{index}"),
                    format!("attempt-{index}"),
                    run.canonical_instrument.clone(),
                    run.holdout_id.clone(),
                    "false".into(),
                    usize::from(index % 2 == 0).to_string(),
                ];
                for name in &extras {
                    let value = if artifact
                        .artifact
                        .preprocessor
                        .numeric
                        .iter()
                        .any(|v| v == name)
                    {
                        "0"
                    } else if *name == "start_region" || *name == "node_region" {
                        "MEDIAN_CORE"
                    } else if *name == "contributor_producers" {
                        "WAYNE"
                    } else {
                        "<OTHER>"
                    };
                    row.push(value.into());
                }
                text.push_str(&row.join("\t"));
                text.push('\n');
            }
            let file = format!("{}.tsv", candidate.target);
            let path = root.join(&file);
            fs::write(&path, text).unwrap();
            targets.push(input::TargetFile {
                target: candidate.target.clone(),
                file,
                canonical_sha256: canonical::canonical_text_sha256(&path).unwrap(),
            });
        }
        let mut bundle = BundleManifest {
            contract: input::BUNDLE_CONTRACT.into(),
            status: "SEALED".into(),
            research_generation: 2,
            protocol_sha256: protocol.protocol_sha256.clone(),
            runs,
            targets,
            bundle_semantic_sha256: String::new(),
        };
        bundle.bundle_semantic_sha256 =
            canonical::canonical_hash_without(&bundle, "bundle_semantic_sha256").unwrap();
        let bundle_path = root.join("bundle.json");
        canonical::write_pretty_json(&bundle_path, &bundle).unwrap();
        let args = EvaluateArgs {
            protocol: protocol_path,
            authorization: auth_path,
            bundle: bundle_path,
            state_dir: root.join("state"),
            report: root.join("report.json"),
        };
        evaluate_once(&args).unwrap();
        assert!(args.report.exists());
        assert!(evaluate_once(&args).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture_protocol() -> Preauthorization {
        Preauthorization {
            contract: PREAUTH_CONTRACT.into(),
            status: "PREAUTHORIZED_NOT_AUTHORIZED".into(),
            holdout_authorized: false,
            research_generation: 2,
            identities: Identities {
                source_corpus_sha256: "a".repeat(64),
                packed_corpus_semantic_sha256: "b".repeat(64),
                phase11_seal_sha256: "c".repeat(64),
                candidate_protocol_sha256: "d".repeat(64),
                model_registry_sha256: "e".repeat(64),
                reservation_manifest_sha256: "f".repeat(64),
                feature_schema_sha256: "1".repeat(64),
                research_interface_sha256: "2".repeat(64),
                evaluator_code_sha256: "3".repeat(64),
            },
            candidates: vec![Candidate {
                target: "test".into(),
                model_class: "RIDGE_LOGISTIC".into(),
                model_file: "models/test.json".into(),
                model_canonical_sha256: "4".repeat(64),
                model_semantic_sha256: "5".repeat(64),
                eligibility: "test".into(),
                development: DevelopmentReference {
                    prevalence: 0.5,
                    forward_auc: 0.5,
                    forward_average_precision: 0.5,
                    forward_brier: 0.25,
                    forward_log_loss: 0.693,
                    forward_ece: 0.0,
                },
                gates: CandidateGates {
                    minimum_eligible: 1,
                    minimum_analyzable: 1,
                    minimum_positive: 0,
                    minimum_negative: 0,
                    minimum_runs: 1,
                    maximum_censoring_rate: 1.0,
                    minimum_auc: 0.0,
                    minimum_auc_delta: -1.0,
                    maximum_log_loss_delta: 1.0,
                    maximum_brier_delta: 1.0,
                    minimum_calibration_intercept: -10.0,
                    maximum_calibration_intercept: 10.0,
                    minimum_calibration_slope: -10.0,
                    maximum_calibration_slope: 10.0,
                    maximum_ece: 1.0,
                    require_positive_primary_delta_each_calendar_block: false,
                },
            }],
            reservations: Vec::new(),
            evaluation: EvaluationPolicy {
                primary_metric: "PAIRED_LOG_LOSS_IMPROVEMENT_VS_FROZEN_DEVELOPMENT_PREVALENCE"
                    .into(),
                baseline: "FROZEN_DEVELOPMENT_PREVALENCE".into(),
                censoring: "EXCLUDE_CENSORED_FROM_SCORE_RETAIN_IN_DENOMINATORS".into(),
                probability_clip: 1e-15,
                reliability_boundaries: vec![0.0, 1.0],
                bootstrap: BootstrapPolicy {
                    unit: "RUN_CLUSTER".into(),
                    seed: 1,
                    repetitions: 10,
                    confidence_level: 0.95,
                },
                multiplicity: MultiplicityPolicy {
                    method: "HOLM_ONE_SIDED_PRIMARY_P".into(),
                    family_alpha: 0.05,
                    candidate_family: vec!["test".into()],
                },
                one_shot: OneShotPolicy {
                    expected_reservations: 0,
                    infrastructure_failure_status: "ABORT_NO_CANDIDATE_REPORT".into(),
                    consumed_before_scoring: true,
                    atomic_report_publication: true,
                    partial_results_forbidden: true,
                },
            },
            protocol_sha256: String::new(),
        }
    }

    fn first_json_difference(
        left: &serde_json::Value,
        right: &serde_json::Value,
        path: &str,
    ) -> String {
        match (left, right) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                for key in a.keys().chain(b.keys()) {
                    if a.get(key) != b.get(key) {
                        return first_json_difference(
                            a.get(key).unwrap_or(&serde_json::Value::Null),
                            b.get(key).unwrap_or(&serde_json::Value::Null),
                            &format!("{path}.{key}"),
                        );
                    }
                }
                path.into()
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                for index in 0..a.len().max(b.len()) {
                    if a.get(index) != b.get(index) {
                        return first_json_difference(
                            a.get(index).unwrap_or(&serde_json::Value::Null),
                            b.get(index).unwrap_or(&serde_json::Value::Null),
                            &format!("{path}[{index}]"),
                        );
                    }
                }
                path.into()
            }
            _ => format!("{path}: left={left} right={right}"),
        }
    }
}

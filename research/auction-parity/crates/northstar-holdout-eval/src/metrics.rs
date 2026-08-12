use std::cmp::Ordering;

use hashbrown::HashMap;
use serde::Serialize;

use crate::{
    Error, Result,
    manifest::{BootstrapPolicy, Candidate, EvaluationPolicy},
};

#[derive(Clone, Debug)]
pub struct ScoredRow {
    pub run_key: String,
    pub instrument: String,
    pub holdout_id: String,
    pub label: bool,
    pub probability: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Metrics {
    pub n: usize,
    pub positives: usize,
    pub negatives: usize,
    pub prevalence: f64,
    pub auc: Option<f64>,
    pub average_precision: Option<f64>,
    pub brier: f64,
    pub log_loss: f64,
    pub ece: f64,
    pub calibration_intercept: Option<f64>,
    pub calibration_slope: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReliabilityBucket {
    pub lower: f64,
    pub upper: f64,
    pub count: usize,
    pub mean_probability: Option<f64>,
    pub observed_rate: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Breakdown {
    pub key: String,
    pub metrics: Metrics,
    pub primary_delta: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BootstrapResult {
    pub point_delta: f64,
    pub lower: f64,
    pub upper: f64,
    pub one_sided_p: f64,
}

pub struct GateEvidence<'a> {
    pub eligible: usize,
    pub censored: usize,
    pub run_count: usize,
    pub metrics: &'a Metrics,
    pub bootstrap: &'a BootstrapResult,
    pub blocks: &'a [Breakdown],
    pub adjusted_p: f64,
    pub alpha: f64,
}

pub fn calculate(
    rows: &[ScoredRow],
    _prevalence: f64,
    policy: &EvaluationPolicy,
) -> Result<(Metrics, Vec<ReliabilityBucket>)> {
    if rows.is_empty() {
        return Err(Error::Input("no analyzable observations".into()));
    }
    let clip = policy.probability_clip;
    let mut log_loss = 0.0;
    let mut brier = 0.0;
    let mut positives = 0;
    for row in rows {
        let p = row.probability.clamp(clip, 1.0 - clip);
        let y = f64::from(row.label);
        positives += usize::from(row.label);
        log_loss -= y * p.ln() + (1.0 - y) * (1.0 - p).ln();
        brier += (p - y) * (p - y);
    }
    let n = rows.len();
    let buckets = reliability(rows, &policy.reliability_boundaries);
    let ece = buckets
        .iter()
        .filter_map(|b| {
            Some((b.count as f64 / n as f64) * (b.mean_probability? - b.observed_rate?).abs())
        })
        .sum();
    let (intercept, slope) = recalibration(rows, clip);
    Ok((
        Metrics {
            n,
            positives,
            negatives: n - positives,
            prevalence: positives as f64 / n as f64,
            auc: auc(rows),
            average_precision: average_precision(rows),
            brier: brier / n as f64,
            log_loss: log_loss / n as f64,
            ece,
            calibration_intercept: intercept,
            calibration_slope: slope,
        },
        buckets,
    ))
}

pub fn primary_delta(rows: &[ScoredRow], prevalence: f64, clip: f64) -> f64 {
    let q = prevalence.clamp(clip, 1.0 - clip);
    rows.iter()
        .map(|row| {
            let y = f64::from(row.label);
            let p = row.probability.clamp(clip, 1.0 - clip);
            let model = -(y * p.ln() + (1.0 - y) * (1.0 - p).ln());
            let null = -(y * q.ln() + (1.0 - y) * (1.0 - q).ln());
            null - model
        })
        .sum::<f64>()
        / rows.len() as f64
}

pub fn breakdowns(
    rows: &[ScoredRow],
    prevalence: f64,
    policy: &EvaluationPolicy,
    by_block: bool,
) -> Result<Vec<Breakdown>> {
    let mut groups: HashMap<&str, Vec<ScoredRow>> = HashMap::new();
    for row in rows {
        groups
            .entry(if by_block {
                &row.holdout_id
            } else {
                &row.instrument
            })
            .or_default()
            .push(row.clone());
    }
    let mut output = groups
        .into_iter()
        .map(|(key, rows)| {
            let (metrics, _) = calculate(&rows, prevalence, policy)?;
            Ok(Breakdown {
                key: key.into(),
                primary_delta: primary_delta(&rows, prevalence, policy.probability_clip),
                metrics,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    output.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(output)
}

pub fn bootstrap(
    rows: &[ScoredRow],
    prevalence: f64,
    policy: &BootstrapPolicy,
    clip: f64,
) -> Result<BootstrapResult> {
    let mut groups: HashMap<&str, Vec<&ScoredRow>> = HashMap::new();
    for row in rows {
        groups.entry(&row.run_key).or_default().push(row);
    }
    if groups.len() < 2 {
        return Err(Error::Input(
            "run-cluster bootstrap needs at least two runs".into(),
        ));
    }
    let runs = groups.into_values().collect::<Vec<_>>();
    let point_delta = primary_delta(rows, prevalence, clip);
    let mut samples = Vec::with_capacity(policy.repetitions);
    let mut rng = SplitMix64(policy.seed);
    for _ in 0..policy.repetitions {
        let mut sum = 0.0;
        let mut count = 0;
        for _ in 0..runs.len() {
            let group = &runs[(rng.next() as usize) % runs.len()];
            for row in group {
                sum += row_delta(row, prevalence, clip);
                count += 1;
            }
        }
        samples.push(sum / count as f64);
    }
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let tail = (1.0 - policy.confidence_level) / 2.0;
    let lower = quantile(&samples, tail);
    let upper = quantile(&samples, 1.0 - tail);
    let nonpositive = samples.iter().filter(|&&value| value <= 0.0).count();
    Ok(BootstrapResult {
        point_delta,
        lower,
        upper,
        one_sided_p: (nonpositive + 1) as f64 / (samples.len() + 1) as f64,
    })
}

pub fn candidate_gate(
    candidate: &Candidate,
    evidence: &GateEvidence<'_>,
) -> (&'static str, Vec<String>) {
    let g = &candidate.gates;
    let GateEvidence {
        eligible,
        censored,
        run_count,
        metrics,
        bootstrap: boot,
        blocks,
        adjusted_p,
        alpha,
    } = *evidence;
    let mut failures = Vec::new();
    let censoring = if eligible == 0 {
        1.0
    } else {
        censored as f64 / eligible as f64
    };
    let support = eligible >= g.minimum_eligible
        && metrics.n >= g.minimum_analyzable
        && metrics.positives >= g.minimum_positive
        && metrics.negatives >= g.minimum_negative
        && run_count >= g.minimum_runs
        && censoring <= g.maximum_censoring_rate;
    if !support {
        failures.push("SUPPORT_OR_CENSORING".into());
    }
    if support {
        if metrics.auc.unwrap_or(0.0) < g.minimum_auc
            || metrics.auc.unwrap_or(0.0) - candidate.development.forward_auc < g.minimum_auc_delta
        {
            failures.push("DISCRIMINATION".into());
        }
        if metrics.log_loss - candidate.development.forward_log_loss > g.maximum_log_loss_delta
            || metrics.brier - candidate.development.forward_brier > g.maximum_brier_delta
        {
            failures.push("DEGRADATION".into());
        }
        if metrics.ece > g.maximum_ece
            || !within(
                metrics.calibration_intercept,
                g.minimum_calibration_intercept,
                g.maximum_calibration_intercept,
            )
            || !within(
                metrics.calibration_slope,
                g.minimum_calibration_slope,
                g.maximum_calibration_slope,
            )
        {
            failures.push("CALIBRATION".into());
        }
    }
    if boot.lower <= 0.0 || adjusted_p > alpha {
        failures.push("PRIMARY_UNCERTAINTY".into());
    }
    if g.require_positive_primary_delta_each_calendar_block
        && (blocks.len() != 2 || blocks.iter().any(|b| b.primary_delta <= 0.0))
    {
        failures.push("TEMPORAL_BLOCK_TRANSPORT".into());
    }
    let status = if failures.is_empty() {
        "PASS"
    } else if support && boot.point_delta > 0.0 {
        "WEAK_RESEARCH_ONLY"
    } else {
        "FAIL"
    };
    (status, failures)
}

pub fn holm_adjust(raw: &[(String, f64)]) -> HashMap<String, f64> {
    let mut order = raw.to_vec();
    order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    let mut output = HashMap::with_capacity(order.len());
    let mut previous = 0.0_f64;
    let total = order.len();
    for (rank, (name, p)) in order.into_iter().enumerate() {
        let adjusted = ((total - rank) as f64 * p).min(1.0).max(previous);
        previous = adjusted;
        output.insert(name, adjusted);
    }
    output
}

fn reliability(rows: &[ScoredRow], boundaries: &[f64]) -> Vec<ReliabilityBucket> {
    boundaries
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let selected = rows
                .iter()
                .filter(|r| {
                    r.probability >= pair[0]
                        && (r.probability < pair[1]
                            || (index + 2 == boundaries.len() && r.probability <= pair[1]))
                })
                .collect::<Vec<_>>();
            let count = selected.len();
            ReliabilityBucket {
                lower: pair[0],
                upper: pair[1],
                count,
                mean_probability: (count > 0)
                    .then(|| selected.iter().map(|r| r.probability).sum::<f64>() / count as f64),
                observed_rate: (count > 0).then(|| {
                    selected.iter().map(|r| f64::from(r.label)).sum::<f64>() / count as f64
                }),
            }
        })
        .collect()
}

fn auc(rows: &[ScoredRow]) -> Option<f64> {
    let positives = rows.iter().filter(|r| r.label).count();
    let negatives = rows.len() - positives;
    if positives == 0 || negatives == 0 {
        return None;
    }
    let mut sorted = rows.iter().collect::<Vec<_>>();
    sorted.sort_unstable_by(|a, b| {
        a.probability
            .partial_cmp(&b.probability)
            .unwrap_or(Ordering::Equal)
    });
    let mut rank_sum = 0.0;
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end].probability == sorted[start].probability {
            end += 1;
        }
        let avg_rank = (start + 1 + end) as f64 / 2.0;
        rank_sum += avg_rank * sorted[start..end].iter().filter(|r| r.label).count() as f64;
        start = end;
    }
    Some(
        (rank_sum - positives as f64 * (positives + 1) as f64 / 2.0)
            / (positives * negatives) as f64,
    )
}

fn average_precision(rows: &[ScoredRow]) -> Option<f64> {
    let positives = rows.iter().filter(|r| r.label).count();
    if positives == 0 {
        return None;
    }
    let mut sorted = rows.iter().collect::<Vec<_>>();
    sorted.sort_unstable_by(|a, b| {
        b.probability
            .partial_cmp(&a.probability)
            .unwrap_or(Ordering::Equal)
    });
    let mut hits = 0;
    let mut sum = 0.0;
    for (index, row) in sorted.iter().enumerate() {
        if row.label {
            hits += 1;
            sum += hits as f64 / (index + 1) as f64;
        }
    }
    Some(sum / positives as f64)
}

fn recalibration(rows: &[ScoredRow], clip: f64) -> (Option<f64>, Option<f64>) {
    if rows.iter().all(|r| r.label) || rows.iter().all(|r| !r.label) {
        return (None, None);
    }
    let mut a = 0.0;
    let mut b = 1.0;
    for _ in 0..50 {
        let (mut g0, mut g1, mut h00, mut h01, mut h11) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for row in rows {
            let p = row.probability.clamp(clip, 1.0 - clip);
            let x = (p / (1.0 - p)).ln();
            let q = 1.0 / (1.0 + (-(a + b * x)).exp());
            let residual = f64::from(row.label) - q;
            let w = q * (1.0 - q);
            g0 += residual;
            g1 += residual * x;
            h00 += w;
            h01 += w * x;
            h11 += w * x * x;
        }
        let det = h00 * h11 - h01 * h01;
        if !det.is_finite() || det.abs() < 1e-12 {
            return (None, None);
        }
        let da = (g0 * h11 - g1 * h01) / det;
        let db = (g1 * h00 - g0 * h01) / det;
        a += da;
        b += db;
        if da.abs().max(db.abs()) < 1e-10 {
            return (Some(a), Some(b));
        }
    }
    (Some(a), Some(b))
}

fn within(value: Option<f64>, low: f64, high: f64) -> bool {
    value.is_some_and(|v| v >= low && v <= high)
}
fn row_delta(row: &ScoredRow, prevalence: f64, clip: f64) -> f64 {
    let y = f64::from(row.label);
    let p = row.probability.clamp(clip, 1.0 - clip);
    let q = prevalence.clamp(clip, 1.0 - clip);
    -(y * q.ln() + (1.0 - y) * (1.0 - q).ln()) + y * p.ln() + (1.0 - y) * (1.0 - p).ln()
}
fn quantile(sorted: &[f64], probability: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * probability).round() as usize]
}

struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perfect_classifier_has_perfect_auc() {
        let rows = vec![
            row(false, 0.1),
            row(true, 0.9),
            row(false, 0.2),
            row(true, 0.8),
        ];
        assert_eq!(auc(&rows), Some(1.0));
        assert_eq!(average_precision(&rows), Some(1.0));
    }
    #[test]
    fn holm_is_monotonic_in_rank_order() {
        let out = holm_adjust(&[("a".into(), 0.01), ("b".into(), 0.03), ("c".into(), 0.2)]);
        assert_eq!(out["a"], 0.03);
        assert_eq!(out["b"], 0.06);
        assert_eq!(out["c"], 0.2);
    }
    fn row(label: bool, probability: f64) -> ScoredRow {
        ScoredRow {
            run_key: "r".into(),
            instrument: "x".into(),
            holdout_id: "a".into(),
            label,
            probability,
        }
    }
}

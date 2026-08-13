use crate::{RawCorpus, RawError};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[path = "gate15_cluster.rs"]
mod cluster;
use cluster::{adjusted_rand, analyze_view};

const RECIPE: &str =
    "NORTHSTAR_RG3_GATE15_MULTI_VIEW_DISCOVERY_V2|C16|E32|K2:8|SEEDS8|ITER100|WINSOR01|NOISE_LT30";
const MIN_SAMPLES: usize = 4;
const MIN_FAMILY_SUPPORT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ObjectKind {
    Compression,
    Expansion,
}

impl ObjectKind {
    fn name(self) -> &'static str {
        match self {
            Self::Compression => "COMPRESSION",
            Self::Expansion => "EXPANSION",
        }
    }
}

#[derive(Debug, Clone)]
struct ObjectRecord {
    kind: ObjectKind,
    run_key: String,
    instrument: String,
    object_id: i64,
    terminal_reason: i64,
    censored: bool,
    eligible: bool,
    summary: Vec<f32>,
    shape: Vec<f32>,
    hybrid: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateAssignment {
    pub object_kind: String,
    pub run_key: String,
    pub canonical_instrument: String,
    pub object_id: i64,
    pub terminal_reason_code: i64,
    pub censored: bool,
    pub eligible: bool,
    pub representation: String,
    pub candidate_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilySupport {
    pub candidate_id: String,
    pub count: usize,
    pub fraction: f64,
    pub instrument_count: usize,
    pub instruments: Vec<String>,
    pub instrument_counts: BTreeMap<String, usize>,
    pub largest_instrument_fraction: f64,
    pub window_count: usize,
    pub window_counts: BTreeMap<String, usize>,
    pub largest_window_fraction: f64,
    pub run_count: usize,
    pub common_support: bool,
    pub medoid_run_key: String,
    pub medoid_object_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewReport {
    pub representation: String,
    pub family_system_sha256: String,
    pub dimensions: usize,
    pub selected_k: usize,
    pub selection_score: f64,
    pub centroid_silhouette: f64,
    pub mean_seed_ari: f64,
    pub minimum_cluster_support: usize,
    pub supported_objects: usize,
    pub noise_objects: usize,
    pub noise_fraction: f64,
    pub families: Vec<FamilySupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrespondenceEdge {
    pub left_candidate: String,
    pub right_candidate: String,
    pub count: usize,
    pub fraction_of_left: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgreementReport {
    pub left: String,
    pub right: String,
    pub raw_partition_ari: f64,
    pub assignment_ari_with_null: f64,
    pub supported_only_ari: Option<f64>,
    pub supported_pair_objects: usize,
    pub shared_noise_objects: usize,
    pub adjusted_rand_index: f64,
    pub correspondence: Vec<CorrespondenceEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindReport {
    pub object_kind: String,
    pub total: usize,
    pub eligible: usize,
    pub censored: usize,
    pub terminal_reason_counts: BTreeMap<String, usize>,
    pub views: Vec<ViewReport>,
    pub cross_view_agreement: Vec<AgreementReport>,
    pub mean_cross_view_ari: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate15ExitChecks {
    pub minimum_total_objects: bool,
    pub minimum_objects_per_instrument: bool,
    pub minimum_eligible_per_object_kind: bool,
    pub common_family_support: bool,
    pub within_view_stability: bool,
    pub cross_view_agreement: bool,
    pub raw_generation_consistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate15Report {
    pub contract: String,
    pub status: String,
    pub epistemic_status: String,
    pub research_generation: u32,
    pub recipe_id: String,
    pub recipe_sha256: String,
    pub source_run_count: usize,
    pub source_run_hashes: Vec<String>,
    pub total_objects: usize,
    pub objects_by_instrument: BTreeMap<String, usize>,
    pub minimum_instrument_objects: usize,
    pub missingness: BTreeMap<String, usize>,
    pub kinds: Vec<KindReport>,
    pub exit_checks: Gate15ExitChecks,
    pub limitations: Vec<String>,
}

/// Complete fitted representation system needed to replay Gate 15 assignments.
///
/// `NULL` is deliberately not represented as a centroid. It is the result of
/// an unsupported training partition. Gate 15.5 does not invent an OOS
/// distance-rejection boundary; that remains a Gate 16 concern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FittedFamilySystem {
    pub object_kind: String,
    pub representation: String,
    pub family_system_sha256: String,
    pub recipe_id: String,
    pub feature_order: Vec<String>,
    pub winsor_low: Vec<f32>,
    pub winsor_high: Vec<f32>,
    pub means: Vec<f64>,
    pub scales: Vec<f64>,
    pub standardized_clip: [f32; 2],
    pub distance: String,
    pub selected_k: usize,
    pub centroids: Vec<Vec<f32>>,
    pub centroid_labels: Vec<String>,
    pub supported: Vec<bool>,
    pub assignment_rule: String,
    pub oos_rejection_rule: String,
}

#[derive(Clone, Copy)]
struct CompressionSample {
    age: f32,
    close: f32,
    atr: f32,
    seed_top: f32,
    seed_bottom: f32,
    seed_mid: f32,
    contain_top: f32,
    contain_bottom: f32,
    contain_mid: f32,
}

#[derive(Clone, Copy)]
struct ExpansionSample {
    age: f32,
    atr: f32,
    displacement: f32,
    velocity: f32,
}

fn is_censor(reason: i64) -> bool {
    (5..=8).contains(&reason)
}

fn resample<const C: usize>(samples: &[(f32, [f32; C])], points: usize) -> Vec<f32> {
    let mut output = Vec::with_capacity(points * C);
    let first = samples[0].0;
    let last = samples[samples.len() - 1].0;
    let mut right = 1usize;
    for point in 0..points {
        let target = if points == 1 || last <= first {
            first
        } else {
            first + (last - first) * point as f32 / (points - 1) as f32
        };
        while right < samples.len() && samples[right].0 < target {
            right += 1;
        }
        let hi = right.min(samples.len() - 1);
        let lo = hi.saturating_sub(1);
        let span = samples[hi].0 - samples[lo].0;
        let weight = if span > f32::EPSILON {
            (target - samples[lo].0) / span
        } else {
            0.0
        };
        for channel in 0..C {
            output.push(samples[lo].1[channel] * (1.0 - weight) + samples[hi].1[channel] * weight);
        }
    }
    output
}

fn compression_records(
    corpus: &RawCorpus,
    instrument: &str,
) -> Result<Vec<ObjectRecord>, RawError> {
    let mut samples: HashMap<i64, Vec<CompressionSample>> = HashMap::new();
    for row in corpus.rows("compression_samples") {
        samples
            .entry(row.i64("compression_id")?)
            .or_default()
            .push(CompressionSample {
                age: row.f64("age_bars")? as f32,
                close: row.f64("close")? as f32,
                atr: row.f64("atr")? as f32,
                seed_top: row.f64("seed_top")? as f32,
                seed_bottom: row.f64("seed_bottom")? as f32,
                seed_mid: row.f64("seed_mid")? as f32,
                contain_top: row.f64("contain_top")? as f32,
                contain_bottom: row.f64("contain_bottom")? as f32,
                contain_mid: row.f64("contain_mid")? as f32,
            });
    }
    let mut output = Vec::new();
    for row in corpus.rows("compression_objects") {
        let id = row.i64("compression_id")?;
        let reason = row.i64("terminal_reason_code")?;
        let direction = row.i64("direction")? as f32;
        let object_samples = samples.remove(&id).unwrap_or_default();
        let eligible = !is_censor(reason) && object_samples.len() >= MIN_SAMPLES;
        let mut summary = Vec::new();
        let mut shape = Vec::new();
        let mut hybrid = Vec::new();
        if let (Some(first), Some(last)) = (object_samples.first(), object_samples.last()) {
            let atr = first.atr.max(f32::EPSILON);
            let seed_width = (first.seed_top - first.seed_bottom).abs().max(f32::EPSILON);
            let terminal_width = (last.contain_top - last.contain_bottom).abs();
            let mirror = if direction == 0.0 { 1.0 } else { direction };
            summary = vec![
                (object_samples.len() as f32).ln_1p(),
                seed_width / atr,
                terminal_width / seed_width,
                (last.contain_mid - first.seed_mid) / atr,
                row.f64("escape_count")? as f32,
                (last.close - last.contain_mid) / atr,
            ];
            let trajectory: Vec<_> = object_samples
                .iter()
                .map(|sample| {
                    let width = (sample.contain_top - sample.contain_bottom).abs() / seed_width;
                    (
                        sample.age,
                        [
                            width,
                            (sample.contain_mid - first.seed_mid) / atr,
                            (sample.close - sample.contain_mid) / atr,
                        ],
                    )
                })
                .collect();
            shape = resample(&trajectory, 16);
            hybrid = summary.clone();
            hybrid[3] *= mirror;
            hybrid[5] *= mirror;
            for chunk in shape.chunks_exact(3) {
                hybrid.extend_from_slice(&[chunk[0], chunk[1] * mirror, chunk[2] * mirror]);
            }
        }
        output.push(ObjectRecord {
            kind: ObjectKind::Compression,
            run_key: corpus.report().run_key.clone(),
            instrument: instrument.to_owned(),
            object_id: id,
            terminal_reason: reason,
            censored: is_censor(reason),
            eligible,
            summary,
            shape,
            hybrid,
        });
    }
    Ok(output)
}

fn expansion_records(corpus: &RawCorpus, instrument: &str) -> Result<Vec<ObjectRecord>, RawError> {
    let mut samples: HashMap<i64, Vec<ExpansionSample>> = HashMap::new();
    for row in corpus.rows("expansion_samples") {
        samples
            .entry(row.i64("expansion_id")?)
            .or_default()
            .push(ExpansionSample {
                age: row.f64("age_bars")? as f32,
                atr: row.f64("origin_atr")? as f32,
                displacement: row.f64("raw_displacement")? as f32,
                velocity: row.f64("raw_velocity_per_bar")? as f32,
            });
    }
    let mut output = Vec::new();
    for row in corpus.rows("expansion_objects") {
        let id = row.i64("expansion_id")?;
        let reason = row.i64("terminal_reason_code")?;
        let direction = row.i64("direction")? as f32;
        let object_samples = samples.remove(&id).unwrap_or_default();
        let eligible = !is_censor(reason) && object_samples.len() >= MIN_SAMPLES;
        let mut summary = Vec::new();
        let mut shape = Vec::new();
        let mut hybrid = Vec::new();
        if let Some(first) = object_samples.first() {
            let atr = first.atr.max(f32::EPSILON);
            let mirror = if direction == 0.0 { 1.0 } else { direction };
            let max = row.f64("max_displacement")? as f32;
            let path = row.f64("close_path_length")? as f32;
            let terminal =
                row.f64("terminal_price")? as f32 - row.f64("origin_terminal_price")? as f32;
            summary = vec![
                (object_samples.len() as f32).ln_1p(),
                terminal / atr,
                max / atr,
                row.f64("opposite_displacement")? as f32 / atr,
                if max > f32::EPSILON {
                    row.f64("return_depth")? as f32 / max
                } else {
                    0.0
                },
                if path > f32::EPSILON {
                    terminal.abs() / path
                } else {
                    0.0
                },
                (row.f64("origin_containment_frontier")? as f32
                    - row.f64("origin_seed_frontier")? as f32)
                    .abs()
                    / atr,
            ];
            let trajectory: Vec<_> = object_samples
                .iter()
                .map(|sample| {
                    (
                        sample.age,
                        [sample.displacement / atr, sample.velocity / atr],
                    )
                })
                .collect();
            shape = resample(&trajectory, 32);
            hybrid = summary.clone();
            hybrid[1] *= mirror;
            for chunk in shape.chunks_exact(2) {
                hybrid.extend_from_slice(&[chunk[0] * mirror, chunk[1] * mirror]);
            }
        }
        output.push(ObjectRecord {
            kind: ObjectKind::Expansion,
            run_key: corpus.report().run_key.clone(),
            instrument: instrument.to_owned(),
            object_id: id,
            terminal_reason: reason,
            censored: is_censor(reason),
            eligible,
            summary,
            shape,
            hybrid,
        });
    }
    Ok(output)
}

fn analyze_kind(
    records: &[ObjectRecord],
    kind: ObjectKind,
    assignments: &mut Vec<CandidateAssignment>,
    fitted: &mut Vec<FittedFamilySystem>,
) -> KindReport {
    let all: Vec<_> = records
        .iter()
        .filter(|record| record.kind == kind)
        .collect();
    let eligible: Vec<_> = all
        .iter()
        .copied()
        .filter(|record| record.eligible)
        .collect();
    let mut reasons = BTreeMap::new();
    for record in &all {
        *reasons
            .entry(record.terminal_reason.to_string())
            .or_insert(0) += 1;
    }
    let views = [
        "summary_geometry_v1",
        "resampled_shape_v1",
        "hybrid_mirrored_v1",
    ];
    let analyses: Vec<_> = views
        .iter()
        .map(|view| analyze_view(&eligible, view))
        .collect();
    for (view, analysis) in views.iter().zip(&analyses) {
        fitted.push(FittedFamilySystem {
            object_kind: kind.name().into(),
            representation: (*view).into(),
            family_system_sha256: analysis.report.family_system_sha256.clone(),
            recipe_id: RECIPE.into(),
            feature_order: feature_order(kind, view),
            winsor_low: analysis.winsor_low.clone(),
            winsor_high: analysis.winsor_high.clone(),
            means: analysis.means.clone(),
            scales: analysis.scales.clone(),
            standardized_clip: [-6.0, 6.0],
            distance: "SQUARED_EUCLIDEAN_STANDARDIZED".into(),
            selected_k: analysis.report.selected_k,
            centroids: analysis
                .centroids
                .chunks_exact(analysis.report.dimensions)
                .map(<[f32]>::to_vec)
                .collect(),
            centroid_labels: (0..analysis.report.selected_k)
                .map(|index| format!("C{}", index + 1))
                .collect(),
            supported: analysis
                .report
                .families
                .iter()
                .map(|family| family.common_support)
                .collect(),
            assignment_rule:
                "nearest fitted centroid; unsupported training centroid maps to NULL_FAMILY".into(),
            oos_rejection_rule: "NONE_FROZEN_GATE15_5; distance rejection deferred to Gate16"
                .into(),
        });
    }
    for (view, analysis) in views.iter().zip(&analyses) {
        for record in &all {
            assignments.push(CandidateAssignment {
                object_kind: kind.name().into(),
                run_key: record.run_key.clone(),
                canonical_instrument: record.instrument.clone(),
                object_id: record.object_id,
                terminal_reason_code: record.terminal_reason,
                censored: record.censored,
                eligible: record.eligible,
                representation: (*view).into(),
                candidate_id: None,
            });
        }
        for (index, record) in eligible.iter().enumerate() {
            let target = assignments
                .iter_mut()
                .rev()
                .find(|row| {
                    row.representation == *view
                        && row.run_key == record.run_key
                        && row.object_id == record.object_id
                        && row.object_kind == kind.name()
                })
                .expect("assignment exists");
            let label = analysis.labels[index];
            target.candidate_id = analysis.report.families[label]
                .common_support
                .then(|| format!("C{}", label + 1));
        }
    }
    let mut agreements = Vec::new();
    for left in 0..analyses.len() {
        for right in left + 1..analyses.len() {
            let mut edges = BTreeMap::new();
            let mut left_totals = BTreeMap::new();
            let mut left_with_null = Vec::with_capacity(eligible.len());
            let mut right_with_null = Vec::with_capacity(eligible.len());
            let mut supported_left = Vec::new();
            let mut supported_right = Vec::new();
            let mut shared_noise_objects = 0usize;
            for index in 0..eligible.len() {
                let left_label = analyses[left].labels[index];
                let right_label = analyses[right].labels[index];
                let left_supported = analyses[left].report.families[left_label].common_support;
                let right_supported = analyses[right].report.families[right_label].common_support;
                let left_name = if left_supported {
                    format!("C{}", left_label + 1)
                } else {
                    "NULL".into()
                };
                let right_name = if right_supported {
                    format!("C{}", right_label + 1)
                } else {
                    "NULL".into()
                };
                *left_totals.entry(left_name.clone()).or_insert(0usize) += 1;
                *edges.entry((left_name, right_name)).or_insert(0usize) += 1;
                left_with_null.push(if left_supported {
                    left_label
                } else {
                    usize::MAX
                });
                right_with_null.push(if right_supported {
                    right_label
                } else {
                    usize::MAX
                });
                if left_supported && right_supported {
                    supported_left.push(left_label);
                    supported_right.push(right_label);
                } else if !left_supported && !right_supported {
                    shared_noise_objects += 1;
                }
            }
            let correspondence = edges
                .into_iter()
                .map(
                    |((left_candidate, right_candidate), count)| CorrespondenceEdge {
                        fraction_of_left: count as f64
                            / left_totals.get(&left_candidate).copied().unwrap_or(1) as f64,
                        left_candidate,
                        right_candidate,
                        count,
                    },
                )
                .collect();
            let raw_partition_ari = adjusted_rand(&analyses[left].labels, &analyses[right].labels);
            agreements.push(AgreementReport {
                left: views[left].into(),
                right: views[right].into(),
                raw_partition_ari,
                assignment_ari_with_null: adjusted_rand(&left_with_null, &right_with_null),
                supported_only_ari: (supported_left.len() >= 2)
                    .then(|| adjusted_rand(&supported_left, &supported_right)),
                supported_pair_objects: supported_left.len(),
                shared_noise_objects,
                adjusted_rand_index: raw_partition_ari,
                correspondence,
            });
        }
    }
    let mean_cross_view_ari = agreements
        .iter()
        .map(|item| item.adjusted_rand_index)
        .sum::<f64>()
        / agreements.len() as f64;
    KindReport {
        object_kind: kind.name().into(),
        total: all.len(),
        eligible: eligible.len(),
        censored: all.iter().filter(|record| record.censored).count(),
        terminal_reason_counts: reasons,
        views: analyses
            .into_iter()
            .map(|analysis| analysis.report)
            .collect(),
        cross_view_agreement: agreements,
        mean_cross_view_ari,
    }
}

pub fn discover_trajectory_families(
    corpora: &[RawCorpus],
) -> Result<(Gate15Report, Vec<CandidateAssignment>), RawError> {
    let (report, assignments, _) = fit_trajectory_family_systems(corpora)?;
    Ok((report, assignments))
}

pub fn fit_trajectory_family_systems(
    corpora: &[RawCorpus],
) -> Result<
    (
        Gate15Report,
        Vec<CandidateAssignment>,
        Vec<FittedFamilySystem>,
    ),
    RawError,
> {
    let mut records = Vec::new();
    let mut missingness = BTreeMap::new();
    let mut run_hashes = Vec::with_capacity(corpora.len());
    for corpus in corpora {
        let instrument = corpus
            .rows("measurement_runs")
            .nth(1)
            .ok_or_else(|| RawError::Invariant("missing END row".into()))?
            .field("canonical_instrument")?
            .to_owned();
        records.extend(compression_records(corpus, &instrument)?);
        records.extend(expansion_records(corpus, &instrument)?);
        for (field, count) in corpus.missing_counts()? {
            *missingness.entry(field).or_insert(0) += count;
        }
        run_hashes.push(format!(
            "{}:{}",
            corpus.report().run_key,
            corpus.report().canonical_sha256
        ));
    }
    run_hashes.sort();
    let mut by_instrument = BTreeMap::new();
    for record in &records {
        *by_instrument.entry(record.instrument.clone()).or_insert(0) += 1;
    }
    let minimum_instrument_objects = by_instrument.values().copied().min().unwrap_or(0);
    let mut assignments = Vec::with_capacity(records.len() * 3);
    let mut fitted = Vec::with_capacity(6);
    let kinds = vec![
        analyze_kind(
            &records,
            ObjectKind::Compression,
            &mut assignments,
            &mut fitted,
        ),
        analyze_kind(
            &records,
            ObjectKind::Expansion,
            &mut assignments,
            &mut fitted,
        ),
    ];
    let common_family_support = kinds.iter().all(|kind| {
        kind.views
            .iter()
            .find(|view| view.representation == "hybrid_mirrored_v1")
            .is_some_and(|view| {
                view.families
                    .iter()
                    .filter(|family| family.common_support)
                    .count()
                    >= 2
            })
    });
    let exit_checks = Gate15ExitChecks {
        minimum_total_objects: records.len() >= 600,
        minimum_objects_per_instrument: minimum_instrument_objects >= 70,
        minimum_eligible_per_object_kind: kinds.iter().all(|kind| kind.eligible >= 240),
        common_family_support,
        within_view_stability: kinds
            .iter()
            .all(|kind| kind.views.iter().all(|view| view.mean_seed_ari >= 0.65)),
        cross_view_agreement: kinds.iter().all(|kind| kind.mean_cross_view_ari >= 0.35),
        raw_generation_consistent: corpora
            .iter()
            .all(|corpus| corpus.report().contract == "NORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1"),
    };
    let pass = exit_checks.minimum_total_objects
        && exit_checks.minimum_objects_per_instrument
        && exit_checks.minimum_eligible_per_object_kind
        && exit_checks.common_family_support
        && exit_checks.within_view_stability
        && exit_checks.cross_view_agreement
        && exit_checks.raw_generation_consistent;
    let report = Gate15Report {
        contract: "NORTHSTAR_RG3_GATE15_DISCOVERY_REPORT_V1".into(),
        status: if pass { "PASS" } else { "COLLECT_MORE" }.into(),
        epistemic_status: "CANDIDATE_ONLY_NOT_MARKET_TRUTH".into(),
        research_generation: 3,
        recipe_id: RECIPE.into(),
        recipe_sha256: format!("{:x}", Sha256::digest(RECIPE.as_bytes())),
        source_run_count: corpora.len(),
        source_run_hashes: run_hashes,
        total_objects: records.len(),
        objects_by_instrument: by_instrument,
        minimum_instrument_objects,
        missingness,
        kinds,
        exit_checks,
        limitations: vec![
            "Candidate IDs are representation-dependent statistical partitions, not discovered market truth.".into(),
            "Censored objects remain in coverage denominators but are not forced into terminal trajectory families.".into(),
            "Centroid silhouette is a compact selection diagnostic and is not a probabilistic confidence.".into(),
            "No trading, profitability, entry, exit, or directional-value label is present.".into(),
        ],
    };
    Ok((report, assignments, fitted))
}

fn feature_order(kind: ObjectKind, view: &str) -> Vec<String> {
    let summary: &[&str] = match kind {
        ObjectKind::Compression => &[
            "log_sample_count",
            "seed_width_atr",
            "terminal_width_seed",
            "mid_migration_atr",
            "escape_count",
            "terminal_close_mid_atr",
        ],
        ObjectKind::Expansion => &[
            "log_sample_count",
            "terminal_displacement_atr",
            "max_displacement_atr",
            "opposite_displacement_atr",
            "return_depth_max",
            "path_efficiency",
            "containment_extension_atr",
        ],
    };
    if view == "summary_geometry_v1" {
        return summary.iter().map(|name| (*name).into()).collect();
    }
    let (points, channels): (usize, &[&str]) = match kind {
        ObjectKind::Compression => (16, &["width_seed", "mid_migration_atr", "close_mid_atr"]),
        ObjectKind::Expansion => (32, &["displacement_atr", "velocity_atr_bar"]),
    };
    let mut shape = Vec::with_capacity(points * channels.len());
    for point in 0..points {
        for channel in channels {
            shape.push(format!("t{point:02}_{channel}"));
        }
    }
    if view == "resampled_shape_v1" {
        return shape;
    }
    summary
        .iter()
        .map(|name| format!("summary_{name}"))
        .chain(shape.into_iter().map(|name| format!("mirrored_{name}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjusted_rand_is_label_permutation_invariant() {
        assert!((adjusted_rand(&[0, 0, 1, 1], &[7, 7, 3, 3]) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn simd_distance_matches_scalar_truth() {
        let left: Vec<_> = (0..19).map(|value| value as f32 * 0.25).collect();
        let right: Vec<_> = (0..19).map(|value| value as f32 * -0.5).collect();
        let truth: f32 = left.iter().zip(&right).map(|(a, b)| (a - b).powi(2)).sum();
        assert!((cluster::squared_distance(&left, &right) - truth).abs() < 1e-4);
    }

    #[test]
    fn normalized_age_resampling_preserves_endpoints() {
        let values = [(2.0, [1.0, 2.0]), (5.0, [7.0, 8.0])];
        assert_eq!(resample(&values, 3), vec![1.0, 2.0, 4.0, 5.0, 7.0, 8.0]);
    }
}

use crate::gate16::Gate16Reference;
use crate::gate16_compare::{PreparedObject, PrimarySpec, compare};
use crate::gate16_distance::{
    bounded_dtw, mixed_gower, robust_l1, robust_l2, squared_l2_scalar, squared_l2_simd,
    trajectory_derivative_aware, trajectory_pointwise, typed_graph_distance, wl2_graph_distance,
};
use crate::gate16_types::{
    CapabilityProfile, CollisionDiagnostic, DistanceProbe, LocalGeometryDiagnostic,
    MetricDiagnostic, NeighborRow, NeighborhoodAgreement, RetrospectiveDiagnostic,
};
use std::collections::{BTreeMap, BTreeSet};

fn quantile(mut values: Vec<f64>, numerator: usize, denominator: usize) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[((values.len() - 1) * numerator) / denominator])
}

fn admitted(spec: PrimarySpec, object: &PreparedObject) -> bool {
    match spec {
        PrimarySpec::SummaryCanonical => object.summary_canonical.is_some(),
        PrimarySpec::TrajectoryRawComplete => object.trajectory_raw_full.is_some(),
        PrimarySpec::TrajectoryCanonicalComplete => object.trajectory_canonical_full.is_some(),
        PrimarySpec::TrajectoryCanonicalSharedPrefix
        | PrimarySpec::EventCanonicalObserved
        | PrimarySpec::GraphCanonicalObserved => true,
    }
}

pub(crate) fn metric_diagnostics(objects: &[PreparedObject]) -> Vec<MetricDiagnostic> {
    let mut output = Vec::new();
    for spec in PrimarySpec::ALL {
        let sample: Vec<_> = objects
            .iter()
            .filter(|object| admitted(spec, object))
            .take(32)
            .collect();
        let mut identity = true;
        let mut nonnegative = true;
        let mut symmetric = true;
        let mut pairs = 0usize;
        let mut maximum_symmetry_error = 0.0f64;
        for (left_index, left) in sample.iter().enumerate() {
            if let Ok(value) = compare(spec, left, left) {
                identity &= value.distance.abs() <= 1e-9;
            }
            for right in sample.iter().skip(left_index + 1) {
                let (Ok(lr), Ok(rl)) = (compare(spec, left, right), compare(spec, right, left))
                else {
                    continue;
                };
                pairs += 1;
                nonnegative &= lr.distance >= 0.0 && lr.distance.is_finite();
                let error = (lr.distance - rl.distance).abs();
                maximum_symmetry_error = maximum_symmetry_error.max(error);
                symmetric &= error <= 1e-9;
            }
        }
        let mut triplets = 0usize;
        let mut maximum_triangle_violation = 0.0f64;
        let triangle_sample = sample.iter().take(16).copied().collect::<Vec<_>>();
        for (a_index, a) in triangle_sample.iter().enumerate() {
            for (b_index, b) in triangle_sample.iter().enumerate() {
                if b_index == a_index {
                    continue;
                }
                for (c_index, c) in triangle_sample.iter().enumerate() {
                    if c_index == a_index || c_index == b_index {
                        continue;
                    }
                    let (Ok(ab), Ok(bc), Ok(ac)) = (
                        compare(spec, a, b),
                        compare(spec, b, c),
                        compare(spec, a, c),
                    ) else {
                        continue;
                    };
                    triplets += 1;
                    maximum_triangle_violation = maximum_triangle_violation
                        .max((ac.distance - ab.distance - bc.distance).max(0.0));
                }
            }
        }
        let triangle = if matches!(spec, PrimarySpec::TrajectoryCanonicalSharedPrefix) {
            "NOT_CLAIMED_PAIR_DEPENDENT_SUPPORT"
        } else if maximum_triangle_violation <= 1e-8 {
            "NO_VIOLATION_IN_DETERMINISTIC_SAMPLE"
        } else {
            "NONMETRIC_OR_VIOLATION_OBSERVED"
        };
        output.push(MetricDiagnostic {
            representation_id: spec.representation().into(),
            distance_contract: spec.distance().into(),
            comparison_mode: spec.mode().into(),
            identity: if identity { "PASS" } else { "FAIL" }.into(),
            nonnegative: if nonnegative { "PASS" } else { "FAIL" }.into(),
            symmetric: if symmetric { "PASS" } else { "FAIL" }.into(),
            triangle_inequality: triangle.into(),
            tested_pairs: pairs,
            tested_triplets: triplets,
            maximum_symmetry_error,
            maximum_triangle_violation,
        });
    }
    output
}

pub(crate) fn distance_probes(objects: &[PreparedObject]) -> Vec<DistanceProbe> {
    let mut output = Vec::new();
    for kind in ["COMPRESSION", "EXPANSION"] {
        let pair = objects
            .iter()
            .filter(|object| object.object.kind.name() == kind && object.object.complete())
            .take(2)
            .collect::<Vec<_>>();
        if pair.len() != 2 {
            continue;
        }
        let (left, right) = (pair[0], pair[1]);
        let mut emit =
            |representation: &str, distance: &str, mode: &str, identity: f64, lr: f64, rl: f64| {
                let symmetry = (lr - rl).abs();
                output.push(DistanceProbe {
                    object_kind: kind.into(),
                    representation_id: representation.into(),
                    distance_contract: distance.into(),
                    comparison_mode: mode.into(),
                    left_object_key: left.object.key(),
                    right_object_key: right.object.key(),
                    identity_distance: identity,
                    pair_distance: lr,
                    symmetry_error: symmetry,
                    status: if identity.abs() <= 1e-8
                        && lr.is_finite()
                        && rl.is_finite()
                        && symmetry <= 1e-8
                    {
                        "PASS"
                    } else {
                        "FAIL"
                    }
                    .into(),
                });
            };
        let ls = left.summary_canonical.as_ref().expect("complete summary");
        let rs = right.summary_canonical.as_ref().expect("complete summary");
        let lsr = left.summary_raw.as_ref().expect("complete raw summary");
        let rsr = right.summary_raw.as_ref().expect("complete raw summary");
        emit(
            "SUMMARY_RAW_V1",
            "ROBUST_L2_V1",
            "COMPLETE",
            robust_l2(lsr, lsr),
            robust_l2(lsr, rsr),
            robust_l2(rsr, lsr),
        );
        emit(
            "SUMMARY_RAW_V1",
            "MIXED_GOWER_V1",
            "COMPLETE",
            mixed_gower(
                lsr,
                lsr,
                &left.object.categories_raw,
                &left.object.categories_raw,
            ),
            mixed_gower(
                lsr,
                rsr,
                &left.object.categories_raw,
                &right.object.categories_raw,
            ),
            mixed_gower(
                rsr,
                lsr,
                &right.object.categories_raw,
                &left.object.categories_raw,
            ),
        );
        emit(
            "SUMMARY_CANONICAL_V1",
            "ROBUST_L1_V1",
            "COMPLETE",
            robust_l1(ls, ls),
            robust_l1(ls, rs),
            robust_l1(rs, ls),
        );
        emit(
            "SUMMARY_CANONICAL_V1",
            "ROBUST_L2_V1",
            "COMPLETE",
            robust_l2(ls, ls),
            robust_l2(ls, rs),
            robust_l2(rs, ls),
        );
        emit(
            "SUMMARY_CANONICAL_V1",
            "MIXED_GOWER_V1",
            "COMPLETE",
            mixed_gower(
                ls,
                ls,
                &left.object.categories_canonical,
                &left.object.categories_canonical,
            ),
            mixed_gower(
                ls,
                rs,
                &left.object.categories_canonical,
                &right.object.categories_canonical,
            ),
            mixed_gower(
                rs,
                ls,
                &right.object.categories_canonical,
                &left.object.categories_canonical,
            ),
        );
        let lt = left
            .trajectory_canonical_full
            .as_ref()
            .expect("complete trajectory");
        let rt = right
            .trajectory_canonical_full
            .as_ref()
            .expect("complete trajectory");
        emit(
            "TRAJECTORY_CANONICAL_U21_V1",
            "POINTWISE_L2_V1",
            "COMPLETE",
            trajectory_pointwise(lt, lt),
            trajectory_pointwise(lt, rt),
            trajectory_pointwise(rt, lt),
        );
        emit(
            "TRAJECTORY_CANONICAL_U21_V1",
            "DERIVATIVE_AWARE_L2_V1",
            "COMPLETE",
            trajectory_derivative_aware(lt, lt),
            trajectory_derivative_aware(lt, rt),
            trajectory_derivative_aware(rt, lt),
        );
        emit(
            "TRAJECTORY_CANONICAL_U21_V1",
            "BOUNDED_DTW_W2_V1",
            "COMPLETE",
            bounded_dtw(lt, lt),
            bounded_dtw(lt, rt),
            bounded_dtw(rt, lt),
        );
        emit(
            "INTRINSIC_TYPED_PROCESS_GRAPH_RAW_V1",
            "WL2_MULTISET_JACCARD_V1",
            "OBSERVED",
            wl2_graph_distance(&left.graph_raw, &left.graph_raw),
            wl2_graph_distance(&left.graph_raw, &right.graph_raw),
            wl2_graph_distance(&right.graph_raw, &left.graph_raw),
        );
        emit(
            "INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1",
            "TYPED_MULTISET_JACCARD_V1",
            "OBSERVED",
            typed_graph_distance(&left.graph_canonical, &left.graph_canonical),
            typed_graph_distance(&left.graph_canonical, &right.graph_canonical),
            typed_graph_distance(&right.graph_canonical, &left.graph_canonical),
        );
        emit(
            "INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1",
            "WL2_MULTISET_JACCARD_V1",
            "OBSERVED",
            wl2_graph_distance(&left.graph_canonical, &left.graph_canonical),
            wl2_graph_distance(&left.graph_canonical, &right.graph_canonical),
            wl2_graph_distance(&right.graph_canonical, &left.graph_canonical),
        );
        let scalar = squared_l2_scalar(ls, rs);
        let simd = squared_l2_simd(ls, rs);
        emit(
            "SUMMARY_CANONICAL_V1",
            "SIMD_SCALAR_SQUARED_L2_PARITY_V1",
            "DIAGNOSTIC",
            0.0,
            scalar,
            simd,
        );
        output.last_mut().expect("parity probe").status = if (scalar - simd).abs() <= 1e-3 {
            "PASS"
        } else {
            "FAIL"
        }
        .into();
    }
    output
}

fn rows_for(spec: PrimarySpec, rows: &[NeighborRow]) -> Vec<&NeighborRow> {
    rows.iter()
        .filter(|row| {
            row.representation_id == spec.representation()
                && row.distance_contract == spec.distance()
                && row.comparison_mode == spec.mode()
        })
        .collect()
}

pub(crate) fn collision_diagnostics(
    objects: &[PreparedObject],
    neighbors: &[NeighborRow],
) -> Vec<CollisionDiagnostic> {
    let history: BTreeMap<_, _> = objects
        .iter()
        .map(|object| {
            (
                object.object.key(),
                object.object.raw_history_sha256.as_str(),
            )
        })
        .collect();
    PrimarySpec::ALL
        .into_iter()
        .map(|spec| {
            let eligible = objects
                .iter()
                .filter(|object| admitted(spec, object))
                .count();
            let comparable_pairs = eligible.saturating_mul(eligible.saturating_sub(1)) / 2;
            let mut seen = BTreeSet::new();
            let mut examples = Vec::new();
            let mut material = 0usize;
            for row in rows_for(spec, neighbors) {
                if row.distance > 1e-7 {
                    continue;
                }
                let pair = if row.object_key < row.neighbor_key {
                    (row.object_key.clone(), row.neighbor_key.clone())
                } else {
                    (row.neighbor_key.clone(), row.object_key.clone())
                };
                if seen.insert(pair.clone()) {
                    if history.get(&pair.0) != history.get(&pair.1) {
                        material += 1;
                    }
                    if examples.len() < 12 {
                        examples.push([pair.0, pair.1]);
                    }
                }
            }
            CollisionDiagnostic {
                representation_id: spec.representation().into(),
                distance_contract: spec.distance().into(),
                comparison_mode: spec.mode().into(),
                eligible_objects: eligible,
                comparable_pairs,
                collision_pairs: seen.len(),
                materially_distinct_history_collisions: material,
                epsilon: 1e-7,
                audit_scope: "EXACT_WITHIN_EMITTED_TOP_10_NEIGHBOR_EDGES_NOT_ALL_PAIRS".into(),
                examples,
            }
        })
        .collect()
}

pub(crate) fn local_geometry(
    objects: &[PreparedObject],
    neighbors: &[NeighborRow],
) -> Vec<LocalGeometryDiagnostic> {
    PrimarySpec::ALL
        .into_iter()
        .map(|spec| {
            let rows = rows_for(spec, neighbors);
            let eligible = objects
                .iter()
                .filter(|object| admitted(spec, object))
                .count();
            let mut by_object = BTreeMap::<&str, Vec<&NeighborRow>>::new();
            let mut hubs = BTreeMap::<&str, usize>::new();
            for row in &rows {
                by_object.entry(&row.object_key).or_default().push(row);
                *hubs.entry(&row.neighbor_key).or_default() += 1;
            }
            let nearest = by_object
                .values()
                .filter_map(|rows| {
                    rows.iter()
                        .min_by_key(|row| row.rank)
                        .map(|row| row.distance)
                })
                .collect::<Vec<_>>();
            let kth = by_object
                .values()
                .filter_map(|rows| {
                    rows.iter()
                        .find(|row| row.rank == 10)
                        .map(|row| row.distance)
                })
                .collect::<Vec<_>>();
            let sampled = rows
                .iter()
                .enumerate()
                .filter(|(index, _)| index % 7 == 0)
                .map(|(_, row)| row.distance)
                .collect::<Vec<_>>();
            let p10 = quantile(sampled.clone(), 1, 10);
            let p90 = quantile(sampled.clone(), 9, 10);
            LocalGeometryDiagnostic {
                representation_id: spec.representation().into(),
                distance_contract: spec.distance().into(),
                comparison_mode: spec.mode().into(),
                eligible_objects: eligible,
                objects_with_k_neighbors: kth.len(),
                nearest_p10: quantile(nearest.clone(), 1, 10),
                nearest_median: quantile(nearest.clone(), 1, 2),
                nearest_p90: quantile(nearest, 9, 10),
                kth_median: quantile(kth, 1, 2),
                pair_distance_p10: p10,
                pair_distance_median: quantile(sampled.clone(), 1, 2),
                pair_distance_p90: p90,
                distance_concentration_ratio: p10.zip(p90).map(|(low, high)| {
                    if high.abs() <= f64::EPSILON {
                        0.0
                    } else {
                        low / high
                    }
                }),
                maximum_hub_count: hubs.values().copied().max().unwrap_or(0),
                isolated_object_rate: if eligible == 0 {
                    0.0
                } else {
                    (eligible.saturating_sub(by_object.len())) as f64 / eligible as f64
                },
                distance_sample_scope: "EMITTED_TOP_10_NEIGHBOR_EDGES_WITH_DETERMINISTIC_EVERY_SEVENTH_PAIR_QUANTILE_SAMPLE".into(),
            }
        })
        .collect()
}

fn rank_correlation(left: &BTreeMap<&str, usize>, right: &BTreeMap<&str, usize>) -> Option<f64> {
    let common = left
        .keys()
        .copied()
        .filter(|key| right.contains_key(key))
        .collect::<Vec<_>>();
    if common.len() < 2 {
        return None;
    }
    let mean_left = common.iter().map(|key| left[key] as f64).sum::<f64>() / common.len() as f64;
    let mean_right = common.iter().map(|key| right[key] as f64).sum::<f64>() / common.len() as f64;
    let mut covariance = 0.0;
    let mut variance_left = 0.0;
    let mut variance_right = 0.0;
    for key in common {
        let a = left[key] as f64 - mean_left;
        let b = right[key] as f64 - mean_right;
        covariance += a * b;
        variance_left += a * a;
        variance_right += b * b;
    }
    let denominator = (variance_left * variance_right).sqrt();
    (denominator > f64::EPSILON).then_some(covariance / denominator)
}

pub(crate) fn neighborhood_agreement(
    objects: &[PreparedObject],
    neighbors: &[NeighborRow],
) -> Vec<NeighborhoodAgreement> {
    let kinds: BTreeMap<_, _> = objects
        .iter()
        .map(|object| (object.object.key(), object.object.kind.name()))
        .collect();
    let mut by_spec = BTreeMap::<String, BTreeMap<String, Vec<&NeighborRow>>>::new();
    for spec in PrimarySpec::ALL {
        let mut objects = BTreeMap::<String, Vec<&NeighborRow>>::new();
        for row in rows_for(spec, neighbors) {
            objects.entry(row.object_key.clone()).or_default().push(row);
        }
        by_spec.insert(spec.id(), objects);
    }
    let mut output = Vec::new();
    for (left_index, left_spec) in PrimarySpec::ALL.iter().enumerate() {
        for right_spec in PrimarySpec::ALL.iter().skip(left_index + 1) {
            for kind in ["COMPRESSION", "EXPANSION"] {
                let left = &by_spec[&left_spec.id()];
                let right = &by_spec[&right_spec.id()];
                let mut jaccard = Vec::new();
                let mut ranks = Vec::new();
                for (key, left_rows) in left {
                    if kinds.get(key).copied() != Some(kind) {
                        continue;
                    }
                    let Some(right_rows) = right.get(key) else {
                        continue;
                    };
                    let left_set = left_rows
                        .iter()
                        .map(|row| row.neighbor_key.as_str())
                        .collect::<BTreeSet<_>>();
                    let right_set = right_rows
                        .iter()
                        .map(|row| row.neighbor_key.as_str())
                        .collect::<BTreeSet<_>>();
                    let union = left_set.union(&right_set).count();
                    if union == 0 {
                        continue;
                    }
                    jaccard.push(left_set.intersection(&right_set).count() as f64 / union as f64);
                    let left_rank = left_rows
                        .iter()
                        .map(|row| (row.neighbor_key.as_str(), row.rank))
                        .collect::<BTreeMap<_, _>>();
                    let right_rank = right_rows
                        .iter()
                        .map(|row| (row.neighbor_key.as_str(), row.rank))
                        .collect::<BTreeMap<_, _>>();
                    if let Some(value) = rank_correlation(&left_rank, &right_rank) {
                        ranks.push(value);
                    }
                }
                let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
                output.push(NeighborhoodAgreement {
                    object_kind: kind.into(),
                    left_representation: left_spec.id(),
                    right_representation: right_spec.id(),
                    comparable_objects: jaccard.len(),
                    mean_top_k_jaccard: mean(&jaccard),
                    mean_shared_rank_correlation: (!ranks.is_empty()).then(|| mean(&ranks)),
                    interpretation: "LOCAL_GEOMETRY_CORRESPONDENCE_ONLY_NOT_TAXONOMY".into(),
                });
            }
        }
    }
    output
}

pub(crate) fn capability_profiles(agreements: &[NeighborhoodAgreement]) -> Vec<CapabilityProfile> {
    let graph_sequence = agreements
        .iter()
        .filter(|row| {
            row.left_representation.contains("EVENT_SEQUENCE")
                && row.right_representation.contains("GRAPH")
        })
        .map(|row| row.mean_top_k_jaccard)
        .collect::<Vec<_>>();
    let graph_status = if graph_sequence.iter().all(|value| *value >= 0.85) {
        "TYPED_REENCODING_NO_INCREMENTAL_GEOMETRY_CLAIM"
    } else {
        "LOCAL_NEIGHBORHOODS_DIFFER_INCREMENTAL_CAUSE_UNRESOLVED"
    };
    vec![
        CapabilityProfile {
            representation_id: "SUMMARY_RAW_AND_CANONICAL_V1".into(),
            preserves: vec!["endpoint morphology".into(), "duration and counts".into()],
            destroys_or_omits: vec!["within-life ordering".into(), "path recurrence".into()],
            comparison_domain: "completed objects only".into(),
            formal_status: "FIXED_WIDTH_PARTIAL_GEOMETRY".into(),
            packed_bytes_per_object: Some(40.0),
            graph_incremental_information_status: None,
        },
        CapabilityProfile {
            representation_id: "TRAJECTORY_RAW_AND_CANONICAL_U21_V1".into(),
            preserves: vec!["continuous path shape".into(), "as-of state path".into()],
            destroys_or_omits: vec![
                "absolute full-life duration in normalized shape".into(),
                "events between grid points".into(),
            ],
            comparison_domain: "completed full-life or causal shared prefix".into(),
            formal_status: "RESAMPLED_PARTIAL_GEOMETRY".into(),
            packed_bytes_per_object: None,
            graph_incremental_information_status: None,
        },
        CapabilityProfile {
            representation_id: "EVENT_SEQUENCE_RAW_AND_CANONICAL_V1".into(),
            preserves: vec![
                "event order".into(),
                "multiplicity".into(),
                "inter-event time".into(),
            ],
            destroys_or_omits: vec!["continuous path between events".into()],
            comparison_domain: "observed event prefixes".into(),
            formal_status: "TYPED_NON_EUCLIDEAN_DISSIMILARITY".into(),
            packed_bytes_per_object: None,
            graph_incremental_information_status: None,
        },
        CapabilityProfile {
            representation_id: "INTRINSIC_TYPED_PROCESS_GRAPH_RAW_AND_CANONICAL_V1".into(),
            preserves: vec![
                "typed event/state relations".into(),
                "terminal relation".into(),
            ],
            destroys_or_omits: vec![
                "unemitted branch/merge topology".into(),
                "Master context".into(),
            ],
            comparison_domain: "observed intrinsic graph".into(),
            formal_status: "WL_MULTISET_DISSIMILARITY".into(),
            packed_bytes_per_object: None,
            graph_incremental_information_status: Some(graph_status.into()),
        },
    ]
}

fn diagnostic_family<'a>(reference: &'a Gate16Reference, representation: &str) -> Option<&'a str> {
    if representation == "SUMMARY_CANONICAL_V1" {
        reference.summary_family.as_deref()
    } else if representation == "TRAJECTORY_RAW_U21_V1" {
        reference.shape_family.as_deref()
    } else {
        reference.hybrid_family.as_deref()
    }
}

pub(crate) fn retrospective(
    objects: &[PreparedObject],
    neighbors: &[NeighborRow],
    references: &BTreeMap<String, Gate16Reference>,
) -> Vec<RetrospectiveDiagnostic> {
    let object_map: BTreeMap<_, _> = objects
        .iter()
        .map(|object| (object.object.key(), object))
        .collect();
    let mut family_matches = 0usize;
    let mut family_total = 0usize;
    let mut same_stratum = Vec::new();
    let mut different_stratum = Vec::new();
    for row in neighbors
        .iter()
        .filter(|row| row.rank == 1 && row.comparison_mode == "COMPLETE")
    {
        let (Some(left), Some(right)) = (
            references.get(&row.object_key),
            references.get(&row.neighbor_key),
        ) else {
            continue;
        };
        if let (Some(a), Some(b)) = (
            diagnostic_family(left, &row.representation_id),
            diagnostic_family(right, &row.representation_id),
        ) {
            family_total += 1;
            family_matches += usize::from(a == b);
        }
        if row.representation_id.contains("TRAJECTORY_CANONICAL") {
            if left.structural_stratum == right.structural_stratum
                && left.structural_stratum != "NULL_STRUCTURAL_CONTEXT"
            {
                same_stratum.push(row.distance);
            } else {
                different_stratum.push(row.distance);
            }
        }
    }
    let summary_null = references
        .values()
        .filter(|row| row.summary_family.as_deref() == Some("NULL"))
        .count();
    let censored = object_map
        .values()
        .filter(|object| object.object.censored)
        .count();
    let instruments = object_map
        .values()
        .map(|object| object.object.instrument.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let observed_duration_seconds = object_map
        .values()
        .map(|object| u64::from(object.object.duration_seconds()))
        .sum::<u64>();
    vec![
        RetrospectiveDiagnostic {
            question: "Do Gate 15 exploratory labels align with Gate 16 nearest neighborhoods?"
                .into(),
            population: family_total,
            result: format!(
                "nearest-neighbor same-local-family rate={:.6};instrument_support={instruments};observed_object_seconds={observed_duration_seconds}",
                family_matches as f64 / family_total.max(1) as f64,
            ),
            interpretation: "DIAGNOSTIC_PROBE_ONLY_PARAMETERS_WERE_FROZEN_FIRST".into(),
            optimization_target: false,
        },
        RetrospectiveDiagnostic {
            question: "Does Master point-state SPACE organize canonical trajectory neighborhoods?"
                .into(),
            population: same_stratum.len() + different_stratum.len(),
            result: format!(
                "same_stratum_median={:?};different_stratum_median={:?}",
                quantile(same_stratum, 1, 2),
                quantile(different_stratum, 1, 2)
            ),
            interpretation: "POINT_STATE_CONDITIONED_ONLY_AUCTION_INTERVALS_NOT_EVALUABLE".into(),
            optimization_target: false,
        },
        RetrospectiveDiagnostic {
            question: "Do summary NULL assignments remain explicit?".into(),
            population: references.len(),
            result: format!("null_reference_count={summary_null}"),
            interpretation: "NULL_IS_NOT_A_FAMILY".into(),
            optimization_target: false,
        },
        RetrospectiveDiagnostic {
            question: "Can censored objects enter complete-life geometry?".into(),
            population: censored,
            result: "zero admitted by construction".into(),
            interpretation: "CENSORED_SUFFIX_REMAINS_TYPED_NONCOMPARABILITY".into(),
            optimization_target: false,
        },
    ]
}

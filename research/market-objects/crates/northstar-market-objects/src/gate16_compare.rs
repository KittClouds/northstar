use crate::gate16_distance::{
    robust_l2, shared_prefix_pointwise, trajectory_pointwise, typed_edit, wl2_graph_distance,
};
use crate::gate16_graph::{GraphSignature, graph_signature};
use crate::gate16_repr::{ResampledTrajectory, full_trajectory, standardized_summary};
use crate::gate16_types::{
    ComparisonReceipt, NEIGHBOR_K, NeighborRow, PartialDistanceCoordinate,
    PartialDistanceVectorRow, ProcessGeometryObject, RobustScale,
};
use hashbrown::HashMap;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrimarySpec {
    SummaryCanonical,
    TrajectoryRawComplete,
    TrajectoryCanonicalComplete,
    TrajectoryCanonicalSharedPrefix,
    EventCanonicalObserved,
    GraphCanonicalObserved,
}

impl PrimarySpec {
    pub(crate) const ALL: [Self; 6] = [
        Self::SummaryCanonical,
        Self::TrajectoryRawComplete,
        Self::TrajectoryCanonicalComplete,
        Self::TrajectoryCanonicalSharedPrefix,
        Self::EventCanonicalObserved,
        Self::GraphCanonicalObserved,
    ];

    pub(crate) fn representation(self) -> &'static str {
        match self {
            Self::SummaryCanonical => "SUMMARY_CANONICAL_V1",
            Self::TrajectoryRawComplete => "TRAJECTORY_RAW_U21_V1",
            Self::TrajectoryCanonicalComplete | Self::TrajectoryCanonicalSharedPrefix => {
                "TRAJECTORY_CANONICAL_U21_V1"
            }
            Self::EventCanonicalObserved => "EVENT_SEQUENCE_CANONICAL_V1",
            Self::GraphCanonicalObserved => "INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1",
        }
    }

    pub(crate) fn distance(self) -> &'static str {
        match self {
            Self::SummaryCanonical => "ROBUST_L2_V1",
            Self::TrajectoryRawComplete
            | Self::TrajectoryCanonicalComplete
            | Self::TrajectoryCanonicalSharedPrefix => "POINTWISE_L2_V1",
            Self::EventCanonicalObserved => "TYPED_EDIT_DURATION_AWARE_V1",
            Self::GraphCanonicalObserved => "WL2_MULTISET_JACCARD_V1",
        }
    }

    pub(crate) fn mode(self) -> &'static str {
        match self {
            Self::SummaryCanonical
            | Self::TrajectoryRawComplete
            | Self::TrajectoryCanonicalComplete => "COMPLETE",
            Self::TrajectoryCanonicalSharedPrefix => "SHARED_PREFIX",
            Self::EventCanonicalObserved | Self::GraphCanonicalObserved => "OBSERVED",
        }
    }

    pub(crate) fn id(self) -> String {
        format!(
            "{}::{}::{}",
            self.representation(),
            self.distance(),
            self.mode()
        )
    }
}

#[derive(Clone)]
pub(crate) struct PreparedObject {
    pub object: ProcessGeometryObject,
    pub summary_raw: Option<Vec<f32>>,
    pub summary_canonical: Option<Vec<f32>>,
    pub trajectory_raw_full: Option<ResampledTrajectory>,
    pub trajectory_canonical_full: Option<ResampledTrajectory>,
    pub graph_raw: GraphSignature,
    pub graph_canonical: GraphSignature,
}

pub(crate) struct Compared {
    pub distance: f64,
    pub support_points: usize,
    pub support_bars: u32,
    pub support_seconds: u32,
    pub observed_fraction_left: f64,
    pub observed_fraction_right: f64,
}

pub(crate) fn prepare(
    objects: Vec<ProcessGeometryObject>,
    scales: &[RobustScale],
) -> Vec<PreparedObject> {
    let scale_map: HashMap<_, _> = scales
        .iter()
        .map(|scale| {
            (
                (scale.object_kind.as_str(), scale.representation_id.as_str()),
                scale,
            )
        })
        .collect();
    objects
        .into_iter()
        .map(|object| {
            let summary_raw = object.complete().then(|| {
                standardized_summary(
                    &object,
                    false,
                    scale_map[&(object.kind.name(), "SUMMARY_RAW_V1")],
                )
            });
            let summary_canonical = object.complete().then(|| {
                standardized_summary(
                    &object,
                    true,
                    scale_map[&(object.kind.name(), "SUMMARY_CANONICAL_V1")],
                )
            });
            let trajectory_raw_full = full_trajectory(&object, false);
            let trajectory_canonical_full = full_trajectory(&object, true);
            let graph_raw = graph_signature(&object, false);
            let graph_canonical = graph_signature(&object, true);
            PreparedObject {
                object,
                summary_raw,
                summary_canonical,
                trajectory_raw_full,
                trajectory_canonical_full,
                graph_raw,
                graph_canonical,
            }
        })
        .collect()
}

pub(crate) fn compare(
    spec: PrimarySpec,
    left: &PreparedObject,
    right: &PreparedObject,
) -> Result<Compared, &'static str> {
    if left.object.kind != right.object.kind {
        return Err("NOT_APPLICABLE_OBJECT_KIND");
    }
    let observed_fraction = |object: &ProcessGeometryObject, support: u32| {
        if object.observed_bars() == 0 {
            0.0
        } else {
            support as f64 / object.observed_bars() as f64
        }
    };
    match spec {
        PrimarySpec::SummaryCanonical => {
            let Some(left_values) = &left.summary_canonical else {
                return Err("CENSORED_SUFFIX_LEFT");
            };
            let Some(right_values) = &right.summary_canonical else {
                return Err("CENSORED_SUFFIX_RIGHT");
            };
            Ok(Compared {
                distance: robust_l2(left_values, right_values),
                support_points: left_values.len(),
                support_bars: left
                    .object
                    .observed_bars()
                    .min(right.object.observed_bars()),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
                observed_fraction_left: 1.0,
                observed_fraction_right: 1.0,
            })
        }
        PrimarySpec::TrajectoryRawComplete => {
            let Some(left_values) = &left.trajectory_raw_full else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP_LEFT");
            };
            let Some(right_values) = &right.trajectory_raw_full else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP_RIGHT");
            };
            Ok(Compared {
                distance: trajectory_pointwise(left_values, right_values),
                support_points: left_values.points,
                support_bars: left_values.horizon_bars.min(right_values.horizon_bars),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
                observed_fraction_left: 1.0,
                observed_fraction_right: 1.0,
            })
        }
        PrimarySpec::TrajectoryCanonicalComplete => {
            let Some(left_values) = &left.trajectory_canonical_full else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP_LEFT");
            };
            let Some(right_values) = &right.trajectory_canonical_full else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP_RIGHT");
            };
            Ok(Compared {
                distance: trajectory_pointwise(left_values, right_values),
                support_points: left_values.points,
                support_bars: left_values.horizon_bars.min(right_values.horizon_bars),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
                observed_fraction_left: 1.0,
                observed_fraction_right: 1.0,
            })
        }
        PrimarySpec::TrajectoryCanonicalSharedPrefix => {
            let Some((distance, support_points, horizon, support_seconds)) =
                shared_prefix_pointwise(&left.object, &right.object, true)
            else {
                return Err("INSUFFICIENT_SHARED_PREFIX");
            };
            Ok(Compared {
                distance,
                support_points,
                support_bars: horizon,
                support_seconds,
                observed_fraction_left: observed_fraction(&left.object, horizon),
                observed_fraction_right: observed_fraction(&right.object, horizon),
            })
        }
        PrimarySpec::EventCanonicalObserved => Ok(Compared {
            distance: typed_edit(
                &left.object.events_canonical,
                &right.object.events_canonical,
            ),
            support_points: left
                .object
                .events_canonical
                .len()
                .min(right.object.events_canonical.len()),
            support_bars: left
                .object
                .observed_bars()
                .min(right.object.observed_bars()),
            support_seconds: left
                .object
                .observed_seconds()
                .min(right.object.observed_seconds()),
            observed_fraction_left: 1.0,
            observed_fraction_right: 1.0,
        }),
        PrimarySpec::GraphCanonicalObserved => Ok(Compared {
            distance: wl2_graph_distance(&left.graph_canonical, &right.graph_canonical),
            support_points: left
                .graph_canonical
                .wl2_multiset
                .values()
                .copied()
                .sum::<u32>()
                .min(
                    right
                        .graph_canonical
                        .wl2_multiset
                        .values()
                        .copied()
                        .sum::<u32>(),
                ) as usize,
            support_bars: left
                .object
                .observed_bars()
                .min(right.object.observed_bars()),
            support_seconds: left
                .object
                .observed_seconds()
                .min(right.object.observed_seconds()),
            observed_fraction_left: 1.0,
            observed_fraction_right: 1.0,
        }),
    }
}

fn receipt(
    spec: PrimarySpec,
    left: &PreparedObject,
    right: Option<&PreparedObject>,
    compared: Result<Compared, &'static str>,
) -> ComparisonReceipt {
    match (right, compared) {
        (Some(right), Ok(value)) => ComparisonReceipt {
            left_object_key: left.object.key(),
            right_object_key: right.object.key(),
            representation_id: spec.representation().into(),
            distance_contract: spec.distance().into(),
            comparison_mode: spec.mode().into(),
            status: "AVAILABLE".into(),
            reason: "COMPARABLE_DOMAIN_ADMITTED".into(),
            distance: Some(value.distance),
            comparable_support_points: value.support_points,
            comparable_support_bars: value.support_bars,
            comparable_support_seconds: value.support_seconds,
            observed_fraction_left: value.observed_fraction_left,
            observed_fraction_right: value.observed_fraction_right,
            left_censored: left.object.censored,
            right_censored: right.object.censored,
        },
        (right, Err(reason)) => ComparisonReceipt {
            left_object_key: left.object.key(),
            right_object_key: right.map_or_else(|| "NONE".into(), |value| value.object.key()),
            representation_id: spec.representation().into(),
            distance_contract: spec.distance().into(),
            comparison_mode: spec.mode().into(),
            status: "NOT_COMPARABLE".into(),
            reason: reason.into(),
            distance: None,
            comparable_support_points: 0,
            comparable_support_bars: 0,
            comparable_support_seconds: 0,
            observed_fraction_left: 0.0,
            observed_fraction_right: 0.0,
            left_censored: left.object.censored,
            right_censored: right.is_some_and(|value| value.object.censored),
        },
        (None, Ok(_)) => unreachable!(),
    }
}

pub(crate) fn neighborhoods(
    objects: &[PreparedObject],
) -> (Vec<NeighborRow>, Vec<ComparisonReceipt>) {
    let nested: Vec<_> = PrimarySpec::ALL
        .into_par_iter()
        .map(|spec| {
            let rows: Vec<_> = objects
                .par_iter()
                .enumerate()
                .map(|(left_index, left)| {
                    let mut candidates = Vec::with_capacity(objects.len().saturating_sub(1));
                    for (right_index, right) in objects.iter().enumerate() {
                        if left_index == right_index {
                            continue;
                        }
                        if let Ok(value) = compare(spec, left, right) {
                            candidates.push((
                                value.distance,
                                right_index,
                                value.support_points,
                                value.support_bars,
                                value.support_seconds,
                            ));
                        }
                    }
                    candidates.sort_by(|left_value, right_value| {
                        left_value.0.total_cmp(&right_value.0).then_with(|| {
                            objects[left_value.1]
                                .object
                                .key()
                                .cmp(&objects[right_value.1].object.key())
                        })
                    });
                    let neighbors: Vec<_> = candidates
                        .iter()
                        .take(NEIGHBOR_K)
                        .enumerate()
                        .map(
                            |(rank, &(distance, index, support, bars, seconds))| NeighborRow {
                                object_key: left.object.key(),
                                representation_id: spec.representation().into(),
                                distance_contract: spec.distance().into(),
                                comparison_mode: spec.mode().into(),
                                rank: rank + 1,
                                neighbor_key: objects[index].object.key(),
                                distance,
                                comparable_support_points: support,
                                comparable_support_bars: bars,
                                comparable_support_seconds: seconds,
                                object_censored: left.object.censored,
                                neighbor_censored: objects[index].object.censored,
                            },
                        )
                        .collect();
                    let nearest = candidates.first().map(|value| &objects[value.1]);
                    let diagnostic = if let Some(nearest) = nearest {
                        receipt(spec, left, Some(nearest), compare(spec, left, nearest))
                    } else {
                        receipt(spec, left, None, Err("NO_ADMITTED_NEIGHBOR"))
                    };
                    (neighbors, diagnostic)
                })
                .collect();
            (spec, rows)
        })
        .collect();
    let mut neighbors = Vec::new();
    let mut receipts = Vec::new();
    for (_, rows) in nested {
        for (mut row_neighbors, diagnostic) in rows {
            neighbors.append(&mut row_neighbors);
            receipts.push(diagnostic);
        }
    }
    neighbors.sort_by(|left, right| {
        left.representation_id
            .cmp(&right.representation_id)
            .then_with(|| left.comparison_mode.cmp(&right.comparison_mode))
            .then_with(|| left.object_key.cmp(&right.object_key))
            .then_with(|| left.rank.cmp(&right.rank))
    });
    receipts.sort_by(|left, right| {
        left.representation_id
            .cmp(&right.representation_id)
            .then_with(|| left.comparison_mode.cmp(&right.comparison_mode))
            .then_with(|| left.left_object_key.cmp(&right.left_object_key))
    });
    (neighbors, receipts)
}

pub(crate) fn partial_distance_vectors(
    objects: &[PreparedObject],
) -> Vec<PartialDistanceVectorRow> {
    let anchors = objects
        .iter()
        .filter(|object| object.object.complete())
        .fold(HashMap::new(), |mut anchors, object| {
            anchors.entry(object.object.kind).or_insert(object);
            anchors
        });
    let mut output = Vec::with_capacity(objects.len());
    for object in objects {
        let Some(anchor) = anchors.get(&object.object.kind).copied() else {
            continue;
        };
        let mut coordinates = Vec::with_capacity(PrimarySpec::ALL.len());
        for spec in PrimarySpec::ALL {
            match compare(spec, object, anchor) {
                Ok(value) => coordinates.push(PartialDistanceCoordinate {
                    coordinate_id: spec.id(),
                    status: "AVAILABLE".into(),
                    reason: "COMPARABLE_DOMAIN_ADMITTED".into(),
                    distance: Some(value.distance),
                    support_points: value.support_points,
                    support_bars: value.support_bars,
                    support_seconds: value.support_seconds,
                }),
                Err(reason) => coordinates.push(PartialDistanceCoordinate {
                    coordinate_id: spec.id(),
                    status: "NOT_COMPARABLE".into(),
                    reason: reason.into(),
                    distance: None,
                    support_points: 0,
                    support_bars: 0,
                    support_seconds: 0,
                }),
            }
        }
        output.push(PartialDistanceVectorRow {
            object_key: object.object.key(),
            anchor_object_key: anchor.object.key(),
            anchor_basis: "FIRST_COMPLETE_CANONICAL_OBJECT_KEY_WITHIN_KIND".into(),
            coordinates,
        });
    }
    output.sort_by(|left, right| left.object_key.cmp(&right.object_key));
    output
}

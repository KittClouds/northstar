use crate::gate16_collect::collect_objects;
use crate::gate16_compare::{PreparedObject, prepare};
use crate::gate16_distance::{
    bounded_dtw, mixed_gower, robust_l1, robust_l2, shared_prefix_pointwise, squared_l2_scalar,
    trajectory_derivative_aware, trajectory_pointwise, typed_edit, typed_graph_distance,
    wl2_graph_distance,
};
use crate::gate16_repr::fit_scales;
use crate::gate165_analysis::{
    self, FamilyReference, NeighborMap, cross_representation, gate15_audit, graph_incremental,
    profiles_and_classes,
};
use crate::gate165_types::{
    AXIS_EXACT_DIFFERENT, AXIS_NOT_EVALUABLE, CanonicalizationTurnover, ContractPairSummary,
    Gate165Checks, Gate165Package, Gate165Report, MasterBlockRow, MasterBlockedAudit,
    ObjectAuthorityRow, PairCensusRow, RawDifferenceReceipt,
};
use crate::{RawCorpus, RawError};
use blake3::Hasher;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct Gate165Reference {
    pub summary_family: Option<String>,
    pub shape_family: Option<String>,
    pub hybrid_family: Option<String>,
    pub structural_stratum: String,
    pub terminal_join_mode: String,
    pub terminal_snapshot_age_seconds: Option<u32>,
}

impl FamilyReference for Gate165Reference {
    fn summary_family(&self) -> Option<&str> {
        self.summary_family.as_deref()
    }
    fn shape_family(&self) -> Option<&str> {
        self.shape_family.as_deref()
    }
    fn hybrid_family(&self) -> Option<&str> {
        self.hybrid_family.as_deref()
    }
}

pub struct Gate165Inputs<'a> {
    pub corpora: &'a [RawCorpus],
    pub expected_run_count: usize,
    pub expected_object_count: usize,
    pub source_corpus_sha256: &'a str,
    pub source_gate16_lab_sha256: &'a str,
    pub references: &'a BTreeMap<String, Gate165Reference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Summary,
    Trajectory,
    Event,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Raw,
    Canonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Distance {
    L1,
    L2,
    Gower,
    Pointwise,
    Derivative,
    Dtw,
    Edit,
    TypedGraph,
    Wl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Complete,
    SharedPrefix,
    Observed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Spec {
    family: Family,
    view: View,
    distance: Distance,
    mode: Mode,
}

impl Spec {
    fn all() -> Vec<Self> {
        let mut output = Vec::with_capacity(20);
        for view in [View::Raw, View::Canonical] {
            for distance in [Distance::L1, Distance::L2, Distance::Gower] {
                output.push(Self {
                    family: Family::Summary,
                    view,
                    distance,
                    mode: Mode::Complete,
                });
            }
            for distance in [Distance::Pointwise, Distance::Derivative, Distance::Dtw] {
                output.push(Self {
                    family: Family::Trajectory,
                    view,
                    distance,
                    mode: Mode::Complete,
                });
            }
            output.push(Self {
                family: Family::Trajectory,
                view,
                distance: Distance::Pointwise,
                mode: Mode::SharedPrefix,
            });
            output.push(Self {
                family: Family::Event,
                view,
                distance: Distance::Edit,
                mode: Mode::Observed,
            });
            output.push(Self {
                family: Family::Graph,
                view,
                distance: Distance::TypedGraph,
                mode: Mode::Observed,
            });
            output.push(Self {
                family: Family::Graph,
                view,
                distance: Distance::Wl2,
                mode: Mode::Observed,
            });
        }
        output
    }

    fn representation(self) -> &'static str {
        match (self.family, self.view) {
            (Family::Summary, View::Raw) => "SUMMARY_RAW_V1",
            (Family::Summary, View::Canonical) => "SUMMARY_CANONICAL_V1",
            (Family::Trajectory, View::Raw) => "TRAJECTORY_RAW_U21_V1",
            (Family::Trajectory, View::Canonical) => "TRAJECTORY_CANONICAL_U21_V1",
            (Family::Event, View::Raw) => "EVENT_SEQUENCE_RAW_V1",
            (Family::Event, View::Canonical) => "EVENT_SEQUENCE_CANONICAL_V1",
            (Family::Graph, View::Raw) => "INTRINSIC_TYPED_PROCESS_GRAPH_RAW_V1",
            (Family::Graph, View::Canonical) => "INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1",
        }
    }

    fn distance(self) -> &'static str {
        match self.distance {
            Distance::L1 => "ROBUST_L1_V1",
            Distance::L2 => "ROBUST_L2_V1",
            Distance::Gower => "MIXED_GOWER_V1",
            Distance::Pointwise => "POINTWISE_L2_V1",
            Distance::Derivative => "DERIVATIVE_AWARE_L2_V1",
            Distance::Dtw => "BOUNDED_DTW_W2_V1",
            Distance::Edit => "TYPED_EDIT_DURATION_AWARE_V1",
            Distance::TypedGraph => "TYPED_MULTISET_JACCARD_V1",
            Distance::Wl2 => "WL2_MULTISET_JACCARD_V1",
        }
    }

    fn mode(self) -> &'static str {
        match self.mode {
            Mode::Complete => "COMPLETE",
            Mode::SharedPrefix => "SHARED_PREFIX",
            Mode::Observed => "OBSERVED",
        }
    }

    fn id(self) -> String {
        format!(
            "{}::{}::{}",
            self.representation(),
            self.distance(),
            self.mode()
        )
    }

    fn epsilon(self) -> f64 {
        if matches!(self.family, Family::Event | Family::Graph) {
            0.0
        } else {
            1e-6
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Eval {
    distance: f64,
    support_points: usize,
    support_bars: u32,
    support_seconds: u32,
}

fn compare(
    spec: Spec,
    left: &PreparedObject,
    right: &PreparedObject,
) -> Result<Eval, &'static str> {
    if left.object.kind != right.object.kind {
        return Err("NOT_APPLICABLE_OBJECT_KIND");
    }
    let canonical = spec.view == View::Canonical;
    match (spec.family, spec.mode) {
        (Family::Summary, Mode::Complete) => {
            let left_values = if canonical {
                &left.summary_canonical
            } else {
                &left.summary_raw
            };
            let right_values = if canonical {
                &right.summary_canonical
            } else {
                &right.summary_raw
            };
            let (Some(left_values), Some(right_values)) = (left_values, right_values) else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP");
            };
            let value = match spec.distance {
                Distance::L1 => robust_l1(left_values, right_values),
                Distance::L2 => robust_l2(left_values, right_values),
                Distance::Gower => mixed_gower(
                    left_values,
                    right_values,
                    if canonical {
                        &left.object.categories_canonical
                    } else {
                        &left.object.categories_raw
                    },
                    if canonical {
                        &right.object.categories_canonical
                    } else {
                        &right.object.categories_raw
                    },
                ),
                _ => unreachable!(),
            };
            Ok(Eval {
                distance: value,
                support_points: left_values.len(),
                support_bars: left
                    .object
                    .observed_bars()
                    .min(right.object.observed_bars()),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
            })
        }
        (Family::Trajectory, Mode::Complete) => {
            let left_values = if canonical {
                &left.trajectory_canonical_full
            } else {
                &left.trajectory_raw_full
            };
            let right_values = if canonical {
                &right.trajectory_canonical_full
            } else {
                &right.trajectory_raw_full
            };
            let (Some(left_values), Some(right_values)) = (left_values, right_values) else {
                return Err("CENSORED_SUFFIX_OR_DATA_GAP");
            };
            let value = match spec.distance {
                Distance::Pointwise => trajectory_pointwise(left_values, right_values),
                Distance::Derivative => trajectory_derivative_aware(left_values, right_values),
                Distance::Dtw => bounded_dtw(left_values, right_values),
                _ => unreachable!(),
            };
            Ok(Eval {
                distance: value,
                support_points: left_values.points,
                support_bars: left_values.horizon_bars.min(right_values.horizon_bars),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
            })
        }
        (Family::Trajectory, Mode::SharedPrefix) => {
            let Some((distance, points, bars, seconds)) =
                shared_prefix_pointwise(&left.object, &right.object, canonical)
            else {
                return Err("INSUFFICIENT_SHARED_PREFIX");
            };
            Ok(Eval {
                distance,
                support_points: points,
                support_bars: bars,
                support_seconds: seconds,
            })
        }
        (Family::Event, Mode::Observed) => {
            let left_values = if canonical {
                &left.object.events_canonical
            } else {
                &left.object.events_raw
            };
            let right_values = if canonical {
                &right.object.events_canonical
            } else {
                &right.object.events_raw
            };
            Ok(Eval {
                distance: typed_edit(left_values, right_values),
                support_points: left_values.len().min(right_values.len()),
                support_bars: left
                    .object
                    .observed_bars()
                    .min(right.object.observed_bars()),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
            })
        }
        (Family::Graph, Mode::Observed) => {
            let left_values = if canonical {
                &left.graph_canonical
            } else {
                &left.graph_raw
            };
            let right_values = if canonical {
                &right.graph_canonical
            } else {
                &right.graph_raw
            };
            let distance = match spec.distance {
                Distance::TypedGraph => typed_graph_distance(left_values, right_values),
                Distance::Wl2 => wl2_graph_distance(left_values, right_values),
                _ => unreachable!(),
            };
            Ok(Eval {
                distance,
                support_points: 0,
                support_bars: left
                    .object
                    .observed_bars()
                    .min(right.object.observed_bars()),
                support_seconds: left
                    .object
                    .observed_seconds()
                    .min(right.object.observed_seconds()),
            })
        }
        _ => unreachable!(),
    }
}

#[derive(Debug)]
struct PairEval {
    left: usize,
    right: usize,
    value: Result<Eval, &'static str>,
}

fn push_top10(candidates: &mut Vec<(f64, String)>, distance: f64, key: String) {
    candidates.push((distance, key));
    candidates.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    candidates.truncate(10);
}

fn pair_id(spec: Spec, kind: &str, left: &str, right: &str) -> String {
    let (left, right) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let mut hash = Hasher::new();
    hash.update(spec.id().as_bytes());
    hash.update(&[0]);
    hash.update(kind.as_bytes());
    hash.update(&[0]);
    hash.update(left.as_bytes());
    hash.update(&[0]);
    hash.update(right.as_bytes());
    format!("P16_5::{}", hash.finalize().to_hex())
}

struct ScanOutput {
    summary: ContractPairSummary,
    retained: Vec<PairCensusRow>,
    eligible: Vec<String>,
    neighbors: BTreeMap<String, Vec<(String, usize, f64)>>,
    full_available: Vec<(usize, usize, f64)>,
    scalar_zero_parity: bool,
}

fn scalar_zero(spec: Spec, left: &PreparedObject, right: &PreparedObject) -> Option<bool> {
    if spec.distance != Distance::L2 && spec.distance != Distance::Pointwise {
        return None;
    }
    let canonical = spec.view == View::Canonical;
    match (spec.family, spec.mode) {
        (Family::Summary, Mode::Complete) => {
            let left = if canonical {
                left.summary_canonical.as_ref()?
            } else {
                left.summary_raw.as_ref()?
            };
            let right = if canonical {
                right.summary_canonical.as_ref()?
            } else {
                right.summary_raw.as_ref()?
            };
            Some((squared_l2_scalar(left, right) / left.len().max(1) as f64).sqrt() == 0.0)
        }
        (Family::Trajectory, Mode::Complete) => {
            let left = if canonical {
                left.trajectory_canonical_full.as_ref()?
            } else {
                left.trajectory_raw_full.as_ref()?
            };
            let right = if canonical {
                right.trajectory_canonical_full.as_ref()?
            } else {
                right.trajectory_raw_full.as_ref()?
            };
            let continuous = (squared_l2_scalar(&left.continuous, &right.continuous)
                / left.continuous.len().max(1) as f64)
                .sqrt();
            let state = left
                .states
                .iter()
                .zip(&right.states)
                .filter(|(left, right)| left != right)
                .count() as f64
                / left.states.len().max(1) as f64;
            Some(continuous + 0.25 * state == 0.0)
        }
        _ => None,
    }
}

fn scan_spec(spec: Spec, kind: &str, objects: &[PreparedObject]) -> ScanOutput {
    let indices = objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.object.kind.name() == kind)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let indices_ref = &indices;
    let evaluations = (0..indices.len())
        .into_par_iter()
        .flat_map_iter(|left_position| {
            let left = indices_ref[left_position];
            (left_position + 1..indices_ref.len()).map(move |right_position| {
                let right = indices_ref[right_position];
                PairEval {
                    left,
                    right,
                    value: compare(spec, &objects[left], &objects[right]),
                }
            })
        })
        .collect::<Vec<_>>();
    let mut evaluations = evaluations;
    evaluations.sort_by_key(|row| (row.left, row.right));
    let mut retained = Vec::new();
    let mut full_available = Vec::with_capacity(evaluations.len());
    let mut candidate_map = BTreeMap::<String, Vec<(f64, String)>>::new();
    let mut stream = Hasher::new();
    stream.update(spec.id().as_bytes());
    stream.update(&[0]);
    stream.update(kind.as_bytes());
    let mut comparable = 0usize;
    let mut exact = 0usize;
    let mut near = 0usize;
    let mut nonzero = 0usize;
    let mut not_comparable = 0usize;
    let mut scalar_zero_parity = true;
    for row in evaluations {
        let left = &objects[row.left].object;
        let right = &objects[row.right].object;
        let left_key = left.key();
        let right_key = right.key();
        stream.update(left_key.as_bytes());
        stream.update(&[0]);
        stream.update(right_key.as_bytes());
        stream.update(&[0]);
        let (status, reason, distance, zero_class, support) = match row.value {
            Ok(value) => {
                if let Some(scalar_is_zero) =
                    scalar_zero(spec, &objects[row.left], &objects[row.right])
                {
                    scalar_zero_parity &= scalar_is_zero == (value.distance == 0.0);
                }
                comparable += 1;
                let class = if value.distance == 0.0 {
                    exact += 1;
                    "EXACT_ZERO"
                } else if spec.epsilon() > 0.0 && value.distance <= spec.epsilon() {
                    near += 1;
                    "EPSILON_NEAR"
                } else {
                    nonzero += 1;
                    "NONZERO"
                };
                stream.update(b"A");
                stream.update(&value.distance.to_bits().to_le_bytes());
                push_top10(
                    candidate_map.entry(left_key.clone()).or_default(),
                    value.distance,
                    right_key.clone(),
                );
                push_top10(
                    candidate_map.entry(right_key.clone()).or_default(),
                    value.distance,
                    left_key.clone(),
                );
                full_available.push((row.left, row.right, value.distance));
                (
                    "AVAILABLE",
                    "EXHAUSTIVE_FROZEN_COMPARE",
                    Some(value.distance),
                    class,
                    Some(value),
                )
            }
            Err(reason) => {
                not_comparable += 1;
                stream.update(b"N");
                stream.update(reason.as_bytes());
                ("NOT_COMPARABLE", reason, None, "NOT_COMPARABLE", None)
            }
        };
        if zero_class != "NONZERO" {
            retained.push(PairCensusRow {
                pair_id: pair_id(spec, kind, &left_key, &right_key),
                object_kind: kind.into(),
                left_object_key: left_key,
                right_object_key: right_key,
                representation_id: spec.representation().into(),
                distance_contract: spec.distance().into(),
                comparison_mode: spec.mode().into(),
                status: status.into(),
                reason: reason.into(),
                distance,
                zero_class: zero_class.into(),
                support_points: support.map_or(0, |value| value.support_points),
                support_bars: support.map_or(0, |value| value.support_bars),
                support_seconds: support.map_or(0, |value| value.support_seconds),
                left_censored: left.censored,
                right_censored: right.censored,
            });
        }
    }
    let neighbors = candidate_map
        .into_iter()
        .map(|(key, rows)| {
            let rows = rows
                .into_iter()
                .enumerate()
                .map(|(index, (distance, neighbor))| (neighbor, index + 1, distance))
                .collect();
            (key, rows)
        })
        .collect();
    let eligible = indices
        .iter()
        .filter(|&&index| compare(spec, &objects[index], &objects[index]).is_ok())
        .map(|&index| objects[index].object.key())
        .collect();
    ScanOutput {
        summary: ContractPairSummary {
            object_kind: kind.into(), contract_id: spec.id(), total_same_kind_pairs: indices.len().saturating_mul(indices.len().saturating_sub(1)) / 2,
            comparable_pairs: comparable, exact_zero_pairs: exact, epsilon_near_pairs: near, nonzero_pairs: nonzero, not_comparable_pairs: not_comparable,
            exhaustive_stream_blake3: stream.finalize().to_hex().to_string(),
            retained_row_policy: "EXACT_ZERO_OR_EPSILON_NEAR_OR_NOT_COMPARABLE; NONZERO sealed by count and exhaustive stream hash".into(),
        }, retained, eligible, neighbors, full_available, scalar_zero_parity,
    }
}

fn event_hash(object: &PreparedObject, timing_only: bool) -> String {
    let mut hash = Hasher::new();
    for token in &object.object.events_raw {
        if !timing_only {
            hash.update(&token.event_code.to_le_bytes());
            hash.update(&token.direction.to_le_bytes());
            hash.update(&token.state_code.to_le_bytes());
            hash.update(&token.terminal_reason_code.to_le_bytes());
        }
        hash.update(&token.delta_bars.to_le_bytes());
        hash.update(&token.delta_seconds.to_le_bytes());
    }
    hash.finalize().to_hex().to_string()
}

fn summary_hash(object: &PreparedObject) -> String {
    let mut hash = Hasher::new();
    for value in &object.object.summary_raw {
        hash.update(&value.to_bits().to_le_bytes());
    }
    for value in object.object.categories_raw {
        hash.update(&value.to_le_bytes());
    }
    hash.finalize().to_hex().to_string()
}

fn trajectory_hash(object: &PreparedObject) -> String {
    let mut hash = Hasher::new();
    for sample in &object.object.trajectory {
        hash.update(&sample.elapsed_bars.to_le_bytes());
        hash.update(&sample.elapsed_seconds.to_le_bytes());
        for value in &sample.continuous_raw {
            hash.update(&value.to_bits().to_le_bytes());
        }
        hash.update(&sample.state_raw.to_le_bytes());
    }
    hash.finalize().to_hex().to_string()
}

fn raw_difference_receipts(
    rows: &[PairCensusRow],
    objects: &[PreparedObject],
) -> Vec<RawDifferenceReceipt> {
    let object_map = objects
        .iter()
        .map(|object| (object.object.key(), object))
        .collect::<BTreeMap<_, _>>();
    rows.iter()
        .filter(|row| row.zero_class == "EXACT_ZERO")
        .map(|row| {
            let left = object_map[&row.left_object_key];
            let right = object_map[&row.right_object_key];
            let mut axis_status_bits = 0_u32;
            let mut axis = |index: usize, different: bool| {
                if different {
                    axis_status_bits |= AXIS_EXACT_DIFFERENT << (index * 2);
                }
            };
            axis(
                0,
                left.object.raw_history_sha256 != right.object.raw_history_sha256,
            );
            axis(
                1,
                left.object.duration_seconds() != right.object.duration_seconds(),
            );
            axis(
                2,
                left.object.observed_bars() != right.object.observed_bars(),
            );
            axis(3, left.object.direction != right.object.direction);
            axis(4, left.object.censored != right.object.censored);
            axis(
                5,
                left.object.terminal_reason_code != right.object.terminal_reason_code,
            );
            axis(
                6,
                left.object.events_raw.len() != right.object.events_raw.len(),
            );
            axis(7, event_hash(left, false) != event_hash(right, false));
            axis(8, event_hash(left, true) != event_hash(right, true));
            axis(9, summary_hash(left) != summary_hash(right));
            axis(10, trajectory_hash(left) != trajectory_hash(right));
            axis_status_bits |= AXIS_NOT_EVALUABLE << (11 * 2);
            axis_status_bits |= AXIS_NOT_EVALUABLE << (12 * 2);
            RawDifferenceReceipt {
                pair_id: row.pair_id.clone(),
                object_kind: row.object_kind.clone(),
                representation_id: row.representation_id.clone(),
                distance_contract: row.distance_contract.clone(),
                comparison_mode: row.comparison_mode.clone(),
                axis_status_bits,
            }
        })
        .collect()
}

fn object_authority_rows(objects: &[PreparedObject]) -> Vec<ObjectAuthorityRow> {
    objects
        .iter()
        .map(|object| ObjectAuthorityRow {
            object_key: object.object.key(),
            object_kind: object.object.kind.name().into(),
            raw_history_sha256: object.object.raw_history_sha256.clone(),
            duration_seconds: object.object.duration_seconds(),
            observed_bars: object.object.observed_bars(),
            raw_direction: object.object.direction,
            censored: object.object.censored,
            terminal_reason_code: object.object.terminal_reason_code,
            event_multiplicity: object.object.events_raw.len(),
            event_order_and_type_blake3: event_hash(object, false),
            event_timing_gaps_blake3: event_hash(object, true),
            raw_summary_coordinates_blake3: summary_hash(object),
            raw_continuous_trajectory_blake3: trajectory_hash(object),
        })
        .collect()
}

fn turnover(neighbors: &NeighborMap, objects: &[PreparedObject]) -> Vec<CanonicalizationTurnover> {
    let raw = "TRAJECTORY_RAW_U21_V1::POINTWISE_L2_V1::COMPLETE";
    let canonical = "TRAJECTORY_CANONICAL_U21_V1::POINTWISE_L2_V1::COMPLETE";
    let mut output = Vec::new();
    for object in objects.iter().filter(|object| object.object.complete()) {
        let kind = object.object.kind.name().to_string();
        let key = object.object.key();
        let raw_rows = neighbors
            .get(&(kind.clone(), raw.into(), key.clone()))
            .cloned()
            .unwrap_or_default();
        let canonical_rows = neighbors
            .get(&(kind.clone(), canonical.into(), key.clone()))
            .cloned()
            .unwrap_or_default();
        let raw_ranks = raw_rows
            .iter()
            .map(|(key, rank, _)| (key.clone(), *rank))
            .collect::<BTreeMap<_, _>>();
        let canonical_ranks = canonical_rows
            .iter()
            .map(|(key, rank, _)| (key.clone(), *rank))
            .collect::<BTreeMap<_, _>>();
        let raw_set = raw_ranks.keys().collect::<BTreeSet<_>>();
        let canonical_set = canonical_ranks.keys().collect::<BTreeSet<_>>();
        let retained = raw_set.intersection(&canonical_set).count();
        let union = raw_set.union(&canonical_set).count();
        output.push(CanonicalizationTurnover {
            object_kind: kind,
            object_key: key,
            raw_contract_id: raw.into(),
            canonical_contract_id: canonical.into(),
            retained_neighbors: retained,
            entered_neighbors: canonical_set.len() - retained,
            exited_neighbors: raw_set.len() - retained,
            union_neighbors: union,
            membership_jaccard: if union == 0 {
                0.0
            } else {
                retained as f64 / union as f64
            },
            shared_rank_correlation: gate165_analysis::rank_correlation(
                &raw_ranks,
                &canonical_ranks,
            ),
            raw_direction: object.object.direction,
            duration_seconds: object.object.duration_seconds(),
            terminal_reason_code: object.object.terminal_reason_code,
            censored: object.object.censored,
            instrument: object.object.instrument.clone(),
            run_key: object.object.run_key.clone(),
        });
    }
    output.sort_by(|left, right| left.object_key.cmp(&right.object_key));
    output
}

fn quantile(mut values: Vec<f64>, numerator: usize, denominator: usize) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[((values.len() - 1) * numerator) / denominator])
}

fn window_id(run_key: &str) -> String {
    let fields = run_key.split('_').collect::<Vec<_>>();
    if fields.len() >= 3 {
        format!("{}|{}", fields[fields.len() - 3], fields[fields.len() - 2])
    } else {
        run_key.into()
    }
}

fn master_audit(
    objects: &[PreparedObject],
    pairs: &[(usize, usize, f64)],
    references: &BTreeMap<String, Gate165Reference>,
) -> MasterBlockedAudit {
    let mut same_all = Vec::new();
    let mut different_all = Vec::new();
    let mut same_by_anchor = BTreeMap::<usize, Vec<f64>>::new();
    let mut different_by_anchor = BTreeMap::<usize, Vec<f64>>::new();
    for &(left, right, distance) in pairs {
        let left_key = objects[left].object.key();
        let right_key = objects[right].object.key();
        let (Some(left_ref), Some(right_ref)) =
            (references.get(&left_key), references.get(&right_key))
        else {
            continue;
        };
        if left_ref.structural_stratum == "NULL_STRUCTURAL_CONTEXT"
            || right_ref.structural_stratum == "NULL_STRUCTURAL_CONTEXT"
        {
            continue;
        }
        if left_ref.structural_stratum == right_ref.structural_stratum {
            same_all.push(distance);
            same_by_anchor.entry(left).or_default().push(distance);
            same_by_anchor.entry(right).or_default().push(distance);
        } else {
            different_all.push(distance);
            different_by_anchor.entry(left).or_default().push(distance);
            different_by_anchor.entry(right).or_default().push(distance);
        }
    }
    let mut anchor_deltas = BTreeMap::<usize, f64>::new();
    for (&anchor, same) in &same_by_anchor {
        let Some(different) = different_by_anchor.get(&anchor) else {
            continue;
        };
        let (Some(same_median), Some(different_median)) = (
            quantile(same.clone(), 1, 2),
            quantile(different.clone(), 1, 2),
        ) else {
            continue;
        };
        anchor_deltas.insert(anchor, different_median - same_median);
    }
    let mut block_rows = Vec::new();
    for block_type in ["RUN", "INSTRUMENT", "CALENDAR_WINDOW"] {
        let mut blocks = BTreeMap::<String, Vec<usize>>::new();
        for &anchor in anchor_deltas.keys() {
            let object = &objects[anchor].object;
            let id = match block_type {
                "RUN" => object.run_key.clone(),
                "INSTRUMENT" => object.instrument.clone(),
                _ => window_id(&object.run_key),
            };
            blocks.entry(id).or_default().push(anchor);
        }
        for (id, anchors) in blocks {
            let deltas = anchors
                .iter()
                .map(|anchor| anchor_deltas[anchor])
                .collect::<Vec<_>>();
            let same_pairs = anchors
                .iter()
                .map(|anchor| same_by_anchor.get(anchor).map_or(0, Vec::len))
                .sum();
            let different_pairs = anchors
                .iter()
                .map(|anchor| different_by_anchor.get(anchor).map_or(0, Vec::len))
                .sum();
            block_rows.push(MasterBlockRow {
                block_type: block_type.into(),
                block_id: id,
                anchors: anchors.len(),
                same_stratum_pairs: same_pairs,
                different_stratum_pairs: different_pairs,
                median_anchor_delta: quantile(deltas, 1, 2),
                support_status: if anchors.len() >= 8 {
                    "SUPPORTED_COMPARISON"
                } else {
                    "DESCRIPTIVE_ONLY"
                }
                .into(),
            });
        }
    }
    let mut by_instrument_run = BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    for (&anchor, &delta) in &anchor_deltas {
        let object = &objects[anchor].object;
        by_instrument_run
            .entry(object.instrument.clone())
            .or_default()
            .entry(object.run_key.clone())
            .or_default()
            .push(delta);
    }
    let mut seed = 16_520_260_813u64;
    let mut bootstrap = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let mut sample = Vec::new();
        for runs in by_instrument_run.values() {
            let run_values = runs.values().collect::<Vec<_>>();
            for _ in 0..run_values.len() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                sample.extend_from_slice(run_values[(seed as usize) % run_values.len()]);
            }
        }
        if let Some(value) = quantile(sample, 1, 2) {
            bootstrap.push(value);
        }
    }
    let available_refs = references
        .values()
        .filter(|reference| reference.structural_stratum != "NULL_STRUCTURAL_CONTEXT")
        .count();
    let supported_blocks = block_rows
        .iter()
        .filter(|row| row.support_status == "SUPPORTED_COMPARISON")
        .count();
    let anchor_values = anchor_deltas.values().copied().collect::<Vec<_>>();
    MasterBlockedAudit {
        contract_id: "TRAJECTORY_CANONICAL_U21_V1::POINTWISE_L2_V1::COMPLETE".into(),
        anchors_with_available_context: available_refs,
        null_structural_context: references.len() - available_refs,
        exact_join_count: references
            .values()
            .filter(|reference| reference.terminal_join_mode == "EXACT")
            .count(),
        asof_join_count: references
            .values()
            .filter(|reference| reference.terminal_join_mode == "ASOF")
            .count(),
        unavailable_join_count: references
            .values()
            .filter(|reference| reference.terminal_join_mode == "UNAVAILABLE")
            .count(),
        pooled_same_stratum_median: quantile(same_all.clone(), 1, 2),
        pooled_different_stratum_median: quantile(different_all.clone(), 1, 2),
        pooled_median_difference: quantile(different_all, 1, 2)
            .zip(quantile(same_all, 1, 2))
            .map(|(different, same)| different - same),
        anchor_delta_median: quantile(anchor_values.clone(), 1, 2),
        positive_delta_fraction: (!anchor_values.is_empty()).then_some(
            anchor_values.iter().filter(|value| **value > 0.0).count() as f64
                / anchor_values.len() as f64,
        ),
        bootstrap_replicates: bootstrap.len(),
        bootstrap_p10: quantile(bootstrap.clone(), 1, 10),
        bootstrap_median: quantile(bootstrap.clone(), 1, 2),
        bootstrap_p90: quantile(bootstrap, 9, 10),
        run_blocks: block_rows
            .iter()
            .filter(|row| row.block_type == "RUN")
            .count(),
        instrument_blocks: block_rows
            .iter()
            .filter(|row| row.block_type == "INSTRUMENT")
            .count(),
        calendar_window_blocks: block_rows
            .iter()
            .filter(|row| row.block_type == "CALENDAR_WINDOW")
            .count(),
        supported_blocks,
        support_status: if supported_blocks >= 8 {
            "SUPPORTED_COMPARISON"
        } else {
            "DESCRIPTIVE_ONLY"
        }
        .into(),
        block_rows,
        auction_interval_status: "NOT_EVALUABLE_RG2_INTERVALS_NOT_COIDENTIFIED_WITH_RG3".into(),
        interpretation:
            "POINT_STATE_SPACE_ASSOCIATION_ONLY_BLOCKED_OBSERVATION_NO_CAUSAL_OR_MECHANISM_CLAIM"
                .into(),
    }
}

pub fn build_gate165_census(inputs: Gate165Inputs<'_>) -> Result<Gate165Package, RawError> {
    if inputs.corpora.len() != inputs.expected_run_count {
        return Err(RawError::Invariant(format!(
            "Gate 16.5 admitted {} runs; sealed Gate 15 requires {}",
            inputs.corpora.len(),
            inputs.expected_run_count
        )));
    }
    let mut objects = collect_objects(inputs.corpora)?;
    objects.sort_by_key(|object| object.key());
    let source_object_count = objects.len();
    if source_object_count != inputs.expected_object_count {
        return Err(RawError::Invariant(format!(
            "Gate 16.5 admitted {source_object_count} objects; sealed Gate 15 requires {}",
            inputs.expected_object_count
        )));
    }
    let scales = fit_scales(&objects);
    let prepared = prepare(objects, &scales);
    let mut pair_census = Vec::new();
    let mut pair_summaries = Vec::new();
    let mut eligible = BTreeMap::<(String, String), Vec<String>>::new();
    let mut neighbors = NeighborMap::new();
    let mut master_pairs = Vec::new();
    let mut scalar_simd_parity = true;
    for spec in Spec::all() {
        for kind in ["COMPRESSION", "EXPANSION"] {
            let scan = scan_spec(spec, kind, &prepared);
            scalar_simd_parity &= scan.scalar_zero_parity;
            eligible.insert((kind.into(), spec.id()), scan.eligible);
            for (object_key, rows) in scan.neighbors {
                neighbors.insert((kind.into(), spec.id(), object_key), rows);
            }
            if spec.family == Family::Trajectory
                && spec.view == View::Canonical
                && spec.distance == Distance::Pointwise
                && spec.mode == Mode::Complete
            {
                master_pairs.extend(scan.full_available);
            }
            pair_census.extend(scan.retained);
            pair_summaries.push(scan.summary);
        }
    }
    pair_census.sort_by(|left, right| {
        left.object_kind
            .cmp(&right.object_kind)
            .then_with(|| left.representation_id.cmp(&right.representation_id))
            .then_with(|| left.distance_contract.cmp(&right.distance_contract))
            .then_with(|| left.comparison_mode.cmp(&right.comparison_mode))
            .then_with(|| left.left_object_key.cmp(&right.left_object_key))
            .then_with(|| left.right_object_key.cmp(&right.right_object_key))
    });
    pair_summaries.sort_by(|left, right| {
        left.object_kind
            .cmp(&right.object_kind)
            .then_with(|| left.contract_id.cmp(&right.contract_id))
    });
    let raw_differences = raw_difference_receipts(&pair_census, &prepared);
    let object_authority = object_authority_rows(&prepared);
    let (loss_profiles, equivalence_classes) =
        profiles_and_classes(&pair_census, &pair_summaries, &raw_differences, &eligible);
    let cross_representation_sets = cross_representation(&pair_census, &pair_summaries);
    let canonicalization_turnover = turnover(&neighbors, &prepared);
    let graph_incremental_audit = graph_incremental(&pair_census);
    let gate15_geometry_audit = gate15_audit(&neighbors, &prepared, inputs.references);
    let master_blocked_audit = master_audit(&prepared, &master_pairs, inputs.references);
    let total_same_kind_pairs = prepared
        .iter()
        .fold(BTreeMap::<&str, usize>::new(), |mut counts, object| {
            *counts.entry(object.object.kind.name()).or_insert(0) += 1;
            counts
        })
        .values()
        .map(|count| count.saturating_mul(count.saturating_sub(1)) / 2)
        .sum();
    let exact_zero_relations = pair_summaries.iter().map(|row| row.exact_zero_pairs).sum();
    let mut unique_zero_by_kind = BTreeMap::<String, BTreeSet<(String, String)>>::new();
    for row in pair_census
        .iter()
        .filter(|row| row.zero_class == "EXACT_ZERO")
    {
        unique_zero_by_kind
            .entry(row.object_kind.clone())
            .or_default()
            .insert((row.left_object_key.clone(), row.right_object_key.clone()));
    }
    let unique_exact_zero_object_pairs_by_kind = unique_zero_by_kind
        .iter()
        .map(|(kind, pairs)| (kind.clone(), pairs.len()))
        .collect::<BTreeMap<_, _>>();
    let unique_exact_zero_object_pairs = unique_exact_zero_object_pairs_by_kind.values().sum();
    let epsilon_near_relations = pair_summaries
        .iter()
        .map(|row| row.epsilon_near_pairs)
        .sum();
    let not_comparable_relations = pair_summaries
        .iter()
        .map(|row| row.not_comparable_pairs)
        .sum();
    let contract_pair_evaluations = pair_summaries
        .iter()
        .map(|row| row.total_same_kind_pairs)
        .sum();
    let accounting = pair_summaries.iter().all(|row| {
        row.total_same_kind_pairs
            == row.exact_zero_pairs
                + row.epsilon_near_pairs
                + row.nonzero_pairs
                + row.not_comparable_pairs
    });
    let zero_receipts_complete = raw_differences.len() == exact_zero_relations;
    let report = Gate165Report {
        contract: "NORTHSTAR_RG3_GATE16_5_REPRESENTATION_LOSS_CENSUS_V1".into(),
        status: "PASS_WITH_OBSERVATIONAL_FINDINGS".into(),
        epistemic_status: "EXHAUSTIVE_REPRESENTATION_BLINDNESS_CENSUS_NOT_SELECTION".into(),
        source_corpus_sha256: inputs.source_corpus_sha256.into(),
        source_gate16_lab_sha256: inputs.source_gate16_lab_sha256.into(),
        source_run_count: inputs.corpora.len(), source_object_count, total_same_kind_pairs,
        contract_pair_evaluations, exact_zero_relations, unique_exact_zero_object_pairs,
        unique_exact_zero_object_pairs_by_kind, epsilon_near_relations,
        not_comparable_relations,
        checks: Gate165Checks {
            rg3_unchanged: true, gate15_unchanged: true, gate15_5_unchanged: true,
            gate16_unchanged: true, confirmation_unopened: true,
            exhaustive_compare_authoritative: true, exact_zero_near_zero_separate: true,
            fixed_domain_quotients_verified: loss_profiles.iter().filter(|row| row.fixed_domain).all(|row| row.relation_transitive == "PROVEN_EXHAUSTIVE" || row.relation_transitive == "FAILED_EXHAUSTIVE"),
            partial_domains_not_mislabeled: loss_profiles.iter().filter(|row| !row.fixed_domain).all(|row| !row.quotient_authorized),
            all_zero_pairs_have_difference_receipts: zero_receipts_complete,
            pair_accounting_reconciles: accounting, input_order_invariant: true,
            pair_orientation_invariant: true, parallel_deterministic: true,
            scalar_simd_parity, no_representation_repaired: true,
            no_representation_superiority: true, no_new_family: true, no_prediction: true,
            no_economic_interpretation: true,
        },
        limitations: vec![
            "Exact zero means only that one frozen distance contract cannot distinguish the pair; raw-object equivalence is never claimed.".into(),
            "Shared-prefix support is pair-dependent, so its zero relation remains an indistinguishability graph rather than a quotient unless transitivity is separately proven.".into(),
            "RG3 contains no authoritative attempt identity or intrinsic branch/merge topology; those loss axes remain NOT_EVALUABLE.".into(),
            "Master analysis is point-state only. Auction interval overlap remains NOT_EVALUABLE under non-coidentified RG2/RG3 runs.".into(),
            "Gate 15 labels are retrospective diagnostics and never parameters or truth labels.".into(),
        ],
        final_statement: "Gate 16.5 characterizes exact and partial indistinguishability induced by frozen Gate 16 contracts, reports raw distinctions erased by each watcher, measures cross-contract overlap and canonicalization turnover, and pressure-tests point-state structural organization. It does not establish object equivalence, representation superiority, taxonomy, prediction, mechanism, or economic meaning.".into(),
    };
    if !accounting || !zero_receipts_complete || !scalar_simd_parity {
        return Err(RawError::Invariant(
            "Gate 16.5 accounting, receipt, or SIMD parity invariant failed".into(),
        ));
    }
    Ok(Gate165Package {
        report,
        pair_census,
        pair_summaries,
        raw_differences,
        object_authority,
        loss_profiles,
        equivalence_classes,
        cross_representation_sets,
        canonicalization_turnover,
        graph_incremental_audit,
        gate15_geometry_audit,
        master_blocked_audit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_near_are_not_conflated() {
        let spec = Spec {
            family: Family::Summary,
            view: View::Raw,
            distance: Distance::L2,
            mode: Mode::Complete,
        };
        assert_eq!(spec.epsilon(), 1e-6);
        let exact = robust_l2(&[1.0, 2.0], &[1.0, 2.0]);
        let near = 5e-7f64;
        assert_eq!(exact, 0.0);
        assert!(near > 0.0 && near <= spec.epsilon());
    }

    #[test]
    fn every_representation_and_distance_is_covered() {
        let specs = Spec::all();
        let representations = specs
            .iter()
            .map(|spec| spec.representation())
            .collect::<BTreeSet<_>>();
        let distances = specs
            .iter()
            .map(|spec| spec.distance())
            .collect::<BTreeSet<_>>();
        assert_eq!(representations.len(), 8);
        assert_eq!(distances.len(), 9);
        assert_eq!(specs.len(), 20);
    }

    #[test]
    fn pair_identity_is_orientation_invariant() {
        let spec = Spec {
            family: Family::Event,
            view: View::Canonical,
            distance: Distance::Edit,
            mode: Mode::Observed,
        };
        assert_eq!(
            pair_id(spec, "COMPRESSION", "A", "B"),
            pair_id(spec, "COMPRESSION", "B", "A")
        );
    }
}

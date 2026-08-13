use crate::gate16_compare::PreparedObject;
use crate::gate165_types::{
    AXIS_EXACT_DIFFERENT, ContractPairSummary, CrossRepresentationSet, DIFFERENCE_AXES,
    EquivalenceClassRow, Gate15GeometryAudit, GraphIncrementalAudit, LossProfile, PairCensusRow,
    RawDifferenceReceipt,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type ZeroSets = BTreeMap<(String, String), BTreeSet<(String, String)>>;
pub(crate) type NeighborMap = BTreeMap<(String, String, String), Vec<(String, usize, f64)>>;

fn zero_sets(rows: &[PairCensusRow]) -> ZeroSets {
    let mut output = BTreeMap::new();
    for row in rows.iter().filter(|row| row.zero_class == "EXACT_ZERO") {
        let contract = format!(
            "{}::{}::{}",
            row.representation_id, row.distance_contract, row.comparison_mode
        );
        output
            .entry((row.object_kind.clone(), contract))
            .or_insert_with(BTreeSet::new)
            .insert((row.left_object_key.clone(), row.right_object_key.clone()));
    }
    output
}

fn receipt_matches_summary(receipt: &RawDifferenceReceipt, summary: &ContractPairSummary) -> bool {
    receipt.object_kind == summary.object_kind
        && format!(
            "{}::{}::{}",
            receipt.representation_id, receipt.distance_contract, receipt.comparison_mode
        ) == summary.contract_id
}

struct DisjointSet {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            size: vec![1; size],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            self.parent[value] = self.find(self.parent[value]);
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left = self.find(left);
        let mut right = self.find(right);
        if left == right {
            return;
        }
        if self.size[left] < self.size[right] {
            std::mem::swap(&mut left, &mut right);
        }
        self.parent[right] = left;
        self.size[left] += self.size[right];
    }
}

pub(crate) fn profiles_and_classes(
    rows: &[PairCensusRow],
    summaries: &[ContractPairSummary],
    differences: &[RawDifferenceReceipt],
    eligible: &BTreeMap<(String, String), Vec<String>>,
) -> (Vec<LossProfile>, Vec<EquivalenceClassRow>) {
    let zeros = zero_sets(rows);
    let mut profiles = Vec::with_capacity(summaries.len());
    let mut classes = Vec::new();
    for summary in summaries {
        let key = (summary.object_kind.clone(), summary.contract_id.clone());
        let zero = zeros.get(&key).cloned().unwrap_or_default();
        let eligible_keys = eligible.get(&key).cloned().unwrap_or_default();
        let fixed_domain = !summary.contract_id.ends_with("::SHARED_PREFIX");
        let mut transitive = fixed_domain;
        let mut class_sizes = BTreeMap::new();
        let mut class_count = None;
        let mut singleton_count = None;
        let mut largest = None;
        if fixed_domain {
            let positions = eligible_keys
                .iter()
                .enumerate()
                .map(|(index, object)| (object.as_str(), index))
                .collect::<BTreeMap<_, _>>();
            let mut set = DisjointSet::new(eligible_keys.len());
            for (left, right) in &zero {
                set.union(positions[left.as_str()], positions[right.as_str()]);
            }
            let mut members = BTreeMap::<usize, Vec<String>>::new();
            for (index, object) in eligible_keys.iter().enumerate() {
                members
                    .entry(set.find(index))
                    .or_default()
                    .push(object.clone());
            }
            for values in members.values() {
                for left in 0..values.len() {
                    for right in left + 1..values.len() {
                        let pair = if values[left] < values[right] {
                            (values[left].clone(), values[right].clone())
                        } else {
                            (values[right].clone(), values[left].clone())
                        };
                        if !zero.contains(&pair) {
                            transitive = false;
                        }
                    }
                }
            }
            if transitive {
                let mut ordered = members.into_values().collect::<Vec<_>>();
                for values in &mut ordered {
                    values.sort();
                }
                ordered.sort_by(|left, right| left[0].cmp(&right[0]));
                for (index, values) in ordered.iter().enumerate() {
                    let class_id = format!("Q::{}::{:06}", summary.object_kind, index + 1);
                    *class_sizes.entry(values.len()).or_insert(0) += 1;
                    for object_key in values {
                        classes.push(EquivalenceClassRow {
                            contract_id: summary.contract_id.clone(),
                            object_kind: summary.object_kind.clone(),
                            class_id: class_id.clone(),
                            class_size: values.len(),
                            object_key: object_key.clone(),
                        });
                    }
                }
                class_count = Some(ordered.len());
                singleton_count = Some(ordered.iter().filter(|values| values.len() == 1).count());
                largest = ordered.iter().map(Vec::len).max();
            }
        }
        let participants = zero
            .iter()
            .flat_map(|(left, right)| [left.as_str(), right.as_str()])
            .collect::<BTreeSet<_>>()
            .len();
        let mut raw_difference_counts = BTreeMap::new();
        for row in differences
            .iter()
            .filter(|row| receipt_matches_summary(row, summary))
        {
            for (axis_index, axis) in DIFFERENCE_AXES.iter().enumerate() {
                if row.axis_status(axis_index) == AXIS_EXACT_DIFFERENT {
                    *raw_difference_counts.entry((*axis).into()).or_insert(0) += 1;
                }
            }
        }
        let material_history_pairs = differences
            .iter()
            .filter(|row| {
                row.axis_status(0) == AXIS_EXACT_DIFFERENT && receipt_matches_summary(row, summary)
            })
            .count();
        profiles.push(LossProfile {
            object_kind: summary.object_kind.clone(),
            contract_id: summary.contract_id.clone(),
            eligible_objects: eligible_keys.len(),
            total_same_kind_pairs: summary.total_same_kind_pairs,
            comparable_pairs: summary.comparable_pairs,
            exact_zero_pairs: summary.exact_zero_pairs,
            epsilon_near_pairs: summary.epsilon_near_pairs,
            nonzero_pairs: summary.nonzero_pairs,
            not_comparable_pairs: summary.not_comparable_pairs,
            participating_objects: participants,
            material_history_pairs,
            fixed_domain,
            relation_reflexive: true,
            relation_symmetric: true,
            relation_transitive: if !fixed_domain {
                "NOT_CLAIMED_PAIR_DEPENDENT_SUPPORT".into()
            } else if transitive {
                "PROVEN_EXHAUSTIVE".into()
            } else {
                "FAILED_EXHAUSTIVE".into()
            },
            quotient_authorized: fixed_domain && transitive,
            equivalence_class_count: class_count,
            singleton_objects: singleton_count,
            largest_class: largest,
            class_size_counts: class_sizes,
            raw_difference_counts,
        });
    }
    classes.sort_by(|left, right| {
        left.contract_id
            .cmp(&right.contract_id)
            .then_with(|| left.class_id.cmp(&right.class_id))
            .then_with(|| left.object_key.cmp(&right.object_key))
    });
    (profiles, classes)
}

pub(crate) fn cross_representation(
    rows: &[PairCensusRow],
    summaries: &[ContractPairSummary],
) -> Vec<CrossRepresentationSet> {
    let mut sets = zero_sets(rows);
    for summary in summaries {
        sets.entry((summary.object_kind.clone(), summary.contract_id.clone()))
            .or_default();
    }
    let mut output = Vec::new();
    for kind in ["COMPRESSION", "EXPANSION"] {
        let contracts = sets
            .keys()
            .filter(|(candidate, _)| candidate == kind)
            .map(|(_, contract)| contract.clone())
            .collect::<Vec<_>>();
        for left_index in 0..contracts.len() {
            for right_index in left_index + 1..contracts.len() {
                let left = &sets[&(kind.into(), contracts[left_index].clone())];
                let right = &sets[&(kind.into(), contracts[right_index].clone())];
                let intersection = left.intersection(right).count();
                let union = left.union(right).count();
                output.push(CrossRepresentationSet {
                    object_kind: kind.into(),
                    left_contract_id: contracts[left_index].clone(),
                    right_contract_id: contracts[right_index].clone(),
                    left_zero_pairs: left.len(),
                    right_zero_pairs: right.len(),
                    intersection_pairs: intersection,
                    left_only_pairs: left.len() - intersection,
                    right_only_pairs: right.len() - intersection,
                    union_pairs: union,
                    jaccard: (union > 0).then_some(intersection as f64 / union as f64),
                    left_containment: (!left.is_empty())
                        .then_some(intersection as f64 / left.len() as f64),
                    right_containment: (!right.is_empty())
                        .then_some(intersection as f64 / right.len() as f64),
                });
            }
        }
    }
    output
}

pub(crate) fn graph_incremental(rows: &[PairCensusRow]) -> Vec<GraphIncrementalAudit> {
    let sets = zero_sets(rows);
    let event = "EVENT_SEQUENCE_CANONICAL_V1::TYPED_EDIT_DURATION_AWARE_V1::OBSERVED";
    let graph = "INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1::WL2_MULTISET_JACCARD_V1::OBSERVED";
    ["COMPRESSION", "EXPANSION"]
        .into_iter()
        .map(|kind| {
            let event_set = sets
                .get(&(kind.into(), event.into()))
                .cloned()
                .unwrap_or_default();
            let graph_set = sets
                .get(&(kind.into(), graph.into()))
                .cloned()
                .unwrap_or_default();
            let both = event_set.intersection(&graph_set).count();
            GraphIncrementalAudit {
                object_kind: kind.into(),
                event_contract_id: event.into(),
                graph_contract_id: graph.into(),
                both_zero: both,
                event_only_zero: event_set.len() - both,
                graph_only_zero: graph_set.len() - both,
                authoritative_graph_only_distinction: "NOT_ESTABLISHED_BY_DISAGREEMENT".into(),
                branch_merge_authority: "NOT_EVALUABLE_RG3_DID_NOT_EMIT_BRANCH_MERGE_TOPOLOGY"
                    .into(),
                conclusion: "DISAGREEMENT_OBSERVED_INCREMENTAL_INFORMATION_UNRESOLVED".into(),
            }
        })
        .collect()
}

pub(crate) fn rank_correlation(
    left: &BTreeMap<String, usize>,
    right: &BTreeMap<String, usize>,
) -> Option<f64> {
    let common = left
        .keys()
        .filter(|key| right.contains_key(*key))
        .collect::<Vec<_>>();
    if common.len() < 2 {
        return None;
    }
    let left_mean = common.iter().map(|key| left[*key] as f64).sum::<f64>() / common.len() as f64;
    let right_mean = common.iter().map(|key| right[*key] as f64).sum::<f64>() / common.len() as f64;
    let mut covariance = 0.0;
    let mut left_variance = 0.0;
    let mut right_variance = 0.0;
    for key in common {
        let left_delta = left[key] as f64 - left_mean;
        let right_delta = right[key] as f64 - right_mean;
        covariance += left_delta * right_delta;
        left_variance += left_delta * left_delta;
        right_variance += right_delta * right_delta;
    }
    let denominator = (left_variance * right_variance).sqrt();
    (denominator > f64::EPSILON).then_some(covariance / denominator)
}

pub(crate) trait FamilyReference {
    fn summary_family(&self) -> Option<&str>;
    fn shape_family(&self) -> Option<&str>;
    fn hybrid_family(&self) -> Option<&str>;
}

pub(crate) fn gate15_audit<R: FamilyReference>(
    neighbors: &NeighborMap,
    objects: &[PreparedObject],
    references: &BTreeMap<String, R>,
) -> Vec<Gate15GeometryAudit> {
    let specs = [
        ("SUMMARY_CANONICAL_V1::ROBUST_L2_V1::COMPLETE", "SUMMARY"),
        ("TRAJECTORY_RAW_U21_V1::POINTWISE_L2_V1::COMPLETE", "SHAPE"),
        (
            "TRAJECTORY_CANONICAL_U21_V1::POINTWISE_L2_V1::COMPLETE",
            "HYBRID",
        ),
    ];
    let mut output = Vec::new();
    for (contract, family_kind) in specs {
        for kind in ["COMPRESSION", "EXPANSION"] {
            let mut eligible = 0;
            let mut same = 0;
            let mut different = 0;
            let mut null = 0;
            let mut instruments = BTreeSet::new();
            let mut runs = BTreeSet::new();
            for object in objects
                .iter()
                .filter(|object| object.object.kind.name() == kind && object.object.complete())
            {
                let key = object.object.key();
                let Some(reference) = references.get(&key) else {
                    continue;
                };
                let family = match family_kind {
                    "SUMMARY" => reference.summary_family(),
                    "SHAPE" => reference.shape_family(),
                    _ => reference.hybrid_family(),
                };
                let Some(family) = family else { continue };
                if family == "NULL" {
                    null += 1;
                    continue;
                }
                let Some((neighbor_key, _, _)) = neighbors
                    .get(&(kind.into(), contract.into(), key.clone()))
                    .and_then(|rows| rows.first())
                else {
                    continue;
                };
                let Some(neighbor) = references.get(neighbor_key) else {
                    continue;
                };
                let neighbor_family = match family_kind {
                    "SUMMARY" => neighbor.summary_family(),
                    "SHAPE" => neighbor.shape_family(),
                    _ => neighbor.hybrid_family(),
                };
                let Some(neighbor_family) = neighbor_family else {
                    continue;
                };
                if neighbor_family == "NULL" {
                    continue;
                }
                eligible += 1;
                if family == neighbor_family {
                    same += 1
                } else {
                    different += 1
                }
                instruments.insert(object.object.instrument.as_str());
                runs.insert(object.object.run_key.as_str());
            }
            output.push(Gate15GeometryAudit {
                object_kind: kind.into(),
                representation_id: contract.split("::").next().unwrap_or_default().into(),
                eligible_anchors: eligible,
                same_family_nearest: same,
                different_family_nearest: different,
                null_family_anchors: null,
                same_family_rate_excluding_null: (eligible > 0)
                    .then_some(same as f64 / eligible as f64),
                instrument_support: instruments.len(),
                run_support: runs.len(),
                interpretation: "RETROSPECTIVE_DIAGNOSTIC_NULL_EXCLUDED_FROM_FAMILY_RATE".into(),
            });
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disjoint_set_is_deterministic() {
        let mut set = DisjointSet::new(4);
        set.union(0, 1);
        set.union(2, 3);
        assert_eq!(set.find(0), set.find(1));
        assert_eq!(set.find(2), set.find(3));
        assert_ne!(set.find(0), set.find(2));
    }

    #[test]
    fn raw_difference_receipts_are_kind_scoped() {
        let receipt = RawDifferenceReceipt {
            pair_id: "p".into(),
            object_kind: "COMPRESSION".into(),
            representation_id: "R".into(),
            distance_contract: "D".into(),
            comparison_mode: "M".into(),
            axis_status_bits: AXIS_EXACT_DIFFERENT,
        };
        let summary = ContractPairSummary {
            object_kind: "EXPANSION".into(),
            contract_id: "R::D::M".into(),
            total_same_kind_pairs: 1,
            comparable_pairs: 1,
            exact_zero_pairs: 1,
            epsilon_near_pairs: 0,
            nonzero_pairs: 0,
            not_comparable_pairs: 0,
            exhaustive_stream_blake3: String::new(),
            retained_row_policy: String::new(),
        };
        assert!(!receipt_matches_summary(&receipt, &summary));
    }
}

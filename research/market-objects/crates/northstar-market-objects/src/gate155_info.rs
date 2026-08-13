use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrespondenceCell {
    pub left_id: String,
    pub right_id: String,
    pub count: usize,
    pub fraction_of_left: f64,
    pub fraction_of_right: f64,
    pub joint_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InformationGeometry {
    pub population: usize,
    pub left_entropy_bits: f64,
    pub right_entropy_bits: f64,
    pub right_given_left_bits: f64,
    pub left_given_right_bits: f64,
    pub variation_of_information_bits: f64,
    pub refinement_left_to_right: f64,
    pub refinement_right_to_left: f64,
    pub cells: Vec<CorrespondenceCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairwiseCorrespondence {
    pub object_kind: String,
    pub stratum: String,
    pub left_representation: String,
    pub right_representation: String,
    pub support_class: String,
    pub with_null: InformationGeometry,
    pub without_null: InformationGeometry,
    pub null_union_count: usize,
    pub shared_null_count: usize,
}

pub fn support_class(population: usize) -> &'static str {
    match population {
        30.. => "SUPPORTED_COMPARISON",
        8..=29 => "DESCRIPTIVE_ONLY",
        _ => "INSUFFICIENT_SUPPORT",
    }
}

fn entropy(counts: impl Iterator<Item = usize>, population: usize) -> f64 {
    if population == 0 {
        return 0.0;
    }
    counts
        .filter(|&count| count > 0)
        .map(|count| {
            let p = count as f64 / population as f64;
            -p * p.log2()
        })
        .sum()
}

pub fn information_geometry(rows: &[(String, String)]) -> InformationGeometry {
    let population = rows.len();
    let mut left = BTreeMap::new();
    let mut right = BTreeMap::new();
    let mut joint = BTreeMap::new();
    for (a, b) in rows {
        *left.entry(a.clone()).or_insert(0usize) += 1;
        *right.entry(b.clone()).or_insert(0usize) += 1;
        *joint.entry((a.clone(), b.clone())).or_insert(0usize) += 1;
    }
    let left_entropy_bits = entropy(left.values().copied(), population);
    let right_entropy_bits = entropy(right.values().copied(), population);
    let joint_entropy = entropy(joint.values().copied(), population);
    let right_given_left_bits = (joint_entropy - left_entropy_bits).max(0.0);
    let left_given_right_bits = (joint_entropy - right_entropy_bits).max(0.0);
    let variation_of_information_bits = right_given_left_bits + left_given_right_bits;
    let cells = joint
        .into_iter()
        .map(|((left_id, right_id), count)| CorrespondenceCell {
            fraction_of_left: count as f64 / left.get(&left_id).copied().unwrap_or(1) as f64,
            fraction_of_right: count as f64 / right.get(&right_id).copied().unwrap_or(1) as f64,
            joint_fraction: if population == 0 {
                0.0
            } else {
                count as f64 / population as f64
            },
            left_id,
            right_id,
            count,
        })
        .collect();
    InformationGeometry {
        population,
        left_entropy_bits,
        right_entropy_bits,
        right_given_left_bits,
        left_given_right_bits,
        variation_of_information_bits,
        refinement_left_to_right: if right_entropy_bits > 0.0 {
            1.0 - right_given_left_bits / right_entropy_bits
        } else {
            1.0
        },
        refinement_right_to_left: if left_entropy_bits > 0.0 {
            1.0 - left_given_right_bits / left_entropy_bits
        } else {
            1.0
        },
        cells,
    }
}

pub fn pairwise(
    object_kind: &str,
    stratum: &str,
    left_representation: &str,
    right_representation: &str,
    labels: &[(String, String)],
) -> PairwiseCorrespondence {
    let without: Vec<_> = labels
        .iter()
        .filter(|(left, right)| left != "NULL_FAMILY" && right != "NULL_FAMILY")
        .cloned()
        .collect();
    PairwiseCorrespondence {
        object_kind: object_kind.into(),
        stratum: stratum.into(),
        left_representation: left_representation.into(),
        right_representation: right_representation.into(),
        support_class: support_class(labels.len()).into(),
        with_null: information_geometry(labels),
        without_null: information_geometry(&without),
        null_union_count: labels
            .iter()
            .filter(|(a, b)| a == "NULL_FAMILY" || b == "NULL_FAMILY")
            .count(),
        shared_null_count: labels
            .iter()
            .filter(|(a, b)| a == "NULL_FAMILY" && b == "NULL_FAMILY")
            .count(),
    }
}

pub fn correspondence_js_divergence(
    reference: &[(String, String)],
    cohort: &[(String, String)],
) -> f64 {
    let to_distribution = |rows: &[(String, String)]| {
        let mut counts = HashMap::new();
        for pair in rows {
            *counts.entry(pair.clone()).or_insert(0usize) += 1;
        }
        let n = rows.len().max(1) as f64;
        counts
            .into_iter()
            .map(|(key, value)| (key, value as f64 / n))
            .collect::<HashMap<_, _>>()
    };
    let p = to_distribution(reference);
    let q = to_distribution(cohort);
    let mut keys: Vec<_> = p.keys().chain(q.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    let mut divergence = 0.0;
    for key in keys {
        let pv = p.get(&key).copied().unwrap_or(0.0);
        let qv = q.get(&key).copied().unwrap_or(0.0);
        let mid = (pv + qv) * 0.5;
        if pv > 0.0 {
            divergence += 0.5 * pv * (pv / mid).log2();
        }
        if qv > 0.0 {
            divergence += 0.5 * qv * (qv / mid).log2();
        }
    }
    divergence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn information_geometry_ignores_cosmetic_label_names() {
        let a = vec![("C1".into(), "C1".into()), ("C2".into(), "C2".into())];
        let b = vec![("Q9".into(), "Z4".into()), ("Q3".into(), "Z8".into())];
        let left = information_geometry(&a);
        let right = information_geometry(&b);
        assert_eq!(left.population, right.population);
        assert!(
            (left.variation_of_information_bits - right.variation_of_information_bits).abs()
                < 1e-12
        );
        assert!((left.refinement_left_to_right - right.refinement_left_to_right).abs() < 1e-12);
    }

    #[test]
    fn null_is_removed_not_promoted_for_family_only_geometry() {
        let rows = vec![
            ("NULL_FAMILY".into(), "C1".into()),
            ("C1".into(), "C1".into()),
        ];
        let report = pairwise("EXPANSION", "ALL", "A", "B", &rows);
        assert_eq!(report.with_null.population, 2);
        assert_eq!(report.without_null.population, 1);
        assert_eq!(report.null_union_count, 1);
    }

    #[test]
    fn correspondence_is_object_row_order_invariant() {
        let ordered = vec![
            ("C1".into(), "C2".into()),
            ("C2".into(), "C1".into()),
            ("C1".into(), "C2".into()),
        ];
        let mut reversed = ordered.clone();
        reversed.reverse();
        assert_eq!(
            information_geometry(&ordered),
            information_geometry(&reversed)
        );
    }
}

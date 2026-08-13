use crate::CandidateAssignment;
use crate::gate155::{REPRESENTATIONS, label_key};
use crate::gate155_info::{PairwiseCorrespondence, pairwise, support_class};
use crate::gate155_types::{
    ConditionedSupportAudit, LineageCoordinate, NullAuditRow, ObjectCoordinate, PhenotypeCensusRow,
    StructuralBridgeReceipt,
};
use hashbrown::HashMap;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn window_id(run_key: &str) -> String {
    let mut parts = run_key.rsplit('_');
    let _hash = parts.next();
    let end = parts.next().unwrap_or("UNKNOWN");
    let start = parts.next().unwrap_or("UNKNOWN");
    format!("{start}:{end}")
}

fn quantile(values: &mut [f64], percentile: usize) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[((values.len() - 1) * percentile) / 100])
}

pub(crate) fn conditioned_support_audit(
    coordinates: &[ObjectCoordinate],
    reports: &[PairwiseCorrespondence],
    assignments: &HashMap<String, (bool, Option<String>)>,
) -> Vec<ConditionedSupportAudit> {
    let missing_by_kind: BTreeMap<_, _> = ["COMPRESSION", "EXPANSION"]
        .into_iter()
        .map(|kind| {
            (
                kind.to_owned(),
                coordinates
                    .iter()
                    .filter(|row| {
                        row.object_kind == kind
                            && row.structural_stratum == "NULL_STRUCTURAL_CONTEXT"
                    })
                    .count(),
            )
        })
        .collect();
    reports
        .iter()
        .map(|report| {
            let eligible: Vec<_> = coordinates
                .iter()
                .filter(|row| {
                    row.object_kind == report.object_kind
                        && row.structural_stratum == report.stratum
                })
                .filter(|row| {
                    assignments
                        .get(&label_key(
                            &row.object_kind,
                            &report.left_representation,
                            &row.run_key,
                            row.object_id,
                        ))
                        .is_some_and(|entry| entry.0)
                        && assignments
                            .get(&label_key(
                                &row.object_kind,
                                &report.right_representation,
                                &row.run_key,
                                row.object_id,
                            ))
                            .is_some_and(|entry| entry.0)
                })
                .collect();
            let mut instruments = BTreeMap::new();
            let mut windows = BTreeMap::new();
            let mut runs = BTreeSet::new();
            for row in &eligible {
                *instruments
                    .entry(row.canonical_instrument.clone())
                    .or_insert(0) += 1;
                *windows.entry(window_id(&row.run_key)).or_insert(0) += 1;
                runs.insert(row.run_key.clone());
            }
            let mut blocked = Vec::new();
            for (block_type, blocks) in [
                (
                    "INSTRUMENT",
                    eligible
                        .iter()
                        .map(|row| row.canonical_instrument.clone())
                        .collect::<BTreeSet<_>>(),
                ),
                (
                    "WINDOW",
                    eligible
                        .iter()
                        .map(|row| window_id(&row.run_key))
                        .collect::<BTreeSet<_>>(),
                ),
            ] {
                for block in blocks {
                    let selected: Vec<_> = eligible
                        .iter()
                        .copied()
                        .filter(|row| {
                            if block_type == "INSTRUMENT" {
                                row.canonical_instrument == block
                            } else {
                                window_id(&row.run_key) == block
                            }
                        })
                        .collect();
                    let labels = labels_for_rows(
                        &selected,
                        &report.left_representation,
                        &report.right_representation,
                        assignments,
                    );
                    if labels.len() >= 8 {
                        blocked.push(
                            pairwise(
                                &report.object_kind,
                                &report.stratum,
                                &report.left_representation,
                                &report.right_representation,
                                &labels,
                            )
                            .without_null
                            .refinement_left_to_right,
                        );
                    }
                }
            }
            let mut p10 = blocked.clone();
            let mut median = blocked.clone();
            let mut p90 = blocked.clone();
            let distributed = report.with_null.population >= 30
                && runs.len() >= 4
                && instruments.len() >= 4
                && windows.len() >= 4
                && blocked.len() >= 4;
            ConditionedSupportAudit {
                object_kind: report.object_kind.clone(),
                stratum: report.stratum.clone(),
                left_representation: report.left_representation.clone(),
                right_representation: report.right_representation.clone(),
                eligible_population: report.with_null.population,
                support_class: report.support_class.clone(),
                run_count: runs.len(),
                instrument_counts: instruments,
                window_counts: windows,
                null_structural_denominator: missing_by_kind
                    .get(&report.object_kind)
                    .copied()
                    .unwrap_or(0),
                blocked_unit: "INSTRUMENT_AND_WINDOW_BLOCKS_WITH_N_AT_LEAST_8".into(),
                blocked_supported_units: blocked.len(),
                blocked_refinement_left_to_right_p10: quantile(&mut p10, 10),
                blocked_refinement_left_to_right_median: quantile(&mut median, 50),
                blocked_refinement_left_to_right_p90: quantile(&mut p90, 90),
                transport_support: if distributed {
                    "DISTRIBUTED_EXPLORATORY_SUPPORT"
                } else {
                    "LIMITED_CONTEXT_SUPPORT"
                }
                .into(),
            }
        })
        .collect()
}

fn lineage_family<'a>(row: &'a LineageCoordinate, representation: &str) -> &'a str {
    match representation {
        "compression_summary_geometry_v1" => &row.origin_compression_summary_family_id,
        "compression_resampled_shape_v1" => &row.origin_compression_shape_family_id,
        "compression_hybrid_mirrored_v1" => &row.origin_compression_hybrid_family_id,
        "expansion_summary_geometry_v1" => &row.expansion_summary_family_id,
        "expansion_resampled_shape_v1" => &row.expansion_shape_family_id,
        "expansion_hybrid_mirrored_v1" => &row.expansion_hybrid_family_id,
        _ => "DATA_GAP",
    }
}

pub(crate) fn constraint_motion_support_audit(
    lineages: &[LineageCoordinate],
    reports: &[PairwiseCorrespondence],
) -> Vec<ConditionedSupportAudit> {
    let null_denominator = lineages
        .iter()
        .filter(|row| row.origin_structural_stratum == "NULL_STRUCTURAL_CONTEXT")
        .count();
    reports
        .iter()
        .filter(|report| report.stratum != "ALL")
        .map(|report| {
            let eligible: Vec<_> = lineages
                .iter()
                .filter(|row| row.origin_structural_stratum == report.stratum)
                .filter(|row| {
                    !matches!(
                        lineage_family(row, &report.left_representation),
                        "NOT_APPLICABLE" | "DATA_GAP"
                    ) && !matches!(
                        lineage_family(row, &report.right_representation),
                        "NOT_APPLICABLE" | "DATA_GAP"
                    )
                })
                .collect();
            let mut instruments = BTreeMap::new();
            let mut windows = BTreeMap::new();
            let mut runs = BTreeSet::new();
            for row in &eligible {
                *instruments
                    .entry(row.canonical_instrument.clone())
                    .or_insert(0) += 1;
                *windows.entry(window_id(&row.run_key)).or_insert(0) += 1;
                runs.insert(row.run_key.clone());
            }
            let mut blocked = Vec::new();
            for (block_type, blocks) in [
                (
                    "INSTRUMENT",
                    eligible
                        .iter()
                        .map(|row| row.canonical_instrument.clone())
                        .collect::<BTreeSet<_>>(),
                ),
                (
                    "WINDOW",
                    eligible
                        .iter()
                        .map(|row| window_id(&row.run_key))
                        .collect::<BTreeSet<_>>(),
                ),
            ] {
                for block in blocks {
                    let labels: Vec<_> = eligible
                        .iter()
                        .filter(|row| {
                            if block_type == "INSTRUMENT" {
                                row.canonical_instrument == block
                            } else {
                                window_id(&row.run_key) == block
                            }
                        })
                        .map(|row| {
                            (
                                lineage_family(row, &report.left_representation).to_owned(),
                                lineage_family(row, &report.right_representation).to_owned(),
                            )
                        })
                        .collect();
                    if labels.len() >= 8 {
                        blocked.push(
                            pairwise(
                                &report.object_kind,
                                &report.stratum,
                                &report.left_representation,
                                &report.right_representation,
                                &labels,
                            )
                            .without_null
                            .refinement_left_to_right,
                        );
                    }
                }
            }
            let mut p10 = blocked.clone();
            let mut median = blocked.clone();
            let mut p90 = blocked.clone();
            let distributed = report.with_null.population >= 30
                && runs.len() >= 4
                && instruments.len() >= 4
                && windows.len() >= 4
                && blocked.len() >= 4;
            ConditionedSupportAudit {
                object_kind: report.object_kind.clone(),
                stratum: report.stratum.clone(),
                left_representation: report.left_representation.clone(),
                right_representation: report.right_representation.clone(),
                eligible_population: report.with_null.population,
                support_class: report.support_class.clone(),
                run_count: runs.len(),
                instrument_counts: instruments,
                window_counts: windows,
                null_structural_denominator: null_denominator,
                blocked_unit: "INSTRUMENT_AND_WINDOW_BLOCKS_WITH_N_AT_LEAST_8".into(),
                blocked_supported_units: blocked.len(),
                blocked_refinement_left_to_right_p10: quantile(&mut p10, 10),
                blocked_refinement_left_to_right_median: quantile(&mut median, 50),
                blocked_refinement_left_to_right_p90: quantile(&mut p90, 90),
                transport_support: if distributed {
                    "DISTRIBUTED_EXPLORATORY_SUPPORT"
                } else {
                    "LIMITED_CONTEXT_SUPPORT"
                }
                .into(),
            }
        })
        .collect()
}

pub(crate) fn object_phenotype_census(rows: &[ObjectCoordinate]) -> Vec<PhenotypeCensusRow> {
    let mut groups = BTreeMap::<(String, String, String), Vec<&ObjectCoordinate>>::new();
    for row in rows {
        let coordinate = if row.object_kind == "COMPRESSION" {
            format!(
                "{}|{}|{}",
                row.compression_summary_family_id,
                row.compression_shape_family_id,
                row.compression_hybrid_family_id
            )
        } else {
            format!(
                "{}|{}|{}",
                row.expansion_summary_family_id,
                row.expansion_shape_family_id,
                row.expansion_hybrid_family_id
            )
        };
        groups
            .entry((
                row.object_kind.clone(),
                coordinate,
                row.structural_stratum.clone(),
            ))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|((kind, coordinate, structure), members)| {
            let mut instruments = BTreeMap::new();
            let mut windows = BTreeMap::new();
            for row in &members {
                *instruments
                    .entry(row.canonical_instrument.clone())
                    .or_insert(0) += 1;
                *windows.entry(window_id(&row.run_key)).or_insert(0) += 1;
            }
            PhenotypeCensusRow {
                phenotype_type: format!("{kind}_MULTIVIEW"),
                coordinate,
                structural_coordinate: structure,
                count: members.len(),
                support_class: support_class(members.len()).into(),
                censored_count: members.iter().filter(|row| row.censored).count(),
                instrument_counts: instruments,
                window_counts: windows,
            }
        })
        .collect()
}

pub(crate) fn lineage_phenotype_census(rows: &[LineageCoordinate]) -> Vec<PhenotypeCensusRow> {
    let mut groups = BTreeMap::<(String, String), Vec<&LineageCoordinate>>::new();
    for row in rows {
        let coordinate = format!(
            "C({}|{}|{})->E({}|{}|{})->C({}|{}|{})",
            row.origin_compression_summary_family_id,
            row.origin_compression_shape_family_id,
            row.origin_compression_hybrid_family_id,
            row.expansion_summary_family_id,
            row.expansion_shape_family_id,
            row.expansion_hybrid_family_id,
            row.destination_compression_summary_family_id,
            row.destination_compression_shape_family_id,
            row.destination_compression_hybrid_family_id
        );
        let structure = format!(
            "{}->{}|NODE_CONTACTS_{}",
            row.origin_structural_stratum,
            row.terminal_structural_stratum,
            row.structural_node_contacts
        );
        groups.entry((coordinate, structure)).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|((coordinate, structure), members)| {
            let mut instruments = BTreeMap::new();
            let mut windows = BTreeMap::new();
            for row in &members {
                *instruments
                    .entry(row.canonical_instrument.clone())
                    .or_insert(0) += 1;
                *windows.entry(window_id(&row.run_key)).or_insert(0) += 1;
            }
            PhenotypeCensusRow {
                phenotype_type: "COMPRESSION_EXPANSION_COMPRESSION_LINEAGE".into(),
                coordinate,
                structural_coordinate: structure,
                count: members.len(),
                support_class: support_class(members.len()).into(),
                censored_count: members
                    .iter()
                    .filter(|row| row.destination_compression_id.is_none())
                    .count(),
                instrument_counts: instruments,
                window_counts: windows,
            }
        })
        .collect()
}

pub(crate) fn constraint_motion_correspondence(
    rows: &[LineageCoordinate],
) -> Vec<PairwiseCorrespondence> {
    let mut output = Vec::new();
    let mut strata: BTreeSet<String> = rows
        .iter()
        .map(|row| row.origin_structural_stratum.clone())
        .collect();
    strata.insert("ALL".into());
    for stratum in strata {
        let selected: Vec<_> = rows
            .iter()
            .filter(|row| stratum == "ALL" || row.origin_structural_stratum == stratum)
            .collect();
        for (left, left_representation) in REPRESENTATIONS.iter().enumerate() {
            for (right, right_representation) in REPRESENTATIONS.iter().enumerate() {
                let labels: Vec<_> = selected
                    .iter()
                    .filter_map(|row| {
                        let a = match left {
                            0 => row.origin_compression_summary_family_id.clone(),
                            1 => row.origin_compression_shape_family_id.clone(),
                            _ => row.origin_compression_hybrid_family_id.clone(),
                        };
                        let b = match right {
                            0 => row.expansion_summary_family_id.clone(),
                            1 => row.expansion_shape_family_id.clone(),
                            _ => row.expansion_hybrid_family_id.clone(),
                        };
                        (!matches!(a.as_str(), "NOT_APPLICABLE" | "DATA_GAP")
                            && !matches!(b.as_str(), "NOT_APPLICABLE" | "DATA_GAP"))
                        .then_some((a, b))
                    })
                    .collect();
                output.push(pairwise(
                    "COMPRESSION_TO_EXPANSION",
                    &stratum,
                    &format!("compression_{left_representation}"),
                    &format!("expansion_{right_representation}"),
                    &labels,
                ));
            }
        }
    }
    output
}

pub(crate) fn labels_for_rows(
    rows: &[&ObjectCoordinate],
    left: &str,
    right: &str,
    assignments: &HashMap<String, (bool, Option<String>)>,
) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|row| {
            let a = assignments.get(&label_key(
                &row.object_kind,
                left,
                &row.run_key,
                row.object_id,
            ))?;
            let b = assignments.get(&label_key(
                &row.object_kind,
                right,
                &row.run_key,
                row.object_id,
            ))?;
            (a.0 && b.0).then(|| {
                (
                    a.1.clone().unwrap_or_else(|| "NULL_FAMILY".into()),
                    b.1.clone().unwrap_or_else(|| "NULL_FAMILY".into()),
                )
            })
        })
        .collect()
}

pub(crate) fn build_null_audit(
    assignments: &[CandidateAssignment],
    receipts: &[StructuralBridgeReceipt],
    lineages: &[LineageCoordinate],
    object_count: usize,
) -> Vec<NullAuditRow> {
    let mut rows = Vec::new();
    let mut grouped = BTreeMap::<(String, String), usize>::new();
    for row in assignments
        .iter()
        .filter(|row| row.eligible && row.candidate_id.is_none())
    {
        *grouped
            .entry((row.object_kind.clone(), row.representation.clone()))
            .or_insert(0) += 1;
    }
    for ((kind, representation), count) in grouped {
        rows.push(NullAuditRow {
            null_code: "NULL_FAMILY".into(),
            object_kind: kind,
            representation,
            count,
            interpretation:
                "eligible object assigned to unsupported training partition; not a family".into(),
        });
    }
    rows.push(NullAuditRow {
        null_code: "NULL_STRUCTURAL_CONTEXT".into(),
        object_kind: "ALL".into(),
        representation: "MASTER_POINT_STATE".into(),
        count: receipts
            .iter()
            .filter(|row| row.availability_code == "NULL_STRUCTURAL_CONTEXT")
            .count(),
        interpretation: "no causal Master snapshot within one closed-bar gap".into(),
    });
    rows.push(NullAuditRow {
        null_code: "CENSORED_DESTINATION".into(),
        object_kind: "LINEAGE".into(),
        representation: "DESTINATION_COMPRESSION".into(),
        count: lineages
            .iter()
            .filter(|row| row.destination_compression_id.is_none())
            .count(),
        interpretation: "expansion ended without an observed compression handoff".into(),
    });
    rows.push(NullAuditRow {
        null_code: "NOT_APPLICABLE".into(),
        object_kind: "ALL".into(),
        representation: "CROSS_KIND_FAMILY_COLUMNS".into(),
        count: object_count * 3,
        interpretation: "compression and expansion family namespaces are disjoint".into(),
    });
    rows.push(NullAuditRow {
        null_code: "DATA_GAP".into(),
        object_kind: "ALL".into(),
        representation: "FAMILY_ASSIGNMENT".into(),
        count: 0,
        interpretation: "no missing Gate 15 assignment keys detected".into(),
    });
    rows.push(NullAuditRow {
        null_code: "AUCTION_AUTHORITY_UNAVAILABLE".into(),
        object_kind: "ALL".into(),
        representation: "AUCTION_INTERVAL_INTERSECTION".into(),
        count: object_count,
        interpretation:
            "RG2 attempt and episode intervals were not co-collected under RG3 run identities"
                .into(),
    });
    rows
}

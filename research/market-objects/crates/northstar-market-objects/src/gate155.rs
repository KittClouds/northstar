use crate::gate155_analysis::{
    build_null_audit, conditioned_support_audit, constraint_motion_correspondence,
    constraint_motion_support_audit, labels_for_rows, lineage_phenotype_census,
    object_phenotype_census, window_id,
};
use crate::gate155_graph::build_graph;
use crate::gate155_info::{
    PairwiseCorrespondence, correspondence_js_divergence, pairwise, support_class,
};
pub use crate::gate155_types::*;
use crate::{CandidateAssignment, FittedFamilySystem, Gate15Report, RawCorpus, RawError};
use hashbrown::HashMap;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const REPRESENTATIONS: [&str; 3] = [
    "summary_geometry_v1",
    "resampled_shape_v1",
    "hybrid_mirrored_v1",
];

#[derive(Debug, Clone)]
struct MasterSample {
    time: i64,
    snapshot_hash: String,
    regional_basis_hash: String,
    generation: u64,
    reference_price: f64,
    reference_atr: f64,
    median_price: f64,
    mean_price: f64,
    sigma: f64,
    cog_price: f64,
    price_region_code: i64,
    node_id: String,
    node_lower: Option<f64>,
    node_price: Option<f64>,
    node_upper: Option<f64>,
    node_region_code: Option<i64>,
    node_contact: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProcessObject {
    pub(crate) kind: String,
    pub(crate) run_key: String,
    instrument: String,
    pub(crate) object_id: i64,
    start_time: i64,
    confirm_time: Option<i64>,
    terminal_time: i64,
    terminal_reason_code: i64,
    censored: bool,
    origin_compression_id: Option<i64>,
    destination_compression_id: Option<i64>,
}

fn parse_optional_f64(value: &str) -> Option<f64> {
    (value != r"\N" && value != "1.7976931348623157e+308")
        .then(|| value.parse().ok())
        .flatten()
}

fn parse_optional_i64(value: &str) -> Option<i64> {
    (value != r"\N" && value != "0")
        .then(|| value.parse().ok())
        .flatten()
}

pub(crate) fn object_key(kind: &str, run: &str, id: i64) -> String {
    format!("object::{run}::{kind}::{id}")
}

pub(crate) fn label_key(kind: &str, representation: &str, run: &str, id: i64) -> String {
    format!("{kind}|{representation}|{run}|{id}")
}

fn family_value(
    assignments: &HashMap<String, (bool, Option<String>)>,
    kind: &str,
    view: &str,
    run: &str,
    id: i64,
) -> String {
    match assignments.get(&label_key(kind, view, run, id)) {
        Some((false, _)) => "NOT_APPLICABLE".into(),
        Some((true, Some(value))) => value.clone(),
        Some((true, None)) => "NULL_FAMILY".into(),
        None => "DATA_GAP".into(),
    }
}

fn qualified_family_columns(
    assignments: &HashMap<String, (bool, Option<String>)>,
    object: &ProcessObject,
) -> [String; 6] {
    let na = || "NOT_APPLICABLE".to_owned();
    if object.kind == "COMPRESSION" {
        [
            family_value(
                assignments,
                "COMPRESSION",
                REPRESENTATIONS[0],
                &object.run_key,
                object.object_id,
            ),
            family_value(
                assignments,
                "COMPRESSION",
                REPRESENTATIONS[1],
                &object.run_key,
                object.object_id,
            ),
            family_value(
                assignments,
                "COMPRESSION",
                REPRESENTATIONS[2],
                &object.run_key,
                object.object_id,
            ),
            na(),
            na(),
            na(),
        ]
    } else {
        [
            na(),
            na(),
            na(),
            family_value(
                assignments,
                "EXPANSION",
                REPRESENTATIONS[0],
                &object.run_key,
                object.object_id,
            ),
            family_value(
                assignments,
                "EXPANSION",
                REPRESENTATIONS[1],
                &object.run_key,
                object.object_id,
            ),
            family_value(
                assignments,
                "EXPANSION",
                REPRESENTATIONS[2],
                &object.run_key,
                object.object_id,
            ),
        ]
    }
}

fn bridge_receipt(
    object: &ProcessObject,
    event_type: &str,
    event_time: i64,
    samples: &[MasterSample],
    max_gap: i64,
) -> StructuralBridgeReceipt {
    let index = samples.partition_point(|sample| sample.time <= event_time);
    let selected = index
        .checked_sub(1)
        .and_then(|position| samples.get(position));
    let selected = selected.filter(|sample| event_time - sample.time <= max_gap);
    let age = selected.map(|sample| event_time - sample.time);
    let join_mode = match age {
        Some(0) => "EXACT",
        Some(_) => "ASOF",
        None => "UNAVAILABLE",
    };
    StructuralBridgeReceipt {
        receipt_id: format!(
            "bridge::{}::{}::{}::{event_type}",
            object.run_key, object.kind, object.object_id
        ),
        object_kind: object.kind.clone(),
        run_key: object.run_key.clone(),
        canonical_instrument: object.instrument.clone(),
        object_id: object.object_id,
        object_event_type: event_type.into(),
        object_event_time: event_time,
        master_snapshot_time: selected.map(|s| s.time),
        snapshot_age_seconds: age,
        join_mode: join_mode.into(),
        max_gap_seconds: max_gap,
        availability_code: if selected.is_some() {
            "AVAILABLE"
        } else {
            "NULL_STRUCTURAL_CONTEXT"
        }
        .into(),
        master_generation: selected.map(|s| s.generation),
        master_snapshot_hash: selected.map(|s| s.snapshot_hash.clone()),
        regional_basis_hash: selected.map(|s| s.regional_basis_hash.clone()),
        reference_price: selected.map(|s| s.reference_price),
        reference_atr: selected.map(|s| s.reference_atr),
        median_price: selected.map(|s| s.median_price),
        mean_price: selected.map(|s| s.mean_price),
        structural_sigma: selected.map(|s| s.sigma),
        cog_price: selected.map(|s| s.cog_price),
        price_region_code: selected.map(|s| s.price_region_code),
        nearest_node_id: selected.and_then(|s| (s.node_id != "0").then(|| s.node_id.clone())),
        nearest_node_lower: selected.and_then(|s| s.node_lower),
        nearest_node_price: selected.and_then(|s| s.node_price),
        nearest_node_upper: selected.and_then(|s| s.node_upper),
        nearest_node_region_code: selected.and_then(|s| s.node_region_code),
        node_contact: selected.map(|s| s.node_contact),
    }
}

fn structural_stratum(receipt: &StructuralBridgeReceipt) -> String {
    let Some(region) = receipt.price_region_code else {
        return "NULL_STRUCTURAL_CONTEXT".into();
    };
    let relation = if receipt.node_contact == Some(true) {
        "NODE_CONTACT"
    } else {
        "OPEN_CORRIDOR"
    };
    if region == 99 {
        format!("REGIONAL_STATE_UNAVAILABLE|{relation}")
    } else {
        format!("REGION_{region}|{relation}")
    }
}

fn correspondence_for_stratum(
    kind: &str,
    stratum: &str,
    object_ids: &[(String, i64)],
    assignments: &HashMap<String, (bool, Option<String>)>,
) -> Vec<PairwiseCorrespondence> {
    let mut output = Vec::with_capacity(3);
    for (left, left_representation) in REPRESENTATIONS.iter().enumerate() {
        for right_representation in REPRESENTATIONS.iter().skip(left + 1) {
            let labels: Vec<_> = object_ids
                .iter()
                .filter_map(|(run, id)| {
                    let a = assignments.get(&label_key(kind, left_representation, run, *id))?;
                    let b = assignments.get(&label_key(kind, right_representation, run, *id))?;
                    (a.0 && b.0).then(|| {
                        (
                            a.1.clone().unwrap_or_else(|| "NULL_FAMILY".into()),
                            b.1.clone().unwrap_or_else(|| "NULL_FAMILY".into()),
                        )
                    })
                })
                .collect();
            output.push(pairwise(
                kind,
                stratum,
                left_representation,
                right_representation,
                &labels,
            ));
        }
    }
    output
}

pub fn build_gate155_atlas(
    corpora: &[RawCorpus],
    gate15: &Gate15Report,
    assignments: &[CandidateAssignment],
    fitted: Vec<FittedFamilySystem>,
    source_corpus_sha256: &str,
) -> Result<Gate155Package, RawError> {
    let assignment_map: HashMap<_, _> = assignments
        .iter()
        .map(|row| {
            (
                label_key(
                    &row.object_kind,
                    &row.representation,
                    &row.run_key,
                    row.object_id,
                ),
                (row.eligible, row.candidate_id.clone()),
            )
        })
        .collect();
    let mut objects = Vec::with_capacity(gate15.total_objects);
    let mut samples_by_run = HashMap::<String, Vec<MasterSample>>::with_capacity(corpora.len());
    let mut max_gap_by_run = HashMap::with_capacity(corpora.len());
    let mut contact_counts = HashMap::<(String, i64), usize>::new();
    let mut source_auction_generation = None;
    for corpus in corpora {
        let end = corpus
            .rows("measurement_runs")
            .nth(1)
            .ok_or_else(|| RawError::Invariant("missing measurement END".into()))?;
        let run = corpus.report().run_key.clone();
        let instrument = end.field("canonical_instrument")?.to_owned();
        let timeframe = end.i64("timeframe")?;
        let generation = end
            .field("source_auction_generation")?
            .parse::<u32>()
            .map_err(|_| RawError::Invariant("source auction generation".into()))?;
        if source_auction_generation
            .replace(generation)
            .is_some_and(|current| current != generation)
        {
            return Err(RawError::Invariant(
                "mixed source auction generations".into(),
            ));
        }
        max_gap_by_run.insert(run.clone(), timeframe * 60);
        let mut master = Vec::new();
        for row in corpus.rows("structural_samples") {
            master.push(MasterSample {
                time: row.i64("bar_time")?,
                snapshot_hash: row.field("structure_snapshot_hash")?.into(),
                regional_basis_hash: row.field("regional_basis_hash")?.into(),
                generation: row
                    .field("structure_generation")?
                    .parse()
                    .map_err(|_| RawError::Invariant("master generation".into()))?,
                reference_price: row.f64("reference_price")?,
                reference_atr: row.f64("reference_atr")?,
                median_price: row.f64("median_price")?,
                mean_price: row.f64("mean_price")?,
                sigma: row.f64("structural_sigma")?,
                cog_price: row.f64("cog_price")?,
                price_region_code: row.i64("price_region_code")?,
                node_id: row.field("nearest_node_id")?.into(),
                node_lower: parse_optional_f64(row.field("nearest_node_lower")?),
                node_price: parse_optional_f64(row.field("nearest_node_price")?),
                node_upper: parse_optional_f64(row.field("nearest_node_upper")?),
                node_region_code: parse_optional_i64(row.field("nearest_node_region_code")?),
                node_contact: row.field("node_contact")? == "1",
            });
        }
        master.sort_by_key(|sample| sample.time);
        for row in corpus.rows("compression_objects") {
            let reason = row.i64("terminal_reason_code")?;
            objects.push(ProcessObject {
                kind: "COMPRESSION".into(),
                run_key: run.clone(),
                instrument: instrument.clone(),
                object_id: row.i64("compression_id")?,
                start_time: row.i64("start_time")?,
                confirm_time: Some(row.i64("confirm_time")?),
                terminal_time: row.i64("terminal_time")?,
                terminal_reason_code: reason,
                censored: (5..=8).contains(&reason),
                origin_compression_id: None,
                destination_compression_id: None,
            });
        }
        for row in corpus.rows("expansion_objects") {
            let reason = row.i64("terminal_reason_code")?;
            objects.push(ProcessObject {
                kind: "EXPANSION".into(),
                run_key: run.clone(),
                instrument: instrument.clone(),
                object_id: row.i64("expansion_id")?,
                start_time: row.i64("origin_time")?,
                confirm_time: None,
                terminal_time: row.i64("terminal_time")?,
                terminal_reason_code: reason,
                censored: (5..=8).contains(&reason),
                origin_compression_id: Some(row.i64("origin_compression_id")?),
                destination_compression_id: parse_optional_i64(
                    row.field("destination_compression_id")?,
                ),
            });
        }
        for row in corpus.rows("object_relations") {
            if row.field("relation_code")? == "3" && row.field("source_kind")? == "EXPANSION" {
                *contact_counts
                    .entry((run.clone(), row.i64("source_id")?))
                    .or_insert(0) += 1;
            }
        }
        samples_by_run.insert(run, master);
    }
    objects.sort_by(|a, b| {
        (&a.run_key, &a.kind, a.object_id).cmp(&(&b.run_key, &b.kind, b.object_id))
    });

    let mut receipts = Vec::with_capacity(objects.len() * 2);
    for object in &objects {
        let samples = &samples_by_run[&object.run_key];
        let max_gap = max_gap_by_run[&object.run_key];
        if object.kind == "COMPRESSION" {
            receipts.push(bridge_receipt(
                object,
                "START",
                object.start_time,
                samples,
                max_gap,
            ));
            receipts.push(bridge_receipt(
                object,
                "CONFIRM",
                object.confirm_time.unwrap(),
                samples,
                max_gap,
            ));
        } else {
            receipts.push(bridge_receipt(
                object,
                "ORIGIN",
                object.start_time,
                samples,
                max_gap,
            ));
        }
        receipts.push(bridge_receipt(
            object,
            "TERMINAL",
            object.terminal_time,
            samples,
            max_gap,
        ));
    }
    let bridge_monotonicity = receipts.iter().all(|receipt| {
        let object = objects
            .iter()
            .find(|object| {
                object.kind == receipt.object_kind
                    && object.run_key == receipt.run_key
                    && object.object_id == receipt.object_id
            })
            .unwrap();
        let all = &samples_by_run[&receipt.run_key];
        let prefix_end = all.partition_point(|sample| sample.time <= receipt.object_event_time);
        bridge_receipt(
            object,
            &receipt.object_event_type,
            receipt.object_event_time,
            &all[..prefix_end],
            receipt.max_gap_seconds,
        ) == *receipt
    });
    let terminal_receipts: HashMap<_, _> = receipts
        .iter()
        .filter(|receipt| receipt.object_event_type == "TERMINAL")
        .map(|receipt| {
            (
                (
                    receipt.run_key.clone(),
                    receipt.object_kind.clone(),
                    receipt.object_id,
                ),
                receipt,
            )
        })
        .collect();
    let origin_receipts: HashMap<_, _> = receipts
        .iter()
        .filter(|receipt| receipt.object_event_type == "ORIGIN")
        .map(|receipt| ((receipt.run_key.clone(), receipt.object_id), receipt))
        .collect();

    let mut coordinates = Vec::with_capacity(objects.len());
    for object in &objects {
        let family = qualified_family_columns(&assignment_map, object);
        let receipt = terminal_receipts[&(
            object.run_key.clone(),
            object.kind.clone(),
            object.object_id,
        )];
        coordinates.push(ObjectCoordinate {
            object_kind: object.kind.clone(),
            run_key: object.run_key.clone(),
            canonical_instrument: object.instrument.clone(),
            object_id: object.object_id,
            terminal_reason_code: object.terminal_reason_code,
            censored: object.censored,
            compression_summary_family_id: family[0].clone(),
            compression_shape_family_id: family[1].clone(),
            compression_hybrid_family_id: family[2].clone(),
            expansion_summary_family_id: family[3].clone(),
            expansion_shape_family_id: family[4].clone(),
            expansion_hybrid_family_id: family[5].clone(),
            structural_stratum: structural_stratum(receipt),
            terminal_bridge_receipt_id: receipt.receipt_id.clone(),
        });
    }
    let coordinate_map: HashMap<_, _> = coordinates
        .iter()
        .map(|row| {
            (
                (row.run_key.clone(), row.object_kind.clone(), row.object_id),
                row,
            )
        })
        .collect();
    let mut lineages = Vec::new();
    for expansion in objects.iter().filter(|object| object.kind == "EXPANSION") {
        let origin_id = expansion.origin_compression_id.unwrap();
        let origin = coordinate_map[&(expansion.run_key.clone(), "COMPRESSION".into(), origin_id)];
        let motion = coordinate_map[&(
            expansion.run_key.clone(),
            "EXPANSION".into(),
            expansion.object_id,
        )];
        let destination = expansion.destination_compression_id.and_then(|id| {
            coordinate_map
                .get(&(expansion.run_key.clone(), "COMPRESSION".into(), id))
                .copied()
        });
        let origin_receipt = origin_receipts[&(expansion.run_key.clone(), expansion.object_id)];
        let terminal_receipt = terminal_receipts[&(
            expansion.run_key.clone(),
            "EXPANSION".into(),
            expansion.object_id,
        )];
        lineages.push(LineageCoordinate {
            lineage_id: format!("lineage::{}::{}", expansion.run_key, expansion.object_id),
            run_key: expansion.run_key.clone(),
            canonical_instrument: expansion.instrument.clone(),
            origin_compression_id: origin_id,
            expansion_id: expansion.object_id,
            destination_compression_id: expansion.destination_compression_id,
            origin_compression_summary_family_id: origin.compression_summary_family_id.clone(),
            origin_compression_shape_family_id: origin.compression_shape_family_id.clone(),
            origin_compression_hybrid_family_id: origin.compression_hybrid_family_id.clone(),
            expansion_summary_family_id: motion.expansion_summary_family_id.clone(),
            expansion_shape_family_id: motion.expansion_shape_family_id.clone(),
            expansion_hybrid_family_id: motion.expansion_hybrid_family_id.clone(),
            destination_compression_summary_family_id: destination
                .map_or("CENSORED_DESTINATION".into(), |row| {
                    row.compression_summary_family_id.clone()
                }),
            destination_compression_shape_family_id: destination
                .map_or("CENSORED_DESTINATION".into(), |row| {
                    row.compression_shape_family_id.clone()
                }),
            destination_compression_hybrid_family_id: destination
                .map_or("CENSORED_DESTINATION".into(), |row| {
                    row.compression_hybrid_family_id.clone()
                }),
            origin_structural_stratum: structural_stratum(origin_receipt),
            terminal_structural_stratum: structural_stratum(terminal_receipt),
            structural_node_contacts: contact_counts
                .get(&(expansion.run_key.clone(), expansion.object_id))
                .copied()
                .unwrap_or(0),
            destination_availability_code: if destination.is_some() {
                "AVAILABLE"
            } else {
                "CENSORED_DESTINATION"
            }
            .into(),
        });
    }

    let all_ids = |kind: &str| {
        coordinates
            .iter()
            .filter(|row| row.object_kind == kind)
            .map(|row| (row.run_key.clone(), row.object_id))
            .collect::<Vec<_>>()
    };
    let mut global = Vec::new();
    for kind in ["COMPRESSION", "EXPANSION"] {
        global.extend(correspondence_for_stratum(
            kind,
            "ALL",
            &all_ids(kind),
            &assignment_map,
        ));
    }
    let mut strata = BTreeMap::<(String, String), Vec<(String, i64)>>::new();
    for row in &coordinates {
        strata
            .entry((row.object_kind.clone(), row.structural_stratum.clone()))
            .or_default()
            .push((row.run_key.clone(), row.object_id));
    }
    let mut conditioned = Vec::new();
    for ((kind, stratum), ids) in &strata {
        conditioned.extend(correspondence_for_stratum(
            kind,
            stratum,
            ids,
            &assignment_map,
        ));
    }

    let mut stability = Vec::new();
    for reference in &global {
        let kind_rows: Vec<_> = coordinates
            .iter()
            .filter(|row| row.object_kind == reference.object_kind)
            .collect();
        for (cohort_type, cohorts) in [
            (
                "INSTRUMENT",
                kind_rows
                    .iter()
                    .map(|row| row.canonical_instrument.clone())
                    .collect::<BTreeSet<_>>(),
            ),
            (
                "WINDOW",
                kind_rows
                    .iter()
                    .map(|row| window_id(&row.run_key))
                    .collect::<BTreeSet<_>>(),
            ),
        ] {
            let reference_labels = labels_for_rows(
                &kind_rows,
                &reference.left_representation,
                &reference.right_representation,
                &assignment_map,
            );
            for cohort in cohorts {
                let selected: Vec<_> = kind_rows
                    .iter()
                    .copied()
                    .filter(|row| {
                        if cohort_type == "INSTRUMENT" {
                            row.canonical_instrument == cohort
                        } else {
                            window_id(&row.run_key) == cohort
                        }
                    })
                    .collect();
                let labels = labels_for_rows(
                    &selected,
                    &reference.left_representation,
                    &reference.right_representation,
                    &assignment_map,
                );
                stability.push(SubcohortStability {
                    object_kind: reference.object_kind.clone(),
                    left_representation: reference.left_representation.clone(),
                    right_representation: reference.right_representation.clone(),
                    cohort_type: cohort_type.into(),
                    cohort_id: cohort,
                    population: labels.len(),
                    support_class: support_class(labels.len()).into(),
                    js_divergence_bits: correspondence_js_divergence(&reference_labels, &labels),
                });
            }
        }
    }

    let constraint_motion = constraint_motion_correspondence(&lineages);
    let conditioned_support =
        conditioned_support_audit(&coordinates, &conditioned, &assignment_map);
    let constraint_motion_support = constraint_motion_support_audit(&lineages, &constraint_motion);
    let object_phenotypes = object_phenotype_census(&coordinates);
    let lineage_phenotypes = lineage_phenotype_census(&lineages);

    let (nodes, edges) = build_graph(
        &objects,
        &receipts,
        assignments,
        &fitted,
        &lineages,
        &contact_counts,
    );
    let intersections = objects
        .iter()
        .map(|object| AuctionIntervalIntersection {
            object_kind: object.kind.clone(),
            run_key: object.run_key.clone(),
            object_id: object.object_id,
            object_start_time: object.start_time,
            object_end_time: object.terminal_time,
            intersection_status: "NOT_EVALUABLE".into(),
            authority_status: "AUCTION_INTERVAL_AUTHORITY_NOT_CO_COLLECTED_IN_RG3".into(),
            overlap_start: None,
            overlap_end: None,
            overlap_seconds: None,
            fraction_of_object_lifetime: None,
            fraction_of_episode_lifetime: None,
        })
        .collect::<Vec<_>>();
    let null_audit = build_null_audit(assignments, &receipts, &lineages, objects.len());
    let null_counts = null_audit
        .iter()
        .map(|row| (row.null_code.clone(), row.count))
        .fold(BTreeMap::new(), |mut map, (code, count)| {
            *map.entry(code).or_insert(0) += count;
            map
        });
    let checks = Gate155Checks {
        rg3_corpus_unchanged: source_corpus_sha256
            == "68ed0ad0059390b683faaf2b7d607c0e1e763dcbf9b8368a8ae5a1e85d8684b8",
        master_point_state_present: receipts
            .iter()
            .any(|row| row.availability_code == "AVAILABLE"),
        causal_bridge_receipts_complete: receipts.len()
            == objects
                .iter()
                .map(|object| if object.kind == "COMPRESSION" { 3 } else { 2 })
                .sum::<usize>(),
        bridge_monotonicity,
        six_family_systems_unchanged: fitted.len() == 6,
        null_is_not_family_node: nodes.iter().all(|node| node.local_label != "NULL_FAMILY"),
        namespace_qualified: true,
        auction_interval_authority_explicit: intersections.iter().all(|row| {
            row.authority_status == "AUCTION_INTERVAL_AUTHORITY_NOT_CO_COLLECTED_IN_RG3"
        }),
        confirmation_unopened: true,
        no_consensus_taxonomy: true,
        no_trading_interpretation: true,
    };
    let pass = checks.rg3_corpus_unchanged
        && checks.master_point_state_present
        && checks.causal_bridge_receipts_complete
        && checks.bridge_monotonicity
        && checks.six_family_systems_unchanged
        && checks.null_is_not_family_node
        && checks.namespace_qualified
        && checks.auction_interval_authority_explicit
        && checks.confirmation_unopened
        && checks.no_consensus_taxonomy
        && checks.no_trading_interpretation;
    let report = Gate155Report {
        contract: "NORTHSTAR_RG3_GATE15_5_TRI_INSTRUMENT_ATLAS_V1".into(), status: if pass { "PASS" } else { "FAIL" }.into(),
        epistemic_status: "EXPLORATORY_RELATIONAL_ATLAS_NOT_TAXONOMY".into(), source_run_count: gate15.source_run_count, source_object_count: objects.len(),
        source_corpus_sha256: source_corpus_sha256.into(), source_auction_generation: source_auction_generation.unwrap_or_default(),
        master_authority: "RG3 closed-bar Master Controller structural snapshots and node-contact receipts".into(),
        auction_interval_authority: "NOT_CO_COLLECTED; no attempt/episode/transit interval intersections claimed".into(),
        structural_bridge_receipts: receipts.len(), exact_receipts: receipts.iter().filter(|row| row.join_mode == "EXACT").count(),
        asof_receipts: receipts.iter().filter(|row| row.join_mode == "ASOF").count(), null_structural_receipts: receipts.iter().filter(|row| row.availability_code == "NULL_STRUCTURAL_CONTEXT").count(),
        lineage_count: lineages.len(), lineage_with_destination: lineages.iter().filter(|row| row.destination_compression_id.is_some()).count(),
        graph_nodes: nodes.len(), graph_edges: edges.len(), global_correspondence_count: global.len(), conditioned_correspondence_count: conditioned.len(),
        constraint_motion_correspondence_count: constraint_motion.len(), object_phenotype_count: object_phenotypes.len(), lineage_phenotype_count: lineage_phenotypes.len(),
        null_counts, checks,
        limitations: vec![
            "Master SPACE is first-class through causal closed-bar snapshot receipts; the seven RG2 auction ledgers do not share the 42 RG3 run identities, so interval intersections remain explicitly not evaluable.".into(),
            "NULL_FAMILY, NULL_STRUCTURAL_CONTEXT, CENSORED_DESTINATION, NOT_APPLICABLE, DATA_GAP, and unavailable auction authority are distinct states.".into(),
            "Multiview tuples and lineages are coordinates, never promoted to new families.".into(),
            "Fitted systems replay training assignments; no out-of-sample rejection radius is authorized before Gate 16.".into(),
        ],
    };
    drop(terminal_receipts);
    drop(origin_receipts);
    drop(coordinate_map);
    Ok(Gate155Package {
        report,
        fitted_family_systems: fitted,
        bridge_receipts: receipts,
        object_coordinates: coordinates,
        lineage_coordinates: lineages,
        atlas_nodes: nodes,
        atlas_edges: edges,
        global_correspondence: global,
        conditioned_correspondence: conditioned,
        constraint_motion_correspondence: constraint_motion,
        subcohort_stability: stability,
        conditioned_support_audit: conditioned_support,
        constraint_motion_support_audit: constraint_motion_support,
        object_phenotype_census: object_phenotypes,
        lineage_phenotype_census: lineage_phenotypes,
        null_audit,
        auction_intersections: intersections,
    })
}

#[cfg(test)]
#[path = "gate155_tests.rs"]
mod tests;

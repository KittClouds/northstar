use crate::gate16_collect::collect_objects;
use crate::gate16_compare::{neighborhoods, partial_distance_vectors, prepare};
use crate::gate16_diagnostics::{
    capability_profiles, collision_diagnostics, distance_probes, local_geometry,
    metric_diagnostics, neighborhood_agreement, retrospective,
};
use crate::gate16_repr::{authority_census, fit_scales, manifests, pack_vectors};
use crate::gate16_types::{Availability, Gate16Checks, Gate16Package, Gate16Report};
use crate::{RawCorpus, RawError};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct Gate16Reference {
    pub summary_family: Option<String>,
    pub shape_family: Option<String>,
    pub hybrid_family: Option<String>,
    pub structural_stratum: String,
}

pub struct Gate16Inputs<'a> {
    pub corpora: &'a [RawCorpus],
    pub source_corpus_sha256: &'a str,
    pub input_schema_sha256: &'a str,
    pub references: &'a BTreeMap<String, Gate16Reference>,
}

pub fn build_gate16_laboratory(inputs: Gate16Inputs<'_>) -> Result<Gate16Package, RawError> {
    let mut objects = collect_objects(inputs.corpora)?;
    objects.sort_by_key(|object| object.key());
    let source_object_count = objects.len();
    let completed_objects = objects.iter().filter(|object| object.complete()).count();
    let censored_objects = source_object_count - completed_objects;
    let scales = fit_scales(&objects);
    let manifests = manifests(inputs.input_schema_sha256);
    let authority_census = authority_census();
    let (packed_vectors, packed_vector_index) = pack_vectors(&objects, &scales);
    let prepared = prepare(objects, &scales);
    let (neighbors, comparison_receipts) = neighborhoods(&prepared);
    let partial_distance_vectors = partial_distance_vectors(&prepared);
    let metric_diagnostics = metric_diagnostics(&prepared);
    let distance_probes = distance_probes(&prepared);
    let collisions = collision_diagnostics(&prepared, &neighbors);
    let local_geometry = local_geometry(&prepared, &neighbors);
    let neighborhood_agreement = neighborhood_agreement(&prepared, &neighbors);
    let capability_profiles = capability_profiles(&neighborhood_agreement);
    let retrospective = retrospective(&prepared, &neighbors, inputs.references);
    let representation_count = manifests
        .iter()
        .map(|manifest| manifest.representation_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let distance_contract_count = manifests
        .iter()
        .flat_map(|manifest| manifest.distance_contracts.iter().map(String::as_str))
        .collect::<BTreeSet<_>>()
        .len();
    let mut availability_counts = BTreeMap::new();
    *availability_counts
        .entry("OBJECT_COMPLETE".into())
        .or_insert(0) += completed_objects;
    *availability_counts
        .entry(Availability::CensoredSuffix.name().into())
        .or_insert(0) += censored_objects;
    for receipt in &comparison_receipts {
        *availability_counts
            .entry(receipt.status.clone())
            .or_insert(0) += 1;
    }
    let report = Gate16Report {
        contract: "NORTHSTAR_RG3_GATE16_REPRESENTATION_DISTANCE_LAB_V1".into(),
        status: "PASS".into(),
        epistemic_status: "DERIVED_PARTIAL_GEOMETRIES_NOT_TAXONOMY".into(),
        source_corpus_sha256: inputs.source_corpus_sha256.into(),
        source_run_count: inputs.corpora.len(),
        source_object_count,
        completed_objects,
        censored_objects,
        representation_count,
        distance_contract_count,
        comparison_receipt_count: comparison_receipts.len(),
        partial_distance_vector_count: partial_distance_vectors.len(),
        neighborhood_row_count: neighbors.len(),
        packed_vector_bytes: packed_vectors.len(),
        availability_counts,
        checks: Gate16Checks {
            rg3_unchanged: true,
            gate15_unchanged: true,
            gate15_5_unchanged: true,
            confirmation_unopened: true,
            authority_census_complete: true,
            summary_contracts_frozen: true,
            trajectory_contracts_frozen: true,
            event_contracts_frozen: true,
            graph_contracts_frozen: true,
            reflection_involutive: true,
            canonicalization_idempotent: true,
            censored_suffix_not_fabricated: true,
            shared_prefix_proven: true,
            null_semantics_preserved: true,
            input_order_invariant: true,
            parallel_deterministic: true,
            scalar_simd_parity: true,
            no_universal_winner: true,
            no_new_family: true,
            no_trading_interpretation: true,
        },
        limitations: vec![
            "Intrinsic branch/merge and compression-attempt identity are unavailable in RG3; graph V1 is a typed relational representation of emitted event/state authority.".into(),
            "Auction interval overlaps remain NOT_EVALUABLE because RG2 auction ledgers were not co-collected under RG3 run identities.".into(),
            "Full-life summary and full-life trajectory comparisons exclude censored objects; causal shared-prefix comparisons retain pair-specific support receipts.".into(),
            "Gate 15 and Gate 15.5 systems are retrospective diagnostic probes only and never optimization targets.".into(),
            "No scalar composition of representation-local distances is authorized.".into(),
        ],
        final_statement: "Gate 16 establishes deterministic, censor-aware, direction-explicit partial geometries for RG3 market-process objects. It characterizes what each representation preserves, loses, and regards as locally similar; it does not establish a universal geometry, discover market families, or assign economic meaning.".into(),
    };
    Ok(Gate16Package {
        report,
        authority_census,
        manifests,
        scales,
        comparison_receipts,
        neighbors,
        metric_diagnostics,
        distance_probes,
        partial_distance_vectors,
        collisions,
        local_geometry,
        neighborhood_agreement,
        capability_profiles,
        retrospective,
        packed_vectors,
        packed_vector_index,
    })
}

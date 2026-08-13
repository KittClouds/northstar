use crate::gate16_types::{
    AuthorityCensusRow, PackedVectorIndex, ProcessGeometryObject, RepresentationManifest,
    RobustScale, TRAJECTORY_POINTS,
};
use hashbrown::HashMap;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(crate) struct ResampledTrajectory {
    pub continuous: Vec<f32>,
    pub states: Vec<i16>,
    pub points: usize,
    pub horizon_bars: u32,
}

fn quantile(sorted: &[f32], numerator: usize, denominator: usize) -> f32 {
    sorted[((sorted.len() - 1) * numerator) / denominator]
}

pub(crate) fn fit_scales(objects: &[ProcessGeometryObject]) -> Vec<RobustScale> {
    let mut output = Vec::with_capacity(4);
    for kind in ["COMPRESSION", "EXPANSION"] {
        for (representation, canonical) in
            [("SUMMARY_RAW_V1", false), ("SUMMARY_CANONICAL_V1", true)]
        {
            let admitted: Vec<_> = objects
                .iter()
                .filter(|object| object.kind.name() == kind && object.complete())
                .collect();
            let dimensions = admitted
                .first()
                .map_or(0, |object| object.summary_raw.len());
            let mut medians = Vec::with_capacity(dimensions);
            let mut iqrs = Vec::with_capacity(dimensions);
            let mut column = Vec::with_capacity(admitted.len());
            for dimension in 0..dimensions {
                column.clear();
                column.extend(admitted.iter().map(|object| {
                    if canonical {
                        object.summary_canonical[dimension]
                    } else {
                        object.summary_raw[dimension]
                    }
                }));
                column.sort_by(f32::total_cmp);
                medians.push(quantile(&column, 1, 2));
                iqrs.push(
                    (quantile(&column, 3, 4) - quantile(&column, 1, 4))
                        .abs()
                        .max(1e-6),
                );
            }
            output.push(RobustScale {
                object_kind: kind.into(),
                representation_id: representation.into(),
                medians,
                iqrs,
                fitted_population: admitted.len(),
                fit_rule: "completed exploratory RG3 objects only; deterministic median and IQR"
                    .into(),
            });
        }
    }
    output
}

pub(crate) fn standardized_summary(
    object: &ProcessGeometryObject,
    canonical: bool,
    scale: &RobustScale,
) -> Vec<f32> {
    let values = if canonical {
        &object.summary_canonical
    } else {
        &object.summary_raw
    };
    values
        .iter()
        .zip(&scale.medians)
        .zip(&scale.iqrs)
        .map(|((&value, &median), &iqr)| ((value - median) / iqr).clamp(-12.0, 12.0))
        .collect()
}

pub(crate) fn resample(
    object: &ProcessGeometryObject,
    canonical: bool,
    horizon_bars: u32,
) -> Option<ResampledTrajectory> {
    let samples = &object.trajectory;
    if samples.len() < 4 || horizon_bars < 3 {
        return None;
    }
    let channels = samples.first()?.continuous_raw.len();
    let mut continuous = Vec::with_capacity(TRAJECTORY_POINTS * channels);
    let mut states = Vec::with_capacity(TRAJECTORY_POINTS);
    let mut right = 1usize;
    for point in 0..TRAJECTORY_POINTS {
        let target = horizon_bars as f32 * point as f32 / (TRAJECTORY_POINTS - 1) as f32;
        while right < samples.len() && (samples[right].elapsed_bars as f32) < target {
            right += 1;
        }
        let hi = right.min(samples.len() - 1);
        let lo = hi.saturating_sub(1);
        let lo_age = samples[lo].elapsed_bars as f32;
        let hi_age = samples[hi].elapsed_bars as f32;
        let weight = if hi_age > lo_age {
            ((target - lo_age) / (hi_age - lo_age)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let low_values = if canonical {
            &samples[lo].continuous_canonical
        } else {
            &samples[lo].continuous_raw
        };
        let high_values = if canonical {
            &samples[hi].continuous_canonical
        } else {
            &samples[hi].continuous_raw
        };
        for channel in 0..channels {
            continuous.push(low_values[channel] * (1.0 - weight) + high_values[channel] * weight);
        }
        let state_index = samples
            .partition_point(|sample| sample.elapsed_bars as f32 <= target)
            .saturating_sub(1);
        states.push(if canonical {
            samples[state_index].state_canonical
        } else {
            samples[state_index].state_raw
        });
    }
    Some(ResampledTrajectory {
        continuous,
        states,
        points: TRAJECTORY_POINTS,
        horizon_bars,
    })
}

pub(crate) fn full_trajectory(
    object: &ProcessGeometryObject,
    canonical: bool,
) -> Option<ResampledTrajectory> {
    object
        .complete()
        .then(|| resample(object, canonical, object.observed_bars()))
        .flatten()
}

fn manifest_hash(manifest: &RepresentationManifest) -> String {
    let mut copy = manifest.clone();
    copy.manifest_sha256.clear();
    let bytes = serde_json::to_vec(&copy).expect("manifest serialization");
    format!("{:x}", Sha256::digest(bytes))
}

#[allow(clippy::too_many_arguments)]
fn manifest(
    id: &str,
    kind: &str,
    input_hash: &str,
    features: &[&str],
    units: &[&str],
    normalization: &str,
    resampling: &str,
    censor: &str,
    reflection: &str,
    canonicalization: &str,
    distances: &[&str],
) -> RepresentationManifest {
    let mut result = RepresentationManifest {
        representation_id: id.into(),
        object_kind: kind.into(),
        intrinsic: true,
        input_schema_sha256: input_hash.into(),
        feature_or_channel_order: features.iter().map(|value| (*value).into()).collect(),
        units: units.iter().map(|value| (*value).into()).collect(),
        normalization: normalization.into(),
        resampling: resampling.into(),
        censor_policy: censor.into(),
        reflection_policy: reflection.into(),
        canonicalization_policy: canonicalization.into(),
        distance_contracts: distances.iter().map(|value| (*value).into()).collect(),
        missingness_policy: "typed availability; NOT_COMPARABLE is returned, never numeric zero"
            .into(),
        manifest_sha256: String::new(),
    };
    result.manifest_sha256 = manifest_hash(&result);
    result
}

pub(crate) fn manifests(input_hash: &str) -> Vec<RepresentationManifest> {
    let compression_summary = [
        "sample_count",
        "duration_seconds",
        "seed_width_atr",
        "terminal_width_seed",
        "center_migration_atr",
        "upper_expansion_atr",
        "lower_expansion_atr",
        "containment_asymmetry",
        "escape_count",
        "terminal_close_mid_atr",
        "direction_category",
        "terminal_mechanism_category",
    ];
    let expansion_summary = [
        "sample_count",
        "duration_seconds",
        "terminal_signed_displacement_atr",
        "maximum_directional_displacement_atr",
        "opposite_displacement_atr",
        "return_ratio",
        "close_path_length_atr",
        "path_efficiency",
        "maximum_velocity_atr_per_bar",
        "maximum_acceleration_atr_per_bar2",
        "direction_category",
        "terminal_mechanism_category",
    ];
    let compression_trajectory = [
        "contain_upper_from_seed_mid_atr",
        "contain_lower_from_seed_mid_atr",
        "contain_mid_from_seed_mid_atr",
        "close_from_contain_mid_atr",
        "contain_width_seed",
        "state_code_asof",
    ];
    let expansion_trajectory = [
        "signed_displacement_atr",
        "running_up_excursion_atr",
        "running_down_excursion_atr",
        "close_path_length_atr",
        "signed_velocity_atr_per_bar",
        "signed_acceleration_atr_per_bar2",
        "state_code_asof",
    ];
    let summary_units = [
        "count",
        "seconds",
        "ATR",
        "ratio",
        "ATR",
        "ATR",
        "ATR",
        "ratio",
        "count",
        "ATR",
        "categorical",
        "categorical",
    ];
    let trajectory_distances = [
        "POINTWISE_L2_V1",
        "DERIVATIVE_AWARE_L2_V1",
        "BOUNDED_DTW_W2_V1",
    ];
    let mut output = Vec::new();
    for (kind, summary_features) in [
        ("COMPRESSION", compression_summary.as_slice()),
        ("EXPANSION", expansion_summary.as_slice()),
    ] {
        output.push(manifest(
            "SUMMARY_RAW_V1",
            kind,
            input_hash,
            summary_features,
            &summary_units,
            "median/IQR by object kind; continuous values clipped to +/-12 IQR",
            "NONE",
            "completed-object terminal summary only",
            "M is available and involutive",
            "NONE",
            &["ROBUST_L1_V1", "ROBUST_L2_V1", "MIXED_GOWER_V1"],
        ));
        output.push(manifest(
            "SUMMARY_CANONICAL_V1",
            kind,
            input_hash,
            summary_features,
            &summary_units,
            "median/IQR by object kind; continuous values clipped to +/-12 IQR",
            "NONE",
            "completed-object terminal summary only",
            "M is available and involutive",
            "C reflects negative-direction objects; C(C(x))=C(x)",
            &["ROBUST_L1_V1", "ROBUST_L2_V1", "MIXED_GOWER_V1"],
        ));
    }
    for (kind, channels) in [
        ("COMPRESSION", compression_trajectory.as_slice()),
        ("EXPANSION", expansion_trajectory.as_slice()),
    ] {
        for (id, canonical) in [
            ("TRAJECTORY_RAW_U21_V1", false),
            ("TRAJECTORY_CANONICAL_U21_V1", true),
        ] {
            output.push(manifest(
                id,
                kind,
                input_hash,
                channels,
                &vec!["dimensionless"; channels.len()],
                "origin ATR / seed-relative coordinates; absolute bars and seconds retained separately",
                "21 common-grid points; continuous linear; state left-continuous as-of",
                "complete mode or causal common shared-prefix mode; no suffix fabrication",
                "M is channel-declared and involutive",
                if canonical {
                    "C conditionally reflects negative-direction objects and is idempotent"
                } else {
                    "NONE"
                },
                &trajectory_distances,
            ));
        }
    }
    for kind in ["COMPRESSION", "EXPANSION"] {
        for (id, canonical) in [
            ("EVENT_SEQUENCE_RAW_V1", false),
            ("EVENT_SEQUENCE_CANONICAL_V1", true),
        ] {
            output.push(manifest(
                id,
                kind,
                input_hash,
                &[
                    "event_code",
                    "direction",
                    "state_code",
                    "terminal_reason_code",
                    "delta_bars",
                    "delta_seconds",
                ],
                &[
                    "categorical",
                    "categorical",
                    "categorical",
                    "categorical",
                    "bars",
                    "seconds",
                ],
                "NONE",
                "NONE",
                "observed events only; completed and prefix modes remain explicit",
                "event-pair/state-pair reflection table from frozen grammars",
                if canonical {
                    "conditional grammar reflection"
                } else {
                    "NONE"
                },
                &["TYPED_EDIT_DURATION_AWARE_V1"],
            ));
        }
        for (id, canonical) in [
            ("INTRINSIC_TYPED_PROCESS_GRAPH_RAW_V1", false),
            ("INTRINSIC_TYPED_PROCESS_GRAPH_CANONICAL_V1", true),
        ] {
            output.push(manifest(
                id,
                kind,
                input_hash,
                &[
                    "typed_nodes",
                    "typed_directed_edges",
                    "wl_round_1",
                    "wl_round_2",
                ],
                &["multiset", "multiset", "multiset", "multiset"],
                "NONE",
                "NONE",
                "observed intrinsic graph only",
                "typed event/state reflection",
                if canonical {
                    "conditional grammar reflection"
                } else {
                    "NONE"
                },
                &["TYPED_MULTISET_JACCARD_V1", "WL2_MULTISET_JACCARD_V1"],
            ));
        }
    }
    output.sort_by(|left, right| {
        left.object_kind
            .cmp(&right.object_kind)
            .then_with(|| left.representation_id.cmp(&right.representation_id))
    });
    output
}

pub(crate) fn authority_census() -> Vec<AuthorityCensusRow> {
    let rows = [
        (
            "BOTH",
            "closed_bar_path",
            "RAW_AUTHORITY",
            "compression_samples.tsv / expansion_samples.tsv",
            "AVAILABLE",
            "ordered closed-bar prices and timestamps",
        ),
        (
            "BOTH",
            "typed_event_sequence",
            "RAW_AUTHORITY",
            "compression_events.tsv / expansion_events.tsv",
            "AVAILABLE",
            "multiplicity and sequence preserved",
        ),
        (
            "COMPRESSION",
            "seed_and_containment_geometry",
            "RAW_AUTHORITY",
            "compression_objects.tsv and compression_samples.tsv",
            "AVAILABLE",
            "price coordinates remain raw authority",
        ),
        (
            "COMPRESSION",
            "attempt_identity",
            "UNAVAILABLE",
            "none",
            "NOT_EVALUABLE",
            "escape_count exists but attempt identities are not emitted",
        ),
        (
            "COMPRESSION",
            "reclaim_count",
            "EXACT_DERIVATION",
            "compression_events.event_code 4/5",
            "AVAILABLE",
            "frozen RCM V2 event mapping",
        ),
        (
            "EXPANSION",
            "origin_geometry_and_atr",
            "RAW_AUTHORITY",
            "expansion_objects.tsv",
            "AVAILABLE",
            "frozen at expansion origin",
        ),
        (
            "EXPANSION",
            "raw_velocity_and_acceleration",
            "RAW_AUTHORITY",
            "expansion_samples.tsv",
            "AVAILABLE",
            "Gate 16 also verifies exact close-path derivation",
        ),
        (
            "BOTH",
            "normalized_coordinates",
            "EXACT_DERIVATION",
            "raw price coordinates and frozen ATR/seed geometry",
            "AVAILABLE",
            "never written back into RG3",
        ),
        (
            "BOTH",
            "terminal_full_life_shape_when_censored",
            "UNAVAILABLE",
            "censored suffix is unobserved",
            "CENSORED_SUFFIX",
            "no padding, extrapolation or independent stretching",
        ),
        (
            "BOTH",
            "master_structural_context",
            "CONTEXTUAL_AUTHORITY_EXCLUDED",
            "Gate 15.5 causal bridge",
            "NOT_APPLICABLE",
            "intrinsic Gate 16 representations exclude SPACE",
        ),
        (
            "BOTH",
            "auction_interval_overlap",
            "UNAVAILABLE",
            "RG2 intervals not co-collected under RG3 run identities",
            "NOT_EVALUABLE",
            "known Gate 15.5 limitation remains explicit",
        ),
        (
            "BOTH",
            "branch_or_merge_topology",
            "UNAVAILABLE",
            "no intrinsic branch records",
            "NOT_EVALUABLE",
            "first graph is a typed relational re-encoding and must earn incremental status",
        ),
    ];
    rows.into_iter()
        .map(
            |(kind, coordinate, classification, source, availability, note)| AuthorityCensusRow {
                object_kind: kind.into(),
                coordinate: coordinate.into(),
                classification: classification.into(),
                source: source.into(),
                availability: availability.into(),
                note: note.into(),
            },
        )
        .collect()
}

pub(crate) fn pack_vectors(
    objects: &[ProcessGeometryObject],
    scales: &[RobustScale],
) -> (Vec<u8>, Vec<PackedVectorIndex>) {
    let scale_map: HashMap<_, _> = scales
        .iter()
        .map(|scale| {
            (
                (scale.object_kind.as_str(), scale.representation_id.as_str()),
                scale,
            )
        })
        .collect();
    let mut bytes = b"NSG16V1\0".to_vec();
    let mut index = Vec::new();
    for object in objects {
        for (representation, canonical) in
            [("SUMMARY_RAW_V1", false), ("SUMMARY_CANONICAL_V1", true)]
        {
            let start = bytes.len();
            if object.complete() {
                let scale = scale_map[&(object.kind.name(), representation)];
                let values = standardized_summary(object, canonical, scale);
                for value in &values {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                index.push(PackedVectorIndex {
                    object_key: object.key(),
                    representation_id: representation.into(),
                    availability: "AVAILABLE".into(),
                    byte_offset: start,
                    byte_length: values.len() * 4,
                    dimensions: values.len(),
                });
            } else {
                index.push(PackedVectorIndex {
                    object_key: object.key(),
                    representation_id: representation.into(),
                    availability: "CENSORED_SUFFIX".into(),
                    byte_offset: start,
                    byte_length: 0,
                    dimensions: object.summary_raw.len(),
                });
            }
        }
        for (representation, canonical) in [
            ("TRAJECTORY_RAW_U21_V1", false),
            ("TRAJECTORY_CANONICAL_U21_V1", true),
        ] {
            let start = bytes.len();
            if let Some(values) = full_trajectory(object, canonical) {
                for value in &values.continuous {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                index.push(PackedVectorIndex {
                    object_key: object.key(),
                    representation_id: representation.into(),
                    availability: "AVAILABLE".into(),
                    byte_offset: start,
                    byte_length: values.continuous.len() * 4,
                    dimensions: values.continuous.len(),
                });
            } else {
                index.push(PackedVectorIndex {
                    object_key: object.key(),
                    representation_id: representation.into(),
                    availability: if object.censored {
                        "CENSORED_SUFFIX"
                    } else {
                        "DATA_GAP"
                    }
                    .into(),
                    byte_offset: start,
                    byte_length: 0,
                    dimensions: 0,
                });
            }
        }
    }
    (bytes, index)
}

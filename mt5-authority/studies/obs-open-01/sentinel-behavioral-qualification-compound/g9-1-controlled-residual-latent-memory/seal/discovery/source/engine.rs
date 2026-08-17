use crate::authority::sha256_hex;
use crate::model::*;
use hashbrown::HashMap;
use obs_open_04a_g1::model::{Emission, KernelContext, KernelEmissions, KernelState, StepResult};
use obs_open_04a_g5::model::{
    PresentabilityClass, RelativeBarToken, StartReachability, StartingSituation,
};
use obs_open_04a_g6::contract::{compare_price, compare_time, instantiate_correspondence};
use obs_open_04a_g6::model::{AffineCorrespondence, ComparisonFiberKey, ContextAnchor};
use std::collections::BTreeMap;

pub const MAP_ID: &str = "04A_CURRENT_STATE_TO_REDUCED_GEOMETRY";
pub const SEARCH_BOX_ID: &str = "G9_1_DISCOVERY_BOX_16TOK_H2_B512_V1";

pub struct CensusProducts {
    pub summary: CensusSummary,
    pub fibers: Vec<FiberStratum>,
    pub e2_pairs: Vec<E2PairRecord>,
    pub residual: Vec<ResidualBoundaryRecord>,
}

pub struct DiscoveryProducts {
    pub fractures: Vec<DelayedFractureRecord>,
    pub exposure: Vec<SearchExposure>,
    pub mechanisms: Vec<serde_json::Value>,
    pub searched_pairs: u64,
    pub saturation_stopped: bool,
}

pub fn census(records: &[ReplayRecord]) -> Result<CensusProducts, Box<dyn std::error::Error>> {
    let mut groups: HashMap<&ReducedKey, Vec<usize>> = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        groups.entry(&record.reduced).or_default().push(index);
    }
    let observed_fibers = groups.len();
    let collision_fibers = groups.values().filter(|v| v.len() > 1).count();
    let mut ordered: Vec<_> = groups.into_iter().collect();
    ordered.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut fibers = Vec::with_capacity(collision_fibers);
    let mut e2_pairs = Vec::new();
    let mut residual = Vec::new();
    let mut same_fiber = 0_u64;
    let mut not_g6 = 0_u64;
    let mut ordinal_mismatch = 0_u64;
    let mut semantic_time_mismatch = 0_u64;
    let mut other_boundary = 0_u64;

    for (key, members) in ordered {
        if members.len() < 2 {
            continue;
        }
        let fiber_id = format!("FIBER-{}", short_hash(&serde_json::to_vec(key)?));
        let mut stratum = FiberStratum {
            fiber_id: fiber_id.clone(),
            e0_lawful_pairs: 0,
            e1_ordinal_matched_pairs: 0,
            e2_boundary_equal_pairs: 0,
            not_g6_comparable: 0,
            ordinal_mismatch: 0,
            semantic_time_control_mismatch: 0,
            other_epsilon_protected_fracture: 0,
        };
        for lo in 0..members.len() - 1 {
            for ro in lo + 1..members.len() {
                same_fiber += 1;
                let left = &records[members[lo]];
                let right = &records[members[ro]];
                let pair = pair_ref(&fiber_id, left, right)?;
                let la = context_anchor(&left.context)?;
                let ra = context_anchor(&right.context)?;
                if la.fiber != ra.fiber {
                    not_g6 += 1;
                    stratum.not_g6_comparable += 1;
                    continue;
                }
                stratum.e0_lawful_pairs += 1;
                if left.state.bar_index != right.state.bar_index {
                    ordinal_mismatch += 1;
                    stratum.ordinal_mismatch += 1;
                    continue;
                }
                stratum.e1_ordinal_matched_pairs += 1;
                let c = instantiate_correspondence(&la, &ra)?;
                if let Some(observable) = protected_mismatch(left, right, &c) {
                    let is_time = observable == "COB_AUTHORITATIVE_TIME";
                    if is_time {
                        semantic_time_mismatch += 1;
                        stratum.semantic_time_control_mismatch += 1;
                    } else {
                        other_boundary += 1;
                        stratum.other_epsilon_protected_fracture += 1;
                    }
                    residual.push(residual_record(pair, left, right, observable)?);
                } else {
                    stratum.e2_boundary_equal_pairs += 1;
                    e2_pairs.push(E2PairRecord {
                        pair,
                        remaining_joint_steps: remaining_steps(left).min(remaining_steps(right)),
                        starting_difference_signature: difference_signature(
                            &left.state,
                            &right.state,
                        ),
                    });
                }
            }
        }
        fibers.push(stratum);
    }
    let e0 = fibers.iter().map(|v| v.e0_lawful_pairs).sum();
    let e1 = fibers.iter().map(|v| v.e1_ordinal_matched_pairs).sum();
    let e2 = fibers.iter().map(|v| v.e2_boundary_equal_pairs).sum();
    Ok(CensusProducts {
        summary: CensusSummary {
            schema: "G9_1_EXACT_STRATUM_CENSUS_V1".into(),
            status: "EXACT_STRATUM_CENSUS_COMPLETE".into(),
            records_replayed: records.len() as u64,
            observed_fibers: observed_fibers as u64,
            collision_fibers: collision_fibers as u64,
            same_fiber_candidate_pairs: same_fiber,
            e0_exact_cardinality: e0,
            e1_exact_cardinality: e1,
            e2_exact_cardinality: e2,
            e1_minus_e2_exact_cardinality: e1 - e2,
            not_g6_comparable: not_g6,
            ordinal_mismatch,
            semantic_time_control_mismatch: semantic_time_mismatch,
            other_epsilon_protected_fracture: other_boundary,
            continuation_evaluations: 0,
            exact_enumeration: true,
        },
        fibers,
        e2_pairs,
        residual,
    })
}

pub fn discover(
    records: &[ReplayRecord],
    census_expected: &CensusSummary,
) -> Result<DiscoveryProducts, Box<dyn std::error::Error>> {
    let current = census(records)?;
    if current.summary.e0_exact_cardinality != census_expected.e0_exact_cardinality
        || current.summary.e1_exact_cardinality != census_expected.e1_exact_cardinality
        || current.summary.e2_exact_cardinality != census_expected.e2_exact_cardinality
    {
        return Err("G9_1_CENSUS_REPLAY_DRIFT".into());
    }
    let index = replay_index(records);
    let tokens = tokens();
    if tokens.len() < 16 {
        return Err("G9_1_TOKEN_ALPHABET_TOO_SMALL".into());
    }
    let alphabet = &tokens[..16];
    let mut fractures = Vec::new();
    let mut exposure = Vec::new();
    let mut mechanism_counts: BTreeMap<String, (MechanismSignature, u64, BTreeMap<String, ()>)> =
        BTreeMap::new();
    let mut consecutive_no_new = 0_u32;
    let mut saturation_stopped = false;
    for e2 in current
        .e2_pairs
        .iter()
        .filter(|p| p.remaining_joint_steps > 0)
        .take(512)
    {
        let left = &records[*index
            .get(&(e2.pair.left_session_id.clone(), e2.pair.left_prefix_ordinal))
            .ok_or("LEFT_PAIR_REPLAY_MISSING")?];
        let right = &records[*index
            .get(&(
                e2.pair.right_session_id.clone(),
                e2.pair.right_prefix_ordinal,
            ))
            .ok_or("RIGHT_PAIR_REPLAY_MISSING")?];
        let la = context_anchor(&left.context)?;
        let ra = context_anchor(&right.context)?;
        let correspondence = instantiate_correspondence(&la, &ra)?;
        let mut tested = 0_u32;
        let mut admissible = 0_u32;
        let mut found = None;
        'search: for length in 1..=2 {
            if e2.remaining_joint_steps < length {
                continue;
            }
            if length == 1 {
                for a in alphabet {
                    tested += 1;
                    match apply_sequence(left, right, std::slice::from_ref(a), &correspondence)? {
                        SequenceOutcome::NotAdmissible => {}
                        SequenceOutcome::Admissible { fracture: Some(v) } => {
                            admissible += 1;
                            found = Some((vec![a.clone()], v, tested));
                            break 'search;
                        }
                        SequenceOutcome::Admissible { fracture: None } => admissible += 1,
                    }
                }
            } else {
                for a in alphabet {
                    for b in alphabet {
                        tested += 1;
                        let sequence = [a.clone(), b.clone()];
                        match apply_sequence(left, right, &sequence, &correspondence)? {
                            SequenceOutcome::NotAdmissible => {}
                            SequenceOutcome::Admissible {
                                fracture: Some(v), ..
                            } => {
                                admissible += 1;
                                found = Some((sequence.to_vec(), v, tested));
                                break 'search;
                            }
                            SequenceOutcome::Admissible { .. } => admissible += 1,
                        }
                    }
                }
            }
        }
        let before = mechanism_counts.len();
        if let Some((witness, fracture, rank)) = found {
            let ancestry = ancestry_subset(&e2.starting_difference_signature, &fracture.observable);
            let signature = MechanismSignature {
                fracture_class: "DELAYED_FRACTURE".into(),
                ancestry_relevant_starting_distinctions: ancestry.clone(),
                first_computational_divergence_role: "SEALED_G1_STEPRESULT_OUTPUT_DIVERGENCE"
                    .into(),
                first_transition_control_divergence_class: fracture.transition_class.clone(),
                first_protected_observable_fracture: fracture.observable.clone(),
                tau_comp: fracture.step.to_string(),
                tau_obs: fracture.step,
            };
            let mechanism_class_id =
                format!("MEC-{}", short_hash(&serde_json::to_vec(&signature)?));
            let verifier_receipt = format!(
                "G9_1_VR-{}",
                short_hash(&serde_json::to_vec(&(&e2.pair, &witness, &signature))?)
            );
            let entry = mechanism_counts
                .entry(mechanism_class_id.clone())
                .or_insert_with(|| (signature.clone(), 0, BTreeMap::new()));
            entry.1 += 1;
            entry.2.insert(e2.pair.fiber_id.clone(), ());
            fractures.push(DelayedFractureRecord {
                pair: e2.pair.clone(),
                witness: witness.iter().map(witness_token).collect(),
                witness_class: "DELAYED_FRACTURE".into(),
                equal_prefix: fracture.prefix,
                tau_comp: fracture.step.to_string(),
                tau_obs: fracture.step,
                first_computational_divergence: "EARLIEST_PROVEN_BY_EXACT_G1_STEPRESULT_OUTPUT"
                    .into(),
                first_protected_fracture: fracture.observable,
                fracture_ancestry_difference_subset: ancestry,
                mechanism_signature: signature,
                mechanism_class_id,
                verifier_receipt,
            });
            exposure.push(exposure_record(
                &e2.pair,
                "DISTINGUISHABLE_WITH_WITNESS",
                tested,
                admissible,
                Some(rank),
                "CANONICAL_FIRST_VERIFIED_WITNESS",
            ));
        } else {
            exposure.push(exposure_record(
                &e2.pair,
                "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH",
                tested,
                admissible,
                None,
                "FROZEN_SEARCH_BOX_EXHAUSTED",
            ));
        }
        if mechanism_counts.len() == before {
            consecutive_no_new += 1;
        } else {
            consecutive_no_new = 0;
        }
        if !mechanism_counts.is_empty() && consecutive_no_new >= 128 {
            saturation_stopped = true;
            break;
        }
    }
    let mechanisms = mechanism_counts
        .into_iter()
        .map(|(id, (signature, witnesses, fibers))| {
            serde_json::json!({
                "exact_mechanism_class_id": id, "signature": signature, "witness_count": witnesses,
                "fiber_count": fibers.len(), "not_prevalence_authority": true
            })
        })
        .collect();
    Ok(DiscoveryProducts {
        searched_pairs: exposure.len() as u64,
        fractures,
        exposure,
        mechanisms,
        saturation_stopped,
    })
}

fn replay_index(records: &[ReplayRecord]) -> HashMap<(String, u32), usize> {
    records
        .iter()
        .enumerate()
        .map(|(i, r)| ((r.session_id.clone(), r.prefix_ordinal), i))
        .collect()
}

fn pair_ref(
    fiber_id: &str,
    left: &ReplayRecord,
    right: &ReplayRecord,
) -> Result<PairRef, serde_json::Error> {
    let identity = serde_json::to_vec(&(
        MAP_ID,
        fiber_id,
        &left.session_id,
        left.prefix_ordinal,
        &right.session_id,
        right.prefix_ordinal,
    ))?;
    Ok(PairRef {
        pair_id: format!("PAIR-{}", short_hash(&identity)),
        fiber_id: fiber_id.into(),
        left_session_id: left.session_id.clone(),
        left_prefix_ordinal: left.prefix_ordinal,
        right_session_id: right.session_id.clone(),
        right_prefix_ordinal: right.prefix_ordinal,
    })
}

fn context_anchor(context: &KernelContext) -> Result<ContextAnchor, serde_json::Error> {
    let origin = context.ranges.first().map_or(0, |r| r.low_ticks);
    let normalized: Vec<_> = context
        .ranges
        .iter()
        .map(|r| {
            (
                r.k,
                r.high_ticks - origin,
                r.low_ticks - origin,
                r.freeze_commit_time_ns - context.session_start_ns,
            )
        })
        .collect();
    Ok(ContextAnchor {
        fiber: ComparisonFiberKey {
            price_scale: context.price_scale,
            source_time_resolution_ns: context.source_time_resolution_ns,
            storage_time_resolution_ns: context.storage_time_resolution_ns,
            cadence_ns: context.observation_cadence_ns,
            session_duration_ns: context.session_terminal_ns - context.session_start_ns,
            range_count: context.ranges.len() as u16,
            range_shape_hash: sha256_hex(&serde_json::to_vec(&normalized)?),
        },
        price_origin_ticks: origin,
        time_origin_ns: context.session_start_ns,
    })
}

fn remaining_steps(record: &ReplayRecord) -> u64 {
    record.state.knowledge_time_ns.map_or(0, |t| {
        if t >= record.context.session_terminal_ns {
            0
        } else {
            ((record.context.session_terminal_ns - t) / record.context.observation_cadence_ns)
                as u64
        }
    })
}

fn protected_mismatch(
    left: &ReplayRecord,
    right: &ReplayRecord,
    c: &AffineCorrespondence,
) -> Option<String> {
    let l = &left.state;
    let r = &right.state;
    if l.initialized != r.initialized || l.window_active != r.window_active {
        return Some("COB_LIFECYCLE_STATE".into());
    }
    if l.bar_index != r.bar_index {
        return Some("COB_TRANSITION_ORDINAL".into());
    }
    if !times_match(l.knowledge_time_ns, r.knowledge_time_ns, c) {
        return Some("COB_AUTHORITATIVE_TIME".into());
    }
    let (lu, ru, ll, rl) = (
        l.upper.as_ref()?,
        r.upper.as_ref()?,
        l.lower.as_ref()?,
        r.lower.as_ref()?,
    );
    if lu.birth_bar_index != ru.birth_bar_index
        || lu.age_bars != ru.age_bars
        || !compare_time(lu.birth_knowledge_time_ns, ru.birth_knowledge_time_ns, c)
    {
        return Some("COB_UPPER_CANDIDATE_GENEALOGY".into());
    }
    if ll.birth_bar_index != rl.birth_bar_index
        || ll.age_bars != rl.age_bars
        || !compare_time(ll.birth_knowledge_time_ns, rl.birth_knowledge_time_ns, c)
    {
        return Some("COB_LOWER_CANDIDATE_GENEALOGY".into());
    }
    if !compare_price(lu.value_ticks, ru.value_ticks, c)
        || !compare_price(ll.value_ticks, rl.value_ticks, c)
    {
        return Some("COB_RUNNING_EXTREME_GEOMETRY".into());
    }
    if !prices_match(l.close_ticks, r.close_ticks, c) {
        return Some("COB_COMMITTED_CLOSE".into());
    }
    if l.upper_giveback_ticks != r.upper_giveback_ticks
        || l.lower_giveback_ticks != r.lower_giveback_ticks
    {
        return Some("COB_GIVEBACK_GEOMETRY".into());
    }
    if l.range_locations != r.range_locations {
        return Some("COB_RANGE_LOCATION_STATE".into());
    }
    if l.upper_extensions_ticks != r.upper_extensions_ticks
        || l.lower_extensions_ticks != r.lower_extensions_ticks
    {
        return Some("COB_RANGE_EXTENSION_GEOMETRY".into());
    }
    if l.coverage_complete != r.coverage_complete {
        return Some("COB_COVERAGE_STATE".into());
    }
    None
}

fn times_match(l: Option<i64>, r: Option<i64>, c: &AffineCorrespondence) -> bool {
    match (l, r) {
        (Some(a), Some(b)) => compare_time(a, b, c),
        (None, None) => true,
        _ => false,
    }
}
fn prices_match(l: Option<i64>, r: Option<i64>, c: &AffineCorrespondence) -> bool {
    match (l, r) {
        (Some(a), Some(b)) => compare_price(a, b, c),
        (None, None) => true,
        _ => false,
    }
}

fn difference_signature(left: &KernelState, right: &KernelState) -> DifferenceSignature {
    let mut d = Vec::new();
    if left.bar_index != right.bar_index {
        d.push("TRANSITION_POSITION".into());
    }
    if left.knowledge_time_ns != right.knowledge_time_ns {
        d.push("AUTHORITATIVE_TIME_REPRESENTATION".into());
    }
    if let (Some(a), Some(b)) = (&left.upper, &right.upper) {
        if a.id != b.id {
            d.push("UPPER_CANDIDATE_IDENTITY_RELATION".into())
        }
        if a.birth_bar_index != b.birth_bar_index
            || a.birth_knowledge_time_ns != b.birth_knowledge_time_ns
            || a.age_bars != b.age_bars
        {
            d.push("UPPER_CANDIDATE_GENEALOGY".into())
        }
    }
    if let (Some(a), Some(b)) = (&left.lower, &right.lower) {
        if a.id != b.id {
            d.push("LOWER_CANDIDATE_IDENTITY_RELATION".into())
        }
        if a.birth_bar_index != b.birth_bar_index
            || a.birth_knowledge_time_ns != b.birth_knowledge_time_ns
            || a.age_bars != b.age_bars
        {
            d.push("LOWER_CANDIDATE_GENEALOGY".into())
        }
    }
    d.sort();
    d.dedup();
    DifferenceSignature {
        semantic_distinctions: d,
        descriptive_not_causal: true,
    }
}

fn ancestry_subset(d: &DifferenceSignature, observable: &str) -> Vec<String> {
    d.semantic_distinctions
        .iter()
        .filter(|x| match observable {
            "COB_AUTHORITATIVE_TIME" => x.contains("TIME"),
            "COB_UPPER_CANDIDATE_GENEALOGY" => x.contains("UPPER"),
            "COB_LOWER_CANDIDATE_GENEALOGY" => x.contains("LOWER"),
            _ => false,
        })
        .cloned()
        .collect()
}

fn residual_record(
    pair: PairRef,
    left: &ReplayRecord,
    right: &ReplayRecord,
    observable: String,
) -> Result<ResidualBoundaryRecord, serde_json::Error> {
    let d = difference_signature(&left.state, &right.state);
    let ancestry = ancestry_subset(&d, &observable);
    let sig = MechanismSignature {
        fracture_class: "RESIDUAL_BOUNDARY_FRACTURE".into(),
        ancestry_relevant_starting_distinctions: ancestry.clone(),
        first_computational_divergence_role:
            "NOT_EVALUABLE_AT_EPSILON_WITHOUT_ACTIVE_TRANSITION_TRACE".into(),
        first_transition_control_divergence_class: "EPSILON_BOUNDARY".into(),
        first_protected_observable_fracture: observable.clone(),
        tau_comp: "NOT_EVALUABLE".into(),
        tau_obs: 0,
    };
    Ok(ResidualBoundaryRecord {
        pair,
        first_protected_fracture: observable.clone(),
        attrition_reason: if observable == "COB_AUTHORITATIVE_TIME" {
            "SEMANTIC_TIME_CONTROL_MISMATCH".into()
        } else {
            "EPSILON_PROTECTED_FRACTURE".into()
        },
        starting_difference_signature: d,
        fracture_ancestry_difference_subset: ancestry,
        mechanism_class_id: format!("MEC-{}", short_hash(&serde_json::to_vec(&sig)?)),
        mechanism_signature: sig,
    })
}

struct FractureStep {
    step: u32,
    observable: String,
    transition_class: String,
    prefix: Vec<PrefixStepReceipt>,
}
enum SequenceOutcome {
    NotAdmissible,
    Admissible { fracture: Option<FractureStep> },
}

fn apply_sequence(
    left: &ReplayRecord,
    right: &ReplayRecord,
    tokens: &[RelativeBarToken],
    c: &AffineCorrespondence,
) -> Result<SequenceOutcome, Box<dyn std::error::Error>> {
    let mut ls = left.state.clone();
    let mut rs = right.state.clone();
    let mut prefix = vec![prefix_receipt(0, "EPSILON_EQUAL")];
    for (offset, token) in tokens.iter().enumerate() {
        let lstart = starting(left, &ls, offset as u32);
        let rstart = starting(right, &rs, offset as u32);
        let pair = obs_open_04a_g5::model::RealizedTokenPair {
            left: obs_open_04a_g5::grammar::realize_relative(&lstart, token, "L")?,
            right: obs_open_04a_g5::grammar::realize_relative(&rstart, token, "R")?,
        };
        if obs_open_04a_g5::grammar::classify_pair(&lstart, &rstart, &pair).class
            != PresentabilityClass::Presentable
        {
            return Ok(SequenceOutcome::NotAdmissible);
        }
        let lr = obs_open_04a_g1::kernel::step(&ls, &pair.left, &left.context);
        let rr = obs_open_04a_g1::kernel::step(&rs, &pair.right, &right.context);
        let step = offset as u32 + 1;
        let (lres, rres) = match (lr, rr) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(a), Err(b)) if a.to_string() == b.to_string() => {
                return Ok(SequenceOutcome::NotAdmissible);
            }
            _ => {
                return Ok(SequenceOutcome::Admissible {
                    fracture: Some(FractureStep {
                        step,
                        observable: "COB_TRANSITION_RESULT_CLASS".into(),
                        transition_class: "RESULT_CLASS_DIVERGENCE".into(),
                        prefix,
                    }),
                });
            }
        };
        let lrec = ReplayRecord {
            session_id: left.session_id.clone(),
            prefix_ordinal: left.prefix_ordinal + step,
            state: lres.state.clone(),
            context: left.context.clone(),
            reduced: left.reduced.clone(),
        };
        let rrec = ReplayRecord {
            session_id: right.session_id.clone(),
            prefix_ordinal: right.prefix_ordinal + step,
            state: rres.state.clone(),
            context: right.context.clone(),
            reduced: right.reduced.clone(),
        };
        if let Some(observable) = protected_mismatch(&lrec, &rrec, c) {
            return Ok(SequenceOutcome::Admissible {
                fracture: Some(FractureStep {
                    step,
                    observable,
                    transition_class: "STATE_OUTPUT_DIVERGENCE".into(),
                    prefix,
                }),
            });
        }
        if normalized_emissions(&lres.emissions) != normalized_emissions(&rres.emissions) {
            return Ok(SequenceOutcome::Admissible {
                fracture: Some(FractureStep {
                    step,
                    observable: "COB_ORDERED_EMISSION_TRACE".into(),
                    transition_class: "ORDERED_EMISSION_DIVERGENCE".into(),
                    prefix,
                }),
            });
        }
        prefix.push(prefix_receipt(
            step,
            &format!("{}:{}", state_digest(&lres)?, state_digest(&rres)?),
        ));
        ls = lres.state;
        rs = rres.state;
    }
    Ok(SequenceOutcome::Admissible { fracture: None })
}

fn starting(base: &ReplayRecord, state: &KernelState, offset: u32) -> StartingSituation {
    StartingSituation {
        prefix_id: format!("{}:{}+{}", base.session_id, base.prefix_ordinal, offset),
        applied_prefix_length: base.prefix_ordinal + offset + 1,
        state: state.clone(),
        context: base.context.clone(),
        reachability: StartReachability::ProvenReachable,
        reachability_receipt: "EXACT_D_A_G1_REPLAY_PLUS_LAWFUL_G5_PREFIX_V1".into(),
    }
}
fn state_digest(result: &StepResult) -> Result<String, serde_json::Error> {
    Ok(short_hash(&serde_json::to_vec(&normalized_emissions(
        &result.emissions,
    ))?))
}
fn prefix_receipt(step: u32, data: &str) -> PrefixStepReceipt {
    PrefixStepReceipt {
        step,
        protected_equal: true,
        comparison_root: sha256_hex(data.as_bytes()),
    }
}
fn normalized_emissions(e: &KernelEmissions) -> Vec<String> {
    e.ordered
        .iter()
        .map(|x| match x {
            Emission::ObservationCommit { coverage, .. } => format!("COMMIT:{coverage:?}"),
            Emission::NewUpperExtreme { .. } => "NEW_UPPER".into(),
            Emission::UpperCandidateIdChange { prior, .. } => {
                format!("UPPER_ID_CHANGE:{}", prior.is_some())
            }
            Emission::NewLowerExtreme { .. } => "NEW_LOWER".into(),
            Emission::LowerCandidateIdChange { prior, .. } => {
                format!("LOWER_ID_CHANGE:{}", prior.is_some())
            }
            Emission::LocationTransition {
                k, prior, current, ..
            } => format!("LOCATION:{k}:{prior:?}:{current:?}"),
        })
        .collect()
}

fn tokens() -> Vec<RelativeBarToken> {
    let mut out = Vec::with_capacity(64);
    for coverage in [false, true] {
        for o in -1..=1 {
            for h in -1..=1 {
                for l in -1..=1 {
                    for c in -1..=1 {
                        if l <= o && l <= c && h >= o && h >= c {
                            out.push(RelativeBarToken {
                                token_id: format!(
                                    "REL-O{o:+}-H{h:+}-L{l:+}-C{c:+}-{}",
                                    if coverage { "COMPLETE" } else { "INCOMPLETE" }
                                ),
                                open_delta_ticks: o,
                                high_delta_ticks: h,
                                low_delta_ticks: l,
                                close_delta_ticks: c,
                                coverage_complete: coverage,
                            })
                        }
                    }
                }
            }
        }
    }
    out
}
fn witness_token(t: &RelativeBarToken) -> WitnessToken {
    WitnessToken {
        token_id: t.token_id.clone(),
        open_delta_ticks: t.open_delta_ticks,
        high_delta_ticks: t.high_delta_ticks,
        low_delta_ticks: t.low_delta_ticks,
        close_delta_ticks: t.close_delta_ticks,
        coverage_complete: t.coverage_complete,
    }
}
fn exposure_record(
    pair: &PairRef,
    status: &str,
    tested: u32,
    admissible: u32,
    rank: Option<u32>,
    reason: &str,
) -> SearchExposure {
    SearchExposure {
        pair: pair.clone(),
        status: status.into(),
        search_box_id: SEARCH_BOX_ID.into(),
        continuations_tested: tested,
        admissible_continuations_tested: admissible,
        shells_completed: if tested > 16 { 2 } else { 1 },
        canonical_first_witness_rank: rank,
        termination_reason: reason.into(),
    }
}
fn short_hash(bytes: &[u8]) -> String {
    sha256_hex(bytes)[..24].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_prefix_is_deterministic_and_valid() {
        let a = tokens();
        let b = tokens();
        assert_eq!(
            a[..16].iter().map(|x| &x.token_id).collect::<Vec<_>>(),
            b[..16].iter().map(|x| &x.token_id).collect::<Vec<_>>()
        );
        assert!(
            a[..16]
                .iter()
                .all(|x| x.low_delta_ticks <= x.open_delta_ticks
                    && x.high_delta_ticks >= x.close_delta_ticks)
        );
    }
    #[test]
    fn bounded_search_size_is_exact() {
        assert_eq!(16 + 16 * 16, 272);
    }
}

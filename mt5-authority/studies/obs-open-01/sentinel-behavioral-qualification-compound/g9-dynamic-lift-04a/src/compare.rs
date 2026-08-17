use crate::authority::sha256_hex;
use crate::model::{
    ComparabilityRecord, Delta04a, FractureRecord, PairRef, ReplayRecord, SearchRecord,
    WitnessToken,
};
use hashbrown::HashMap;
use obs_open_04a_g1::model::{Emission, KernelContext, KernelEmissions, KernelState, StepResult};
use obs_open_04a_g5::model::{
    PresentabilityClass, RelativeBarToken, StartReachability, StartingSituation,
};
use obs_open_04a_g6::contract::{compare_price, compare_time, instantiate_correspondence};
use obs_open_04a_g6::model::{
    AffineCorrespondence, ComparabilityStatus, ComparisonFiberKey, ContextAnchor,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const MAP_ID: &str = "04A_CURRENT_STATE_TO_REDUCED_GEOMETRY";
pub const PAIR_SEARCH_BUDGET: usize = 1024;

pub struct AttackProducts {
    pub comparability: Vec<ComparabilityRecord>,
    pub fractures: Vec<FractureRecord>,
    pub search: Vec<SearchRecord>,
    pub observed_fibers: usize,
    pub collision_fibers: usize,
    pub same_fiber_pairs: u64,
}

pub fn attack(records: &[ReplayRecord]) -> Result<AttackProducts, Box<dyn std::error::Error>> {
    let mut groups: HashMap<&crate::model::ReducedKey, Vec<usize>> = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        groups.entry(&record.reduced).or_default().push(index);
    }
    let observed_fibers = groups.len();
    let collision_fibers = groups.values().filter(|members| members.len() > 1).count();
    let mut ordered: Vec<_> = groups.into_iter().collect();
    ordered.sort_unstable_by(|left, right| left.0.cmp(right.0));

    let tokens = tokens();
    let mut comparability = Vec::new();
    let mut fractures = Vec::new();
    let mut search = Vec::new();
    let mut same_fiber_pairs = 0_u64;
    let mut searched = 0_usize;

    for (key, members) in ordered {
        if members.len() < 2 {
            continue;
        }
        let fiber_id = format!("FIBER-{}", short_hash(&serde_json::to_vec(key)?));
        for left_offset in 0..members.len() - 1 {
            for right_offset in left_offset + 1..members.len() {
                same_fiber_pairs += 1;
                let left = &records[members[left_offset]];
                let right = &records[members[right_offset]];
                let pair = pair_ref(&fiber_id, left, right)?;
                let left_anchor = context_anchor(&left.context)?;
                let right_anchor = context_anchor(&right.context)?;
                let same_context_fiber = left_anchor.fiber == right_anchor.fiber;
                let remaining_left = remaining_steps(left);
                let remaining_right = remaining_steps(right);
                let epsilon_only = remaining_left.min(remaining_right) == 0;
                let status = obs_open_04a_g6::contract::classify_comparability(
                    same_context_fiber,
                    true,
                    epsilon_only,
                    true,
                );
                comparability.push(ComparabilityRecord {
                    pair: pair.clone(),
                    same_04a_fiber: true,
                    g6_context_fiber_match: same_context_fiber,
                    reachability: "PROVEN_BY_EXACT_D_A_G1_REPLAY".into(),
                    continuation_language: if epsilon_only {
                        "EPSILON_ONLY".into()
                    } else {
                        "RELATIVE_COMPLETED_BAR_COUPLING_V1".into()
                    },
                    status: comparability_status(status).into(),
                });
                if !same_context_fiber || searched >= PAIR_SEARCH_BUDGET {
                    continue;
                }
                searched += 1;
                let correspondence = instantiate_correspondence(&left_anchor, &right_anchor)?;
                let delta = delta_04a(&left.state, &right.state);
                if let Some(observable) = protected_mismatch(
                    &left.state,
                    &right.state,
                    &left.context,
                    &right.context,
                    &correspondence,
                ) {
                    let receipt = verifier_receipt(&pair, None, &observable, 0)?;
                    fractures.push(fracture(
                        pair.clone(),
                        delta,
                        None,
                        "IMMEDIATE_FRACTURE",
                        0,
                        observable,
                        receipt,
                    ));
                    search.push(search_record(pair, "DISTINGUISHABLE_WITH_WITNESS", 0, 0, 1));
                    continue;
                }

                if epsilon_only {
                    search.push(search_record(
                        pair,
                        "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH",
                        0,
                        0,
                        0,
                    ));
                    continue;
                }

                let mut tested = 0_u32;
                let mut admissible = 0_u32;
                let mut found = None;
                for token in &tokens {
                    tested += 1;
                    match apply_pair(left, right, token, &correspondence)? {
                        TokenOutcome::NotComparable => {}
                        TokenOutcome::ProtectedMatch => admissible += 1,
                        TokenOutcome::ProtectedMismatch(observable) => {
                            admissible += 1;
                            found = Some((token.clone(), observable));
                            break;
                        }
                    }
                }
                if let Some((token, observable)) = found {
                    let witness = witness(&token);
                    let receipt = verifier_receipt(&pair, Some(&witness), &observable, 1)?;
                    fractures.push(fracture(
                        pair.clone(),
                        delta,
                        Some(witness),
                        "DELAYED_FRACTURE",
                        1,
                        observable,
                        receipt,
                    ));
                    search.push(search_record(
                        pair,
                        "DISTINGUISHABLE_WITH_WITNESS",
                        tested,
                        admissible,
                        1,
                    ));
                } else {
                    search.push(search_record(
                        pair,
                        "NO_WITNESS_FOUND_UNDER_BOUNDED_SEARCH",
                        tested,
                        admissible,
                        0,
                    ));
                }
            }
        }
    }
    Ok(AttackProducts {
        comparability,
        fractures,
        search,
        observed_fibers,
        collision_fibers,
        same_fiber_pairs,
    })
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
    let price_origin = context.ranges.first().map_or(0, |range| range.low_ticks);
    let normalized: Vec<_> = context
        .ranges
        .iter()
        .map(|range| {
            (
                range.k,
                range.high_ticks - price_origin,
                range.low_ticks - price_origin,
                range.freeze_commit_time_ns - context.session_start_ns,
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
        price_origin_ticks: price_origin,
        time_origin_ns: context.session_start_ns,
    })
}

fn remaining_steps(record: &ReplayRecord) -> u64 {
    let Some(time) = record.state.knowledge_time_ns else {
        return 0;
    };
    if time >= record.context.session_terminal_ns {
        0
    } else {
        ((record.context.session_terminal_ns - time) / record.context.observation_cadence_ns) as u64
    }
}

fn comparability_status(status: ComparabilityStatus) -> &'static str {
    match status {
        ComparabilityStatus::Comparable => "COMPARABLE",
        ComparabilityStatus::EpsilonOnly => "EPSILON_ONLY",
        ComparabilityStatus::CouplingUnavailable => "COUPLING_UNAVAILABLE",
        ComparabilityStatus::CorrespondenceInvalid => "CORRESPONDENCE_INVALID",
        ComparabilityStatus::ContextIncompatible => "CONTEXT_INCOMPATIBLE",
        ComparabilityStatus::ConditionalOnReachability => "CONDITIONAL_ON_REACHABILITY",
        ComparabilityStatus::NotEvaluable => "NOT_EVALUABLE",
    }
}

fn delta_04a(left: &KernelState, right: &KernelState) -> Delta04a {
    let lu = left.upper.as_ref().expect("replayed state");
    let ru = right.upper.as_ref().expect("replayed state");
    let ll = left.lower.as_ref().expect("replayed state");
    let rl = right.lower.as_ref().expect("replayed state");
    Delta04a {
        upper_id_changed: lu.id != ru.id,
        upper_birth_bar_changed: lu.birth_bar_index != ru.birth_bar_index,
        upper_birth_time_changed: lu.birth_knowledge_time_ns != ru.birth_knowledge_time_ns,
        upper_age_changed: lu.age_bars != ru.age_bars,
        lower_id_changed: ll.id != rl.id,
        lower_birth_bar_changed: ll.birth_bar_index != rl.birth_bar_index,
        lower_birth_time_changed: ll.birth_knowledge_time_ns != rl.birth_knowledge_time_ns,
        lower_age_changed: ll.age_bars != rl.age_bars,
        transition_ordinal_changed: left.bar_index != right.bar_index,
        authoritative_time_changed: left.knowledge_time_ns != right.knowledge_time_ns,
    }
}

fn protected_mismatch(
    left: &KernelState,
    right: &KernelState,
    left_context: &KernelContext,
    right_context: &KernelContext,
    correspondence: &AffineCorrespondence,
) -> Option<String> {
    if left.initialized != right.initialized || left.window_active != right.window_active {
        return Some("COB_LIFECYCLE_STATE".into());
    }
    if left.bar_index != right.bar_index {
        return Some("COB_TRANSITION_ORDINAL".into());
    }
    if !times_match(
        left.knowledge_time_ns,
        right.knowledge_time_ns,
        correspondence,
    ) {
        return Some("COB_AUTHORITATIVE_TIME".into());
    }
    let lu = left.upper.as_ref()?;
    let ru = right.upper.as_ref()?;
    if lu.birth_bar_index != ru.birth_bar_index
        || lu.age_bars != ru.age_bars
        || !compare_time(
            lu.birth_knowledge_time_ns,
            ru.birth_knowledge_time_ns,
            correspondence,
        )
    {
        return Some("COB_UPPER_CANDIDATE_GENEALOGY".into());
    }
    let ll = left.lower.as_ref()?;
    let rl = right.lower.as_ref()?;
    if ll.birth_bar_index != rl.birth_bar_index
        || ll.age_bars != rl.age_bars
        || !compare_time(
            ll.birth_knowledge_time_ns,
            rl.birth_knowledge_time_ns,
            correspondence,
        )
    {
        return Some("COB_LOWER_CANDIDATE_GENEALOGY".into());
    }
    if !compare_price(lu.value_ticks, ru.value_ticks, correspondence)
        || !compare_price(ll.value_ticks, rl.value_ticks, correspondence)
    {
        return Some("COB_RUNNING_EXTREME_GEOMETRY".into());
    }
    if !prices_match(left.close_ticks, right.close_ticks, correspondence) {
        return Some("COB_COMMITTED_CLOSE".into());
    }
    if left.upper_giveback_ticks != right.upper_giveback_ticks
        || left.lower_giveback_ticks != right.lower_giveback_ticks
    {
        return Some("COB_GIVEBACK_GEOMETRY".into());
    }
    if left.range_locations != right.range_locations {
        return Some("COB_RANGE_LOCATION_STATE".into());
    }
    if left.upper_extensions_ticks != right.upper_extensions_ticks
        || left.lower_extensions_ticks != right.lower_extensions_ticks
    {
        return Some("COB_RANGE_EXTENSION_GEOMETRY".into());
    }
    if left.coverage_complete != right.coverage_complete {
        return Some("COB_COVERAGE_STATE".into());
    }
    if left_context.price_scale != right_context.price_scale
        || left_context.source_time_resolution_ns != right_context.source_time_resolution_ns
        || left_context.storage_time_resolution_ns != right_context.storage_time_resolution_ns
    {
        return Some("COB_PRICE_OR_TIME_UNIT_CONTEXT".into());
    }
    None
}

fn times_match(left: Option<i64>, right: Option<i64>, c: &AffineCorrespondence) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => compare_time(left, right, c),
        (None, None) => true,
        _ => false,
    }
}

fn prices_match(left: Option<i64>, right: Option<i64>, c: &AffineCorrespondence) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => compare_price(left, right, c),
        (None, None) => true,
        _ => false,
    }
}

enum TokenOutcome {
    NotComparable,
    ProtectedMatch,
    ProtectedMismatch(String),
}

fn apply_pair(
    left: &ReplayRecord,
    right: &ReplayRecord,
    token: &RelativeBarToken,
    correspondence: &AffineCorrespondence,
) -> Result<TokenOutcome, Box<dyn std::error::Error>> {
    let left_start = starting(left);
    let right_start = starting(right);
    let pair = obs_open_04a_g5::model::RealizedTokenPair {
        left: obs_open_04a_g5::grammar::realize_relative(&left_start, token, "L")?,
        right: obs_open_04a_g5::grammar::realize_relative(&right_start, token, "R")?,
    };
    let presentation = obs_open_04a_g5::grammar::classify_pair(&left_start, &right_start, &pair);
    if presentation.class != PresentabilityClass::Presentable {
        return Ok(TokenOutcome::NotComparable);
    }
    let left_result = obs_open_04a_g1::kernel::step(&left.state, &pair.left, &left.context);
    let right_result = obs_open_04a_g1::kernel::step(&right.state, &pair.right, &right.context);
    match (left_result, right_result) {
        (Err(left), Err(right)) if left.to_string() == right.to_string() => {
            Ok(TokenOutcome::ProtectedMatch)
        }
        (Err(_), Err(_)) | (Err(_), Ok(_)) | (Ok(_), Err(_)) => Ok(
            TokenOutcome::ProtectedMismatch("COB_TRANSITION_RESULT_CLASS".into()),
        ),
        (Ok(left_result), Ok(right_result)) => {
            compare_step_results(left, right, &left_result, &right_result, correspondence)
        }
    }
}

fn compare_step_results(
    left_start: &ReplayRecord,
    right_start: &ReplayRecord,
    left: &StepResult,
    right: &StepResult,
    correspondence: &AffineCorrespondence,
) -> Result<TokenOutcome, Box<dyn std::error::Error>> {
    if let Some(observable) = protected_mismatch(
        &left.state,
        &right.state,
        &left_start.context,
        &right_start.context,
        correspondence,
    ) {
        return Ok(TokenOutcome::ProtectedMismatch(observable));
    }
    if normalized_emissions(&left.emissions) != normalized_emissions(&right.emissions) {
        return Ok(TokenOutcome::ProtectedMismatch(
            "COB_ORDERED_EMISSION_TRACE".into(),
        ));
    }
    Ok(TokenOutcome::ProtectedMatch)
}

fn normalized_emissions(emissions: &KernelEmissions) -> Vec<String> {
    emissions
        .ordered
        .iter()
        .map(|event| match event {
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

fn starting(record: &ReplayRecord) -> StartingSituation {
    StartingSituation {
        prefix_id: format!("{}:{}", record.session_id, record.prefix_ordinal),
        applied_prefix_length: record.prefix_ordinal + 1,
        state: record.state.clone(),
        context: record.context.clone(),
        reachability: StartReachability::ProvenReachable,
        reachability_receipt: "EXACT_D_A_G1_REPLAY_V1".into(),
    }
}

fn tokens() -> Vec<RelativeBarToken> {
    let mut out = Vec::with_capacity(64);
    for coverage_complete in [false, true] {
        for open in -1_i64..=1 {
            for high in -1_i64..=1 {
                for low in -1_i64..=1 {
                    for close in -1_i64..=1 {
                        if low <= open && low <= close && high >= open && high >= close {
                            out.push(RelativeBarToken {
                                token_id: format!(
                                    "REL-O{open:+}-H{high:+}-L{low:+}-C{close:+}-{}",
                                    if coverage_complete {
                                        "COMPLETE"
                                    } else {
                                        "INCOMPLETE"
                                    }
                                ),
                                open_delta_ticks: open,
                                high_delta_ticks: high,
                                low_delta_ticks: low,
                                close_delta_ticks: close,
                                coverage_complete,
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

fn witness(token: &RelativeBarToken) -> WitnessToken {
    WitnessToken {
        token_id: token.token_id.clone(),
        open_delta_ticks: token.open_delta_ticks,
        high_delta_ticks: token.high_delta_ticks,
        low_delta_ticks: token.low_delta_ticks,
        close_delta_ticks: token.close_delta_ticks,
        coverage_complete: token.coverage_complete,
    }
}

fn fracture(
    pair: PairRef,
    delta_04a: Delta04a,
    witness: Option<WitnessToken>,
    witness_class: &str,
    depth: u32,
    observable: String,
    verifier_receipt: String,
) -> FractureRecord {
    FractureRecord {
        pair,
        map_id: MAP_ID.into(),
        delta_04a,
        witness,
        witness_class: witness_class.into(),
        equal_protected_prefix_steps: depth,
        first_computational_divergence: depth,
        first_protected_observable_divergence: depth,
        first_divergent_observable: observable.clone(),
        first_divergent_transition: if depth == 0 {
            "EPSILON_BOUNDARY".into()
        } else {
            "FIRST_PRESENTED_RELATIVE_COMPLETED_BAR".into()
        },
        g2_role_ancestry: vec![
            "RUNNING_EXTREME_MEMORY".into(),
            "CANDIDATE_GENEALOGY".into(),
            "CAUSAL_TRANSITION_ORDINAL".into(),
        ],
        g4_fracture_surface: vec![observable],
        reachability_evidence: "BOTH_STARTS_PROVEN_BY_EXACT_D_A_G1_REPLAY".into(),
        verifier_receipt,
    }
}

fn search_record(
    pair: PairRef,
    status: &str,
    tested: u32,
    admissible: u32,
    separators: u32,
) -> SearchRecord {
    SearchRecord {
        pair,
        pair_status: status.into(),
        epsilon_tested: true,
        continuations_tested: tested,
        admissible_continuations_tested: admissible,
        verified_separators: separators,
        bounded_separator_incidence_numerator: separators,
        bounded_separator_incidence_denominator: admissible,
        nonclaims: vec![
            "GLOBAL_EQUIVALENCE".into(),
            "GLOBAL_MINIMUM_WITNESS".into(),
            "CONTINUATION_SPACE_DENSITY".into(),
        ],
    }
}

fn verifier_receipt(
    pair: &PairRef,
    token: Option<&WitnessToken>,
    observable: &str,
    depth: u32,
) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct Receipt<'a> {
        verifier: &'static str,
        pair: &'a PairRef,
        token: Option<&'a WitnessToken>,
        observable: &'a str,
        depth: u32,
        source: &'static str,
    }
    let bytes = serde_json::to_vec(&Receipt {
        verifier: "G8_REAL_HISTORY_EXACT_VERIFIER_DESCENDANT_V1",
        pair,
        token,
        observable,
        depth,
        source: "EXACT_G1_G4_G5_G6_RECOMPUTATION",
    })?;
    Ok(format!("G9VR-{}", short_hash(&bytes)))
}

fn short_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_token_box_is_valid_and_deterministic() {
        let first = tokens();
        let second = tokens();
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].token_id, second[0].token_id);
        assert!(first.iter().all(|token| {
            token.low_delta_ticks <= token.open_delta_ticks
                && token.low_delta_ticks <= token.close_delta_ticks
                && token.high_delta_ticks >= token.open_delta_ticks
                && token.high_delta_ticks >= token.close_delta_ticks
        }));
    }
}

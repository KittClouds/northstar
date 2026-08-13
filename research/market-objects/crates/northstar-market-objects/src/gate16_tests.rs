use crate::gate16_collect::{
    canonical_tokens, reflect_channels, reflect_event, reflect_state, reflect_summary,
    reflect_terminal, reflect_token,
};
use crate::gate16_distance::{
    shared_prefix_pointwise, squared_l2_scalar, squared_l2_simd, typed_edit,
};
use crate::gate16_graph::graph_signature;
use crate::gate16_repr::full_trajectory;
use crate::gate16_types::{
    Availability, EventToken, GeometryObjectKind, ProcessGeometryObject, TrajectorySample,
};

fn object(censored: bool, direction: i8, ages: &[u32]) -> ProcessGeometryObject {
    let events = vec![
        EventToken {
            event_code: 2,
            direction,
            state_code: 3,
            terminal_reason_code: 0,
            delta_bars: 1,
            delta_seconds: 300,
        },
        EventToken {
            event_code: 4,
            direction,
            state_code: 4,
            terminal_reason_code: 0,
            delta_bars: 2,
            delta_seconds: 600,
        },
        EventToken {
            event_code: 6,
            direction,
            state_code: 4,
            terminal_reason_code: if censored { 6 } else { 1 },
            delta_bars: 1,
            delta_seconds: 300,
        },
    ];
    let canonical = if direction < 0 {
        events
            .iter()
            .map(|token| reflect_token(GeometryObjectKind::Compression, token))
            .collect()
    } else {
        events.clone()
    };
    ProcessGeometryObject {
        kind: GeometryObjectKind::Compression,
        run_key: "TEST".into(),
        instrument: "US30".into(),
        object_id: ages.len() as i64,
        start_time: 0,
        terminal_time: i64::from(*ages.last().unwrap_or(&0)) * 300,
        terminal_reason_code: if censored { 6 } else { 1 },
        direction,
        censored,
        raw_history_sha256: "history".into(),
        summary_raw: vec![1.0, 2.0],
        summary_canonical: vec![1.0, 2.0],
        categories_raw: [direction as i16, 1],
        categories_canonical: [1, 1],
        trajectory: ages
            .iter()
            .map(|&age| TrajectorySample {
                elapsed_bars: age,
                elapsed_seconds: age * 300,
                continuous_raw: vec![age as f32, -(age as f32)],
                continuous_canonical: vec![age as f32, -(age as f32)],
                state_raw: 3,
                state_canonical: 3,
            })
            .collect(),
        events_raw: events,
        events_canonical: canonical,
    }
}

#[test]
fn reflection_is_involutive_for_frozen_grammar_codes() {
    for code in 0..=9 {
        assert_eq!(
            reflect_event(
                GeometryObjectKind::Compression,
                reflect_event(GeometryObjectKind::Compression, code)
            ),
            code
        );
    }
    for code in 0..=4 {
        assert_eq!(
            reflect_state(
                GeometryObjectKind::Compression,
                reflect_state(GeometryObjectKind::Compression, code)
            ),
            code
        );
    }
    for code in 0..=8 {
        assert_eq!(reflect_terminal(reflect_terminal(code)), code);
    }
    let token = object(false, -1, &[0, 1, 2, 3]).events_raw[0].clone();
    assert_eq!(
        reflect_token(
            GeometryObjectKind::Compression,
            &reflect_token(GeometryObjectKind::Compression, &token)
        ),
        token
    );
    for (kind, channels) in [
        (
            GeometryObjectKind::Compression,
            vec![1.0, -2.0, 3.0, -4.0, 5.0],
        ),
        (
            GeometryObjectKind::Expansion,
            vec![1.0, 2.0, -3.0, 4.0, -5.0, 6.0],
        ),
    ] {
        assert_eq!(
            reflect_channels(kind, &reflect_channels(kind, &channels)),
            channels
        );
    }
    for kind in [
        GeometryObjectKind::Compression,
        GeometryObjectKind::Expansion,
    ] {
        let summary = (0..10).map(|value| value as f32 - 4.0).collect::<Vec<_>>();
        assert_eq!(
            reflect_summary(kind, &reflect_summary(kind, &summary)),
            summary
        );
    }
}

#[test]
fn canonicalization_is_idempotent() {
    let raw = object(false, -1, &[0, 1, 2, 3]).events_raw;
    let once = canonical_tokens(GeometryObjectKind::Compression, -1, &raw);
    let twice = canonical_tokens(GeometryObjectKind::Compression, 1, &once);
    assert_eq!(once, twice);
}

#[test]
fn censored_suffix_never_becomes_a_complete_trajectory() {
    assert!(full_trajectory(&object(true, 1, &[0, 1, 2, 3]), true).is_none());
    assert!(full_trajectory(&object(false, 1, &[0, 1, 2, 3]), true).is_some());
    assert_eq!(Availability::CensoredSuffix.name(), "CENSORED_SUFFIX");
}

#[test]
fn shared_prefix_uses_common_causal_horizon() {
    let short = object(true, 1, &[0, 1, 2, 4]);
    let long = object(true, 1, &[0, 2, 4, 8]);
    let (_, points, horizon, seconds) =
        shared_prefix_pointwise(&short, &long, true).expect("prefix");
    assert_eq!(points, 21);
    assert_eq!(horizon, 4);
    assert_eq!(seconds, 1_200);
}

#[test]
fn scalar_and_simd_squared_l2_are_equivalent() {
    let left = (0..103)
        .map(|value| value as f32 * 0.125)
        .collect::<Vec<_>>();
    let right = (0..103)
        .map(|value| value as f32 * -0.0625)
        .collect::<Vec<_>>();
    let scalar = squared_l2_scalar(&left, &right);
    let simd = squared_l2_simd(&left, &right);
    assert!((scalar - simd).abs() <= 1e-3);
}

#[test]
fn event_order_and_multiplicity_change_distance() {
    let source = object(false, 1, &[0, 1, 2, 3]);
    let mut reordered = source.events_raw.clone();
    reordered.swap(0, 1);
    let mut shortened = source.events_raw.clone();
    shortened.pop();
    assert_eq!(typed_edit(&source.events_raw, &source.events_raw), 0.0);
    assert!(typed_edit(&source.events_raw, &reordered) > 0.0);
    assert!(typed_edit(&source.events_raw, &shortened) > 0.0);
}

#[test]
fn graph_signature_is_deterministic_and_typed() {
    let source = object(false, 1, &[0, 1, 2, 3]);
    let first = graph_signature(&source, true);
    let second = graph_signature(&source, true);
    assert_eq!(first.typed_multiset, second.typed_multiset);
    assert_eq!(first.wl2_multiset, second.wl2_multiset);
    assert!(first.typed_multiset.keys().any(|key| key.contains("EVENT")));
}

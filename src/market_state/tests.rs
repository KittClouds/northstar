use super::*;
use crate::data_plane::bars::{CanonicalBar, Timeframe, MINUTE_NS};
use crate::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use crate::data_plane::ids::{
    instruments, CalendarId, CatalogVersion, DerivationVersion, InstrumentId, JournalSequence,
};
use std::sync::Arc;

const VERSION: DerivationVersion = DerivationVersion(7);

fn calendar() -> Arc<SessionCalendar> {
    Arc::new(
        SessionCalendar::new(
            CalendarId(9),
            CatalogVersion(3),
            vec![
                Session {
                    instrument: instruments::US100,
                    open_ns: 0,
                    close_ns: 40 * MINUTE_NS,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                },
                Session {
                    instrument: instruments::US100,
                    open_ns: 60 * MINUTE_NS,
                    close_ns: 100 * MINUTE_NS,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                },
            ],
        )
        .unwrap(),
    )
}

fn bar(slot: i64, open: i64, high: i64, low: i64, close: i64) -> CanonicalBar {
    let sequence = (slot.unsigned_abs() + 1).max(1);
    CanonicalBar {
        ts_open_ns: slot * 4 * MINUTE_NS,
        ts_close_ns: (slot + 1) * 4 * MINUTE_NS,
        open,
        high,
        low,
        close,
        first_sequence: sequence,
        last_sequence: sequence,
        instrument_id: instruments::US100.get(),
        observation_count: 4,
        revision: 0,
        derivation_version: VERSION.get(),
        timeframe: Timeframe::M4 as u16,
        flags: 0,
        reserved: 0,
    }
}

fn second_session_bar(offset: i64, open: i64, high: i64, low: i64, close: i64) -> CanonicalBar {
    let mut item = bar(15 + offset, open, high, low, close);
    item.first_sequence = 100 + offset as u64;
    item.last_sequence = item.first_sequence;
    item
}

fn engine() -> StructuralEngine {
    StructuralEngine::new(
        instruments::US100,
        PriceSpace::ReferenceIndex,
        calendar(),
        VERSION,
        1,
        2,
        OpeningRangeSpec {
            duration_ns: 8 * MINUTE_NS,
        },
        RangeProjectionSpec {
            lookback_bars: 3,
            fractions_ppm: Box::new([
                0, 213_000, 333_000, 500_000, 666_000, 750_000, 900_000, 1_000_000,
            ]),
        },
        NodeMergeSpec::default(),
    )
    .unwrap()
}

fn sample_level(
    id: u64,
    family: LevelFamily,
    kind: LevelKind,
    role: LevelRole,
    price: i64,
    group: u64,
) -> StructuralObject {
    StructuralObject::Level(StructuralLevel {
        id: LevelId::from_producer(ProducerId(90), id),
        instrument_id: instruments::US100,
        price_space: PriceSpace::ReferenceIndex,
        kind,
        family,
        role,
        price,
        created_at_ns: 10,
        effective_at_ns: 10,
        updated_at_ns: 10,
        state: LevelState::Developing,
        provenance: LevelProvenance {
            producer_id: ProducerId(90),
            first_sequence: JournalSequence(1),
            last_sequence: JournalSequence(1),
            derivation_version: VERSION,
            calendar_id: CalendarId(9),
            timeframe: Some(Timeframe::M4),
            epoch_ns: 0,
            exclusive_group: group,
            ordinal: id as u16,
        },
        metrics: LevelMetrics::default(),
    })
}

#[test]
fn level_book_is_atomic_stable_and_freeze_safe() {
    let mut book = LevelBook::new();
    let original = sample_level(
        1,
        LevelFamily::Session,
        LevelKind::CurrentSessionHigh,
        LevelRole::UpperBoundary,
        1_000,
        7,
    );
    book.apply_batch(1, &[StructuralMutation::Created(original.clone())])
        .unwrap();
    let mut moved = original.clone();
    if let StructuralObject::Level(level) = &mut moved {
        level.price = 1_010;
        level.updated_at_ns = 20;
        level.provenance.last_sequence = JournalSequence(2);
    }
    book.apply_batch(2, &[StructuralMutation::Updated(moved.clone())])
        .unwrap();
    book.apply_batch(
        3,
        &[StructuralMutation::Frozen {
            id: moved.id(),
            at_ns: 30,
        }],
    )
    .unwrap();
    let before = book.all().to_vec();
    let error = book
        .apply_batch(4, &[StructuralMutation::Updated(moved)])
        .unwrap_err();
    assert!(matches!(error, LevelBookError::ImmutableIdentity(_)));
    assert_eq!(book.generation(), 3);
    assert_eq!(book.all(), before);
}

#[test]
fn price_only_engine_is_bit_stable_across_two_replays() {
    let stream = [
        bar(0, 1_000, 1_010, 990, 1_005),
        bar(1, 1_005, 1_018, 998, 1_012),
        bar(2, 1_012, 1_016, 1_001, 1_004),
        bar(3, 1_004, 1_022, 1_000, 1_020),
        bar(4, 1_020, 1_025, 1_008, 1_010),
    ];
    let mut first = engine();
    let mut second = engine();
    let mut first_terminal = None;
    let mut second_terminal = None;
    for item in &stream {
        first_terminal = Some(first.on_bar(item).unwrap());
        second_terminal = Some(second.on_bar(item).unwrap());
    }
    let first = first_terminal.unwrap();
    let second = second_terminal.unwrap();
    assert_eq!(first.snapshot.levels, second.snapshot.levels);
    assert_eq!(first.snapshot.graph, second.snapshot.graph);
    assert_eq!(first.snapshot.location, second.snapshot.location);
    assert_eq!(first.chart.lines, second.chart.lines);
    assert_eq!(first.chart.bands, second.chart.bands);
    assert_eq!(first.mutations, second.mutations);
    assert_eq!(
        fingerprint_snapshot(&first.snapshot),
        fingerprint_snapshot(&second.snapshot)
    );
    assert_eq!(
        fingerprint_mutations(&first.mutations),
        fingerprint_mutations(&second.mutations)
    );
}

#[test]
fn mutation_stream_reconstructs_the_terminal_level_book() {
    let stream = [
        bar(0, 1_000, 1_010, 990, 1_005),
        bar(1, 1_005, 1_018, 998, 1_012),
        bar(2, 1_012, 1_016, 1_001, 1_004),
        second_session_bar(0, 1_030, 1_040, 1_025, 1_035),
    ];
    let mut engine = engine();
    let mut replay = LevelBook::new();
    let mut terminal = None;
    for item in &stream {
        let update = engine.on_bar(item).unwrap();
        if !update.mutations.is_empty() {
            replay
                .apply_batch(replay.generation() + 1, &update.mutations)
                .unwrap();
        }
        terminal = Some(update);
    }
    assert_eq!(
        replay.active_snapshot(),
        terminal.expect("stream is non-empty").snapshot.levels
    );
}

#[test]
fn opening_range_freezes_and_exclusive_siblings_never_merge() {
    let mut engine = engine();
    engine.on_bar(&bar(0, 1_000, 1_010, 990, 1_005)).unwrap();
    engine.on_bar(&bar(1, 1_005, 1_018, 998, 1_012)).unwrap();
    let update = engine.on_bar(&bar(2, 1_012, 1_016, 1_001, 1_004)).unwrap();
    let opening: Vec<_> = update
        .snapshot
        .levels
        .iter()
        .filter(|object| object.family() == LevelFamily::OpeningRange)
        .collect();
    assert_eq!(opening.len(), 3);
    assert!(opening
        .iter()
        .all(|object| object.state() == LevelState::Frozen));

    let high = opening
        .iter()
        .find(|object| object.kind() == LevelKind::OpeningRangeHigh)
        .unwrap()
        .id();
    let low = opening
        .iter()
        .find(|object| object.kind() == LevelKind::OpeningRangeLow)
        .unwrap()
        .id();
    assert!(update
        .snapshot
        .graph
        .nodes
        .iter()
        .all(|node| !(node.contributors.contains(&high) && node.contributors.contains(&low))));

    let projection_ids: Vec<_> = update
        .snapshot
        .levels
        .iter()
        .filter(|object| object.family() == LevelFamily::RangeProjection)
        .map(StructuralObject::id)
        .collect();
    assert_eq!(projection_ids.len(), 8);
    assert!(update.snapshot.graph.nodes.iter().all(|node| {
        node.contributors
            .iter()
            .filter(|id| projection_ids.contains(id))
            .count()
            <= 1
    }));
}

#[test]
fn session_rotation_freezes_bands_and_publishes_previous_extremes() {
    let mut engine = engine();
    engine.on_bar(&bar(0, 1_000, 1_010, 990, 1_005)).unwrap();
    engine.on_bar(&bar(1, 1_005, 1_020, 985, 1_012)).unwrap();
    let update = engine
        .on_bar(&second_session_bar(0, 1_030, 1_040, 1_025, 1_035))
        .unwrap();
    let levels = &update.snapshot.levels;
    assert!(levels
        .iter()
        .any(|object| object.kind() == LevelKind::PreviousSessionHigh
            && object.reference_price() == 1_020));
    assert!(levels
        .iter()
        .any(|object| object.kind() == LevelKind::PreviousSessionLow
            && object.reference_price() == 985));
    assert!(levels.iter().any(|object| {
        object.kind() == LevelKind::DailyExtremeUpperBand && object.state() == LevelState::Frozen
    }));
    assert!(!levels.iter().any(|object| {
        object.kind() == LevelKind::CurrentSessionHigh && object.reference_price() == 1_020
    }));
}

#[test]
fn fast_location_reuses_structure_and_volume_fails_closed() {
    let mut engine = engine();
    let update = engine.on_bar(&bar(0, 1_000, 1_010, 990, 1_005)).unwrap();
    let moved = engine.locate_price(1_008, 5 * MINUTE_NS, JournalSequence(9));
    assert!(Arc::ptr_eq(&update.snapshot.levels, &moved.levels));
    assert!(Arc::ptr_eq(&update.snapshot.graph, &moved.graph));
    assert_eq!(moved.quality.volume_quality, VolumeQuality::Unavailable);
    assert!(!moved.quality.adaptive_value_available);
    assert!(!moved.quality.volume_profile_available);
}

#[test]
fn compatible_contributors_merge_and_node_identity_survives_motion() {
    let first = sample_level(
        1,
        LevelFamily::Calendar,
        LevelKind::PreviousSessionHigh,
        LevelRole::UpperBoundary,
        1_000,
        1,
    );
    let second = sample_level(
        2,
        LevelFamily::Liquidity,
        LevelKind::UpperLiquidityShelf,
        LevelRole::UpperLiquidity,
        1_003,
        2,
    );
    let mut nodes = NodeBook::new();
    let first_graph = nodes.rebuild(
        &[first.clone(), second.clone()],
        instruments::US100,
        PriceSpace::ReferenceIndex,
        10,
        100,
        1,
        1,
        NodeMergeSpec::default(),
    );
    assert_eq!(first_graph.nodes.len(), 1);
    assert_eq!(first_graph.nodes[0].contributors.len(), 2);
    let stable_id = first_graph.nodes[0].id;

    let mut moved = second;
    if let StructuralObject::Level(level) = &mut moved {
        level.price = 1_006;
        level.updated_at_ns = 20;
    }
    let second_graph = nodes.rebuild(
        &[first, moved],
        instruments::US100,
        PriceSpace::ReferenceIndex,
        20,
        100,
        1,
        1,
        NodeMergeSpec::default(),
    );
    assert_eq!(second_graph.nodes.len(), 1);
    assert_eq!(second_graph.nodes[0].id, stable_id);
    assert!(second_graph
        .genealogy
        .contains(&NodeGenealogy::Preserved(stable_id)));
}

#[test]
fn location_distinguishes_inside_between_and_outside_structure() {
    let graph = StructuralGraphSnapshot {
        generation: 1,
        nodes: Arc::from([
            StructuralNode {
                id: StructuralNodeId(1),
                instrument_id: instruments::US100,
                price_space: PriceSpace::ReferenceIndex,
                lower: 990,
                upper: 1_000,
                center: 995,
                roles: smallvec::smallvec![LevelRole::LowerBoundary],
                contributors: smallvec::smallvec![LevelId(1)],
                families: smallvec::smallvec![LevelFamily::Calendar],
                created_at_ns: 1,
                updated_at_ns: 1,
            },
            StructuralNode {
                id: StructuralNodeId(2),
                instrument_id: instruments::US100,
                price_space: PriceSpace::ReferenceIndex,
                lower: 1_020,
                upper: 1_030,
                center: 1_025,
                roles: smallvec::smallvec![LevelRole::UpperBoundary],
                contributors: smallvec::smallvec![LevelId(2)],
                families: smallvec::smallvec![LevelFamily::Calendar],
                created_at_ns: 1,
                updated_at_ns: 1,
            },
        ]),
        corridors: Arc::from([]),
        genealogy: Arc::from([]),
    };
    assert_eq!(
        locate_market(&graph, 995, 20).location,
        MarketLocation::InsideNode(StructuralNodeId(1))
    );
    assert_eq!(
        locate_market(&graph, 1_010, 20).location,
        MarketLocation::Between {
            lower: StructuralNodeId(1),
            upper: StructuralNodeId(2)
        }
    );
    assert!(matches!(
        locate_market(&graph, 980, 20).location,
        MarketLocation::BelowKnownStructure { .. }
    ));
    assert!(matches!(
        locate_market(&graph, 1_040, 20).location,
        MarketLocation::AboveKnownStructure { .. }
    ));
}

#[test]
fn bar_revision_requires_explicit_rederivation() {
    let mut engine = engine();
    let mut revised = bar(0, 1_000, 1_010, 990, 1_005);
    revised.revision = 1;
    assert!(matches!(
        engine.on_bar(&revised),
        Err(StructuralEngineError::RevisionRequiresRederivation)
    ));
}

#[test]
fn price_spaces_never_share_a_node() {
    let reference = sample_level(
        1,
        LevelFamily::Calendar,
        LevelKind::PreviousSessionHigh,
        LevelRole::UpperBoundary,
        1_000,
        1,
    );
    let mut venue = sample_level(
        2,
        LevelFamily::Liquidity,
        LevelKind::UpperLiquidityShelf,
        LevelRole::UpperLiquidity,
        1_000,
        2,
    );
    if let StructuralObject::Level(level) = &mut venue {
        level.price_space = PriceSpace::VenueExecutable;
    }
    let mut nodes = NodeBook::new();
    let graph = nodes.rebuild(
        &[reference, venue],
        instruments::US100,
        PriceSpace::ReferenceIndex,
        10,
        100,
        1,
        1,
        NodeMergeSpec::default(),
    );
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].contributors.len(), 1);
}

#[test]
fn engine_rejects_wrong_instrument_before_mutating_state() {
    let mut engine = engine();
    let mut wrong = bar(0, 1_000, 1_010, 990, 1_005);
    wrong.instrument_id = InstrumentId(99).get();
    assert!(matches!(
        engine.on_bar(&wrong),
        Err(StructuralEngineError::WrongInstrument)
    ));
    assert_eq!(engine.graph_snapshot().generation, 0);
}

#[test]
fn scheduled_session_gap_is_not_a_missing_bar_gap() {
    let mut engine = engine();
    for slot in 0..10 {
        engine.on_bar(&bar(slot, 1_000, 1_010, 990, 1_005)).unwrap();
    }
    let terminal = engine
        .on_bar(&second_session_bar(0, 1_005, 1_015, 995, 1_010))
        .unwrap();
    assert!(!terminal.snapshot.quality.gaps_present);
}

#[test]
fn missing_intraday_bar_is_visible_in_structural_quality() {
    let mut engine = engine();
    engine.on_bar(&bar(0, 1_000, 1_010, 990, 1_005)).unwrap();
    let update = engine.on_bar(&bar(2, 1_005, 1_015, 995, 1_010)).unwrap();
    assert!(update.snapshot.quality.gaps_present);
}

#[test]
fn producer_ids_are_instrument_scoped() {
    let us100 = LevelId::from_parts(ProducerId(1), instruments::US100, 1);
    let us500 = LevelId::from_parts(ProducerId(1), instruments::US500, 1);
    assert_ne!(us100, us500);
    assert_ne!(us100, LevelId::UNKNOWN);
}

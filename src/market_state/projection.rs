use super::{LevelFamily, LevelKind, LevelState, StructuralGraphSnapshot, StructuralObject};
use crate::data_plane::ids::InstrumentId;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SemanticStyle {
    AdaptiveValue = 1,
    ValueCenter = 2,
    Boundary = 3,
    Calendar = 4,
    Liquidity = 5,
    Range = 6,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartLevelLine {
    pub id: super::LevelId,
    pub price: i64,
    pub label: Arc<str>,
    pub style: SemanticStyle,
    pub priority: u8,
    pub frozen: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartBand {
    pub id: super::LevelId,
    pub lower: i64,
    pub upper: i64,
    pub label: Arc<str>,
    pub style: SemanticStyle,
    pub priority: u8,
    pub frozen: bool,
}

#[derive(Clone, Debug)]
pub struct ChartStructureSnapshot {
    pub instrument_id: InstrumentId,
    pub generation: u64,
    pub lines: Arc<[ChartLevelLine]>,
    pub bands: Arc<[ChartBand]>,
    pub graph: Arc<StructuralGraphSnapshot>,
}

pub fn project_chart_structure(
    instrument_id: InstrumentId,
    generation: u64,
    objects: &[StructuralObject],
    graph: Arc<StructuralGraphSnapshot>,
) -> ChartStructureSnapshot {
    let mut lines = Vec::new();
    let mut bands = Vec::new();
    for object in objects.iter().filter(|object| object.is_active()) {
        let frozen = object.state() == LevelState::Frozen;
        let label: Arc<str> = label(object).into();
        let (style, priority) = presentation(object);
        match object {
            StructuralObject::Level(level) => lines.push(ChartLevelLine {
                id: level.id,
                price: level.price,
                label,
                style,
                priority,
                frozen,
            }),
            StructuralObject::Band(band) => bands.push(ChartBand {
                id: band.id,
                lower: band.lower,
                upper: band.upper,
                label,
                style,
                priority,
                frozen,
            }),
        }
    }
    lines.sort_unstable_by_key(|line| (line.priority, line.price, line.id));
    bands.sort_unstable_by_key(|band| (band.priority, band.lower, band.id));
    ChartStructureSnapshot {
        instrument_id,
        generation,
        lines: lines.into(),
        bands: bands.into(),
        graph,
    }
}

fn label(object: &StructuralObject) -> String {
    match object.kind() {
        LevelKind::CurrentSessionHigh => "CSH".into(),
        LevelKind::CurrentSessionLow => "CSL".into(),
        LevelKind::PreviousSessionHigh => "PSH".into(),
        LevelKind::PreviousSessionLow => "PSL".into(),
        LevelKind::DailyExtremeUpperBand => "D-U".into(),
        LevelKind::DailyExtremeLowerBand => "D-L".into(),
        LevelKind::OpeningRangeHigh => "ORH".into(),
        LevelKind::OpeningRangeLow => "ORL".into(),
        LevelKind::OpeningRangeMid => "ORM".into(),
        LevelKind::RangeProjection => format!("R{}", object.provenance().ordinal),
        LevelKind::ProfilePoc => "POC".into(),
        LevelKind::ProfileVah => "VAH".into(),
        LevelKind::ProfileVal => "VAL".into(),
        LevelKind::VolKittC1 => "C1".into(),
        LevelKind::VolKittC2 => "C2".into(),
        LevelKind::VolKittC3 => "C3".into(),
        LevelKind::VolKittC4 => "C4".into(),
        LevelKind::VolKittC5 => "C5".into(),
        LevelKind::VolKittCog => "COG".into(),
        LevelKind::SessionVwap => "VWAP".into(),
        LevelKind::UpperLiquidityShelf => "LQ-U".into(),
        LevelKind::LowerLiquidityShelf => "LQ-L".into(),
    }
}

fn presentation(object: &StructuralObject) -> (SemanticStyle, u8) {
    match object.family() {
        LevelFamily::AdaptiveValue => (SemanticStyle::AdaptiveValue, 1),
        LevelFamily::Profile | LevelFamily::Vwap => (SemanticStyle::ValueCenter, 2),
        LevelFamily::Liquidity => (SemanticStyle::Liquidity, 2),
        LevelFamily::OpeningRange | LevelFamily::Session => (SemanticStyle::Boundary, 3),
        LevelFamily::Calendar => (SemanticStyle::Calendar, 3),
        LevelFamily::RangeProjection => (SemanticStyle::Range, 4),
    }
}

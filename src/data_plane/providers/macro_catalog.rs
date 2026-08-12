use super::bea::BEA_SERIES;
use super::bls::BLS_SERIES;
use super::boe::BOE_SERIES;
use super::boj::BOJ_SERIES;
use super::census::CENSUS_SERIES;
use super::ecb::ECB_SERIES;
use super::eurostat::EUROSTAT_SERIES;
use super::fred::FRED_SERIES;
use super::ons::ONS_SERIES;
use crate::data_plane::ids::{SeriesId, SourceId, StreamId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroCategory {
    Inflation,
    Labor,
    Rates,
    Growth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroUnit {
    Index,
    Percent,
    Thousands,
    DollarsPerHour,
    MillionsOfDollars,
    PercentagePoints,
}

impl MacroUnit {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::Percent => "%",
            Self::Thousands => "thousands",
            Self::DollarsPerHour => "$/hour",
            Self::MillionsOfDollars => "$mm",
            Self::PercentagePoints => "% pts",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroMeasure {
    NotSeasonallyAdjusted,
    SeasonallyAdjusted,
    TrendCycle,
    AnnualRate,
    QuarterlyAnnualized,
    QuarterOverQuarter,
    PolicyRate,
    MarketRate,
    DiffusionIndex,
}

impl MacroMeasure {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotSeasonallyAdjusted => "NSA",
            Self::SeasonallyAdjusted => "SA",
            Self::TrendCycle => "TC",
            Self::AnnualRate => "YoY",
            Self::QuarterlyAnnualized => "QoQ SAAR",
            Self::QuarterOverQuarter => "QoQ",
            Self::PolicyRate => "policy",
            Self::MarketRate => "market rate",
            Self::DiffusionIndex => "DI",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MacroSeriesBinding {
    pub provider_code: &'static str,
    pub series_id: SeriesId,
    pub label: &'static str,
    pub category: MacroCategory,
    pub unit: MacroUnit,
    pub measure: MacroMeasure,
    pub source_url: &'static str,
    pub source: SourceId,
    pub stream: StreamId,
}

pub const MACRO_SERIES_COUNT: usize = BLS_SERIES.len()
    + EUROSTAT_SERIES.len()
    + FRED_SERIES.len()
    + BEA_SERIES.len()
    + CENSUS_SERIES.len()
    + ECB_SERIES.len()
    + ONS_SERIES.len()
    + BOE_SERIES.len()
    + BOJ_SERIES.len();

pub fn series_bindings() -> impl Iterator<Item = &'static MacroSeriesBinding> {
    BLS_SERIES
        .iter()
        .chain(EUROSTAT_SERIES.iter())
        .chain(FRED_SERIES.iter())
        .chain(BEA_SERIES.iter())
        .chain(CENSUS_SERIES.iter())
        .chain(ECB_SERIES.iter())
        .chain(ONS_SERIES.iter())
        .chain(BOE_SERIES.iter())
        .chain(BOJ_SERIES.iter())
}

pub fn binding_for_series(series: SeriesId) -> Option<&'static MacroSeriesBinding> {
    series_bindings().find(|binding| binding.series_id == series)
}

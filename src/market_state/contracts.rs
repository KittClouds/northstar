use crate::data_plane::bars::Timeframe;
use crate::data_plane::ids::{CalendarId, DerivationVersion, InstrumentId, JournalSequence};
use smallvec::SmallVec;
use std::sync::Arc;

pub type Price = i64;
pub type PriceDelta = i64;

macro_rules! structural_id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            pub const UNKNOWN: Self = Self(0);

            #[inline]
            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

structural_id!(LevelId, u64);
structural_id!(StructuralNodeId, u64);
structural_id!(ProducerId, u16);

impl LevelId {
    /// Deterministic identity: producer (16 bits), instrument (16 bits), and
    /// producer-local first-seen sequence (32 bits).
    pub const fn from_parts(producer: ProducerId, instrument: InstrumentId, local: u64) -> Self {
        Self(
            ((producer.0 as u64) << 48)
                | (((instrument.0 as u64) & 0xFFFF) << 32)
                | (local & 0xFFFF_FFFF),
        )
    }

    /// Primarily useful for synthetic objects without a catalog instrument.
    pub const fn from_producer(producer: ProducerId, local: u64) -> Self {
        Self::from_parts(producer, InstrumentId::UNKNOWN, local)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum PriceSpace {
    ReferenceIndex = 1,
    VenueExecutable = 2,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LevelFamily {
    Session = 1,
    Calendar = 2,
    OpeningRange = 3,
    RangeProjection = 4,
    Profile = 5,
    AdaptiveValue = 6,
    Vwap = 7,
    Liquidity = 8,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum LevelKind {
    CurrentSessionHigh = 1,
    CurrentSessionLow = 2,
    PreviousSessionHigh = 3,
    PreviousSessionLow = 4,
    DailyExtremeUpperBand = 5,
    DailyExtremeLowerBand = 6,
    OpeningRangeHigh = 10,
    OpeningRangeLow = 11,
    OpeningRangeMid = 12,
    RangeProjection = 20,
    ProfilePoc = 30,
    ProfileVah = 31,
    ProfileVal = 32,
    VolKittC1 = 40,
    VolKittC2 = 41,
    VolKittC3 = 42,
    VolKittC4 = 43,
    VolKittC5 = 44,
    VolKittCog = 45,
    SessionVwap = 50,
    UpperLiquidityShelf = 60,
    LowerLiquidityShelf = 61,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LevelRole {
    OuterLow = 1,
    InnerLow = 2,
    FairValue = 3,
    InnerHigh = 4,
    OuterHigh = 5,
    LowerBoundary = 6,
    UpperBoundary = 7,
    Center = 8,
    LowerLiquidity = 9,
    UpperLiquidity = 10,
    NeutralReference = 11,
}

impl LevelRole {
    #[inline]
    pub const fn side(self) -> RoleSide {
        match self {
            Self::OuterLow | Self::InnerLow | Self::LowerBoundary | Self::LowerLiquidity => {
                RoleSide::Lower
            }
            Self::InnerHigh | Self::OuterHigh | Self::UpperBoundary | Self::UpperLiquidity => {
                RoleSide::Upper
            }
            Self::FairValue | Self::Center => RoleSide::Center,
            Self::NeutralReference => RoleSide::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleSide {
    Lower,
    Center,
    Upper,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LevelState {
    Developing = 1,
    Frozen = 2,
    Superseded = 3,
    Expired = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum VolumeQuality {
    ExchangeVolume = 1,
    VenueVolume = 2,
    TickVolumeProxy = 3,
    TimeAtPriceProxy = 4,
    Unavailable = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LevelProvenance {
    pub producer_id: ProducerId,
    pub first_sequence: JournalSequence,
    pub last_sequence: JournalSequence,
    pub derivation_version: DerivationVersion,
    pub calendar_id: CalendarId,
    pub timeframe: Option<Timeframe>,
    /// Semantic epoch such as session open or rolling-window origin.
    pub epoch_ns: i64,
    /// Siblings with the same non-zero key cannot merge into one node.
    pub exclusive_group: u64,
    pub ordinal: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LevelMetrics {
    pub age_bars: u32,
    pub velocity_atr_ppm: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralLevel {
    pub id: LevelId,
    pub instrument_id: InstrumentId,
    pub price_space: PriceSpace,
    pub kind: LevelKind,
    pub family: LevelFamily,
    pub role: LevelRole,
    pub price: Price,
    pub created_at_ns: i64,
    pub effective_at_ns: i64,
    pub updated_at_ns: i64,
    pub state: LevelState,
    pub provenance: LevelProvenance,
    pub metrics: LevelMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralBand {
    pub id: LevelId,
    pub instrument_id: InstrumentId,
    pub price_space: PriceSpace,
    pub kind: LevelKind,
    pub family: LevelFamily,
    pub role: LevelRole,
    pub lower: Price,
    pub upper: Price,
    pub reference_price: Price,
    pub created_at_ns: i64,
    pub effective_at_ns: i64,
    pub updated_at_ns: i64,
    pub state: LevelState,
    pub provenance: LevelProvenance,
    pub metrics: LevelMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralObject {
    Level(StructuralLevel),
    Band(StructuralBand),
}

impl StructuralObject {
    #[inline]
    pub const fn id(&self) -> LevelId {
        match self {
            Self::Level(level) => level.id,
            Self::Band(band) => band.id,
        }
    }

    #[inline]
    pub const fn instrument_id(&self) -> InstrumentId {
        match self {
            Self::Level(level) => level.instrument_id,
            Self::Band(band) => band.instrument_id,
        }
    }

    #[inline]
    pub const fn price_space(&self) -> PriceSpace {
        match self {
            Self::Level(level) => level.price_space,
            Self::Band(band) => band.price_space,
        }
    }

    #[inline]
    pub const fn kind(&self) -> LevelKind {
        match self {
            Self::Level(level) => level.kind,
            Self::Band(band) => band.kind,
        }
    }

    #[inline]
    pub const fn family(&self) -> LevelFamily {
        match self {
            Self::Level(level) => level.family,
            Self::Band(band) => band.family,
        }
    }

    #[inline]
    pub const fn role(&self) -> LevelRole {
        match self {
            Self::Level(level) => level.role,
            Self::Band(band) => band.role,
        }
    }

    #[inline]
    pub const fn state(&self) -> LevelState {
        match self {
            Self::Level(level) => level.state,
            Self::Band(band) => band.state,
        }
    }

    #[inline]
    pub const fn lower(&self) -> Price {
        match self {
            Self::Level(level) => level.price,
            Self::Band(band) => band.lower,
        }
    }

    #[inline]
    pub const fn upper(&self) -> Price {
        match self {
            Self::Level(level) => level.price,
            Self::Band(band) => band.upper,
        }
    }

    #[inline]
    pub const fn reference_price(&self) -> Price {
        match self {
            Self::Level(level) => level.price,
            Self::Band(band) => band.reference_price,
        }
    }

    #[inline]
    pub const fn provenance(&self) -> LevelProvenance {
        match self {
            Self::Level(level) => level.provenance,
            Self::Band(band) => band.provenance,
        }
    }

    pub fn set_state(&mut self, state: LevelState, updated_at_ns: i64) {
        match self {
            Self::Level(level) => {
                level.state = state;
                level.updated_at_ns = updated_at_ns;
            }
            Self::Band(band) => {
                band.state = state;
                band.updated_at_ns = updated_at_ns;
            }
        }
    }

    #[inline]
    pub const fn is_active(&self) -> bool {
        matches!(self.state(), LevelState::Developing | LevelState::Frozen)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralMutation {
    Created(StructuralObject),
    Updated(StructuralObject),
    Frozen {
        id: LevelId,
        at_ns: i64,
    },
    Superseded {
        old: LevelId,
        new: StructuralObject,
        at_ns: i64,
    },
    Expired {
        id: LevelId,
        at_ns: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralNode {
    pub id: StructuralNodeId,
    pub instrument_id: InstrumentId,
    pub price_space: PriceSpace,
    pub lower: Price,
    pub upper: Price,
    pub center: Price,
    pub roles: SmallVec<[LevelRole; 4]>,
    pub contributors: SmallVec<[LevelId; 8]>,
    pub families: SmallVec<[LevelFamily; 4]>,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralCorridor {
    pub lower_node: StructuralNodeId,
    pub upper_node: StructuralNodeId,
    pub lower_edge: Price,
    pub upper_edge: Price,
    pub width: PriceDelta,
    pub width_atr_ppm: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeGenealogy {
    Created(StructuralNodeId),
    Preserved(StructuralNodeId),
    Merged {
        into: StructuralNodeId,
        from: SmallVec<[StructuralNodeId; 4]>,
    },
    Split {
        from: StructuralNodeId,
        into: SmallVec<[StructuralNodeId; 4]>,
    },
    Expired(StructuralNodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralGraphSnapshot {
    pub generation: u64,
    pub nodes: Arc<[StructuralNode]>,
    pub corridors: Arc<[StructuralCorridor]>,
    pub genealogy: Arc<[NodeGenealogy]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketLocation {
    Unavailable,
    InsideNode(StructuralNodeId),
    Between {
        lower: StructuralNodeId,
        upper: StructuralNodeId,
    },
    BelowKnownStructure {
        nearest: StructuralNodeId,
    },
    AboveKnownStructure {
        nearest: StructuralNodeId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarketLocationSnapshot {
    pub location: MarketLocation,
    pub distance_to_lower: Option<PriceDelta>,
    pub distance_to_upper: Option<PriceDelta>,
    pub distance_to_lower_atr_ppm: Option<i64>,
    pub distance_to_upper_atr_ppm: Option<i64>,
}

impl MarketLocationSnapshot {
    pub const UNAVAILABLE: Self = Self {
        location: MarketLocation::Unavailable,
        distance_to_lower: None,
        distance_to_upper: None,
        distance_to_lower_atr_ppm: None,
        distance_to_upper_atr_ppm: None,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralQuality {
    pub price_fresh: bool,
    pub volume_quality: VolumeQuality,
    pub adaptive_value_available: bool,
    pub volume_profile_available: bool,
    pub tpo_profile_available: bool,
    pub calendar_valid: bool,
    pub gaps_present: bool,
    pub revisions_pending: bool,
}

impl StructuralQuality {
    pub const PRICE_ONLY: Self = Self {
        price_fresh: true,
        volume_quality: VolumeQuality::Unavailable,
        adaptive_value_available: false,
        volume_profile_available: false,
        tpo_profile_available: false,
        calendar_valid: true,
        gaps_present: false,
        revisions_pending: false,
    };
}

#[derive(Clone, Debug)]
pub struct StructuralMarketSnapshot {
    pub instrument_id: InstrumentId,
    pub price_space: PriceSpace,
    pub as_of_ns: i64,
    pub journal_sequence: JournalSequence,
    pub derivation_generation: u64,
    pub reference_price: Price,
    pub levels: Arc<[StructuralObject]>,
    pub graph: Arc<StructuralGraphSnapshot>,
    pub location: MarketLocationSnapshot,
    pub quality: StructuralQuality,
}

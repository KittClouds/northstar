use crate::data_plane::ids::{InstrumentId, JournalSequence, SchemaVersion};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

macro_rules! ledger_id {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Pod,
            Serialize,
            Zeroable,
            Deserialize,
        )]
        #[repr(transparent)]
        pub struct $name(pub u64);

        impl $name {
            pub const UNKNOWN: Self = Self(0);

            #[inline]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

ledger_id!(LedgerEntryId);
ledger_id!(TradeCaseId);
ledger_id!(ActorId);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CorrelationId(pub u128);

impl CorrelationId {
    pub const UNKNOWN: Self = Self(0);

    #[inline]
    pub const fn split(self) -> (u64, u64) {
        (self.0 as u64, (self.0 >> 64) as u64)
    }

    #[inline]
    pub const fn from_parts(low: u64, high: u64) -> Self {
        Self((high as u128) << 64 | low as u128)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SourceEventKey(pub u128);

impl SourceEventKey {
    pub const UNKNOWN: Self = Self(0);

    #[inline]
    pub const fn split(self) -> (u64, u64) {
        (self.0 as u64, (self.0 >> 64) as u64)
    }

    #[inline]
    pub const fn from_parts(low: u64, high: u64) -> Self {
        Self((high as u128) << 64 | low as u128)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum LedgerEventKind {
    Session = 1,
    MacroRelease = 2,
    DataIncident = 3,
    Reconciliation = 4,
    RiskChange = 5,
    Decision = 6,
    Order = 7,
    Fill = 8,
    Position = 9,
    PnlSnapshot = 10,
    OperatorPlan = 11,
    OperatorNote = 12,
    OperatorReview = 13,
    Amendment = 14,
    Redaction = 15,
    Positioning = 16,
    OperatorObservation = 17,
    OperatorIntervention = 18,
}

impl LedgerEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::MacroRelease => "Macro",
            Self::DataIncident => "Incident",
            Self::Reconciliation => "Reconcile",
            Self::RiskChange => "Risk",
            Self::Decision => "Decision",
            Self::Order => "Order",
            Self::Fill => "Fill",
            Self::Position => "Position",
            Self::PnlSnapshot => "P&L",
            Self::OperatorPlan => "Plan",
            Self::OperatorNote => "Note",
            Self::OperatorReview => "Review",
            Self::Amendment => "Amendment",
            Self::Redaction => "Redaction",
            Self::Positioning => "Positioning",
            Self::OperatorObservation => "Observation",
            Self::OperatorIntervention => "Intervention",
        }
    }

    pub const fn is_operator(self) -> bool {
        matches!(
            self,
            Self::OperatorPlan
                | Self::OperatorNote
                | Self::OperatorReview
                | Self::Amendment
                | Self::Redaction
                | Self::OperatorObservation
                | Self::OperatorIntervention
        )
    }

    pub const fn requires_trade_case(self) -> bool {
        matches!(
            self,
            Self::Decision | Self::Order | Self::Fill | Self::Position | Self::PnlSnapshot
        )
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        Some(match raw {
            1 => Self::Session,
            2 => Self::MacroRelease,
            3 => Self::DataIncident,
            4 => Self::Reconciliation,
            5 => Self::RiskChange,
            6 => Self::Decision,
            7 => Self::Order,
            8 => Self::Fill,
            9 => Self::Position,
            10 => Self::PnlSnapshot,
            11 => Self::OperatorPlan,
            12 => Self::OperatorNote,
            13 => Self::OperatorReview,
            14 => Self::Amendment,
            15 => Self::Redaction,
            16 => Self::Positioning,
            17 => Self::OperatorObservation,
            18 => Self::OperatorIntervention,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LedgerEventStatus {
    Observed = 1,
    Pending = 2,
    Accepted = 3,
    Rejected = 4,
    Superseded = 5,
    Redacted = 6,
    Incident = 7,
}

impl LedgerEventStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Redacted => "redacted",
            Self::Incident => "incident",
        }
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        Some(match raw {
            1 => Self::Observed,
            2 => Self::Pending,
            3 => Self::Accepted,
            4 => Self::Rejected,
            5 => Self::Superseded,
            6 => Self::Redacted,
            7 => Self::Incident,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ActorKind {
    Northstar = 1,
    Operator = 2,
    Venue = 3,
    Provider = 4,
    Import = 5,
}

impl ActorKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        Some(match raw {
            1 => Self::Northstar,
            2 => Self::Operator,
            3 => Self::Venue,
            4 => Self::Provider,
            5 => Self::Import,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LedgerFlags(pub u16);

impl LedgerFlags {
    pub const NONE: Self = Self(0);
    pub const MATERIAL: Self = Self(1 << 0);
    pub const HAS_ATTACHMENT: Self = Self(1 << 1);
    pub const SENSITIVE: Self = Self(1 << 2);

    #[inline]
    pub const fn bits(self) -> u16 {
        self.0
    }
}

/// Fixed hot record. Bodies stay in the cold pack and are loaded only by the
/// inspector. Explicit field ordering keeps this record padding-free and mmap
/// safe.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct LedgerEventHeader {
    pub entry_id: u64,
    pub case_id: u64,
    pub parent_entry_id: u64,
    pub correlation_low: u64,
    pub correlation_high: u64,
    pub source_key_low: u64,
    pub source_key_high: u64,
    pub actor_id: u64,
    pub ts_event_ns: i64,
    pub ts_received_ns: i64,
    pub ts_recorded_ns: i64,
    pub canonical_first_sequence: u64,
    pub canonical_last_sequence: u64,
    pub body_offset: u64,
    pub account_id: u64,
    pub body_hash: [u8; 32],
    pub body_len: u32,
    pub instrument_id: u32,
    pub schema_version: u16,
    pub kind: u16,
    pub flags: u16,
    pub status: u8,
    pub actor_kind: u8,
    pub reserved: [u8; 8],
}

impl LedgerEventHeader {
    #[inline]
    pub const fn entry_id(&self) -> LedgerEntryId {
        LedgerEntryId(self.entry_id)
    }

    #[inline]
    pub const fn case_id(&self) -> TradeCaseId {
        TradeCaseId(self.case_id)
    }

    #[inline]
    pub const fn source_key(&self) -> SourceEventKey {
        SourceEventKey::from_parts(self.source_key_low, self.source_key_high)
    }

    #[inline]
    pub const fn correlation_id(&self) -> CorrelationId {
        CorrelationId::from_parts(self.correlation_low, self.correlation_high)
    }

    #[inline]
    pub const fn kind(&self) -> Option<LedgerEventKind> {
        LedgerEventKind::from_raw(self.kind)
    }
}

pub struct LedgerEventDraft<'a> {
    pub case_id: TradeCaseId,
    pub parent_entry_id: LedgerEntryId,
    pub correlation_id: CorrelationId,
    /// Stable upstream event or operator-command identity. Required for every
    /// append so retries are idempotent.
    pub source_key: SourceEventKey,
    pub actor_id: ActorId,
    pub actor_kind: ActorKind,
    pub kind: LedgerEventKind,
    pub status: LedgerEventStatus,
    pub flags: LedgerFlags,
    pub instrument_id: InstrumentId,
    pub account_id: u64,
    pub ts_event_ns: i64,
    pub ts_received_ns: i64,
    pub ts_recorded_ns: i64,
    pub canonical_sequences: Option<(JournalSequence, JournalSequence)>,
    pub schema_version: SchemaVersion,
    pub body: &'a [u8],
}

const _: () = assert!(std::mem::size_of::<LedgerEventHeader>() == 176);

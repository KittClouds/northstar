use crate::data_plane::event::CanonicalBatch;
use crate::data_plane::ids::SourceId;
use crate::data_plane::receipt::{RawReceipt, RawReceiptRef};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SourceCapabilities(pub u64);

impl SourceCapabilities {
    pub const INDEX_VALUES: u64 = 1 << 0;
    pub const REFERENCE_AGGREGATES: u64 = 1 << 1;
    pub const VENUE_QUOTES: u64 = 1 << 2;
    pub const VENUE_STATE: u64 = 1 << 3;
    pub const INSTRUMENT_DISCOVERY: u64 = 1 << 4;
    pub const MACRO_OBSERVATIONS: u64 = 1 << 5;
    pub const NATIVE_VINTAGES: u64 = 1 << 6;
    pub const POSITIONING: u64 = 1 << 7;
    pub const DOCUMENTS: u64 = 1 << 8;

    #[inline]
    pub const fn contains(self, capability: u64) -> bool {
        self.0 & capability != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LicenseClass {
    OpenWithAttribution,
    PersonalDisplayOnly,
    LicensedNonDisplay,
    InternalRestricted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExportScope {
    InternalResearch,
    Redistribution,
}

impl LicenseClass {
    #[inline]
    pub const fn permits_canonical_retention(self) -> bool {
        matches!(
            self,
            Self::OpenWithAttribution | Self::LicensedNonDisplay | Self::InternalRestricted
        )
    }

    #[inline]
    pub const fn permits_export(self, scope: ExportScope) -> bool {
        matches!(
            (self, scope),
            (Self::OpenWithAttribution, _)
                | (
                    Self::LicensedNonDisplay | Self::InternalRestricted,
                    ExportScope::InternalResearch,
                )
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceContract {
    pub source_id: SourceId,
    pub name: String,
    pub capabilities: SourceCapabilities,
    pub license: LicenseClass,
    pub contract_version: u32,
}

/// Network implementations live outside the GPUI paint path and emit exact
/// receipts. Provider JSON/CSV/XML never crosses this boundary.
pub trait SourceAdapter {
    fn contract(&self) -> &SourceContract;

    fn poll(&mut self, output: &mut SmallVec<[RawReceipt; 8]>) -> Result<(), SourceError>;
}

/// Pure, fixture-testable provider translation. Decoders do no I/O and append
/// only validated Northstar events to the supplied batch.
pub trait CanonicalDecoder {
    fn source_id(&self) -> SourceId;

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DecodeStats {
    pub provider_records: u32,
    pub canonical_events: u32,
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("receipt source {actual:?} does not match decoder source {expected:?}")]
    WrongSource {
        expected: SourceId,
        actual: SourceId,
    },
    #[error("provider payload is malformed: {0}")]
    Malformed(String),
    #[error("provider schema is unsupported: {0}")]
    UnsupportedSchema(String),
    #[error("provider identity is not bound: {0}")]
    UnknownIdentity(String),
    #[error("provider entitlement denied: {0}")]
    NotEntitled(String),
    #[error("provider rate limit reached")]
    RateLimited,
    #[error("provider identity was not found: {0}")]
    NotFound(String),
    #[error("provider batch is incomplete: expected {expected}, got {actual}")]
    IncompleteBatch { expected: usize, actual: usize },
    #[error("provider entitlement recency changed: {0}")]
    EntitlementMismatch(String),
    #[error("provider numeric value is invalid: {0}")]
    InvalidNumber(String),
    #[error("provider timestamp is out of range")]
    TimestampOverflow,
    #[error("canonical event rejected: {0}")]
    Canonical(#[from] crate::data_plane::event::CanonicalError),
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("source is disconnected")]
    Disconnected,
    #[error("source rate limit reached")]
    RateLimited,
    #[error("source schema changed: {0}")]
    SchemaDrift(String),
    #[error("source rejected request: {0}")]
    Rejected(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_only_data_fails_closed_for_retention() {
        assert!(!LicenseClass::PersonalDisplayOnly.permits_canonical_retention());
        assert!(!LicenseClass::PersonalDisplayOnly.permits_export(ExportScope::InternalResearch));
        assert!(!LicenseClass::LicensedNonDisplay.permits_export(ExportScope::Redistribution));
    }
}

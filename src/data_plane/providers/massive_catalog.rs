use crate::data_plane::catalog::{
    AuthorityDomain, AuthorityLease, CanonicalCatalog, CatalogError, ProviderSymbolBinding,
};
use crate::data_plane::ids::{instruments, CatalogVersion, InstrumentId, SourceId};
use crate::data_plane::source::LicenseClass;
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MASSIVE_CATALOG_SCHEMA_V1: u32 = 1;
pub const MASSIVE_SOURCE: SourceId = SourceId(10);
pub const MASSIVE_DESK_CAPACITY: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MassiveRecency {
    #[serde(rename = "REAL-TIME")]
    RealTime,
    #[serde(rename = "DELAYED")]
    Delayed,
}

/// Relationship between a Massive index and Northstar's permanent desk ID.
/// A proxy may inform the desk, but it must never be presented as the venue's
/// executable contract or used to prove instrument-contract agreement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MassiveReferenceRole {
    ExactBenchmark,
    ReferenceProxy,
}

impl MassiveRecency {
    #[inline]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::RealTime => "REAL-TIME",
            Self::Delayed => "DELAYED",
        }
    }
}

/// Cold, reviewable authority record produced only after provider discovery.
/// No Massive ticker is compiled in as a production assumption.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MassiveCatalogManifestV1 {
    pub schema_version: u32,
    pub catalog_version: CatalogVersion,
    pub source_id: SourceId,
    pub captured_at_ns: i64,
    pub license: LicenseClass,
    pub indices: Vec<MassiveIndexBindingV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MassiveIndexBindingV1 {
    pub instrument: InstrumentId,
    pub northstar_symbol: Box<str>,
    pub display_name: Box<str>,
    pub provider_ticker: Box<str>,
    pub provider_name: Box<str>,
    pub reference_role: MassiveReferenceRole,
    /// Human-reviewable evidence for the exact benchmark or proxy decision.
    /// This is cold provenance, never copied into hot market records.
    pub binding_basis: Box<str>,
    pub price_scale: i64,
    pub effective_from_ns: i64,
    pub verified_at_ns: i64,
    pub recency: MassiveRecency,
}

#[derive(Clone, Debug)]
pub struct ValidatedMassiveCatalog {
    manifest: MassiveCatalogManifestV1,
}

impl MassiveCatalogManifestV1 {
    pub fn from_json(bytes: &[u8]) -> Result<ValidatedMassiveCatalog, MassiveCatalogError> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|error| MassiveCatalogError::Malformed(error.to_string()))?;
        manifest.validate()
    }

    pub fn validate(self) -> Result<ValidatedMassiveCatalog, MassiveCatalogError> {
        if self.schema_version != MASSIVE_CATALOG_SCHEMA_V1 {
            return Err(MassiveCatalogError::UnsupportedSchema(self.schema_version));
        }
        if self.catalog_version == CatalogVersion::UNKNOWN
            || self.source_id != MASSIVE_SOURCE
            || self.captured_at_ns <= 0
        {
            return Err(MassiveCatalogError::InvalidManifestIdentity);
        }
        if self.indices.is_empty() || self.indices.len() > MASSIVE_DESK_CAPACITY {
            return Err(MassiveCatalogError::InvalidInstrumentCount {
                maximum: MASSIVE_DESK_CAPACITY,
                actual: self.indices.len(),
            });
        }

        let mut instrument_mask = 0u8;
        let mut tickers = HashSet::with_capacity(self.indices.len());
        for binding in &self.indices {
            let Some((slot, canonical_symbol)) = canonical_instrument(binding.instrument) else {
                return Err(MassiveCatalogError::UnknownInstrument(binding.instrument));
            };
            if binding.northstar_symbol.as_ref() != canonical_symbol {
                return Err(MassiveCatalogError::WrongNorthstarSymbol {
                    instrument: binding.instrument,
                    actual: binding.northstar_symbol.to_string(),
                });
            }
            let bit = 1u8 << slot;
            if instrument_mask & bit != 0 {
                return Err(MassiveCatalogError::DuplicateInstrument(binding.instrument));
            }
            instrument_mask |= bit;
            if !tickers.insert(binding.provider_ticker.clone()) {
                return Err(MassiveCatalogError::DuplicateTicker(
                    binding.provider_ticker.to_string(),
                ));
            }
            if !valid_provider_ticker(&binding.provider_ticker)
                || binding.display_name.trim().is_empty()
                || binding.provider_name.trim().is_empty()
                || binding.binding_basis.trim().is_empty()
                || binding.price_scale <= 0
                || binding.effective_from_ns < 0
                || binding.verified_at_ns < binding.effective_from_ns
                || binding.verified_at_ns > self.captured_at_ns
            {
                return Err(MassiveCatalogError::InvalidBinding(
                    binding.northstar_symbol.to_string(),
                ));
            }
        }
        debug_assert_eq!(instrument_mask.count_ones() as usize, self.indices.len());
        Ok(ValidatedMassiveCatalog { manifest: self })
    }
}

impl ValidatedMassiveCatalog {
    #[inline]
    pub const fn manifest(&self) -> &MassiveCatalogManifestV1 {
        &self.manifest
    }

    #[inline]
    pub fn bindings(&self) -> &[MassiveIndexBindingV1] {
        &self.manifest.indices
    }

    pub fn binding_for_ticker(&self, ticker: &str) -> Option<&MassiveIndexBindingV1> {
        self.manifest
            .indices
            .iter()
            .find(|binding| binding.provider_ticker.as_ref() == ticker)
    }

    pub fn canonical_catalog(&self) -> Result<CanonicalCatalog, MassiveCatalogError> {
        if !self.manifest.license.permits_canonical_retention() {
            return Err(MassiveCatalogError::RetentionForbidden(
                self.manifest.license,
            ));
        }
        let mut bindings = Vec::with_capacity(self.manifest.indices.len());
        let mut authorities = Vec::with_capacity(self.manifest.indices.len());
        for binding in &self.manifest.indices {
            bindings.push(ProviderSymbolBinding {
                source: self.manifest.source_id,
                instrument: binding.instrument,
                symbol: binding.provider_ticker.clone(),
                price_scale: binding.price_scale,
                effective_from_ns: binding.effective_from_ns,
                effective_to_ns: i64::MAX,
                catalog_version: self.manifest.catalog_version,
            });
            authorities.push(AuthorityLease {
                domain: AuthorityDomain::ReferenceValue,
                entity_id: binding.instrument.get(),
                source: self.manifest.source_id,
                effective_from_ns: binding.effective_from_ns,
                effective_to_ns: i64::MAX,
            });
        }
        CanonicalCatalog::new(self.manifest.catalog_version, bindings, authorities)
            .map_err(MassiveCatalogError::Canonical)
    }
}

fn canonical_instrument(instrument: InstrumentId) -> Option<(u32, &'static str)> {
    match instrument {
        instruments::US100 => Some((0, "US100")),
        instruments::US500 => Some((1, "US500")),
        instruments::US30 => Some((2, "US30")),
        instruments::DE40 => Some((3, "DE40")),
        instruments::UK100 => Some((4, "UK100")),
        instruments::JP225 => Some((5, "JP225")),
        _ => None,
    }
}

fn valid_provider_ticker(ticker: &str) -> bool {
    ticker.len() > 2
        && ticker.starts_with("I:")
        && ticker
            .as_bytes()
            .iter()
            .skip(2)
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_')
}

#[derive(Debug, Error)]
pub enum MassiveCatalogError {
    #[error("Massive catalog JSON is malformed: {0}")]
    Malformed(String),
    #[error("unsupported Massive catalog schema {0}")]
    UnsupportedSchema(u32),
    #[error("Massive catalog identity is invalid")]
    InvalidManifestIdentity,
    #[error("Massive catalog requires 1..={maximum} verified indices, got {actual}")]
    InvalidInstrumentCount { maximum: usize, actual: usize },
    #[error("unknown Northstar instrument {0:?}")]
    UnknownInstrument(InstrumentId),
    #[error("duplicate Northstar instrument {0:?}")]
    DuplicateInstrument(InstrumentId),
    #[error("duplicate Massive ticker {0}")]
    DuplicateTicker(String),
    #[error("wrong Northstar symbol for {instrument:?}: {actual}")]
    WrongNorthstarSymbol {
        instrument: InstrumentId,
        actual: String,
    },
    #[error("invalid Massive binding for {0}")]
    InvalidBinding(String),
    #[error("Massive license {0:?} does not permit canonical retention")]
    RetentionForbidden(LicenseClass),
    #[error(transparent)]
    Canonical(#[from] CatalogError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> MassiveCatalogManifestV1 {
        let definitions = [
            (instruments::US100, "US100", "I:ONE"),
            (instruments::US500, "US500", "I:TWO"),
            (instruments::US30, "US30", "I:THREE"),
            (instruments::DE40, "DE40", "I:FOUR"),
            (instruments::UK100, "UK100", "I:FIVE"),
            (instruments::JP225, "JP225", "I:SIX"),
        ];
        MassiveCatalogManifestV1 {
            schema_version: 1,
            catalog_version: CatalogVersion(7),
            source_id: MASSIVE_SOURCE,
            captured_at_ns: 200,
            license: LicenseClass::InternalRestricted,
            indices: definitions
                .into_iter()
                .map(|(instrument, symbol, ticker)| MassiveIndexBindingV1 {
                    instrument,
                    northstar_symbol: symbol.into(),
                    display_name: symbol.into(),
                    provider_ticker: ticker.into(),
                    provider_name: format!("Provider {symbol}").into(),
                    reference_role: MassiveReferenceRole::ExactBenchmark,
                    binding_basis: format!("verified exact binding for {symbol}").into(),
                    price_scale: 10_000,
                    effective_from_ns: 1,
                    verified_at_ns: 100,
                    recency: MassiveRecency::RealTime,
                })
                .collect(),
        }
    }

    #[test]
    fn complete_verified_manifest_builds_one_authority_per_index() {
        let validated = manifest().validate().unwrap();
        let catalog = validated.canonical_catalog().unwrap();
        for binding in validated.bindings() {
            assert_eq!(
                catalog.authority(
                    AuthorityDomain::ReferenceValue,
                    binding.instrument.get(),
                    150
                ),
                Some(MASSIVE_SOURCE)
            );
        }
    }

    #[test]
    fn verified_subset_is_valid_but_empty_or_duplicate_fails_closed() {
        let mut subset = manifest();
        subset.indices.truncate(3);
        let validated = subset.validate().unwrap();
        let catalog = validated.canonical_catalog().unwrap();
        assert_eq!(validated.bindings().len(), 3);
        assert_eq!(
            catalog.authority(
                AuthorityDomain::ReferenceValue,
                instruments::DE40.get(),
                150
            ),
            None
        );

        let mut empty = manifest();
        empty.indices.clear();
        assert!(matches!(
            empty.validate(),
            Err(MassiveCatalogError::InvalidInstrumentCount { actual: 0, .. })
        ));

        let mut duplicate = manifest();
        duplicate.indices[5].instrument = instruments::US100;
        duplicate.indices[5].northstar_symbol = "US100".into();
        assert!(matches!(
            duplicate.validate(),
            Err(MassiveCatalogError::DuplicateInstrument(_))
        ));
    }

    #[test]
    fn display_only_license_cannot_activate_durable_data_plane() {
        let mut manifest = manifest();
        manifest.license = LicenseClass::PersonalDisplayOnly;
        let validated = manifest.validate().unwrap();
        assert!(matches!(
            validated.canonical_catalog(),
            Err(MassiveCatalogError::RetentionForbidden(_))
        ));
    }
}

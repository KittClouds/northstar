use crate::data_plane::ids::{CatalogVersion, InstrumentId, SourceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderSymbolBinding {
    pub source: SourceId,
    pub instrument: InstrumentId,
    pub symbol: Box<str>,
    /// Canonical integer units per displayed whole unit.
    pub price_scale: i64,
    pub effective_from_ns: i64,
    pub effective_to_ns: i64,
    pub catalog_version: CatalogVersion,
}

impl ProviderSymbolBinding {
    #[inline]
    pub const fn active_at(&self, timestamp_ns: i64) -> bool {
        timestamp_ns >= self.effective_from_ns && timestamp_ns < self.effective_to_ns
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(u16)]
pub enum AuthorityDomain {
    ReferenceValue = 1,
    VenueQuote = 2,
    InstrumentContract = 3,
    MacroSeries = 4,
    Positioning = 5,
    OfficialDocument = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthorityLease {
    pub domain: AuthorityDomain,
    pub entity_id: u32,
    pub source: SourceId,
    pub effective_from_ns: i64,
    pub effective_to_ns: i64,
}

impl AuthorityLease {
    #[inline]
    pub const fn active_at(self, timestamp_ns: i64) -> bool {
        timestamp_ns >= self.effective_from_ns && timestamp_ns < self.effective_to_ns
    }
}

#[derive(Clone, Debug)]
pub struct CanonicalCatalog {
    pub version: CatalogVersion,
    bindings: Box<[ProviderSymbolBinding]>,
    authorities: Box<[AuthorityLease]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VenueInstrumentContract {
    pub instrument: InstrumentId,
    pub source: SourceId,
    pub environment_id: u32,
    pub account_id: u64,
    pub tradable_instrument_id: u64,
    pub info_route_id: u32,
    pub trade_route_id: u32,
    pub session_id: u32,
    pub minimum_quantity_micros: i64,
    pub quantity_step_micros: i64,
    pub price_tick_nanos: i64,
    pub price_precision: u16,
    pub contract_version: u32,
    pub effective_from_ns: i64,
    pub effective_to_ns: i64,
}

impl VenueInstrumentContract {
    pub fn validate(self) -> Result<Self, CatalogError> {
        if self.instrument == InstrumentId::UNKNOWN
            || self.source == SourceId::UNKNOWN
            || self.environment_id == 0
            || self.account_id == 0
            || self.tradable_instrument_id == 0
            || self.info_route_id == 0
            || self.trade_route_id == 0
            || self.session_id == 0
            || self.minimum_quantity_micros <= 0
            || self.quantity_step_micros <= 0
            || self.price_tick_nanos <= 0
            || self.contract_version == 0
            || self.effective_from_ns >= self.effective_to_ns
        {
            return Err(CatalogError::InvalidVenueContract(self.instrument));
        }
        Ok(self)
    }

    #[inline]
    pub const fn active_at(self, timestamp_ns: i64) -> bool {
        timestamp_ns >= self.effective_from_ns && timestamp_ns < self.effective_to_ns
    }

    /// Arming agreement excludes route/session openness, which is dynamic venue
    /// state rather than immutable contract metadata.
    #[inline]
    pub const fn execution_terms_equal(self, other: Self) -> bool {
        self.instrument.0 == other.instrument.0
            && self.environment_id == other.environment_id
            && self.account_id == other.account_id
            && self.tradable_instrument_id == other.tradable_instrument_id
            && self.info_route_id == other.info_route_id
            && self.trade_route_id == other.trade_route_id
            && self.session_id == other.session_id
            && self.minimum_quantity_micros == other.minimum_quantity_micros
            && self.quantity_step_micros == other.quantity_step_micros
            && self.price_tick_nanos == other.price_tick_nanos
            && self.price_precision == other.price_precision
    }
}

#[derive(Clone, Debug)]
pub struct VenueContractCatalog {
    contracts: Box<[VenueInstrumentContract]>,
}

impl VenueContractCatalog {
    pub fn new(mut contracts: Vec<VenueInstrumentContract>) -> Result<Self, CatalogError> {
        for contract in &mut contracts {
            *contract = contract.validate()?;
        }
        contracts.sort_unstable_by_key(|contract| {
            (
                contract.environment_id,
                contract.account_id,
                contract.instrument,
                contract.effective_from_ns,
            )
        });
        for pair in contracts.windows(2) {
            let [left, right] = pair else { unreachable!() };
            if left.environment_id == right.environment_id
                && left.account_id == right.account_id
                && left.instrument == right.instrument
                && left.effective_to_ns > right.effective_from_ns
            {
                return Err(CatalogError::OverlappingVenueContract(left.instrument));
            }
        }
        Ok(Self {
            contracts: contracts.into_boxed_slice(),
        })
    }

    pub fn resolve(
        &self,
        environment_id: u32,
        account_id: u64,
        instrument: InstrumentId,
        timestamp_ns: i64,
    ) -> Option<VenueInstrumentContract> {
        self.contracts.iter().copied().find(|contract| {
            contract.environment_id == environment_id
                && contract.account_id == account_id
                && contract.instrument == instrument
                && contract.active_at(timestamp_ns)
        })
    }
}

impl CanonicalCatalog {
    pub fn new(
        version: CatalogVersion,
        mut bindings: Vec<ProviderSymbolBinding>,
        mut authorities: Vec<AuthorityLease>,
    ) -> Result<Self, CatalogError> {
        if version == CatalogVersion::UNKNOWN {
            return Err(CatalogError::MissingVersion);
        }
        for binding in &bindings {
            if binding.catalog_version != version
                || binding.source == SourceId::UNKNOWN
                || binding.instrument == InstrumentId::UNKNOWN
                || binding.symbol.is_empty()
                || binding.price_scale <= 0
                || binding.effective_from_ns >= binding.effective_to_ns
            {
                return Err(CatalogError::InvalidBinding(binding.symbol.to_string()));
            }
        }
        bindings.sort_unstable_by(|left, right| {
            (left.source, left.symbol.as_ref(), left.effective_from_ns).cmp(&(
                right.source,
                right.symbol.as_ref(),
                right.effective_from_ns,
            ))
        });
        for pair in bindings.windows(2) {
            let [left, right] = pair else { unreachable!() };
            if left.source == right.source
                && left.symbol == right.symbol
                && left.effective_to_ns > right.effective_from_ns
            {
                return Err(CatalogError::OverlappingBinding(left.symbol.to_string()));
            }
        }

        authorities
            .sort_unstable_by_key(|lease| (lease.domain, lease.entity_id, lease.effective_from_ns));
        for lease in &authorities {
            if lease.source == SourceId::UNKNOWN
                || lease.entity_id == 0
                || lease.effective_from_ns >= lease.effective_to_ns
            {
                return Err(CatalogError::InvalidAuthority {
                    domain: lease.domain,
                    entity_id: lease.entity_id,
                });
            }
        }
        for pair in authorities.windows(2) {
            let [left, right] = pair else { unreachable!() };
            if left.domain == right.domain
                && left.entity_id == right.entity_id
                && left.effective_to_ns > right.effective_from_ns
            {
                return Err(CatalogError::DuplicateAuthority {
                    domain: left.domain,
                    entity_id: left.entity_id,
                });
            }
        }

        Ok(Self {
            version,
            bindings: bindings.into_boxed_slice(),
            authorities: authorities.into_boxed_slice(),
        })
    }

    pub fn resolve_symbol(
        &self,
        source: SourceId,
        symbol: &str,
        timestamp_ns: i64,
    ) -> Option<&ProviderSymbolBinding> {
        self.bindings.iter().find(|binding| {
            binding.source == source
                && binding.symbol.as_ref() == symbol
                && binding.active_at(timestamp_ns)
        })
    }

    pub fn authority(
        &self,
        domain: AuthorityDomain,
        entity_id: u32,
        timestamp_ns: i64,
    ) -> Option<SourceId> {
        self.authorities
            .iter()
            .find(|lease| {
                lease.domain == domain
                    && lease.entity_id == entity_id
                    && lease.active_at(timestamp_ns)
            })
            .map(|lease| lease.source)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CatalogError {
    #[error("catalog version must be non-zero")]
    MissingVersion,
    #[error("invalid provider symbol binding {0}")]
    InvalidBinding(String),
    #[error("overlapping provider symbol binding {0}")]
    OverlappingBinding(String),
    #[error("invalid authority for {domain:?}:{entity_id}")]
    InvalidAuthority {
        domain: AuthorityDomain,
        entity_id: u32,
    },
    #[error("duplicate authority for {domain:?}:{entity_id}")]
    DuplicateAuthority {
        domain: AuthorityDomain,
        entity_id: u32,
    },
    #[error("invalid venue instrument contract for {0:?}")]
    InvalidVenueContract(InstrumentId),
    #[error("overlapping venue instrument contract for {0:?}")]
    OverlappingVenueContract(InstrumentId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_authority_is_rejected() {
        let leases = vec![
            AuthorityLease {
                domain: AuthorityDomain::ReferenceValue,
                entity_id: 1,
                source: SourceId(1),
                effective_from_ns: 0,
                effective_to_ns: 20,
            },
            AuthorityLease {
                domain: AuthorityDomain::ReferenceValue,
                entity_id: 1,
                source: SourceId(2),
                effective_from_ns: 10,
                effective_to_ns: 30,
            },
        ];
        assert!(matches!(
            CanonicalCatalog::new(CatalogVersion(1), Vec::new(), leases),
            Err(CatalogError::DuplicateAuthority { .. })
        ));
    }

    #[test]
    fn venue_contract_identity_includes_account_routes_and_session() {
        let contract = VenueInstrumentContract {
            instrument: InstrumentId(1),
            source: SourceId(20),
            environment_id: 1,
            account_id: 42,
            tradable_instrument_id: 99,
            info_route_id: 7,
            trade_route_id: 8,
            session_id: 9,
            minimum_quantity_micros: 100_000,
            quantity_step_micros: 100_000,
            price_tick_nanos: 100_000_000,
            price_precision: 1,
            contract_version: 1,
            effective_from_ns: 0,
            effective_to_ns: 100,
        };
        let catalog = VenueContractCatalog::new(vec![contract]).unwrap();
        assert_eq!(catalog.resolve(1, 42, InstrumentId(1), 50), Some(contract));
        let mut changed = contract;
        changed.quantity_step_micros = 200_000;
        assert!(!contract.execution_terms_equal(changed));
    }
}

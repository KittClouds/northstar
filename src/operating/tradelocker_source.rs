use super::{Availability, VenueSnapshot};
use crate::data_plane::ids::{instruments, InstrumentId};
use crate::data_plane::providers::tradelocker::{
    TradeLockerBinding, TradeLockerDecodeError, TradeLockerDecoder, VenueEpochData,
};
use crate::data_plane::providers::tradelocker_rest::{
    TradeLockerReadClient, TradeLockerTransportError,
};
use crate::data_plane::providers::tradelocker_runtime::{
    TradeLockerRuntimeConfig, TradeLockerRuntimeConfigError,
};
use hashbrown::HashSet;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

const EMBEDDED_CATALOG: &[u8] = include_bytes!("../../tradelocker.catalog.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogManifest {
    schema_version: u32,
    catalog_version: u32,
    environment: Box<str>,
    bindings: Vec<CatalogBinding>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogBinding {
    instrument_id: u32,
    northstar_symbol: Box<str>,
    provider_symbol: Box<str>,
    price_scale: i64,
}

pub(super) struct TradeLockerSourceStartup {
    pub client: TradeLockerReadClient,
    pub decoder: TradeLockerDecoder,
    pub initial_snapshot: Arc<VenueSnapshot>,
}

pub(super) fn open_tradelocker_source(
    root: &Path,
) -> Result<TradeLockerSourceStartup, TradeLockerSourceStartupError> {
    let config = TradeLockerRuntimeConfig::from_environment()?;
    // Validate that the protected handle resolves before any source thread is
    // advertised as configured. The secret is zeroized on this scope exit.
    drop(config.load_secret()?);
    let catalog_path = root.join("tradelocker.catalog.json");
    let bytes = match std::fs::read(&catalog_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => EMBEDDED_CATALOG.to_vec(),
        Err(error) => return Err(TradeLockerSourceStartupError::CatalogIo(error)),
    };
    let manifest: CatalogManifest = serde_json::from_slice(&bytes)?;
    let bindings = validate_manifest(manifest)?;
    let decoder = TradeLockerDecoder::new(bindings.clone(), HashSet::new(), HashSet::new())?;
    let client = TradeLockerReadClient::new(config, bindings)?;
    Ok(TradeLockerSourceStartup {
        client,
        decoder,
        initial_snapshot: Arc::new(VenueSnapshot::unavailable(
            Availability::AwaitingFirstReceipt,
            "TradeLocker configured / coherent epoch pending",
        )),
    })
}

fn validate_manifest(
    manifest: CatalogManifest,
) -> Result<Vec<TradeLockerBinding>, TradeLockerSourceStartupError> {
    if manifest.schema_version != 1
        || manifest.catalog_version == 0
        || manifest.environment.as_ref() != "live"
        || manifest.bindings.len() != 6
    {
        return Err(TradeLockerSourceStartupError::InvalidCatalog);
    }
    let expected = [
        (instruments::US100, "US100"),
        (instruments::US500, "US500"),
        (instruments::US30, "US30"),
        (instruments::DE40, "DE40"),
        (instruments::UK100, "UK100"),
        (instruments::JP225, "JP225"),
    ];
    let mut seen_ids = HashSet::with_capacity(6);
    let mut seen_symbols = HashSet::with_capacity(6);
    let mut output = Vec::with_capacity(6);
    for binding in manifest.bindings {
        let instrument = InstrumentId(binding.instrument_id);
        let Some((_, expected_symbol)) = expected.iter().find(|(id, _)| *id == instrument) else {
            return Err(TradeLockerSourceStartupError::InvalidCatalog);
        };
        if binding.northstar_symbol.as_ref() != *expected_symbol
            || binding.provider_symbol.is_empty()
            || binding.price_scale <= 0
            || !seen_ids.insert(instrument)
            || !seen_symbols.insert(binding.provider_symbol.clone())
        {
            return Err(TradeLockerSourceStartupError::InvalidCatalog);
        }
        output.push(TradeLockerBinding {
            instrument,
            provider_symbol: Arc::from(binding.provider_symbol),
            price_scale: binding.price_scale,
        });
    }
    Ok(output)
}

pub(super) fn poll_epoch(
    client: &mut TradeLockerReadClient,
    decoder: &TradeLockerDecoder,
) -> Result<VenueEpochData, TradeLockerSourcePollError> {
    let refresh = client.refresh()?;
    Ok(decoder.decode(&refresh)?)
}

#[derive(Debug, Error)]
pub(super) enum TradeLockerSourceStartupError {
    #[error(transparent)]
    Config(#[from] TradeLockerRuntimeConfigError),
    #[error("TradeLocker catalog I/O failed: {0}")]
    CatalogIo(std::io::Error),
    #[error("TradeLocker catalog JSON is malformed: {0}")]
    CatalogJson(#[from] serde_json::Error),
    #[error("TradeLocker catalog violates the six-index authority contract")]
    InvalidCatalog,
    #[error(transparent)]
    Decode(#[from] TradeLockerDecodeError),
    #[error(transparent)]
    Transport(#[from] TradeLockerTransportError),
}

impl TradeLockerSourceStartupError {
    pub fn availability(&self) -> Availability {
        match self {
            Self::Config(
                TradeLockerRuntimeConfigError::ServerMissing
                | TradeLockerRuntimeConfigError::CredentialUnavailable(_),
            ) => Availability::NotConfigured,
            _ => Availability::Invalid,
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum TradeLockerSourcePollError {
    #[error(transparent)]
    Transport(#[from] TradeLockerTransportError),
    #[error(transparent)]
    Decode(#[from] TradeLockerDecodeError),
}

impl TradeLockerSourcePollError {
    pub fn availability(&self) -> Availability {
        match self {
            Self::Transport(TradeLockerTransportError::RateLimited { .. }) => Availability::Stale,
            Self::Transport(
                TradeLockerTransportError::AuthorizationRejected(_)
                | TradeLockerTransportError::AuthenticationRejected(_),
            ) => Availability::Disconnected,
            _ => Availability::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_maps_exactly_six_permanent_ids() {
        let manifest: CatalogManifest = serde_json::from_slice(EMBEDDED_CATALOG).unwrap();
        let bindings = validate_manifest(manifest).unwrap();
        assert_eq!(bindings.len(), 6);
        assert_eq!(bindings[0].instrument, instruments::US100);
        assert_eq!(bindings[5].instrument, instruments::JP225);
    }
}

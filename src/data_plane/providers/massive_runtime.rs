use super::massive_catalog::{
    MassiveCatalogError, MassiveCatalogManifestV1, ValidatedMassiveCatalog,
};
use crate::data_plane::source::{SourceCapabilities, SourceContract};
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

pub const MASSIVE_API_KEY_ENV: &str = "MASSIVE_API_KEY";
pub const MASSIVE_REST_ROOT: &str = "https://api.massive.com";
pub const MASSIVE_INDEX_SNAPSHOT_PATH: &str = "/v3/snapshot/indices";

/// Process-memory credential. It cannot be serialized and its Debug output is
/// permanently redacted. Northstar never writes it to receipts or journals.
pub struct MassiveApiKey(Box<str>);

impl MassiveApiKey {
    fn from_os(value: Option<OsString>) -> Result<Self, MassiveRuntimeConfigError> {
        let value = value.ok_or(MassiveRuntimeConfigError::CredentialMissing)?;
        let value = value
            .into_string()
            .map_err(|_| MassiveRuntimeConfigError::CredentialInvalid)?;
        if value.trim().len() < 16 || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(MassiveRuntimeConfigError::CredentialInvalid);
        }
        Ok(Self(value.into_boxed_str()))
    }

    #[inline]
    pub fn authorization_value(&self) -> String {
        let mut value = String::with_capacity(7 + self.0.len());
        value.push_str("Bearer ");
        value.push_str(&self.0);
        value
    }
}

impl fmt::Debug for MassiveApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MassiveApiKey(<redacted>)")
    }
}

pub struct MassiveRuntimeConfig {
    pub catalog_path: PathBuf,
    pub catalog: Arc<ValidatedMassiveCatalog>,
    pub api_key: MassiveApiKey,
    pub rest_root: &'static str,
}

impl fmt::Debug for MassiveRuntimeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MassiveRuntimeConfig")
            .field("catalog_path", &self.catalog_path)
            .field("catalog_version", &self.catalog.manifest().catalog_version)
            .field("api_key", &self.api_key)
            .field("rest_root", &self.rest_root)
            .finish()
    }
}

impl MassiveRuntimeConfig {
    pub fn from_environment(
        catalog_path: impl AsRef<Path>,
    ) -> Result<Self, MassiveRuntimeConfigError> {
        Self::load(catalog_path, std::env::var_os(MASSIVE_API_KEY_ENV))
    }

    fn load(
        catalog_path: impl AsRef<Path>,
        credential: Option<OsString>,
    ) -> Result<Self, MassiveRuntimeConfigError> {
        // Read and validate non-secret authority before accepting a credential.
        // This keeps missing/invalid catalog state explicit and reviewable.
        let catalog_path = catalog_path.as_ref().to_path_buf();
        let bytes = std::fs::read(&catalog_path).map_err(|source| {
            MassiveRuntimeConfigError::CatalogIo {
                path: catalog_path.clone(),
                source,
            }
        })?;
        let catalog = MassiveCatalogManifestV1::from_json(&bytes)?;
        catalog.canonical_catalog()?;
        let api_key = MassiveApiKey::from_os(credential)?;
        Ok(Self {
            catalog_path,
            catalog: Arc::new(catalog),
            api_key,
            rest_root: MASSIVE_REST_ROOT,
        })
    }

    pub fn source_contract(&self) -> SourceContract {
        SourceContract {
            source_id: self.catalog.manifest().source_id,
            name: "Massive Indices".into(),
            capabilities: SourceCapabilities(
                SourceCapabilities::INDEX_VALUES
                    | SourceCapabilities::REFERENCE_AGGREGATES
                    | SourceCapabilities::INSTRUMENT_DISCOVERY,
            ),
            license: self.catalog.manifest().license,
            contract_version: self.catalog.manifest().catalog_version.get(),
        }
    }
}

#[derive(Debug, Error)]
pub enum MassiveRuntimeConfigError {
    #[error("Massive credential is missing; set {MASSIVE_API_KEY_ENV}")]
    CredentialMissing,
    #[error("Massive credential is invalid")]
    CredentialInvalid,
    #[error("Massive catalog cannot be read at {path:?}: {source}")]
    CatalogIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Catalog(#[from] MassiveCatalogError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_is_always_redacted() {
        let secret = "sanitized-secret-value";
        let key = MassiveApiKey::from_os(Some(secret.into())).unwrap();
        let debug = format!("{key:?}");
        assert_eq!(debug, "MassiveApiKey(<redacted>)");
        assert!(!debug.contains(secret));
        assert_eq!(key.authorization_value(), format!("Bearer {secret}"));
    }

    #[test]
    fn missing_secret_is_a_typed_unavailable_state() {
        assert!(matches!(
            MassiveApiKey::from_os(None),
            Err(MassiveRuntimeConfigError::CredentialMissing)
        ));
    }
}

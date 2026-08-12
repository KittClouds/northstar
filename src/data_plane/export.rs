use crate::data_plane::source::{ExportScope, SourceContract};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatasetSegment {
    pub relative_path: String,
    pub layer: String,
    pub bytes: u64,
    pub blake3: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub format: String,
    pub schema_version: u16,
    pub catalog_version: u32,
    pub calendar_version: u32,
    pub derivation_version: u32,
    pub replay_quality: String,
    pub start_effective_ns: i64,
    pub end_effective_ns: i64,
    pub source_contract_versions: Vec<(u16, u32)>,
    pub segments: Vec<DatasetSegment>,
}

impl DatasetManifest {
    pub fn validate_sources(
        &self,
        sources: &[SourceContract],
        scope: ExportScope,
    ) -> Result<(), ExportError> {
        for source in sources {
            if !source.license.permits_export(scope) {
                return Err(ExportError::ForbiddenSource {
                    source_id: source.source_id.get(),
                    name: source.name.clone(),
                    scope,
                });
            }
        }
        if self.start_effective_ns > self.end_effective_ns {
            return Err(ExportError::InvalidTimeRange);
        }
        Ok(())
    }

    pub fn write_atomic(&self, destination: impl AsRef<Path>) -> Result<PathBuf, ExportError> {
        let destination = destination.as_ref();
        let parent = destination.parent().ok_or(ExportError::MissingParent)?;
        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(ExportError::InvalidFileName)?;
        let temporary = parent.join(format!(".{file_name}.northstar-tmp"));
        let bytes = serde_json::to_vec_pretty(self).map_err(ExportError::Serialize)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(ExportError::Io)?;
        file.write_all(&bytes).map_err(ExportError::Io)?;
        file.flush().map_err(ExportError::Io)?;
        file.sync_all().map_err(ExportError::Io)?;
        drop(file);
        std::fs::rename(&temporary, destination).map_err(ExportError::Io)?;
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(destination.to_path_buf())
    }
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("source {source_id} ({name}) forbids {scope:?} export")]
    ForbiddenSource {
        source_id: u16,
        name: String,
        scope: ExportScope,
    },
    #[error("dataset effective-time range is inverted")]
    InvalidTimeRange,
    #[error("dataset manifest destination has no parent")]
    MissingParent,
    #[error("dataset manifest file name is not valid UTF-8")]
    InvalidFileName,
    #[error("serialize dataset manifest: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("dataset manifest I/O: {0}")]
    Io(#[source] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::SourceId;
    use crate::data_plane::source::{LicenseClass, SourceCapabilities};

    fn manifest() -> DatasetManifest {
        DatasetManifest {
            format: "northstar-dataset-v1".into(),
            schema_version: 1,
            catalog_version: 1,
            calendar_version: 1,
            derivation_version: 1,
            replay_quality: "observed-live".into(),
            start_effective_ns: 10,
            end_effective_ns: 20,
            source_contract_versions: vec![(1, 1)],
            segments: Vec::new(),
        }
    }

    #[test]
    fn display_only_sources_block_internal_export() {
        let source = SourceContract {
            source_id: SourceId(1),
            name: "display-only fixture".into(),
            capabilities: SourceCapabilities(SourceCapabilities::INDEX_VALUES),
            license: LicenseClass::PersonalDisplayOnly,
            contract_version: 1,
        };
        assert!(matches!(
            manifest().validate_sources(&[source], ExportScope::InternalResearch),
            Err(ExportError::ForbiddenSource { .. })
        ));
    }

    #[test]
    fn manifest_publication_is_atomic_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let expected = manifest();
        expected.write_atomic(&path).unwrap();
        let decoded: DatasetManifest =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(decoded, expected);
    }
}

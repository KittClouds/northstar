use crate::Root;
use memmap2::MmapOptions;
use std::fs::File;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArtifactIoError {
    #[error("artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("empty artifact cannot be memory mapped")]
    Empty,
}

pub fn mmap_hash(path: &Path) -> Result<Root, ArtifactIoError> {
    let file = File::open(path)?;
    if file.metadata()?.len() == 0 {
        return Err(ArtifactIoError::Empty);
    }
    // SAFETY: the mapping is read-only and cannot outlive this function.
    let mapping = unsafe { MmapOptions::new().map(&file)? };
    Ok(Root::hash(&mapping))
}

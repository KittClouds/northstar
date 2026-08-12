use crate::contracts::Bar;
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("open replay: {0}")]
    Open(#[source] std::io::Error),
    #[error("map replay: {0}")]
    Map(#[source] std::io::Error),
    #[error("replay bytes are not aligned Bar records")]
    Layout,
}

pub struct MappedBarReplay {
    mmap: Mmap,
}

impl MappedBarReplay {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReplayError> {
        let file = File::open(path).map_err(ReplayError::Open)?;
        // SAFETY: the map is read-only and owns the mapping for the lifetime of
        // every slice returned by `bars`.
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(ReplayError::Map)?;
        bytemuck::try_cast_slice::<u8, Bar>(&mmap).map_err(|_| ReplayError::Layout)?;
        Ok(Self { mmap })
    }

    #[inline]
    pub fn bars(&self) -> Result<&[Bar], ReplayError> {
        bytemuck::try_cast_slice(&self.mmap).map_err(|_| ReplayError::Layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn opens_bar_records_without_copying() {
        let bars = [Bar {
            timestamp_ns: 42,
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 7.0,
            flags: 0,
        }];
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytemuck::cast_slice(&bars)).unwrap();
        file.flush().unwrap();
        let replay = MappedBarReplay::open(file.path()).unwrap();
        assert_eq!(replay.bars().unwrap()[0].timestamp_ns, 42);
    }
}

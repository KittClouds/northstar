use crate::{DATASETS, RawCorpus, RawError};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

const MAGIC: [u8; 8] = *b"NSRG3MO1";
const HEADER_BYTES: usize = 64;
const ENTRY_BYTES: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Header {
    magic: [u8; 8],
    version: u32,
    sections: u32,
    table_offset: u64,
    data_offset: u64,
    file_len: u64,
    reserved: [u8; 24],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Entry {
    name_hash: u64,
    offset: u64,
    length: u64,
    rows: u64,
}

fn name_hash(name: &str) -> u64 {
    name.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackReceipt {
    pub contract: String,
    pub raw_corpus_sha256: String,
    pub packed_sha256: String,
    pub bytes: u64,
    pub sections: usize,
}

pub fn pack_raw_corpus(
    corpus: &RawCorpus,
    output: impl AsRef<Path>,
) -> Result<PackReceipt, RawError> {
    let output = output.as_ref();
    let table_offset = HEADER_BYTES as u64;
    let data_offset = (HEADER_BYTES + ENTRY_BYTES * DATASETS.len()) as u64;
    let total_data: usize = DATASETS
        .iter()
        .map(|name| corpus.raw_dataset(name).len())
        .sum();
    let file_len = data_offset + total_data as u64;
    let header = Header {
        magic: MAGIC,
        version: 1,
        sections: DATASETS.len() as u32,
        table_offset,
        data_offset,
        file_len,
        reserved: [0; 24],
    };
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output)
        .map_err(|source| RawError::Io {
            path: output.into(),
            source,
        })?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);
    writer
        .write_all(header.as_bytes())
        .map_err(|source| RawError::Io {
            path: output.into(),
            source,
        })?;
    let mut offset = data_offset;
    for name in DATASETS {
        let bytes = corpus.raw_dataset(name);
        let entry = Entry {
            name_hash: name_hash(name),
            offset,
            length: bytes.len() as u64,
            rows: corpus.report().rows[name] as u64,
        };
        writer
            .write_all(entry.as_bytes())
            .map_err(|source| RawError::Io {
                path: output.into(),
                source,
            })?;
        offset += bytes.len() as u64;
    }
    for name in DATASETS {
        writer
            .write_all(corpus.raw_dataset(name))
            .map_err(|source| RawError::Io {
                path: output.into(),
                source,
            })?;
    }
    writer.flush().map_err(|source| RawError::Io {
        path: output.into(),
        source,
    })?;
    drop(writer);
    let bytes = std::fs::read(output).map_err(|source| RawError::Io {
        path: output.into(),
        source,
    })?;
    Ok(PackReceipt {
        contract: "NORTHSTAR_RG3_LOSSLESS_PACK_V1".into(),
        raw_corpus_sha256: corpus.report().canonical_sha256.clone(),
        packed_sha256: hex(Sha256::digest(&bytes)),
        bytes: bytes.len() as u64,
        sections: DATASETS.len(),
    })
}

pub struct PackedCorpus {
    path: PathBuf,
    mmap: Mmap,
    entries: [Entry; 9],
}
impl PackedCorpus {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RawError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).map_err(|source| RawError::Io {
            path: path.clone(),
            source,
        })?;
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| RawError::Io {
            path: path.clone(),
            source,
        })?;
        if mmap.len() < HEADER_BYTES + ENTRY_BYTES * DATASETS.len() {
            return Err(RawError::Invariant("packed artifact truncated".into()));
        }
        let header = Header::read_from_prefix(&mmap[..HEADER_BYTES])
            .map_err(|_| RawError::Invariant("bad packed header".into()))?
            .0;
        if header.magic != MAGIC || header.version != 1 || header.file_len as usize != mmap.len() {
            return Err(RawError::Invariant(
                "packed header identity mismatch".into(),
            ));
        }
        let mut entries = [Entry {
            name_hash: 0,
            offset: 0,
            length: 0,
            rows: 0,
        }; 9];
        for (i, slot) in entries.iter_mut().enumerate() {
            let start = HEADER_BYTES + i * ENTRY_BYTES;
            *slot = Entry::read_from_prefix(&mmap[start..start + ENTRY_BYTES])
                .map_err(|_| RawError::Invariant("bad packed entry".into()))?
                .0;
            if slot.name_hash != name_hash(DATASETS[i])
                || slot.offset + slot.length > header.file_len
            {
                return Err(RawError::Invariant("packed section bounds mismatch".into()));
            }
        }
        Ok(Self {
            path,
            mmap,
            entries,
        })
    }
    pub fn section(&self, name: &str) -> Option<&[u8]> {
        let index = DATASETS.iter().position(|candidate| *candidate == name)?;
        let entry = self.entries[index];
        Some(&self.mmap[entry.offset as usize..(entry.offset + entry.length) as usize])
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::write_fixture;
    #[test]
    fn lossless_pack_round_trips_every_relation() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(dir.path(), "fixture");
        let corpus = RawCorpus::open(dir.path(), "fixture").unwrap();
        let output = dir.path().join("fixture.nsmor");
        let receipt = pack_raw_corpus(&corpus, &output).unwrap();
        let packed = PackedCorpus::open(&output).unwrap();
        for name in DATASETS {
            assert_eq!(packed.section(name).unwrap(), corpus.raw_dataset(name));
        }
        assert_eq!(receipt.sections, 9);
    }
}

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use memchr::memchr_iter;
use memmap2::{Mmap, MmapOptions};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    format::{
        DIRECTORY_ENTRY_SIZE, ENDIAN_MARKER, HEADER_SIZE, MAGIC, PACKED_FORMAT_VERSION, RELATIONS,
        RelationKind, read_u16, read_u32, read_u64,
    },
    pack::hex,
};

#[derive(Debug, Error)]
pub enum PackedError {
    #[error("I/O failure at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("packed corpus contract failed: {0}")]
    Contract(String),
    #[error("source corpus failed verification: {0}")]
    Corpus(#[from] northstar_mt5_corpus::CorpusError),
    #[error("JSON failure: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<std::io::Error> for PackedError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::new(),
            source,
        }
    }
}

#[derive(Clone, Copy)]
struct Entry {
    kind: RelationKind,
    header_offset: usize,
    header_len: usize,
    data_offset: usize,
    data_len: usize,
    index_offset: usize,
    index_len: usize,
    row_count: usize,
    sha256: [u8; 32],
}

pub struct PackedCorpus {
    path: PathBuf,
    mmap: Mmap,
    source_sha: [u8; 32],
    semantic_sha: [u8; 32],
    metadata_sha: [u8; 32],
    metadata_offset: usize,
    metadata_len: usize,
    entries: [Entry; 7],
}

impl PackedCorpus {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PackedError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).map_err(|source| PackedError::Io {
            path: path.clone(),
            source,
        })?;
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| PackedError::Io {
            path: path.clone(),
            source,
        })?;
        if mmap.len() < HEADER_SIZE || mmap[..16] != MAGIC {
            return Err(PackedError::Contract("magic or header truncated".into()));
        }
        if read_u16(&mmap, 16) != Some(PACKED_FORMAT_VERSION)
            || read_u16(&mmap, 18) != Some(HEADER_SIZE as u16)
            || read_u32(&mmap, 20) != Some(ENDIAN_MARKER)
            || read_u16(&mmap, 24) != Some(RELATIONS.len() as u16)
            || read_u64(&mmap, 32) != Some(mmap.len() as u64)
        {
            return Err(PackedError::Contract("unsupported header identity".into()));
        }
        let source_sha = mmap[40..72].try_into().unwrap();
        let semantic_sha = mmap[72..104].try_into().unwrap();
        let metadata_sha = mmap[136..168].try_into().unwrap();
        let metadata_offset = usize::try_from(
            read_u64(&mmap, 232)
                .ok_or_else(|| PackedError::Contract("metadata offset missing".into()))?,
        )
        .map_err(|_| PackedError::Contract("metadata offset overflow".into()))?;
        let metadata_len = usize::try_from(
            read_u64(&mmap, 240)
                .ok_or_else(|| PackedError::Contract("metadata length missing".into()))?,
        )
        .map_err(|_| PackedError::Contract("metadata length overflow".into()))?;
        validate_range(mmap.len(), metadata_offset, metadata_len)?;
        let entries_vec = (0..RELATIONS.len())
            .map(|index| parse_entry(&mmap, HEADER_SIZE + index * DIRECTORY_ENTRY_SIZE))
            .collect::<Result<Vec<_>, _>>()?;
        let entries: [Entry; 7] = entries_vec
            .try_into()
            .map_err(|_| PackedError::Contract("directory count".into()))?;
        for (index, entry) in entries.iter().enumerate() {
            if entry.kind != RELATIONS[index] {
                return Err(PackedError::Contract("directory order mismatch".into()));
            }
            validate_range(mmap.len(), entry.header_offset, entry.header_len)?;
            validate_range(mmap.len(), entry.data_offset, entry.data_len)?;
            validate_range(mmap.len(), entry.index_offset, entry.index_len)?;
            if entry.index_len != (entry.row_count + 1) * 8 {
                return Err(PackedError::Contract("invalid row index length".into()));
            }
            if read_u64(&mmap, 168 + index * 8) != Some(entry.row_count as u64) {
                return Err(PackedError::Contract(
                    "header and directory row counts differ".into(),
                ));
            }
        }
        Ok(Self {
            path,
            mmap,
            source_sha,
            semantic_sha,
            metadata_sha,
            metadata_offset,
            metadata_len,
            entries,
        })
    }

    pub fn source_corpus_sha256(&self) -> String {
        hex(&self.source_sha)
    }
    pub fn packed_semantic_sha256(&self) -> String {
        hex(&self.semantic_sha)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn section(&self, kind: RelationKind) -> PackedSection<'_> {
        let entry = &self.entries[kind as usize - 1];
        PackedSection {
            corpus: self,
            entry,
        }
    }

    pub fn verify_all_sections(&self) -> Result<(), PackedError> {
        let metadata = slice(&self.mmap, self.metadata_offset, self.metadata_len);
        let actual_metadata: [u8; 32] = Sha256::digest(metadata).into();
        if actual_metadata != self.metadata_sha {
            return Err(PackedError::Contract("metadata hash mismatch".into()));
        }
        let mut semantic = Sha256::new();
        semantic.update(b"RG2_AUCTION_RESEARCH_PACKED_SEMANTIC_V1\0");
        for kind in RELATIONS {
            let section = self.section(kind);
            section.verify()?;
            semantic.update(kind.name().as_bytes());
            semantic.update([0]);
            semantic.update(section.header());
            semantic.update(slice(
                &self.mmap,
                section.entry.data_offset,
                section.entry.data_len,
            ));
        }
        let actual_semantic: [u8; 32] = semantic.finalize().into();
        if actual_semantic != self.semantic_sha {
            return Err(PackedError::Contract(
                "packed semantic hash mismatch".into(),
            ));
        }
        Ok(())
    }
}

pub struct PackedSection<'a> {
    corpus: &'a PackedCorpus,
    entry: &'a Entry,
}

impl<'a> PackedSection<'a> {
    pub fn kind(&self) -> RelationKind {
        self.entry.kind
    }
    pub fn header(&self) -> &'a [u8] {
        slice(
            &self.corpus.mmap,
            self.entry.header_offset,
            self.entry.header_len,
        )
    }
    pub fn len(&self) -> usize {
        self.entry.row_count
    }
    pub fn is_empty(&self) -> bool {
        self.entry.row_count == 0
    }

    pub fn row(&self, index: usize) -> Option<PackedRow<'a>> {
        if index >= self.entry.row_count {
            return None;
        }
        let start = self.offset(index)?;
        let end = self.offset(index + 1)?;
        let mut bytes = slice(
            &self.corpus.mmap,
            self.entry.data_offset + start,
            end - start,
        );
        bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        Some(PackedRow { bytes })
    }

    pub fn rows(&self) -> impl Iterator<Item = PackedRow<'a>> + '_ {
        (0..self.len()).map(|index| self.row(index).expect("validated packed index"))
    }

    pub fn column(&self, name: &str) -> Option<usize> {
        self.header()
            .split(|byte| *byte == b'\t')
            .position(|field| field == name.as_bytes())
    }

    pub fn verify(&self) -> Result<(), PackedError> {
        if self.offset(0) != Some(0) || self.offset(self.len()) != Some(self.entry.data_len) {
            return Err(PackedError::Contract(format!(
                "{} index bounds mismatch",
                self.kind().name()
            )));
        }
        let columns = memchr_iter(b'\t', self.header()).count() + 1;
        let mut previous = 0;
        let mut hash = Sha256::new();
        hash.update(self.kind().name().as_bytes());
        hash.update([0]);
        hash.update(self.header());
        for index in 0..self.len() {
            let start = self.offset(index).unwrap();
            let end = self.offset(index + 1).unwrap();
            if start != previous || end <= start || end > self.entry.data_len {
                return Err(PackedError::Contract(format!(
                    "{} nonmonotonic index",
                    self.kind().name()
                )));
            }
            let row = slice(
                &self.corpus.mmap,
                self.entry.data_offset + start,
                end - start,
            );
            if memchr_iter(b'\t', row.strip_suffix(b"\n").unwrap_or(row)).count() + 1 != columns {
                return Err(PackedError::Contract(format!(
                    "{} row width mismatch",
                    self.kind().name()
                )));
            }
            hash.update(row);
            previous = end;
        }
        let actual: [u8; 32] = hash.finalize().into();
        if actual != self.entry.sha256 {
            return Err(PackedError::Contract(format!(
                "{} section hash mismatch",
                self.kind().name()
            )));
        }
        Ok(())
    }

    fn offset(&self, index: usize) -> Option<usize> {
        let offset = self.entry.index_offset.checked_add(index.checked_mul(8)?)?;
        usize::try_from(read_u64(&self.corpus.mmap, offset)?).ok()
    }
}

#[derive(Clone, Copy)]
pub struct PackedRow<'a> {
    bytes: &'a [u8],
}

impl<'a> PackedRow<'a> {
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn field(self, index: usize) -> Option<&'a [u8]> {
        self.bytes.split(|byte| *byte == b'\t').nth(index)
    }
}

fn parse_entry(bytes: &[u8], offset: usize) -> Result<Entry, PackedError> {
    let kind = RelationKind::from_code(
        read_u16(bytes, offset)
            .ok_or_else(|| PackedError::Contract("directory truncated".into()))?,
    )
    .ok_or_else(|| PackedError::Contract("unknown relation kind".into()))?;
    let to_usize = |at| {
        read_u64(bytes, offset + at)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| PackedError::Contract("directory offset overflow".into()))
    };
    Ok(Entry {
        kind,
        header_offset: to_usize(8)?,
        header_len: to_usize(16)?,
        data_offset: to_usize(24)?,
        data_len: to_usize(32)?,
        index_offset: to_usize(40)?,
        index_len: to_usize(48)?,
        row_count: to_usize(56)?,
        sha256: bytes
            .get(offset + 64..offset + 96)
            .ok_or_else(|| PackedError::Contract("directory hash truncated".into()))?
            .try_into()
            .unwrap(),
    })
}

fn validate_range(total: usize, offset: usize, len: usize) -> Result<(), PackedError> {
    if offset.checked_add(len).is_none_or(|end| end > total) {
        return Err(PackedError::Contract("section escapes artifact".into()));
    }
    Ok(())
}

fn slice(bytes: &[u8], offset: usize, len: usize) -> &[u8] {
    &bytes[offset..offset + len]
}

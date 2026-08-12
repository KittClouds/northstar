use crate::data_plane::ids::{ReceiptId, SourceId, StreamId};
use crate::data_plane::source::LicenseClass;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashSet;
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const FILE_MAGIC: [u8; 8] = *b"NSL0RAW1";
const FRAME_MAGIC: [u8; 8] = *b"NSRECV01";
const COMMIT_MAGIC: [u8; 8] = *b"NSRCMIT1";
const FORMAT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct ReceiptFileHeader {
    magic: [u8; 8],
    version: u16,
    header_size: u16,
    flags: u32,
    created_ns: i64,
    schema_hash: [u8; 32],
    reserved: [u32; 2],
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct ReceiptFrameHeader {
    magic: [u8; 8],
    version: u16,
    header_size: u16,
    flags: u32,
    receipt_id: u64,
    source_id: u16,
    stream_id: u16,
    status_code: u16,
    content_type: u16,
    ts_started_ns: i64,
    ts_received_ns: i64,
    metadata_len: u32,
    payload_len: u32,
    metadata_crc32: u32,
    payload_crc32: u32,
    payload_hash: [u8; 32],
    metadata_hash: [u8; 32],
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct ReceiptCommit {
    magic: [u8; 8],
    receipt_id: u64,
}

#[derive(Clone, Debug)]
pub struct RawReceipt {
    pub id: ReceiptId,
    pub source: SourceId,
    pub stream: StreamId,
    pub status_code: u16,
    pub content_type: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub license: LicenseClass,
    /// Sanitized request/endpoint metadata. Credentials and authorization
    /// headers are forbidden here.
    pub metadata: Vec<u8>,
    /// Exact provider bytes.
    pub payload: Vec<u8>,
}

impl RawReceipt {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if !self.license.permits_canonical_retention() {
            return Err(ReceiptError::RetentionForbidden(self.license));
        }
        if self.ts_received_ns < self.ts_started_ns {
            return Err(ReceiptError::InvalidTimeRange);
        }
        u32::try_from(self.metadata.len()).map_err(|_| ReceiptError::FrameTooLarge)?;
        u32::try_from(self.payload.len()).map_err(|_| ReceiptError::FrameTooLarge)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RawReceiptRef<'a> {
    pub id: ReceiptId,
    pub source: SourceId,
    pub stream: StreamId,
    pub status_code: u16,
    pub content_type: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub metadata: &'a [u8],
    pub payload: &'a [u8],
    pub payload_hash: [u8; 32],
}

pub struct ReceiptStoreWriter {
    path: PathBuf,
    file: File,
    seen: HashSet<u64>,
}

impl ReceiptStoreWriter {
    pub fn create(path: impl AsRef<Path>, created_ns: i64) -> Result<Self, ReceiptError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(ReceiptError::Io)?;
        let schema_hash = *blake3::hash(b"northstar-l0-receipt-v1").as_bytes();
        let header = ReceiptFileHeader {
            magic: FILE_MAGIC,
            version: FORMAT_VERSION,
            header_size: std::mem::size_of::<ReceiptFileHeader>() as u16,
            flags: 0,
            created_ns,
            schema_hash,
            reserved: [0; 2],
        };
        file.write_all(bytemuck::bytes_of(&header))
            .map_err(ReceiptError::Io)?;
        file.sync_data().map_err(ReceiptError::Io)?;
        Ok(Self {
            path,
            file,
            seen: HashSet::new(),
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReceiptError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(ReceiptError::Io)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(ReceiptError::Io)?;
        let scan = scan(&bytes, true)?;
        if scan.incomplete_tail {
            file.set_len(scan.valid_len as u64)
                .map_err(ReceiptError::Io)?;
            file.sync_data().map_err(ReceiptError::Io)?;
        }
        file.seek(SeekFrom::End(0)).map_err(ReceiptError::Io)?;
        Ok(Self {
            path,
            file,
            seen: scan.receipt_ids,
        })
    }

    pub fn append(&mut self, receipt: &RawReceipt) -> Result<[u8; 32], ReceiptError> {
        receipt.validate()?;
        if self.seen.contains(&receipt.id.get()) {
            return Err(ReceiptError::DuplicateReceipt(receipt.id));
        }
        let payload_hash = *blake3::hash(&receipt.payload).as_bytes();
        let metadata_hash = *blake3::hash(&receipt.metadata).as_bytes();
        let header = ReceiptFrameHeader {
            magic: FRAME_MAGIC,
            version: FORMAT_VERSION,
            header_size: std::mem::size_of::<ReceiptFrameHeader>() as u16,
            flags: license_bits(receipt.license),
            receipt_id: receipt.id.get(),
            source_id: receipt.source.get(),
            stream_id: receipt.stream.get(),
            status_code: receipt.status_code,
            content_type: receipt.content_type,
            ts_started_ns: receipt.ts_started_ns,
            ts_received_ns: receipt.ts_received_ns,
            metadata_len: receipt.metadata.len() as u32,
            payload_len: receipt.payload.len() as u32,
            metadata_crc32: crc32fast::hash(&receipt.metadata),
            payload_crc32: crc32fast::hash(&receipt.payload),
            payload_hash,
            metadata_hash,
        };
        let commit = ReceiptCommit {
            magic: COMMIT_MAGIC,
            receipt_id: receipt.id.get(),
        };
        self.file
            .write_all(bytemuck::bytes_of(&header))
            .and_then(|_| self.file.write_all(&receipt.metadata))
            .and_then(|_| self.file.write_all(&receipt.payload))
            .and_then(|_| self.file.write_all(bytemuck::bytes_of(&commit)))
            .map_err(ReceiptError::Io)?;
        self.file.flush().map_err(ReceiptError::Io)?;
        self.file.sync_data().map_err(ReceiptError::Io)?;
        self.seen.insert(receipt.id.get());
        Ok(payload_hash)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn next_receipt_id(&self) -> Result<ReceiptId, ReceiptError> {
        let next = self
            .seen
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ReceiptError::ReceiptIdExhausted)?;
        Ok(ReceiptId(next))
    }
}

pub struct MappedReceiptStore {
    mmap: Mmap,
}

impl MappedReceiptStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReceiptError> {
        let file = File::open(path).map_err(ReceiptError::Io)?;
        // SAFETY: the mapping is read-only and remains owned by this value.
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(ReceiptError::Io)?;
        scan(&mmap, false)?;
        Ok(Self { mmap })
    }

    pub fn receipts(&self) -> ReceiptIter<'_> {
        ReceiptIter {
            bytes: &self.mmap,
            offset: std::mem::size_of::<ReceiptFileHeader>(),
        }
    }
}

pub struct ReceiptIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for ReceiptIter<'a> {
    type Item = RawReceiptRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.bytes.len() {
            return None;
        }
        let header = read_pod::<ReceiptFrameHeader>(self.bytes, self.offset).ok()?;
        let metadata_start = self.offset + std::mem::size_of::<ReceiptFrameHeader>();
        let payload_start = metadata_start + header.metadata_len as usize;
        let payload_end = payload_start + header.payload_len as usize;
        let result = RawReceiptRef {
            id: ReceiptId(header.receipt_id),
            source: SourceId(header.source_id),
            stream: StreamId(header.stream_id),
            status_code: header.status_code,
            content_type: header.content_type,
            ts_started_ns: header.ts_started_ns,
            ts_received_ns: header.ts_received_ns,
            metadata: &self.bytes[metadata_start..payload_start],
            payload: &self.bytes[payload_start..payload_end],
            payload_hash: header.payload_hash,
        };
        self.offset = payload_end + std::mem::size_of::<ReceiptCommit>();
        Some(result)
    }
}

struct ScanResult {
    valid_len: usize,
    incomplete_tail: bool,
    receipt_ids: HashSet<u64>,
}

fn scan(bytes: &[u8], allow_incomplete_tail: bool) -> Result<ScanResult, ReceiptError> {
    let file_size = std::mem::size_of::<ReceiptFileHeader>();
    if bytes.len() < file_size {
        return Err(ReceiptError::TruncatedHeader);
    }
    let file_header = read_pod::<ReceiptFileHeader>(bytes, 0)?;
    if file_header.magic != FILE_MAGIC
        || file_header.version != FORMAT_VERSION
        || file_header.header_size as usize != file_size
    {
        return Err(ReceiptError::BadHeader);
    }

    let mut offset = file_size;
    let mut receipt_ids = HashSet::new();
    while offset < bytes.len() {
        let frame_size = std::mem::size_of::<ReceiptFrameHeader>();
        if bytes.len() - offset < frame_size {
            return incomplete_or_error(offset, allow_incomplete_tail, receipt_ids);
        }
        let header = read_pod::<ReceiptFrameHeader>(bytes, offset)?;
        if header.magic != FRAME_MAGIC
            || header.version != FORMAT_VERSION
            || header.header_size as usize != frame_size
        {
            return Err(ReceiptError::CorruptFrame(offset));
        }
        let body_len = header.metadata_len as usize + header.payload_len as usize;
        let commit_size = std::mem::size_of::<ReceiptCommit>();
        let Some(frame_end) = offset
            .checked_add(frame_size)
            .and_then(|value| value.checked_add(body_len))
            .and_then(|value| value.checked_add(commit_size))
        else {
            return Err(ReceiptError::CorruptFrame(offset));
        };
        if frame_end > bytes.len() {
            return incomplete_or_error(offset, allow_incomplete_tail, receipt_ids);
        }
        let metadata_start = offset + frame_size;
        let payload_start = metadata_start + header.metadata_len as usize;
        let payload_end = payload_start + header.payload_len as usize;
        let metadata = &bytes[metadata_start..payload_start];
        let payload = &bytes[payload_start..payload_end];
        let commit = read_pod::<ReceiptCommit>(bytes, payload_end)?;
        if commit.magic != COMMIT_MAGIC || commit.receipt_id != header.receipt_id {
            return Err(ReceiptError::CorruptFrame(offset));
        }
        if !receipt_ids.insert(header.receipt_id) {
            return Err(ReceiptError::DuplicateReceipt(ReceiptId(header.receipt_id)));
        }
        if crc32fast::hash(metadata) != header.metadata_crc32
            || crc32fast::hash(payload) != header.payload_crc32
            || blake3::hash(metadata).as_bytes() != &header.metadata_hash
            || blake3::hash(payload).as_bytes() != &header.payload_hash
        {
            return Err(ReceiptError::Checksum(offset));
        }
        offset = frame_end;
    }
    Ok(ScanResult {
        valid_len: offset,
        incomplete_tail: false,
        receipt_ids,
    })
}

fn incomplete_or_error(
    valid_len: usize,
    allow: bool,
    receipt_ids: HashSet<u64>,
) -> Result<ScanResult, ReceiptError> {
    if allow {
        Ok(ScanResult {
            valid_len,
            incomplete_tail: true,
            receipt_ids,
        })
    } else {
        Err(ReceiptError::IncompleteTail(valid_len))
    }
}

fn read_pod<T: Pod + Copy>(bytes: &[u8], offset: usize) -> Result<T, ReceiptError> {
    let size = std::mem::size_of::<T>();
    let end = offset
        .checked_add(size)
        .ok_or(ReceiptError::FrameTooLarge)?;
    let slice = bytes
        .get(offset..end)
        .ok_or(ReceiptError::IncompleteTail(offset))?;
    Ok(bytemuck::pod_read_unaligned(slice))
}

const fn license_bits(license: LicenseClass) -> u32 {
    match license {
        LicenseClass::OpenWithAttribution => 1,
        LicenseClass::PersonalDisplayOnly => 2,
        LicenseClass::LicensedNonDisplay => 3,
        LicenseClass::InternalRestricted => 4,
    }
}

#[derive(Debug, Error)]
pub enum ReceiptError {
    #[error("receipt I/O: {0}")]
    Io(#[source] std::io::Error),
    #[error("raw retention is forbidden for {0:?}")]
    RetentionForbidden(LicenseClass),
    #[error("receipt completion predates request start")]
    InvalidTimeRange,
    #[error("duplicate raw receipt id {0:?}")]
    DuplicateReceipt(ReceiptId),
    #[error("raw receipt id space is exhausted")]
    ReceiptIdExhausted,
    #[error("receipt frame exceeds format limits")]
    FrameTooLarge,
    #[error("receipt file header is truncated")]
    TruncatedHeader,
    #[error("invalid receipt file header")]
    BadHeader,
    #[error("incomplete receipt tail at byte {0}")]
    IncompleteTail(usize),
    #[error("corrupt receipt frame at byte {0}")]
    CorruptFrame(usize),
    #[error("receipt checksum mismatch at byte {0}")]
    Checksum(usize),
}

const _: () = assert!(std::mem::size_of::<ReceiptFileHeader>() == 64);
const _: () = assert!(std::mem::size_of::<ReceiptFrameHeader>() == 128);
const _: () = assert!(std::mem::size_of::<ReceiptCommit>() == 16);

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(id: u64) -> RawReceipt {
        RawReceipt {
            id: ReceiptId(id),
            source: SourceId(7),
            stream: StreamId(3),
            status_code: 200,
            content_type: 1,
            ts_started_ns: 10,
            ts_received_ns: 11,
            license: LicenseClass::OpenWithAttribution,
            metadata: br#"{"endpoint":"fixture"}"#.to_vec(),
            payload: format!("payload-{id}").into_bytes(),
        }
    }

    #[test]
    fn exact_receipts_round_trip_through_mmap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipts.nsl0");
        let mut writer = ReceiptStoreWriter::create(&path, 1).unwrap();
        writer.append(&receipt(1)).unwrap();
        writer.append(&receipt(2)).unwrap();
        drop(writer);

        let mapped = MappedReceiptStore::open(&path).unwrap();
        let payloads: Vec<_> = mapped
            .receipts()
            .map(|item| std::str::from_utf8(item.payload).unwrap().to_owned())
            .collect();
        assert_eq!(payloads, ["payload-1", "payload-2"]);
    }

    #[test]
    fn writer_recovers_only_an_incomplete_tail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipts.nsl0");
        let mut writer = ReceiptStoreWriter::create(&path, 1).unwrap();
        writer.append(&receipt(1)).unwrap();
        let valid = std::fs::metadata(&path).unwrap().len();
        writer.file.write_all(&[1, 2, 3]).unwrap();
        drop(writer);
        let reopened = ReceiptStoreWriter::open(&path).unwrap();
        assert_eq!(std::fs::metadata(reopened.path()).unwrap().len(), valid);
    }

    #[test]
    fn duplicate_receipt_identity_fails_before_write_and_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipts.nsl0");
        let mut writer = ReceiptStoreWriter::create(&path, 1).unwrap();
        writer.append(&receipt(7)).unwrap();
        let committed_len = std::fs::metadata(&path).unwrap().len();
        assert!(matches!(
            writer.append(&receipt(7)),
            Err(ReceiptError::DuplicateReceipt(ReceiptId(7)))
        ));
        assert_eq!(std::fs::metadata(&path).unwrap().len(), committed_len);
        drop(writer);

        let mut reopened = ReceiptStoreWriter::open(&path).unwrap();
        assert!(matches!(
            reopened.append(&receipt(7)),
            Err(ReceiptError::DuplicateReceipt(ReceiptId(7)))
        ));
    }
}

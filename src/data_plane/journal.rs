use crate::data_plane::event::{CanonicalBatch, CanonicalEvent};
use crate::data_plane::ids::{BatchId, JournalSequence};
use bytemuck::{Pod, Zeroable};
use hashbrown::HashSet;
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const FILE_MAGIC: [u8; 8] = *b"NSL1JNL1";
const BATCH_MAGIC: [u8; 8] = *b"NSBATCH1";
const COMMIT_MAGIC: [u8; 8] = *b"NSJCMIT1";
const FORMAT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct JournalFileHeader {
    magic: [u8; 8],
    version: u16,
    header_size: u16,
    event_size: u32,
    created_ns: i64,
    schema_hash: [u8; 32],
    flags: u32,
    reserved: u32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct BatchFrameHeader {
    magic: [u8; 8],
    version: u16,
    header_size: u16,
    event_count: u32,
    batch_id: u64,
    first_sequence: u64,
    payload_len: u64,
    payload_hash: [u8; 32],
    flags: u32,
    reserved: u32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct BatchCommit {
    magic: [u8; 8],
    batch_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    pub batch_id: BatchId,
    pub first_sequence: JournalSequence,
    pub last_sequence: JournalSequence,
    pub event_count: u32,
    pub payload_hash: [u8; 32],
}

pub struct JournalWriter {
    path: PathBuf,
    file: File,
    next_sequence: u64,
    seen: HashSet<u128>,
}

impl JournalWriter {
    pub fn create(path: impl AsRef<Path>, created_ns: i64) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(JournalError::Io)?;
        let header = JournalFileHeader {
            magic: FILE_MAGIC,
            version: FORMAT_VERSION,
            header_size: std::mem::size_of::<JournalFileHeader>() as u16,
            event_size: std::mem::size_of::<CanonicalEvent>() as u32,
            created_ns,
            schema_hash: *blake3::hash(b"northstar-canonical-event-v1-128").as_bytes(),
            flags: 0,
            reserved: 0,
        };
        file.write_all(bytemuck::bytes_of(&header))
            .map_err(JournalError::Io)?;
        file.sync_data().map_err(JournalError::Io)?;
        Ok(Self {
            path,
            file,
            next_sequence: 1,
            seen: HashSet::new(),
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(JournalError::Io)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(JournalError::Io)?;
        let scan = scan(&bytes, true)?;
        if scan.incomplete_tail {
            file.set_len(scan.valid_len as u64)
                .map_err(JournalError::Io)?;
            file.sync_data().map_err(JournalError::Io)?;
        }
        file.seek(SeekFrom::End(0)).map_err(JournalError::Io)?;
        Ok(Self {
            path,
            file,
            next_sequence: scan.next_sequence,
            seen: scan.seen,
        })
    }

    pub fn append_batch(
        &mut self,
        batch: &mut CanonicalBatch,
    ) -> Result<CommitReceipt, JournalError> {
        if batch.is_empty() {
            return Err(JournalError::EmptyBatch);
        }
        if batch.id == BatchId::UNKNOWN {
            return Err(JournalError::MissingBatchId);
        }
        let count = u32::try_from(batch.events.len()).map_err(|_| JournalError::BatchTooLarge)?;
        let mut pending_keys = Vec::with_capacity(batch.events.len());
        for event in &batch.events {
            let key = source_event_key(event);
            if self.seen.contains(&key) || pending_keys.contains(&key) {
                return Err(JournalError::DuplicateSourceEvent {
                    source_id: event.header.source_id,
                    stream_id: event.header.stream_id,
                    source_event_id: event.header.source_event_id,
                });
            }
            pending_keys.push(key);
        }

        let first_sequence = self.next_sequence;
        for (offset, event) in batch.events.iter_mut().enumerate() {
            event.header.journal_sequence = first_sequence + offset as u64;
        }
        let payload = bytemuck::cast_slice::<CanonicalEvent, u8>(&batch.events);
        let payload_hash = *blake3::hash(payload).as_bytes();
        let header = BatchFrameHeader {
            magic: BATCH_MAGIC,
            version: FORMAT_VERSION,
            header_size: std::mem::size_of::<BatchFrameHeader>() as u16,
            event_count: count,
            batch_id: batch.id.get(),
            first_sequence,
            payload_len: payload.len() as u64,
            payload_hash,
            flags: 0,
            reserved: 0,
        };
        let commit = BatchCommit {
            magic: COMMIT_MAGIC,
            batch_id: batch.id.get(),
        };
        self.file
            .write_all(bytemuck::bytes_of(&header))
            .and_then(|_| self.file.write_all(payload))
            .and_then(|_| self.file.write_all(bytemuck::bytes_of(&commit)))
            .map_err(JournalError::Io)?;
        self.file.flush().map_err(JournalError::Io)?;
        self.file.sync_data().map_err(JournalError::Io)?;

        self.seen.extend(pending_keys);
        self.next_sequence += count as u64;
        Ok(CommitReceipt {
            batch_id: batch.id,
            first_sequence: JournalSequence(first_sequence),
            last_sequence: JournalSequence(self.next_sequence - 1),
            event_count: count,
            payload_hash,
        })
    }

    pub fn next_sequence(&self) -> JournalSequence {
        JournalSequence(self.next_sequence)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn seal(mut self, destination: impl AsRef<Path>) -> Result<PathBuf, JournalError> {
        self.file.flush().map_err(JournalError::Io)?;
        self.file.sync_all().map_err(JournalError::Io)?;
        let source = self.path.clone();
        drop(self.file);
        std::fs::rename(&source, destination.as_ref()).map_err(JournalError::Io)?;
        Ok(destination.as_ref().to_path_buf())
    }
}

pub struct MappedJournal {
    mmap: Mmap,
    batch_count: usize,
    event_count: usize,
}

impl MappedJournal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let file = File::open(path).map_err(JournalError::Io)?;
        // SAFETY: this read-only mapping is owned for every returned event view.
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(JournalError::Io)?;
        let scan = scan(&mmap, false)?;
        Ok(Self {
            mmap,
            batch_count: scan.batch_count,
            event_count: scan.event_count,
        })
    }

    pub fn batches(&self) -> JournalBatchIter<'_> {
        JournalBatchIter {
            bytes: &self.mmap,
            offset: std::mem::size_of::<JournalFileHeader>(),
        }
    }

    pub const fn batch_count(&self) -> usize {
        self.batch_count
    }

    pub const fn event_count(&self) -> usize {
        self.event_count
    }

    pub fn content_hash(&self) -> [u8; 32] {
        *blake3::hash(&self.mmap).as_bytes()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JournalBatchRef<'a> {
    pub id: BatchId,
    pub first_sequence: JournalSequence,
    pub events: &'a [CanonicalEvent],
    pub payload_hash: [u8; 32],
}

pub struct JournalBatchIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for JournalBatchIter<'a> {
    type Item = JournalBatchRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.bytes.len() {
            return None;
        }
        let header = read_pod::<BatchFrameHeader>(self.bytes, self.offset).ok()?;
        let payload_start = self.offset + std::mem::size_of::<BatchFrameHeader>();
        let payload_end = payload_start + header.payload_len as usize;
        let events = bytemuck::try_cast_slice(&self.bytes[payload_start..payload_end]).ok()?;
        self.offset = payload_end + std::mem::size_of::<BatchCommit>();
        Some(JournalBatchRef {
            id: BatchId(header.batch_id),
            first_sequence: JournalSequence(header.first_sequence),
            events,
            payload_hash: header.payload_hash,
        })
    }
}

struct ScanResult {
    valid_len: usize,
    incomplete_tail: bool,
    next_sequence: u64,
    seen: HashSet<u128>,
    batch_count: usize,
    event_count: usize,
}

fn scan(bytes: &[u8], allow_incomplete_tail: bool) -> Result<ScanResult, JournalError> {
    let file_size = std::mem::size_of::<JournalFileHeader>();
    if bytes.len() < file_size {
        return Err(JournalError::TruncatedHeader);
    }
    let file_header = read_pod::<JournalFileHeader>(bytes, 0)?;
    if file_header.magic != FILE_MAGIC
        || file_header.version != FORMAT_VERSION
        || file_header.header_size as usize != file_size
        || file_header.event_size as usize != std::mem::size_of::<CanonicalEvent>()
    {
        return Err(JournalError::BadHeader);
    }

    let mut result = ScanResult {
        valid_len: file_size,
        incomplete_tail: false,
        next_sequence: 1,
        seen: HashSet::new(),
        batch_count: 0,
        event_count: 0,
    };
    let mut offset = file_size;
    while offset < bytes.len() {
        let header_size = std::mem::size_of::<BatchFrameHeader>();
        if bytes.len() - offset < header_size {
            return incomplete(result, allow_incomplete_tail);
        }
        let header = read_pod::<BatchFrameHeader>(bytes, offset)?;
        if header.magic != BATCH_MAGIC
            || header.version != FORMAT_VERSION
            || header.header_size as usize != header_size
        {
            return Err(JournalError::CorruptBatch(offset));
        }
        let expected_len = header.event_count as usize * std::mem::size_of::<CanonicalEvent>();
        if header.payload_len as usize != expected_len
            || header.first_sequence != result.next_sequence
        {
            return Err(JournalError::CorruptBatch(offset));
        }
        let commit_size = std::mem::size_of::<BatchCommit>();
        let Some(frame_end) = offset
            .checked_add(header_size)
            .and_then(|value| value.checked_add(expected_len))
            .and_then(|value| value.checked_add(commit_size))
        else {
            return Err(JournalError::BatchTooLarge);
        };
        if frame_end > bytes.len() {
            return incomplete(result, allow_incomplete_tail);
        }
        let payload_start = offset + header_size;
        let payload_end = payload_start + expected_len;
        let payload = &bytes[payload_start..payload_end];
        let commit = read_pod::<BatchCommit>(bytes, payload_end)?;
        if commit.magic != COMMIT_MAGIC || commit.batch_id != header.batch_id {
            return Err(JournalError::CorruptBatch(offset));
        }
        if blake3::hash(payload).as_bytes() != &header.payload_hash {
            return Err(JournalError::Checksum(offset));
        }
        let events: &[CanonicalEvent] =
            bytemuck::try_cast_slice(payload).map_err(|_| JournalError::Alignment(offset))?;
        for (index, event) in events.iter().enumerate() {
            let expected = header.first_sequence + index as u64;
            if event.header.journal_sequence != expected {
                return Err(JournalError::Sequence(offset));
            }
            let key = source_event_key(event);
            if !result.seen.insert(key) {
                return Err(JournalError::DuplicateSourceEvent {
                    source_id: event.header.source_id,
                    stream_id: event.header.stream_id,
                    source_event_id: event.header.source_event_id,
                });
            }
        }
        result.next_sequence += header.event_count as u64;
        result.batch_count += 1;
        result.event_count += header.event_count as usize;
        result.valid_len = frame_end;
        offset = frame_end;
    }
    Ok(result)
}

fn incomplete(mut result: ScanResult, allow: bool) -> Result<ScanResult, JournalError> {
    if allow {
        result.incomplete_tail = true;
        Ok(result)
    } else {
        Err(JournalError::IncompleteTail(result.valid_len))
    }
}

#[inline]
fn source_event_key(event: &CanonicalEvent) -> u128 {
    ((event.header.source_id as u128) << 80)
        | ((event.header.stream_id as u128) << 64)
        | event.header.source_event_id as u128
}

fn read_pod<T: Pod + Copy>(bytes: &[u8], offset: usize) -> Result<T, JournalError> {
    let size = std::mem::size_of::<T>();
    let end = offset
        .checked_add(size)
        .ok_or(JournalError::BatchTooLarge)?;
    let slice = bytes
        .get(offset..end)
        .ok_or(JournalError::IncompleteTail(offset))?;
    Ok(bytemuck::pod_read_unaligned(slice))
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal I/O: {0}")]
    Io(#[source] std::io::Error),
    #[error("canonical batch must contain at least one event")]
    EmptyBatch,
    #[error("canonical batch identity must be non-zero")]
    MissingBatchId,
    #[error("canonical batch exceeds journal limits")]
    BatchTooLarge,
    #[error("duplicate source event {source_id}:{stream_id}:{source_event_id}")]
    DuplicateSourceEvent {
        source_id: u16,
        stream_id: u16,
        source_event_id: u64,
    },
    #[error("journal file header is truncated")]
    TruncatedHeader,
    #[error("invalid journal file header")]
    BadHeader,
    #[error("incomplete journal tail at byte {0}")]
    IncompleteTail(usize),
    #[error("corrupt journal batch at byte {0}")]
    CorruptBatch(usize),
    #[error("journal batch checksum mismatch at byte {0}")]
    Checksum(usize),
    #[error("journal event alignment failure at byte {0}")]
    Alignment(usize),
    #[error("journal sequence discontinuity at byte {0}")]
    Sequence(usize),
}

const _: () = assert!(std::mem::size_of::<JournalFileHeader>() == 64);
const _: () = assert!(std::mem::size_of::<BatchFrameHeader>() == 80);
const _: () = assert!(std::mem::size_of::<BatchCommit>() == 16);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::event::{CanonicalEvent, TimeQuality};
    use crate::data_plane::ids::{InstrumentId, ReceiptId, SourceId, StreamId};

    fn event(id: u64) -> CanonicalEvent {
        CanonicalEvent::index_value(
            SourceId(1),
            StreamId(1),
            InstrumentId(1),
            ReceiptId(1),
            id,
            id as i64,
            id as i64 + 10,
            20_000 + id as i64,
            TimeQuality::ObservedLive,
        )
        .unwrap()
    }

    #[test]
    fn journal_round_trips_dense_batches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("open.nsj");
        let mut writer = JournalWriter::create(&path, 1).unwrap();
        let mut batch = CanonicalBatch::new(BatchId(7));
        batch.push(event(1));
        batch.push(event(2));
        let committed = writer.append_batch(&mut batch).unwrap();
        assert_eq!(committed.first_sequence, JournalSequence(1));
        drop(writer);

        let journal = MappedJournal::open(&path).unwrap();
        assert_eq!(journal.batch_count(), 1);
        assert_eq!(journal.event_count(), 2);
        let replayed = journal.batches().next().unwrap();
        assert_eq!(replayed.events[1].header.journal_sequence, 2);
    }

    #[test]
    fn duplicates_fail_before_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("open.nsj");
        let mut writer = JournalWriter::create(&path, 1).unwrap();
        let mut first = CanonicalBatch::new(BatchId(1));
        first.push(event(1));
        writer.append_batch(&mut first).unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        let mut duplicate = CanonicalBatch::new(BatchId(2));
        duplicate.push(event(1));
        assert!(matches!(
            writer.append_batch(&mut duplicate),
            Err(JournalError::DuplicateSourceEvent { .. })
        ));
        assert_eq!(std::fs::metadata(&path).unwrap().len(), before);
    }

    #[test]
    fn open_recovers_an_uncommitted_tail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("open.nsj");
        let mut writer = JournalWriter::create(&path, 1).unwrap();
        let mut batch = CanonicalBatch::new(BatchId(1));
        batch.push(event(1));
        writer.append_batch(&mut batch).unwrap();
        let valid = std::fs::metadata(&path).unwrap().len();
        writer.file.write_all(&[9, 8, 7]).unwrap();
        drop(writer);
        let reopened = JournalWriter::open(&path).unwrap();
        assert_eq!(std::fs::metadata(reopened.path()).unwrap().len(), valid);
        assert_eq!(reopened.next_sequence(), JournalSequence(2));
    }
}

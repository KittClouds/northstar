use super::types::{
    ActorKind, LedgerEntryId, LedgerEventDraft, LedgerEventHeader, LedgerEventKind,
    LedgerEventStatus, SourceEventKey, TradeCaseId,
};
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const HOT_FILE_MAGIC: [u8; 8] = *b"NSLDGH01";
const HOT_FRAME_MAGIC: [u8; 8] = *b"NSLDGE01";
const HOT_COMMIT_MAGIC: [u8; 8] = *b"NSLDGC01";
const COLD_FILE_MAGIC: [u8; 8] = *b"NSLDGB01";
const COLD_FRAME_MAGIC: [u8; 8] = *b"NSLDBD01";
const COLD_COMMIT_MAGIC: [u8; 8] = *b"NSLDBC01";
const FORMAT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct StoreFileHeader {
    magic: [u8; 8],
    version: u16,
    header_size: u16,
    record_size: u32,
    created_ns: i64,
    schema_hash: [u8; 32],
    reserved: [u8; 8],
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct HotFrame {
    magic: [u8; 8],
    header: LedgerEventHeader,
    crc32: u32,
    reserved: u32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct EntryCommit {
    magic: [u8; 8],
    entry_id: u64,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct BodyFrame {
    magic: [u8; 8],
    entry_id: u64,
    body_len: u32,
    schema_version: u16,
    flags: u16,
    body_hash: [u8; 32],
    reserved: [u8; 8],
}

#[derive(Clone, Copy, Debug)]
pub struct LedgerCommit {
    pub entry_id: LedgerEntryId,
    pub case_id: TradeCaseId,
    pub body_hash: [u8; 32],
    pub header: LedgerEventHeader,
    /// False means the stable source key had already committed. Retried
    /// automatic events therefore succeed without creating a second record.
    pub inserted: bool,
}

pub struct LedgerStore {
    hot_path: PathBuf,
    cold_path: PathBuf,
    hot: File,
    cold: File,
    next_entry: u64,
    next_case: u64,
    source_entries: HashMap<u128, LedgerEntryId>,
    entry_cases: HashMap<u64, u64>,
    len: usize,
}

impl LedgerStore {
    pub fn create(
        hot_path: impl AsRef<Path>,
        cold_path: impl AsRef<Path>,
        created_ns: i64,
    ) -> Result<Self, LedgerError> {
        let hot_path = hot_path.as_ref().to_path_buf();
        let cold_path = cold_path.as_ref().to_path_buf();
        let mut hot = create_file(&hot_path)?;
        let mut cold = match create_file(&cold_path) {
            Ok(file) => file,
            Err(error) => {
                drop(hot);
                let _ = std::fs::remove_file(&hot_path);
                return Err(error);
            }
        };

        write_file_header(
            &mut hot,
            HOT_FILE_MAGIC,
            std::mem::size_of::<HotFrame>() as u32,
            created_ns,
            b"northstar-ledger-hot-v1-176",
        )?;
        write_file_header(
            &mut cold,
            COLD_FILE_MAGIC,
            std::mem::size_of::<BodyFrame>() as u32,
            created_ns,
            b"northstar-ledger-cold-v1",
        )?;
        Ok(Self {
            hot_path,
            cold_path,
            hot,
            cold,
            next_entry: 1,
            next_case: 1,
            source_entries: HashMap::new(),
            entry_cases: HashMap::new(),
            len: 0,
        })
    }

    pub fn open(
        hot_path: impl AsRef<Path>,
        cold_path: impl AsRef<Path>,
    ) -> Result<Self, LedgerError> {
        let hot_path = hot_path.as_ref().to_path_buf();
        let cold_path = cold_path.as_ref().to_path_buf();
        let mut hot = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&hot_path)
            .map_err(LedgerError::Io)?;
        let mut cold = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&cold_path)
            .map_err(LedgerError::Io)?;

        let cold_scan = map_and_scan_cold(&cold, true)?;
        if cold_scan.incomplete_tail {
            cold.set_len(cold_scan.valid_len as u64)
                .map_err(LedgerError::Io)?;
            cold.sync_data().map_err(LedgerError::Io)?;
        }
        let hot_scan = map_and_scan_hot(&hot, true)?;
        if hot_scan.incomplete_tail {
            hot.set_len(hot_scan.valid_len as u64)
                .map_err(LedgerError::Io)?;
            hot.sync_data().map_err(LedgerError::Io)?;
        }

        let cold_map = map_read(&cold)?;
        let hot_map = map_read(&hot)?;
        let verified = scan_hot(&hot_map, false)?;
        let referenced_cold_len = validate_body_references(&hot_map, &cold_map, verified.len)?;
        drop(cold_map);
        drop(hot_map);
        if cold_scan.valid_len > referenced_cold_len {
            cold.set_len(referenced_cold_len as u64)
                .map_err(LedgerError::Io)?;
            cold.sync_data().map_err(LedgerError::Io)?;
        }

        hot.seek(SeekFrom::End(0)).map_err(LedgerError::Io)?;
        cold.seek(SeekFrom::End(0)).map_err(LedgerError::Io)?;
        Ok(Self {
            hot_path,
            cold_path,
            hot,
            cold,
            next_entry: verified.next_entry,
            next_case: verified.next_case,
            source_entries: verified.source_entries,
            entry_cases: verified.entry_cases,
            len: verified.len,
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub const fn next_case_id(&self) -> TradeCaseId {
        TradeCaseId(self.next_case)
    }

    /// Reads one fixed hot record without touching its cold body. Operator
    /// amendment validation uses this to protect machine-owned evidence.
    pub fn entry_header(&self, entry_id: LedgerEntryId) -> Result<LedgerEventHeader, LedgerError> {
        header_for_entry(&self.hot, entry_id)
    }

    pub fn append(&mut self, draft: &LedgerEventDraft<'_>) -> Result<LedgerCommit, LedgerError> {
        validate_draft(draft, &self.entry_cases)?;
        if let Some(entry_id) = self.source_entries.get(&draft.source_key.0).copied() {
            let header = header_for_entry(&self.hot, entry_id)?;
            return Ok(LedgerCommit {
                entry_id,
                case_id: draft.case_id,
                body_hash: header.body_hash,
                header,
                inserted: false,
            });
        }

        let entry_id = LedgerEntryId(self.next_entry);
        let body_hash = *blake3::hash(draft.body).as_bytes();
        let body_start = self.cold.seek(SeekFrom::End(0)).map_err(LedgerError::Io)?;
        let body_offset = body_start
            .checked_add(std::mem::size_of::<BodyFrame>() as u64)
            .ok_or(LedgerError::FileTooLarge)?;
        let body_len = u32::try_from(draft.body.len()).map_err(|_| LedgerError::BodyTooLarge)?;
        let body_frame = BodyFrame {
            magic: COLD_FRAME_MAGIC,
            entry_id: entry_id.get(),
            body_len,
            schema_version: draft.schema_version.get(),
            flags: draft.flags.bits(),
            body_hash,
            reserved: [0; 8],
        };
        let body_commit = EntryCommit {
            magic: COLD_COMMIT_MAGIC,
            entry_id: entry_id.get(),
        };
        self.cold
            .write_all(bytemuck::bytes_of(&body_frame))
            .and_then(|_| self.cold.write_all(draft.body))
            .and_then(|_| self.cold.write_all(bytemuck::bytes_of(&body_commit)))
            .and_then(|_| self.cold.flush())
            .map_err(LedgerError::Io)?;
        self.cold.sync_data().map_err(LedgerError::Io)?;

        let (correlation_low, correlation_high) = draft.correlation_id.split();
        let (source_key_low, source_key_high) = draft.source_key.split();
        let (canonical_first_sequence, canonical_last_sequence) = draft
            .canonical_sequences
            .map_or((0, 0), |(first, last)| (first.get(), last.get()));
        let header = LedgerEventHeader {
            entry_id: entry_id.get(),
            case_id: draft.case_id.get(),
            parent_entry_id: draft.parent_entry_id.get(),
            correlation_low,
            correlation_high,
            source_key_low,
            source_key_high,
            actor_id: draft.actor_id.get(),
            ts_event_ns: draft.ts_event_ns,
            ts_received_ns: draft.ts_received_ns,
            ts_recorded_ns: draft.ts_recorded_ns,
            canonical_first_sequence,
            canonical_last_sequence,
            body_offset,
            account_id: draft.account_id,
            body_hash,
            body_len,
            instrument_id: draft.instrument_id.get(),
            schema_version: draft.schema_version.get(),
            kind: draft.kind as u16,
            flags: draft.flags.bits(),
            status: draft.status as u8,
            actor_kind: draft.actor_kind as u8,
            reserved: [0; 8],
        };
        let frame = HotFrame {
            magic: HOT_FRAME_MAGIC,
            header,
            crc32: crc32fast::hash(bytemuck::bytes_of(&header)),
            reserved: 0,
        };
        let commit = EntryCommit {
            magic: HOT_COMMIT_MAGIC,
            entry_id: entry_id.get(),
        };
        self.hot
            .write_all(bytemuck::bytes_of(&frame))
            .and_then(|_| self.hot.write_all(bytemuck::bytes_of(&commit)))
            .and_then(|_| self.hot.flush())
            .map_err(LedgerError::Io)?;
        self.hot.sync_data().map_err(LedgerError::Io)?;

        self.source_entries.insert(draft.source_key.0, entry_id);
        self.entry_cases.insert(entry_id.get(), draft.case_id.get());
        self.next_entry += 1;
        self.next_case = self.next_case.max(draft.case_id.get().saturating_add(1));
        self.len += 1;
        Ok(LedgerCommit {
            entry_id,
            case_id: draft.case_id,
            body_hash,
            header,
            inserted: true,
        })
    }

    pub fn hot_path(&self) -> &Path {
        &self.hot_path
    }

    pub fn cold_path(&self) -> &Path {
        &self.cold_path
    }
}

pub struct LedgerMappedStore {
    hot: Mmap,
    cold: Mmap,
    len: usize,
}

impl LedgerMappedStore {
    pub fn open(
        hot_path: impl AsRef<Path>,
        cold_path: impl AsRef<Path>,
    ) -> Result<Self, LedgerError> {
        let hot_file = File::open(hot_path).map_err(LedgerError::Io)?;
        let cold_file = File::open(cold_path).map_err(LedgerError::Io)?;
        let hot = map_read(&hot_file)?;
        let cold = map_read(&cold_file)?;
        let hot_scan = scan_hot(&hot, false)?;
        let cold_scan = scan_cold(&cold, false)?;
        if hot_scan.incomplete_tail || cold_scan.incomplete_tail {
            return Err(LedgerError::IncompleteTail);
        }
        validate_body_references(&hot, &cold, hot_scan.len)?;
        Ok(Self {
            hot,
            cold,
            len: hot_scan.len,
        })
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> LedgerMappedIter<'_> {
        LedgerMappedIter {
            hot: &self.hot,
            cold: &self.cold,
            offset: std::mem::size_of::<StoreFileHeader>(),
            remaining: self.len,
        }
    }
}

pub struct LedgerMappedIter<'a> {
    hot: &'a [u8],
    cold: &'a [u8],
    offset: usize,
    remaining: usize,
}

impl<'a> Iterator for LedgerMappedIter<'a> {
    type Item = LedgerMappedEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let frame_end = self.offset + std::mem::size_of::<HotFrame>();
        let frame =
            bytemuck::try_from_bytes::<HotFrame>(self.hot.get(self.offset..frame_end)?).ok()?;
        let body_start = usize::try_from(frame.header.body_offset).ok()?;
        let body_end = body_start.checked_add(frame.header.body_len as usize)?;
        let body = self.cold.get(body_start..body_end)?;
        self.offset = frame_end + std::mem::size_of::<EntryCommit>();
        self.remaining -= 1;
        Some(LedgerMappedEntry {
            header: &frame.header,
            body,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for LedgerMappedIter<'_> {}

#[derive(Clone, Copy, Debug)]
pub struct LedgerMappedEntry<'a> {
    pub header: &'a LedgerEventHeader,
    pub body: &'a [u8],
}

#[derive(Debug)]
struct HotScan {
    valid_len: usize,
    incomplete_tail: bool,
    next_entry: u64,
    next_case: u64,
    source_entries: HashMap<u128, LedgerEntryId>,
    entry_cases: HashMap<u64, u64>,
    len: usize,
}

#[derive(Clone, Copy, Debug)]
struct ColdScan {
    valid_len: usize,
    incomplete_tail: bool,
}

fn create_file(path: &Path) -> Result<File, LedgerError> {
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(LedgerError::Io)
}

fn write_file_header(
    file: &mut File,
    magic: [u8; 8],
    record_size: u32,
    created_ns: i64,
    schema: &[u8],
) -> Result<(), LedgerError> {
    let header = StoreFileHeader {
        magic,
        version: FORMAT_VERSION,
        header_size: std::mem::size_of::<StoreFileHeader>() as u16,
        record_size,
        created_ns,
        schema_hash: *blake3::hash(schema).as_bytes(),
        reserved: [0; 8],
    };
    file.write_all(bytemuck::bytes_of(&header))
        .and_then(|_| file.flush())
        .map_err(LedgerError::Io)?;
    file.sync_data().map_err(LedgerError::Io)
}

fn map_read(file: &File) -> Result<Mmap, LedgerError> {
    // SAFETY: the returned map owns its view and callers never expose it while
    // mutating the same file through this function.
    unsafe { MmapOptions::new().map(file) }.map_err(LedgerError::Io)
}

fn map_and_scan_hot(file: &File, recover: bool) -> Result<HotScan, LedgerError> {
    let map = map_read(file)?;
    scan_hot(&map, recover)
}

fn map_and_scan_cold(file: &File, recover: bool) -> Result<ColdScan, LedgerError> {
    let map = map_read(file)?;
    scan_cold(&map, recover)
}

fn validate_file_header(bytes: &[u8], magic: [u8; 8]) -> Result<(), LedgerError> {
    let size = std::mem::size_of::<StoreFileHeader>();
    let header = bytes
        .get(..size)
        .map(bytemuck::pod_read_unaligned::<StoreFileHeader>)
        .ok_or(LedgerError::TruncatedFileHeader)?;
    if header.magic != magic
        || header.version != FORMAT_VERSION
        || header.header_size as usize != size
    {
        return Err(LedgerError::InvalidFileHeader);
    }
    Ok(())
}

fn scan_hot(bytes: &[u8], recover: bool) -> Result<HotScan, LedgerError> {
    validate_file_header(bytes, HOT_FILE_MAGIC)?;
    let mut offset = std::mem::size_of::<StoreFileHeader>();
    let mut next_entry = 1u64;
    let mut next_case = 1u64;
    let mut source_entries = HashMap::new();
    let mut entry_cases = HashMap::new();
    let mut len = 0usize;
    let record_len = std::mem::size_of::<HotFrame>() + std::mem::size_of::<EntryCommit>();
    while offset < bytes.len() {
        let Some(end) = offset.checked_add(record_len) else {
            return Err(LedgerError::FileTooLarge);
        };
        let Some(record) = bytes.get(offset..end) else {
            if recover {
                break;
            }
            return Err(LedgerError::IncompleteTail);
        };
        let frame =
            bytemuck::pod_read_unaligned::<HotFrame>(&record[..std::mem::size_of::<HotFrame>()]);
        let commit =
            bytemuck::pod_read_unaligned::<EntryCommit>(&record[std::mem::size_of::<HotFrame>()..]);
        if frame.magic != HOT_FRAME_MAGIC
            || commit.magic != HOT_COMMIT_MAGIC
            || commit.entry_id != frame.header.entry_id
        {
            return Err(LedgerError::InvalidHotFrame { offset });
        }
        if frame.header.entry_id != next_entry {
            return Err(LedgerError::SequenceMismatch {
                expected: next_entry,
                actual: frame.header.entry_id,
            });
        }
        if crc32fast::hash(bytemuck::bytes_of(&frame.header)) != frame.crc32
            || LedgerEventKind::from_raw(frame.header.kind).is_none()
            || LedgerEventStatus::from_raw(frame.header.status).is_none()
            || ActorKind::from_raw(frame.header.actor_kind).is_none()
        {
            return Err(LedgerError::InvalidHotFrame { offset });
        }
        let key = frame.header.source_key().0;
        if key == 0
            || source_entries
                .insert(key, LedgerEntryId(next_entry))
                .is_some()
        {
            return Err(LedgerError::DuplicateSourceKey(SourceEventKey(key)));
        }
        entry_cases.insert(next_entry, frame.header.case_id);
        next_case = next_case.max(frame.header.case_id.saturating_add(1));
        next_entry += 1;
        len += 1;
        offset = end;
    }
    Ok(HotScan {
        valid_len: offset,
        incomplete_tail: offset != bytes.len(),
        next_entry,
        next_case,
        source_entries,
        entry_cases,
        len,
    })
}

fn scan_cold(bytes: &[u8], recover: bool) -> Result<ColdScan, LedgerError> {
    validate_file_header(bytes, COLD_FILE_MAGIC)?;
    let mut offset = std::mem::size_of::<StoreFileHeader>();
    while offset < bytes.len() {
        let frame_size = std::mem::size_of::<BodyFrame>();
        let Some(frame_end) = offset.checked_add(frame_size) else {
            return Err(LedgerError::FileTooLarge);
        };
        let Some(frame_bytes) = bytes.get(offset..frame_end) else {
            if recover {
                break;
            }
            return Err(LedgerError::IncompleteTail);
        };
        let frame = bytemuck::pod_read_unaligned::<BodyFrame>(frame_bytes);
        if frame.magic != COLD_FRAME_MAGIC {
            return Err(LedgerError::InvalidColdFrame { offset });
        }
        let body_end = frame_end
            .checked_add(frame.body_len as usize)
            .ok_or(LedgerError::FileTooLarge)?;
        let commit_end = body_end
            .checked_add(std::mem::size_of::<EntryCommit>())
            .ok_or(LedgerError::FileTooLarge)?;
        let Some(body) = bytes.get(frame_end..body_end) else {
            if recover {
                break;
            }
            return Err(LedgerError::IncompleteTail);
        };
        let Some(commit_bytes) = bytes.get(body_end..commit_end) else {
            if recover {
                break;
            }
            return Err(LedgerError::IncompleteTail);
        };
        let commit = bytemuck::pod_read_unaligned::<EntryCommit>(commit_bytes);
        if commit.magic != COLD_COMMIT_MAGIC
            || commit.entry_id != frame.entry_id
            || blake3::hash(body).as_bytes() != &frame.body_hash
        {
            return Err(LedgerError::InvalidColdFrame { offset });
        }
        offset = commit_end;
    }
    Ok(ColdScan {
        valid_len: offset,
        incomplete_tail: offset != bytes.len(),
    })
}

fn validate_body_references(hot: &[u8], cold: &[u8], len: usize) -> Result<usize, LedgerError> {
    let mut hot_offset = std::mem::size_of::<StoreFileHeader>();
    let mut referenced_end = std::mem::size_of::<StoreFileHeader>();
    for _ in 0..len {
        let frame_end = hot_offset + std::mem::size_of::<HotFrame>();
        let frame = bytemuck::pod_read_unaligned::<HotFrame>(
            hot.get(hot_offset..frame_end)
                .ok_or(LedgerError::IncompleteTail)?,
        );
        let body_start =
            usize::try_from(frame.header.body_offset).map_err(|_| LedgerError::FileTooLarge)?;
        let frame_start = body_start
            .checked_sub(std::mem::size_of::<BodyFrame>())
            .ok_or(LedgerError::InvalidBodyReference(frame.header.entry_id))?;
        let body_end = body_start
            .checked_add(frame.header.body_len as usize)
            .ok_or(LedgerError::FileTooLarge)?;
        let commit_end = body_end
            .checked_add(std::mem::size_of::<EntryCommit>())
            .ok_or(LedgerError::FileTooLarge)?;
        let body_frame = bytemuck::pod_read_unaligned::<BodyFrame>(
            cold.get(frame_start..body_start)
                .ok_or(LedgerError::InvalidBodyReference(frame.header.entry_id))?,
        );
        let body = cold
            .get(body_start..body_end)
            .ok_or(LedgerError::InvalidBodyReference(frame.header.entry_id))?;
        let body_commit = bytemuck::pod_read_unaligned::<EntryCommit>(
            cold.get(body_end..commit_end)
                .ok_or(LedgerError::InvalidBodyReference(frame.header.entry_id))?,
        );
        if body_frame.magic != COLD_FRAME_MAGIC
            || body_frame.entry_id != frame.header.entry_id
            || body_commit.magic != COLD_COMMIT_MAGIC
            || body_commit.entry_id != frame.header.entry_id
            || body_frame.body_len != frame.header.body_len
            || body_frame.body_hash != frame.header.body_hash
            || blake3::hash(body).as_bytes() != &frame.header.body_hash
        {
            return Err(LedgerError::InvalidBodyReference(frame.header.entry_id));
        }
        referenced_end = referenced_end.max(commit_end);
        hot_offset = frame_end + std::mem::size_of::<EntryCommit>();
    }
    Ok(referenced_end)
}

fn validate_draft(
    draft: &LedgerEventDraft<'_>,
    entry_cases: &HashMap<u64, u64>,
) -> Result<(), LedgerError> {
    if draft.source_key == SourceEventKey::UNKNOWN {
        return Err(LedgerError::MissingSourceKey);
    }
    if draft.actor_id == super::types::ActorId::UNKNOWN || draft.schema_version.get() == 0 {
        return Err(LedgerError::MissingIdentity);
    }
    if draft.ts_received_ns < draft.ts_event_ns || draft.ts_recorded_ns < draft.ts_received_ns {
        return Err(LedgerError::InvalidTimeOrder);
    }
    if let Some((first, last)) = draft.canonical_sequences {
        if first.get() == 0 || last < first {
            return Err(LedgerError::InvalidCanonicalRange);
        }
    }
    if draft.kind.is_operator() && draft.actor_kind != ActorKind::Operator {
        return Err(LedgerError::InvalidActorKind);
    }
    if draft.kind.is_operator() && draft.body.is_empty() {
        return Err(LedgerError::MissingOperatorBody);
    }
    let amendment = matches!(
        draft.kind,
        LedgerEventKind::Amendment | LedgerEventKind::Redaction
    );
    if amendment && draft.parent_entry_id == LedgerEntryId::UNKNOWN {
        return Err(LedgerError::MissingParent);
    }
    if draft.parent_entry_id != LedgerEntryId::UNKNOWN {
        let parent_case = entry_cases
            .get(&draft.parent_entry_id.get())
            .ok_or(LedgerError::UnknownParent(draft.parent_entry_id))?;
        if *parent_case != draft.case_id.get() {
            return Err(LedgerError::ParentCaseMismatch);
        }
    }
    u32::try_from(draft.body.len()).map_err(|_| LedgerError::BodyTooLarge)?;
    Ok(())
}

fn header_for_entry(
    file: &File,
    entry_id: LedgerEntryId,
) -> Result<LedgerEventHeader, LedgerError> {
    let map = map_read(file)?;
    let scan = scan_hot(&map, false)?;
    if entry_id.get() == 0 || entry_id.get() >= scan.next_entry {
        return Err(LedgerError::UnknownEntry(entry_id));
    }
    let index = usize::try_from(entry_id.get() - 1).map_err(|_| LedgerError::FileTooLarge)?;
    let record_len = std::mem::size_of::<HotFrame>() + std::mem::size_of::<EntryCommit>();
    let offset = std::mem::size_of::<StoreFileHeader>() + index * record_len;
    let end = offset + std::mem::size_of::<HotFrame>();
    let frame = bytemuck::pod_read_unaligned::<HotFrame>(
        map.get(offset..end).ok_or(LedgerError::IncompleteTail)?,
    );
    Ok(frame.header)
}

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("ledger I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("ledger file header is truncated")]
    TruncatedFileHeader,
    #[error("ledger file header is invalid or incompatible")]
    InvalidFileHeader,
    #[error("ledger has an incomplete tail")]
    IncompleteTail,
    #[error("invalid ledger hot frame at byte {offset}")]
    InvalidHotFrame { offset: usize },
    #[error("invalid ledger cold frame at byte {offset}")]
    InvalidColdFrame { offset: usize },
    #[error("ledger entry {0} has an invalid cold-body reference")]
    InvalidBodyReference(u64),
    #[error("expected ledger entry {expected}, found {actual}")]
    SequenceMismatch { expected: u64, actual: u64 },
    #[error("duplicate ledger source key {0:?}")]
    DuplicateSourceKey(SourceEventKey),
    #[error("a stable source or operator-command key is required")]
    MissingSourceKey,
    #[error("actor and schema identities must be nonzero")]
    MissingIdentity,
    #[error("ledger timestamps are not causally ordered")]
    InvalidTimeOrder,
    #[error("canonical sequence range is invalid")]
    InvalidCanonicalRange,
    #[error("operator event requires an operator actor")]
    InvalidActorKind,
    #[error("operator event body cannot be empty")]
    MissingOperatorBody,
    #[error("amendment or redaction requires a parent")]
    MissingParent,
    #[error("parent ledger entry {0:?} does not exist")]
    UnknownParent(LedgerEntryId),
    #[error("parent and child ledger events must belong to the same trade case")]
    ParentCaseMismatch,
    #[error("ledger entry {0:?} does not exist")]
    UnknownEntry(LedgerEntryId),
    #[error("ledger body exceeds the u32 cold-pack limit")]
    BodyTooLarge,
    #[error("ledger file exceeds addressable limits")]
    FileTooLarge,
}

const _: () = assert!(std::mem::size_of::<StoreFileHeader>() == 64);
const _: () = assert!(std::mem::size_of::<HotFrame>() == 192);
const _: () = assert!(std::mem::size_of::<BodyFrame>() == 64);

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;

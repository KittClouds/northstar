//! Immutable OSV1 release packing, cold rehydration, and read-only consumption.
//!
//! The archive is a transport container. Scientific identity remains the
//! logical OSV1 root stored in the sealed Gate 16.6 receipt.

use hashbrown::HashSet;
use memchr::memmem;
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Cursor, Write},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

const MAGIC: [u8; 8] = *b"NSOSV1R1";
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 96;
const ENTRY_BYTES: usize = 96;
const MANIFEST_PATH: &str = "release/osv1_release_manifest.json";
const CONSUMER_PATH: &str = "release/osv1_consumer_contract.json";

#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Header {
    magic: [u8; 8],
    version: u32,
    entry_count: u32,
    table_offset: u64,
    names_offset: u64,
    data_offset: u64,
    file_len: u64,
    manifest_entry: u32,
    consumer_entry: u32,
    reserved: [u8; 40],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Entry {
    path_offset: u64,
    path_len: u32,
    kind: u32,
    data_offset: u64,
    compressed_len: u64,
    raw_len: u64,
    sha256: [u8; 32],
    reserved: [u8; 24],
}

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("JSON error at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("release invariant failed: {0}")]
    Invariant(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseEntrySpec {
    pub path: String,
    pub authority_kind: String,
    pub availability: String,
    pub mutability: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub contract: String,
    pub status: String,
    pub release_id: String,
    pub osv1_root_sha256: String,
    pub theta0_sha256: String,
    pub ancestry_sha256: String,
    pub findings_sha256: String,
    pub parity_matrix_sha256: String,
    pub cold_rehydration: String,
    pub cold_source_regeneration: String,
    pub source_regeneration_portability: String,
    pub source_capsule: String,
    pub entries: Vec<ReleaseEntrySpec>,
    pub logical_release_manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicStateKind {
    Null,
    Censored,
    NotApplicable,
    NotEvaluable,
    InsufficientSupport,
    Open,
    FrozenUnopened,
    SourceRecovered,
    NotRunBound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedState {
    pub kind: EpistemicStateKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedStateEvidence {
    pub value: TaggedState,
    pub class: String,
    pub transport: String,
    pub evidence_path: String,
    pub evidence_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerContract {
    pub contract: String,
    pub status: String,
    pub release_id: String,
    pub osv1_root_sha256: String,
    pub authority_namespace: String,
    pub derived_namespace: String,
    pub permissions: Vec<String>,
    pub forbidden: Vec<String>,
    pub typed_states: Vec<TypedStateEvidence>,
    pub source_kinds: Vec<String>,
    pub theta0_source_recovered_count: usize,
    pub theta0_source_recovered_authority_value: String,
    pub theta0_not_run_bound_authority_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackReceipt {
    pub contract: String,
    pub status: String,
    pub release_id: String,
    pub osv1_root_sha256: String,
    pub pack_sha256: String,
    pub manifest_sha256: String,
    pub consumer_contract_sha256: String,
    pub entry_count: usize,
    pub raw_bytes: u64,
    pub packed_bytes: u64,
    pub compression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RehydrationReceipt {
    pub contract: String,
    pub status: String,
    pub release_id: String,
    pub osv1_root_sha256: String,
    pub pack_sha256: String,
    pub entry_count: usize,
    pub extracted_entry_count: usize,
    pub hash_mismatch_count: usize,
    pub original_prep_accessed: bool,
    pub cold_source_regeneration: String,
    pub source_regeneration_portability: String,
    pub declared_authoritative_reads: usize,
    pub undeclared_authoritative_reads: usize,
    pub absolute_authority_resolution_attempts: usize,
    pub allowed_read_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerSmokeReceipt {
    pub contract: String,
    pub status: String,
    pub source_kind: String,
    pub parent_osv1_root_sha256: String,
    pub release_id: String,
    pub pack_sha256_before: String,
    pub pack_sha256_after: String,
    pub authority_entries_read: usize,
    pub typed_states_verified: usize,
    pub source_kinds_verified: usize,
    pub theta0_source_recovered_count: usize,
    pub c1_parity_status: String,
    pub source_mutation_count: usize,
}

#[derive(Debug)]
struct BuildEntry {
    path: String,
    kind: u32,
    raw: Vec<u8>,
    compressed: Vec<u8>,
    sha256: [u8; 32],
}

fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ReleaseError {
    ReleaseError::Io {
        path: path.into(),
        source,
    }
}

fn json_error(path: impl Into<PathBuf>, source: serde_json::Error) -> ReleaseError {
    ReleaseError::Json {
        path: path.into(),
        source,
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn hash_file(path: impl AsRef<Path>) -> Result<String, ReleaseError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| io_error(path, source))?;
    Ok(hex(digest(&bytes)))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ReleaseError> {
    let bytes = std::fs::read(path).map_err(|source| io_error(path, source))?;
    serde_json::from_slice(&bytes).map_err(|source| json_error(path, source))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), ReleaseError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| json_error(path, source))?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).map_err(|source| io_error(path, source))
}

fn validate_relative(path: &str) -> Result<(), ReleaseError> {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ReleaseError::Invariant(format!(
            "unsafe release path: {path}"
        )));
    }
    Ok(())
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = path.components().collect::<Vec<_>>();
    let root = root.components().collect::<Vec<_>>();
    path.len() >= root.len() && path[..root.len()] == root[..]
}

fn validate_tagged_state_contract() -> Result<(), ReleaseError> {
    for illegal in [
        "null",
        "\"\"",
        "0",
        "false",
        "\"unknown\"",
        "{\"kind\":\"UNKNOWN\"}",
    ] {
        if serde_json::from_str::<TaggedState>(illegal).is_ok() {
            return Err(ReleaseError::Invariant(format!(
                "illegal epistemic state accepted: {illegal}"
            )));
        }
    }
    Ok(())
}

fn build_entry(path: String, kind: u32, raw: Vec<u8>) -> Result<BuildEntry, ReleaseError> {
    validate_relative(&path)?;
    let compressed = zstd::stream::encode_all(Cursor::new(&raw), 19)
        .map_err(|source| io_error(&path, source))?;
    Ok(BuildEntry {
        sha256: digest(&raw),
        path,
        kind,
        raw,
        compressed,
    })
}

pub fn build_release_pack(
    repository_root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    consumer_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    receipt_path: impl AsRef<Path>,
) -> Result<PackReceipt, ReleaseError> {
    let root = repository_root.as_ref();
    let manifest_path = manifest_path.as_ref();
    let consumer_path = consumer_path.as_ref();
    let output_path = output_path.as_ref();
    let receipt_path = receipt_path.as_ref();
    let manifest: ReleaseManifest = read_json(manifest_path)?;
    let consumer: ConsumerContract = read_json(consumer_path)?;
    if manifest.status != "SEALED" || consumer.status != "READ_ONLY" {
        return Err(ReleaseError::Invariant(
            "release contracts are not sealed/read-only".into(),
        ));
    }
    if manifest.osv1_root_sha256 != consumer.osv1_root_sha256 {
        return Err(ReleaseError::Invariant(
            "consumer root differs from release root".into(),
        ));
    }

    let manifest_bytes =
        std::fs::read(manifest_path).map_err(|source| io_error(manifest_path, source))?;
    let consumer_bytes =
        std::fs::read(consumer_path).map_err(|source| io_error(consumer_path, source))?;
    let mut entries = Vec::with_capacity(manifest.entries.len() + 2);
    entries.push(build_entry(
        MANIFEST_PATH.into(),
        1,
        manifest_bytes.clone(),
    )?);
    entries.push(build_entry(
        CONSUMER_PATH.into(),
        2,
        consumer_bytes.clone(),
    )?);
    let mut paths = HashSet::with_capacity(manifest.entries.len() + 2);
    paths.insert(MANIFEST_PATH.to_string());
    paths.insert(CONSUMER_PATH.to_string());
    for spec in &manifest.entries {
        validate_relative(&spec.path)?;
        if !paths.insert(spec.path.clone()) {
            return Err(ReleaseError::Invariant(format!(
                "duplicate release path: {}",
                spec.path
            )));
        }
        let source = root.join(&spec.path);
        let raw = std::fs::read(&source).map_err(|error| io_error(&source, error))?;
        if raw.len() as u64 != spec.bytes || hex(digest(&raw)) != spec.sha256 {
            return Err(ReleaseError::Invariant(format!(
                "source identity mismatch: {}",
                spec.path
            )));
        }
        entries.push(build_entry(spec.path.clone(), 3, raw)?);
    }
    entries[2..].sort_unstable_by(|a, b| a.path.cmp(&b.path));

    let names_bytes: usize = entries.iter().map(|entry| entry.path.len()).sum();
    let table_offset = HEADER_BYTES as u64;
    let names_offset = table_offset + (entries.len() * ENTRY_BYTES) as u64;
    let data_offset = names_offset + names_bytes as u64;
    let packed_data: usize = entries.iter().map(|entry| entry.compressed.len()).sum();
    let file_len = data_offset + packed_data as u64;
    let header = Header {
        magic: MAGIC,
        version: VERSION,
        entry_count: entries.len() as u32,
        table_offset,
        names_offset,
        data_offset,
        file_len,
        manifest_entry: 0,
        consumer_entry: 1,
        reserved: [0; 40],
    };
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .map_err(|source| io_error(output_path, source))?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);
    writer
        .write_all(header.as_bytes())
        .map_err(|source| io_error(output_path, source))?;
    let mut path_offset = 0_u64;
    let mut packed_offset = data_offset;
    for entry in &entries {
        let row = Entry {
            path_offset,
            path_len: entry.path.len() as u32,
            kind: entry.kind,
            data_offset: packed_offset,
            compressed_len: entry.compressed.len() as u64,
            raw_len: entry.raw.len() as u64,
            sha256: entry.sha256,
            reserved: [0; 24],
        };
        writer
            .write_all(row.as_bytes())
            .map_err(|source| io_error(output_path, source))?;
        path_offset += entry.path.len() as u64;
        packed_offset += entry.compressed.len() as u64;
    }
    for entry in &entries {
        writer
            .write_all(entry.path.as_bytes())
            .map_err(|source| io_error(output_path, source))?;
    }
    for entry in &entries {
        writer
            .write_all(&entry.compressed)
            .map_err(|source| io_error(output_path, source))?;
    }
    writer
        .flush()
        .map_err(|source| io_error(output_path, source))?;
    drop(writer);
    let receipt = PackReceipt {
        contract: "NORTHSTAR_GATE16_7_OSV1_RELEASE_PACK_RECEIPT_V1".into(),
        status: "PASS".into(),
        release_id: manifest.release_id,
        osv1_root_sha256: manifest.osv1_root_sha256,
        pack_sha256: hash_file(output_path)?,
        manifest_sha256: hex(digest(&manifest_bytes)),
        consumer_contract_sha256: hex(digest(&consumer_bytes)),
        entry_count: entries.len(),
        raw_bytes: entries.iter().map(|entry| entry.raw.len() as u64).sum(),
        packed_bytes: std::fs::metadata(output_path)
            .map_err(|source| io_error(output_path, source))?
            .len(),
        compression: "ZSTD_LEVEL_19_PER_ENTRY_NO_TIMESTAMP".into(),
    };
    write_json(receipt_path, &receipt)?;
    Ok(receipt)
}

pub struct ReleaseArchive {
    path: PathBuf,
    mmap: Mmap,
    header: Header,
    entries: Box<[Entry]>,
}

impl ReleaseArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReleaseError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).map_err(|source| io_error(&path, source))?;
        let mmap =
            unsafe { MmapOptions::new().map(&file) }.map_err(|source| io_error(&path, source))?;
        if mmap.len() < HEADER_BYTES {
            return Err(ReleaseError::Invariant("release pack is truncated".into()));
        }
        let header = Header::read_from_prefix(&mmap[..HEADER_BYTES])
            .map_err(|_| ReleaseError::Invariant("invalid release header layout".into()))?
            .0;
        if header.magic != MAGIC
            || header.version != VERSION
            || header.file_len as usize != mmap.len()
        {
            return Err(ReleaseError::Invariant(
                "release header identity mismatch".into(),
            ));
        }
        let table_end = HEADER_BYTES + header.entry_count as usize * ENTRY_BYTES;
        if table_end > mmap.len()
            || header.names_offset as usize != table_end
            || header.data_offset > header.file_len
        {
            return Err(ReleaseError::Invariant(
                "release table bounds mismatch".into(),
            ));
        }
        let mut entries = Vec::with_capacity(header.entry_count as usize);
        for index in 0..header.entry_count as usize {
            let start = HEADER_BYTES + index * ENTRY_BYTES;
            let entry = Entry::read_from_prefix(&mmap[start..start + ENTRY_BYTES])
                .map_err(|_| ReleaseError::Invariant("invalid release entry layout".into()))?
                .0;
            if header.names_offset + entry.path_offset + u64::from(entry.path_len)
                > header.data_offset
                || entry.data_offset + entry.compressed_len > header.file_len
            {
                return Err(ReleaseError::Invariant(
                    "release entry bounds mismatch".into(),
                ));
            }
            entries.push(entry);
        }
        let archive = Self {
            path,
            mmap,
            header,
            entries: entries.into_boxed_slice(),
        };
        let mut unique = HashSet::with_capacity(archive.entries.len());
        for index in 0..archive.entries.len() {
            let name = archive.path_at(index)?;
            validate_relative(name)?;
            if !unique.insert(name.to_owned()) {
                return Err(ReleaseError::Invariant("duplicate packed path".into()));
            }
        }
        Ok(archive)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn path_at(&self, index: usize) -> Result<&str, ReleaseError> {
        let entry = *self
            .entries
            .get(index)
            .ok_or_else(|| ReleaseError::Invariant("entry index out of range".into()))?;
        let start = (self.header.names_offset + entry.path_offset) as usize;
        let end = start + entry.path_len as usize;
        std::str::from_utf8(&self.mmap[start..end])
            .map_err(|_| ReleaseError::Invariant("packed path is not UTF-8".into()))
    }

    pub fn read_at(&self, index: usize) -> Result<Vec<u8>, ReleaseError> {
        let entry = *self
            .entries
            .get(index)
            .ok_or_else(|| ReleaseError::Invariant("entry index out of range".into()))?;
        let start = entry.data_offset as usize;
        let end = start + entry.compressed_len as usize;
        let raw = zstd::stream::decode_all(Cursor::new(&self.mmap[start..end]))
            .map_err(|source| io_error(&self.path, source))?;
        if raw.len() as u64 != entry.raw_len || digest(&raw) != entry.sha256 {
            return Err(ReleaseError::Invariant(format!(
                "entry hash mismatch: {}",
                self.path_at(index)?
            )));
        }
        Ok(raw)
    }

    pub fn read_path(&self, path: &str) -> Result<Vec<u8>, ReleaseError> {
        let index = (0..self.entries.len())
            .find(|index| self.path_at(*index).ok() == Some(path))
            .ok_or_else(|| ReleaseError::Invariant(format!("release path not found: {path}")))?;
        self.read_at(index)
    }

    pub fn manifest(&self) -> Result<ReleaseManifest, ReleaseError> {
        let raw = self.read_at(self.header.manifest_entry as usize)?;
        serde_json::from_slice(&raw).map_err(|source| json_error(MANIFEST_PATH, source))
    }

    pub fn consumer_contract(&self) -> Result<ConsumerContract, ReleaseError> {
        let raw = self.read_at(self.header.consumer_entry as usize)?;
        serde_json::from_slice(&raw).map_err(|source| json_error(CONSUMER_PATH, source))
    }
}

pub fn rehydrate_release(
    pack_path: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<RehydrationReceipt, ReleaseError> {
    let pack_path = pack_path.as_ref();
    let output_root = output_root.as_ref();
    let archive = ReleaseArchive::open(pack_path)?;
    let manifest = archive.manifest()?;
    std::fs::create_dir_all(output_root).map_err(|source| io_error(output_root, source))?;
    for index in 0..archive.len() {
        let relative = archive.path_at(index)?;
        let output = output_root.join(relative);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        }
        let raw = archive.read_at(index)?;
        std::fs::write(&output, raw).map_err(|source| io_error(&output, source))?;
    }
    let root_receipt_path =
        output_root.join("research/market-objects/artifacts/gate16-6-prep/osv1_root_receipt.json");
    let root: Value = read_json(&root_receipt_path)?;
    if root["logical_osv1_sha256"] != manifest.osv1_root_sha256 {
        return Err(ReleaseError::Invariant(
            "rehydrated OSV1 root mismatch".into(),
        ));
    }
    Ok(RehydrationReceipt {
        contract: "NORTHSTAR_GATE16_7_OSV1_COLD_REHYDRATION_RECEIPT_V1".into(),
        status: "PASS".into(),
        release_id: manifest.release_id,
        osv1_root_sha256: manifest.osv1_root_sha256,
        pack_sha256: hash_file(pack_path)?,
        entry_count: archive.len(),
        extracted_entry_count: archive.len(),
        hash_mismatch_count: 0,
        original_prep_accessed: false,
        cold_source_regeneration: "NOT_EVALUABLE_EXTERNAL_MT5_SOURCE_CAPSULE_ABSENT".into(),
        source_regeneration_portability: "NOT_PORTABLE_WITHOUT_EXTERNAL_SOURCE_CAPSULE".into(),
        declared_authoritative_reads: archive.len(),
        undeclared_authoritative_reads: 0,
        absolute_authority_resolution_attempts: 0,
        allowed_read_authority: "RELEASE_PACK_MEMBERS_ONLY".into(),
    })
}

pub fn consumer_smoke(
    pack_path: impl AsRef<Path>,
    sidecar_path: impl AsRef<Path>,
) -> Result<ConsumerSmokeReceipt, ReleaseError> {
    let pack_path = pack_path.as_ref();
    let sidecar_path = sidecar_path.as_ref();
    let pack_parent = pack_path.parent().unwrap_or_else(|| Path::new("."));
    if path_is_within(sidecar_path, pack_parent) {
        return Err(ReleaseError::Invariant(
            "derived sidecar must be outside release payload directory".into(),
        ));
    }
    validate_tagged_state_contract()?;
    let before = hash_file(pack_path)?;
    let archive = ReleaseArchive::open(pack_path)?;
    let manifest = archive.manifest()?;
    let contract = archive.consumer_contract()?;
    if contract.osv1_root_sha256 != manifest.osv1_root_sha256
        || contract.authority_namespace != "NORTHSTAR_AUTHORITY"
        || contract.derived_namespace != "PHOENIX_DERIVED"
    {
        return Err(ReleaseError::Invariant(
            "consumer namespace/root contract mismatch".into(),
        ));
    }
    let required_permissions = ["READ", "FILTER", "PROJECT", "VISUALIZE", "DERIVE_SIDECAR"];
    if required_permissions
        .iter()
        .any(|permission| !contract.permissions.iter().any(|item| item == permission))
    {
        return Err(ReleaseError::Invariant(
            "consumer permission contract incomplete".into(),
        ));
    }
    for state in &contract.typed_states {
        if state.transport != "TYPED_ENUM" {
            return Err(ReleaseError::Invariant(format!(
                "state is not typed: {:?}",
                state.value.kind
            )));
        }
        let raw = archive.read_path(&state.evidence_path)?;
        if memmem::find(&raw, state.evidence_token.as_bytes()).is_none() {
            return Err(ReleaseError::Invariant(format!(
                "typed-state evidence absent: {:?}",
                state.value.kind
            )));
        }
    }
    let theta_raw = archive
        .read_path("research/market-objects/artifacts/gate16-6-prep/osv1_theta0_fields.json")?;
    let theta: Value = serde_json::from_slice(&theta_raw)
        .map_err(|source| json_error("osv1_theta0_fields.json", source))?;
    let recovered = theta["fields"]
        .as_array()
        .ok_or_else(|| ReleaseError::Invariant("Theta0 fields absent".into()))?
        .iter()
        .filter(|field| {
            field["evidence_class"] == contract.theta0_source_recovered_authority_value
                && field["run_binding"] == contract.theta0_not_run_bound_authority_value
        })
        .count();
    if recovered != contract.theta0_source_recovered_count {
        return Err(ReleaseError::Invariant(
            "source-recovered Theta0 count changed".into(),
        ));
    }
    let parity_raw = archive.read_path(
        "research/market-objects/artifacts/gate16-6-prep/osv1_stagewise_parity_matrix.json",
    )?;
    if memmem::find(&parity_raw, b"OPEN").is_none() {
        return Err(ReleaseError::Invariant("C1 OPEN state absent".into()));
    }
    let findings_raw = archive
        .read_path("research/market-objects/artifacts/gate16-6-prep/osv1_machine_findings.json")?;
    let findings: Value = serde_json::from_slice(&findings_raw)
        .map_err(|source| json_error("osv1_machine_findings.json", source))?;
    let human_count = findings["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(findings["insufficiencies"].as_array().into_iter().flatten())
        .filter(|row| row["source_kind"] == "HUMAN_SUMMARY")
        .count();
    if human_count != 0
        || !contract
            .source_kinds
            .iter()
            .any(|kind| kind == "HUMAN_SUMMARY")
    {
        return Err(ReleaseError::Invariant(
            "source_kind authority boundary changed".into(),
        ));
    }
    let after = hash_file(pack_path)?;
    if before != after {
        return Err(ReleaseError::Invariant(
            "read-only consumer mutated release pack".into(),
        ));
    }
    let receipt = ConsumerSmokeReceipt {
        contract: "NORTHSTAR_GATE16_7_OSV1_PHOENIX_READ_ONLY_SMOKE_V1".into(),
        status: "PASS".into(),
        source_kind: "PHOENIX_DERIVED".into(),
        parent_osv1_root_sha256: manifest.osv1_root_sha256,
        release_id: manifest.release_id,
        pack_sha256_before: before,
        pack_sha256_after: after,
        authority_entries_read: archive.len(),
        typed_states_verified: contract.typed_states.len(),
        source_kinds_verified: contract.source_kinds.len(),
        theta0_source_recovered_count: recovered,
        c1_parity_status: "OPEN".into(),
        source_mutation_count: 0,
    };
    write_json(sidecar_path, &receipt)?;
    Ok(receipt)
}

pub fn write_rehydration_receipt(
    path: impl AsRef<Path>,
    receipt: &RehydrationReceipt,
) -> Result<(), ReleaseError> {
    write_json(path.as_ref(), receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_layout_sizes_are_frozen() {
        assert_eq!(std::mem::size_of::<Header>(), HEADER_BYTES);
        assert_eq!(std::mem::size_of::<Entry>(), ENTRY_BYTES);
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(validate_relative("safe/path.json").is_ok());
        assert!(validate_relative("../escape.json").is_err());
    }

    #[test]
    fn epistemic_states_reject_untyped_coercions() {
        validate_tagged_state_contract().unwrap();
        let state: TaggedState = serde_json::from_str("{\"kind\":\"NOT_EVALUABLE\"}").unwrap();
        assert_eq!(state.kind, EpistemicStateKind::NotEvaluable);
    }
}

use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use memchr::memchr;
use northstar_auction_contract::{DATASET_CONTRACT, DATASET_SCHEMA, RESEARCH_GENERATION};
use northstar_mt5_corpus::{CorpusVerifier, MappedTsv};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    format::{
        DIRECTORY_ENTRY_SIZE, DirectoryEntry, HEADER_SIZE, Header, PACKED_CONTRACT, RELATIONS,
        RelationKind, align_writer, encode_entry, encode_header,
    },
    view::{PackedCorpus, PackedError},
};

const SEMANTIC_DOMAIN: &[u8] = b"RG2_AUCTION_RESEARCH_PACKED_SEMANTIC_V1\0";

#[derive(Debug, Serialize)]
pub struct PackReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub source_corpus_sha256: String,
    pub packed_semantic_sha256: String,
    pub artifact_sha256: String,
    pub schema_dictionary_sha256: String,
    pub artifact_bytes: u64,
    pub counts: [u64; 7],
    pub artifact: PathBuf,
}

#[derive(Serialize)]
struct Metadata<'a> {
    contract: &'static str,
    format_version: u16,
    research_generation: u16,
    dataset_contract: &'static str,
    dataset_schema: u16,
    source_corpus_sha256: &'a str,
    schema_dictionary_sha256: &'a str,
    null_representation: &'static str,
    timestamp_encoding: &'static str,
    timestamp_timezone: &'static str,
    identifier_storage: &'static str,
    ordering: &'static str,
}

pub fn pack_verified_corpus(
    corpus_root: &Path,
    corpus_seal: &Path,
    output: &Path,
) -> Result<PackReceipt, PackedError> {
    let report = CorpusVerifier::new(corpus_root, corpus_seal).verify()?;
    let schema_dictionary = corpus_seal
        .parent()
        .ok_or_else(|| PackedError::Contract("corpus seal has no parent".into()))?
        .join("schema_dictionary.json");
    let schema_hash = sha256_file(&schema_dictionary)?;
    let schema_hash_hex = hex(&schema_hash);
    let metadata = serde_json::to_vec(&Metadata {
        contract: PACKED_CONTRACT,
        format_version: 1,
        research_generation: RESEARCH_GENERATION,
        dataset_contract: DATASET_CONTRACT,
        dataset_schema: DATASET_SCHEMA,
        source_corpus_sha256: &report.canonical_corpus_sha256,
        schema_dictionary_sha256: &schema_hash_hex,
        null_representation: "validity is source \\N; packed rows preserve bytes",
        timestamp_encoding: "unix_seconds",
        timestamp_timezone: "MT5_SERVER",
        identifier_storage: "unsigned decimal u64 source representation",
        ordering: "run_key ascending, source ledger order within run",
    })?;
    let metadata_hash: [u8; 32] = Sha256::digest(&metadata).into();

    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|source| PackedError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let temporary = output.with_extension("building");
    let file = File::create(&temporary).map_err(|source| PackedError::Io {
        path: temporary.clone(),
        source,
    })?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);
    writer.write_all(&vec![
        0_u8;
        HEADER_SIZE + RELATIONS.len() * DIRECTORY_ENTRY_SIZE
    ])?;
    let mut position = (HEADER_SIZE + RELATIONS.len() * DIRECTORY_ENTRY_SIZE) as u64;
    position = align_writer(&mut writer, position)?;
    let metadata_offset = position;
    writer.write_all(&metadata)?;
    position += metadata.len() as u64;

    let mut semantic = Sha256::new();
    semantic.update(SEMANTIC_DOMAIN);
    let mut entries = Vec::with_capacity(RELATIONS.len());
    let mut run_dirs = fs::read_dir(corpus_root)
        .map_err(|source| PackedError::Io {
            path: corpus_root.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    run_dirs.sort_unstable();

    for kind in RELATIONS {
        position = align_writer(&mut writer, position)?;
        let header = relation_header(&run_dirs, kind)?;
        let header_offset = position;
        writer.write_all(&header)?;
        position += header.len() as u64;
        position = align_writer(&mut writer, position)?;
        let data_offset = position;
        let mut offsets = Vec::new();
        offsets.push(0_u64);
        let mut section_hash = Sha256::new();
        section_hash.update(kind.name().as_bytes());
        section_hash.update([0]);
        section_hash.update(&header);
        semantic.update(kind.name().as_bytes());
        semantic.update([0]);
        semantic.update(&header);
        for run_dir in &run_dirs {
            let path = dataset_path(run_dir, kind)?;
            append_rows(
                &path,
                &mut writer,
                &mut position,
                data_offset,
                &mut offsets,
                &mut section_hash,
                &mut semantic,
            )?;
        }
        let data_len = position - data_offset;
        position = align_writer(&mut writer, position)?;
        let index_offset = position;
        for offset in &offsets {
            writer.write_all(&offset.to_le_bytes())?;
        }
        let index_len = (offsets.len() * 8) as u64;
        position += index_len;
        entries.push(DirectoryEntry {
            kind: kind as u16,
            header_offset,
            header_len: header.len() as u64,
            data_offset,
            data_len,
            index_offset,
            index_len,
            row_count: offsets.len() as u64 - 1,
            section_sha256: section_hash.finalize().into(),
        });
    }
    writer.flush()?;
    let file_size = position;
    let packed_semantic_sha256: [u8; 32] = semantic.finalize().into();
    let source_hash = decode_hex_32(&report.canonical_corpus_sha256)?;
    let counts: [u64; 7] = entries
        .iter()
        .map(|entry| entry.row_count)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| PackedError::Contract("relation count mismatch".into()))?;
    let header = Header {
        file_size,
        source_corpus_sha256: source_hash,
        packed_semantic_sha256,
        schema_dictionary_sha256: schema_hash,
        metadata_sha256: metadata_hash,
        counts,
        metadata_offset,
        metadata_len: metadata.len() as u64,
    };
    let mut file = writer.into_inner().map_err(|error| PackedError::Io {
        path: temporary.clone(),
        source: error.into_error(),
    })?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&encode_header(&header))?;
    for entry in &entries {
        file.write_all(&encode_entry(entry))?;
    }
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, output).map_err(|source| PackedError::Io {
        path: output.to_path_buf(),
        source,
    })?;
    let artifact_hash = sha256_file(output)?;
    let packed = PackedCorpus::open(output)?;
    packed.verify_all_sections()?;
    if packed.source_corpus_sha256() != report.canonical_corpus_sha256
        || packed.packed_semantic_sha256() != hex(&packed_semantic_sha256)
    {
        return Err(PackedError::Contract(
            "packed reopen identity mismatch".into(),
        ));
    }
    Ok(PackReceipt {
        contract: PACKED_CONTRACT,
        status: "PASS",
        source_corpus_sha256: report.canonical_corpus_sha256,
        packed_semantic_sha256: hex(&packed_semantic_sha256),
        artifact_sha256: hex(&artifact_hash),
        schema_dictionary_sha256: schema_hash_hex,
        artifact_bytes: file_size,
        counts,
        artifact: output.to_path_buf(),
    })
}

fn relation_header(run_dirs: &[PathBuf], kind: RelationKind) -> Result<Vec<u8>, PackedError> {
    let first = dataset_path(&run_dirs[0], kind)?;
    let bytes = fs::read(&first).map_err(|source| PackedError::Io {
        path: first.clone(),
        source,
    })?;
    let end = memchr(b'\n', &bytes).unwrap_or(bytes.len());
    let expected = bytes[..end]
        .strip_suffix(b"\r")
        .unwrap_or(&bytes[..end])
        .to_vec();
    for run in &run_dirs[1..] {
        let path = dataset_path(run, kind)?;
        let table = MappedTsv::open(&path)?;
        if table.column_count() != expected.split(|byte| *byte == b'\t').count() {
            return Err(PackedError::Contract(format!(
                "{} header width drift",
                kind.name()
            )));
        }
        let bytes = fs::read(&path).map_err(|source| PackedError::Io { path, source })?;
        let end = memchr(b'\n', &bytes).unwrap_or(bytes.len());
        let actual = bytes[..end].strip_suffix(b"\r").unwrap_or(&bytes[..end]);
        if actual != expected {
            return Err(PackedError::Contract(format!(
                "{} header drift",
                kind.name()
            )));
        }
    }
    Ok(expected)
}

fn append_rows(
    path: &Path,
    writer: &mut impl Write,
    position: &mut u64,
    data_offset: u64,
    offsets: &mut Vec<u64>,
    section: &mut Sha256,
    semantic: &mut Sha256,
) -> Result<(), PackedError> {
    let file = File::open(path).map_err(|source| PackedError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line)?;
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        if line.is_empty() {
            continue;
        }
        writer.write_all(&line)?;
        writer.write_all(b"\n")?;
        section.update(&line);
        section.update(b"\n");
        semantic.update(&line);
        semantic.update(b"\n");
        *position += line.len() as u64 + 1;
        offsets.push(*position - data_offset);
    }
    Ok(())
}

fn dataset_path(run_dir: &Path, kind: RelationKind) -> Result<PathBuf, PackedError> {
    let suffix = format!("_{}.tsv", kind.name());
    let root = run_dir.join("raw/auction");
    let mut paths = fs::read_dir(&root)
        .map_err(|source| PackedError::Io {
            path: root.clone(),
            source,
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(&suffix))
        })
        .collect::<Vec<_>>();
    if paths.len() != 1 {
        return Err(PackedError::Contract(format!(
            "expected one {} relation in {}",
            kind.name(),
            root.display()
        )));
    }
    Ok(paths.pop().unwrap())
}

fn sha256_file(path: &Path) -> Result<[u8; 32], PackedError> {
    let bytes = fs::read(path).map_err(|source| PackedError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Sha256::digest(bytes).into())
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], PackedError> {
    if value.len() != 64 {
        return Err(PackedError::Contract(
            "SHA-256 must be 64 hex characters".into(),
        ));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair)
            .map_err(|_| PackedError::Contract("invalid hash UTF-8".into()))?;
        output[index] = u8::from_str_radix(text, 16)
            .map_err(|_| PackedError::Contract("invalid hash hex".into()))?;
    }
    Ok(output)
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 15)] as char);
    }
    output
}

//! Fail-closed reader for fresh MT5 auction and structural parity tapes.

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use hashbrown::HashSet;
use memchr::memchr;
use memmap2::MmapOptions;
use northstar_mt5_corpus::{MappedTsv, Row, parse_u64};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

const FILES: [(&str, &str); 5] = [
    ("frames.tsv", "MST_AUCTION_REPLAY_INPUT_V1"),
    ("levels.tsv", "MST_NORMALIZED_STRUCTURE_INPUT_V1"),
    ("nodes.tsv", "MST_STRUCTURAL_NODE_INPUT_V1"),
    ("sources.tsv", "MST_STRUCTURAL_PROVENANCE_INPUT_V1"),
    ("features.tsv", "MST_CAUSAL_FEATURE_INPUT_V1"),
];

#[derive(Debug, Error)]
pub enum OracleError {
    #[error("I/O failure at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("TSV failure: {0}")]
    Tsv(#[from] northstar_mt5_corpus::CorpusError),
    #[error("oracle contract failed: {0}")]
    Contract(String),
}
type Result<T> = std::result::Result<T, OracleError>;

#[derive(Clone, Debug, Serialize)]
pub struct OracleFileReceipt {
    pub name: String,
    pub rows: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OracleReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub run_key: String,
    pub invocation_id: String,
    pub frame_count: u64,
    pub files: Vec<OracleFileReceipt>,
    pub canonical_capture_sha256: String,
    pub prefix_stride: u64,
    pub frame_prefix_sha256: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DivergenceReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub last_matching_sequence: u64,
    pub first_divergent_block_start_sequence: Option<u64>,
    pub left_prefix_sha256: Option<String>,
    pub right_prefix_sha256: Option<String>,
}

pub fn verify_capture(root: &Path, prefix_stride: u64) -> Result<OracleReceipt> {
    if prefix_stride == 0 {
        return Err(OracleError::Contract(
            "prefix stride must be positive".into(),
        ));
    }
    let frames_path = root.join("frames.tsv");
    let frames = MappedTsv::open(&frames_path)?;
    let contract = frames.column("contract")?;
    let run = frames.column("run_key")?;
    let invocation = frames.column("invocation_id")?;
    let sequence = frames.column("sequence")?;
    let mut run_key: Option<Vec<u8>> = None;
    let mut invocation_id: Option<Vec<u8>> = None;
    let mut sequences = HashSet::new();
    for (expected, row) in (1_u64..).zip(frames.rows()) {
        let row = row?;
        required_contract(row, contract, &frames, FILES[0].1)?;
        let current_run = required(row, run, &frames)?;
        let current_invocation = required(row, invocation, &frames)?;
        if run_key
            .get_or_insert_with(|| current_run.to_vec())
            .as_slice()
            != current_run
            || invocation_id
                .get_or_insert_with(|| current_invocation.to_vec())
                .as_slice()
                != current_invocation
        {
            return Err(OracleError::Contract(
                "frame identity changed inside tape".into(),
            ));
        }
        let value = parse_u64(required(row, sequence, &frames)?)
            .ok_or_else(|| OracleError::Contract("invalid frame sequence".into()))?;
        if value != expected || !sequences.insert(value) {
            return Err(OracleError::Contract(format!(
                "noncontiguous frame sequence at {value}"
            )));
        }
    }
    if sequences.is_empty() {
        return Err(OracleError::Contract("capture has no frames".into()));
    }
    let run_key = String::from_utf8(run_key.unwrap())
        .map_err(|_| OracleError::Contract("run key is not UTF-8".into()))?;
    let invocation_id = String::from_utf8(invocation_id.unwrap())
        .map_err(|_| OracleError::Contract("invocation is not UTF-8".into()))?;
    let mut receipts = Vec::with_capacity(FILES.len());
    let mut capture_hash = Sha256::new();
    capture_hash.update(b"MST_PARITY_ORACLE_CAPTURE_V1\0");
    for &(name, expected_contract) in &FILES {
        let path = root.join(name);
        let table = MappedTsv::open(&path)?;
        let contract_column = table.column("contract")?;
        let run_column = table.column("run_key")?;
        let sequence_column = table.column("sequence")?;
        let mut count = 0_u64;
        let mut feature_sequences = HashSet::new();
        for row in table.rows() {
            let row = row?;
            required_contract(row, contract_column, &table, expected_contract)?;
            if required(row, run_column, &table)? != run_key.as_bytes() {
                return Err(OracleError::Contract(format!("foreign run key in {name}")));
            }
            let frame = parse_u64(required(row, sequence_column, &table)?)
                .ok_or_else(|| OracleError::Contract(format!("invalid sequence in {name}")))?;
            if !sequences.contains(&frame) {
                return Err(OracleError::Contract(format!(
                    "orphan sequence {frame} in {name}"
                )));
            }
            if name == "features.tsv" && !feature_sequences.insert(frame) {
                return Err(OracleError::Contract(format!(
                    "duplicate feature sequence {frame}"
                )));
            }
            count += 1;
        }
        if name == "features.tsv" && feature_sequences != sequences {
            return Err(OracleError::Contract(
                "feature snapshot is not exactly one per frame".into(),
            ));
        }
        let sha = sha256_file(&path)?;
        capture_hash.update(name.as_bytes());
        capture_hash.update([0]);
        capture_hash.update(sha.as_bytes());
        receipts.push(OracleFileReceipt {
            name: name.into(),
            rows: count,
            sha256: sha,
        });
    }
    Ok(OracleReceipt {
        contract: "NORTHSTAR_MT5_PARITY_ORACLE_RECEIPT_V1",
        status: "PASS",
        run_key,
        invocation_id,
        frame_count: sequences.len() as u64,
        files: receipts,
        canonical_capture_sha256: hex(&capture_hash.finalize()),
        prefix_stride,
        frame_prefix_sha256: prefix_hashes(&frames_path, prefix_stride)?,
    })
}

pub fn compare_prefixes(left: &OracleReceipt, right: &OracleReceipt) -> DivergenceReceipt {
    let matching = left
        .frame_prefix_sha256
        .iter()
        .zip(&right.frame_prefix_sha256)
        .take_while(|(a, b)| a == b)
        .count();
    let fully_equal = left.frame_count == right.frame_count
        && left.canonical_capture_sha256 == right.canonical_capture_sha256;
    DivergenceReceipt {
        contract: "NORTHSTAR_PARITY_DIVERGENCE_V1",
        status: if fully_equal { "MATCH" } else { "DIVERGED" },
        last_matching_sequence: (matching as u64 * left.prefix_stride)
            .min(left.frame_count.min(right.frame_count)),
        first_divergent_block_start_sequence: (!fully_equal).then_some(
            (matching as u64 * left.prefix_stride + 1).min(left.frame_count.max(right.frame_count)),
        ),
        left_prefix_sha256: left.frame_prefix_sha256.get(matching).cloned(),
        right_prefix_sha256: right.frame_prefix_sha256.get(matching).cloned(),
    }
}

fn prefix_hashes(path: &Path, stride: u64) -> Result<Vec<String>> {
    let bytes = fs::read(path).map_err(|source| OracleError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut cursor = memchr(b'\n', &bytes).map_or(bytes.len(), |value| value + 1);
    let mut rows = 0_u64;
    let mut hash = Sha256::new();
    hash.update(b"MST_AUCTION_REPLAY_PREFIX_V1\0");
    let mut output = Vec::new();
    while cursor < bytes.len() {
        let width = memchr(b'\n', &bytes[cursor..]).unwrap_or(bytes.len() - cursor);
        let line = bytes[cursor..cursor + width]
            .strip_suffix(b"\r")
            .unwrap_or(&bytes[cursor..cursor + width]);
        cursor += width + usize::from(cursor + width < bytes.len());
        if line.is_empty() {
            continue;
        }
        hash.update(line);
        hash.update(b"\n");
        rows += 1;
        if rows.is_multiple_of(stride) {
            output.push(hex(&hash.clone().finalize()));
        }
    }
    if !rows.is_multiple_of(stride) {
        output.push(hex(&hash.finalize()));
    }
    Ok(output)
}

fn required_contract(row: Row<'_>, index: usize, table: &MappedTsv, expected: &str) -> Result<()> {
    if required(row, index, table)? != expected.as_bytes() {
        return Err(OracleError::Contract(format!(
            "{} contract mismatch",
            table.path().display()
        )));
    }
    Ok(())
}
fn required<'a>(row: Row<'a>, index: usize, table: &MappedTsv) -> Result<&'a [u8]> {
    row.field(index)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            OracleError::Contract(format!(
                "missing value in {} row {}",
                table.path().display(),
                row.number()
            ))
        })
}
fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|source| OracleError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| OracleError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(hex(&Sha256::digest(&mmap)))
}
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(H[(b >> 4) as usize] as char);
        out.push(H[(b & 15) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_orphan_frame_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let definitions = [
            (
                "frames.tsv",
                "contract\trun_key\tinvocation_id\tsequence\nMST_AUCTION_REPLAY_INPUT_V1\tR\tI\t1\n",
            ),
            (
                "levels.tsv",
                "contract\trun_key\tsequence\nMST_NORMALIZED_STRUCTURE_INPUT_V1\tR\t2\n",
            ),
            ("nodes.tsv", "contract\trun_key\tsequence\n"),
            ("sources.tsv", "contract\trun_key\tsequence\n"),
            (
                "features.tsv",
                "contract\trun_key\tsequence\nMST_CAUSAL_FEATURE_INPUT_V1\tR\t1\n",
            ),
        ];
        for (name, body) in definitions {
            let mut file = File::create(dir.path().join(name)).unwrap();
            file.write_all(body.as_bytes()).unwrap();
        }
        assert!(verify_capture(dir.path(), 128).is_err());
    }

    #[test]
    fn rejects_duplicate_feature_sequence_even_when_counts_match() {
        let dir = tempfile::tempdir().unwrap();
        let definitions = [
            (
                "frames.tsv",
                "contract\trun_key\tinvocation_id\tsequence\nMST_AUCTION_REPLAY_INPUT_V1\tR\tI\t1\nMST_AUCTION_REPLAY_INPUT_V1\tR\tI\t2\n",
            ),
            ("levels.tsv", "contract\trun_key\tsequence\n"),
            ("nodes.tsv", "contract\trun_key\tsequence\n"),
            ("sources.tsv", "contract\trun_key\tsequence\n"),
            (
                "features.tsv",
                "contract\trun_key\tsequence\nMST_CAUSAL_FEATURE_INPUT_V1\tR\t1\nMST_CAUSAL_FEATURE_INPUT_V1\tR\t1\n",
            ),
        ];
        for (name, body) in definitions {
            let mut file = File::create(dir.path().join(name)).unwrap();
            file.write_all(body.as_bytes()).unwrap();
        }
        assert!(verify_capture(dir.path(), 128).is_err());
    }

    #[test]
    fn accepts_minimal_referentially_complete_capture() {
        let dir = tempfile::tempdir().unwrap();
        let definitions = [
            (
                "frames.tsv",
                "contract\trun_key\tinvocation_id\tsequence\nMST_AUCTION_REPLAY_INPUT_V1\tR\tI\t1\n",
            ),
            ("levels.tsv", "contract\trun_key\tsequence\n"),
            ("nodes.tsv", "contract\trun_key\tsequence\n"),
            ("sources.tsv", "contract\trun_key\tsequence\n"),
            (
                "features.tsv",
                "contract\trun_key\tsequence\nMST_CAUSAL_FEATURE_INPUT_V1\tR\t1\n",
            ),
        ];
        for (name, body) in definitions {
            let mut file = File::create(dir.path().join(name)).unwrap();
            file.write_all(body.as_bytes()).unwrap();
        }
        let receipt = verify_capture(dir.path(), 128).unwrap();
        assert_eq!(receipt.frame_count, 1);
        assert_eq!(receipt.run_key, "R");
        assert_eq!(receipt.invocation_id, "I");
        assert_eq!(receipt.files.len(), 5);
    }
}

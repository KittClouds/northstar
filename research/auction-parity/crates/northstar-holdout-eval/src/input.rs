use std::{
    fs::File,
    path::{Path, PathBuf},
};

use hashbrown::{HashMap, HashSet};
use memchr::memchr_iter;
use memmap2::MmapOptions;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, canonical, manifest::Preauthorization};

pub const BUNDLE_CONTRACT: &str = "NORTHSTAR_RG2_HOLDOUT_BUNDLE_V1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleManifest {
    pub contract: String,
    pub status: String,
    pub research_generation: u32,
    pub protocol_sha256: String,
    pub runs: Vec<BundleRun>,
    pub targets: Vec<TargetFile>,
    pub bundle_semantic_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleRun {
    pub run_key: String,
    pub canonical_instrument: String,
    pub broker_symbol: String,
    pub data_source_id: String,
    pub holdout_id: String,
    pub window_start: i64,
    pub window_end_exclusive: i64,
    pub run_receipt_file: String,
    pub run_receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetFile {
    pub target: String,
    pub file: String,
    pub canonical_sha256: String,
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub run_key: String,
    pub episode_id: String,
    pub instrument: String,
    pub holdout_id: String,
    pub censored: bool,
    pub label: Option<bool>,
    pub raw: HashMap<String, String>,
}

impl BundleManifest {
    pub fn load_and_validate(path: &Path, protocol: &Preauthorization) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        let value: Self = serde_json::from_slice(&bytes)?;
        if value.contract != BUNDLE_CONTRACT
            || value.status != "SEALED"
            || value.research_generation != protocol.research_generation
            || value.protocol_sha256 != protocol.protocol_sha256
            || value.runs.len() != protocol.reservations.len()
        {
            return Err(Error::Contract("holdout bundle identity mismatch".into()));
        }
        if canonical::json_hash_without(&bytes, "bundle_semantic_sha256")?
            != value.bundle_semantic_sha256
        {
            return Err(Error::Contract(
                "holdout bundle semantic hash mismatch".into(),
            ));
        }
        let expected = protocol
            .reservations
            .iter()
            .map(|r| {
                (
                    r.canonical_instrument.clone(),
                    r.broker_symbol.clone(),
                    r.data_source_id.clone(),
                    r.holdout_id.clone(),
                    r.window_start,
                    r.window_end_exclusive,
                )
            })
            .collect::<HashSet<_>>();
        let actual = value
            .runs
            .iter()
            .map(|r| {
                (
                    r.canonical_instrument.clone(),
                    r.broker_symbol.clone(),
                    r.data_source_id.clone(),
                    r.holdout_id.clone(),
                    r.window_start,
                    r.window_end_exclusive,
                )
            })
            .collect::<HashSet<_>>();
        if expected != actual
            || value.runs.iter().any(|r| {
                r.run_key.is_empty()
                    || r.run_receipt_file.is_empty()
                    || r.run_receipt_sha256.len() != 64
            })
            || value
                .runs
                .iter()
                .map(|r| r.run_key.as_str())
                .collect::<HashSet<_>>()
                .len()
                != value.runs.len()
        {
            return Err(Error::Contract(
                "bundle runs do not exactly match reservation allowlist".into(),
            ));
        }
        let expected_targets = protocol
            .candidates
            .iter()
            .map(|c| c.target.clone())
            .collect::<HashSet<_>>();
        let actual_targets = value
            .targets
            .iter()
            .map(|t| t.target.clone())
            .collect::<HashSet<_>>();
        if expected_targets != actual_targets || value.targets.len() != expected_targets.len() {
            return Err(Error::Contract("bundle target set mismatch".into()));
        }
        Ok(value)
    }
}

pub fn load_observations(
    root: &Path,
    row: &TargetFile,
    required_raw: impl Iterator<Item = String>,
    runs: &[BundleRun],
) -> Result<Vec<Observation>> {
    let path = safe_join(root, &row.file)?;
    if canonical::canonical_text_sha256(&path)? != row.canonical_sha256 {
        return Err(Error::Contract(format!(
            "{} target file hash mismatch",
            row.target
        )));
    }
    let file = File::open(&path).map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    let lines = line_ranges(&mmap);
    if lines.is_empty() {
        return Err(Error::Input(format!("{} input is empty", row.target)));
    }
    let header = fields(&mmap[lines[0].0..lines[0].1])?;
    let index = header
        .iter()
        .enumerate()
        .map(|(i, v)| (v.as_str(), i))
        .collect::<HashMap<_, _>>();
    let identity = [
        "target",
        "run_key",
        "episode_id",
        "attempt_id",
        "canonical_instrument",
        "holdout_id",
        "target_censored",
        "target_label",
    ];
    let raw_names = required_raw.collect::<Vec<_>>();
    for name in identity
        .iter()
        .copied()
        .chain(raw_names.iter().map(String::as_str))
    {
        if !index.contains_key(name) {
            return Err(Error::Input(format!(
                "{} missing required column {name}",
                row.target
            )));
        }
    }
    let allowed_runs = runs
        .iter()
        .map(|r| {
            (
                r.run_key.as_str(),
                (r.canonical_instrument.as_str(), r.holdout_id.as_str()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut output = Vec::with_capacity(lines.len().saturating_sub(1));
    let mut ids = HashSet::with_capacity(lines.len());
    for &(start, end) in &lines[1..] {
        if start == end {
            continue;
        }
        let values = fields(&mmap[start..end])?;
        if values.len() != header.len() {
            return Err(Error::Input(format!("{} ragged TSV row", row.target)));
        }
        let get = |name: &str| values[*index.get(name).expect("validated header")].clone();
        if get("target") != row.target {
            return Err(Error::Input("cross-target row detected".into()));
        }
        let run_key = get("run_key");
        let instrument = get("canonical_instrument");
        let holdout_id = get("holdout_id");
        if allowed_runs.get(run_key.as_str()).copied()
            != Some((instrument.as_str(), holdout_id.as_str()))
        {
            return Err(Error::Input(
                "observation references non-reserved run".into(),
            ));
        }
        let episode_id = get("episode_id");
        let attempt_id = get("attempt_id");
        if !ids.insert((run_key.clone(), episode_id.clone(), attempt_id.clone())) {
            return Err(Error::Input("duplicate observation identity".into()));
        }
        let censored = parse_bool(&get("target_censored"))?;
        let label_text = get("target_label");
        let label = if label_text == "\\N" || label_text.is_empty() {
            None
        } else {
            Some(parse_bool(&label_text)?)
        };
        if censored == label.is_some() {
            return Err(Error::Input("censoring and label contract mismatch".into()));
        }
        let mut raw = HashMap::with_capacity(raw_names.len());
        for name in &raw_names {
            raw.insert(name.clone(), get(name));
        }
        output.push(Observation {
            run_key,
            episode_id,
            instrument,
            holdout_id,
            censored,
            label,
            raw,
        });
    }
    Ok(output)
}

pub(crate) fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error::Contract("unsafe bundle path".into()));
    }
    Ok(root.join(path))
}

fn parse_bool(text: &str) -> Result<bool> {
    match text {
        "1" | "true" | "TRUE" => Ok(true),
        "0" | "false" | "FALSE" => Ok(false),
        _ => Err(Error::Input(format!("invalid bool {text}"))),
    }
}

fn line_ranges(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut output = Vec::new();
    let mut start = 0;
    for end in memchr_iter(b'\n', bytes) {
        output.push((
            start,
            end - usize::from(end > start && bytes[end - 1] == b'\r'),
        ));
        start = end + 1;
    }
    if start < bytes.len() {
        output.push((start, bytes.len()));
    }
    output
}

fn fields(line: &[u8]) -> Result<Vec<String>> {
    let mut output = Vec::new();
    let mut start = 0;
    for end in memchr_iter(b'\t', line) {
        output.push(text(&line[start..end])?);
        start = end + 1;
    }
    output.push(text(&line[start..])?);
    Ok(output)
}

fn text(bytes: &[u8]) -> Result<String> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| Error::Input("TSV is not UTF-8".into()))
}

use hashbrown::{HashMap, HashSet};
use memchr::memchr_iter;
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const DATASETS: [&str; 9] = [
    "measurement_runs",
    "compression_objects",
    "compression_samples",
    "compression_events",
    "expansion_objects",
    "expansion_samples",
    "expansion_events",
    "object_relations",
    "structural_samples",
];

#[derive(Debug, Error)]
pub enum RawError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("missing column {column} in {dataset}")]
    MissingColumn { dataset: String, column: String },
    #[error("invalid UTF-8 or numeric value in {dataset}: {detail}")]
    Invalid { dataset: String, detail: String },
    #[error("RG3 invariant failed: {0}")]
    Invariant(String),
}

struct MappedDataset {
    name: &'static str,
    mmap: Mmap,
    header: HashMap<String, usize>,
    lines: Vec<(usize, usize)>,
}

impl MappedDataset {
    fn open(path: &Path, name: &'static str) -> Result<Self, RawError> {
        let file = File::open(path).map_err(|source| RawError::Io {
            path: path.into(),
            source,
        })?;
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|source| RawError::Io {
            path: path.into(),
            source,
        })?;
        let mut lines = Vec::new();
        let mut start = 0;
        for end in memchr_iter(b'\n', &mmap) {
            if end > start {
                lines.push((start, end - usize::from(mmap[end - 1] == b'\r')));
            }
            start = end + 1;
        }
        if start < mmap.len() {
            lines.push((start, mmap.len()));
        }
        let Some(&(hs, he)) = lines.first() else {
            return Err(RawError::Invalid {
                dataset: name.into(),
                detail: "empty file".into(),
            });
        };
        let header = std::str::from_utf8(&mmap[hs..he])
            .map_err(|_| RawError::Invalid {
                dataset: name.into(),
                detail: "header UTF-8".into(),
            })?
            .split('\t')
            .enumerate()
            .map(|(i, value)| (value.to_owned(), i))
            .collect();
        Ok(Self {
            name,
            mmap,
            header,
            lines,
        })
    }

    fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.lines.iter().skip(1).map(|&(s, e)| Row {
            dataset: self,
            bytes: &self.mmap[s..e],
        })
    }

    fn raw(&self) -> &[u8] {
        &self.mmap
    }
}

pub(crate) struct Row<'a> {
    dataset: &'a MappedDataset,
    bytes: &'a [u8],
}
impl Row<'_> {
    pub(crate) fn field(&self, name: &str) -> Result<&str, RawError> {
        let index = *self
            .dataset
            .header
            .get(name)
            .ok_or_else(|| RawError::MissingColumn {
                dataset: self.dataset.name.into(),
                column: name.into(),
            })?;
        let mut start = 0;
        for (current, end) in memchr_iter(b'\t', self.bytes)
            .chain(std::iter::once(self.bytes.len()))
            .enumerate()
        {
            if current == index {
                return std::str::from_utf8(&self.bytes[start..end]).map_err(|_| {
                    RawError::Invalid {
                        dataset: self.dataset.name.into(),
                        detail: format!("field {name} UTF-8"),
                    }
                });
            }
            start = end + 1;
        }
        Err(RawError::Invalid {
            dataset: self.dataset.name.into(),
            detail: format!("short row for {name}"),
        })
    }
    pub(crate) fn i64(&self, name: &str) -> Result<i64, RawError> {
        self.field(name)?.parse().map_err(|_| RawError::Invalid {
            dataset: self.dataset.name.into(),
            detail: format!("field {name} is not i64"),
        })
    }
    pub(crate) fn usize(&self, name: &str) -> Result<usize, RawError> {
        self.field(name)?.parse().map_err(|_| RawError::Invalid {
            dataset: self.dataset.name.into(),
            detail: format!("field {name} is not usize"),
        })
    }
    pub(crate) fn f64(&self, name: &str) -> Result<f64, RawError> {
        self.field(name)?.parse().map_err(|_| RawError::Invalid {
            dataset: self.dataset.name.into(),
            detail: format!("field {name} is not f64"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReport {
    pub contract: String,
    pub run_key: String,
    pub byte_sha256: String,
    pub canonical_sha256: String,
    pub rows: HashMap<String, usize>,
    pub balanced: bool,
}

pub struct RawCorpus {
    root: PathBuf,
    stem: String,
    datasets: Vec<MappedDataset>,
    report: RawReport,
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

impl RawCorpus {
    pub fn open(root: impl AsRef<Path>, stem: impl Into<String>) -> Result<Self, RawError> {
        let root = root.as_ref().to_path_buf();
        let stem = stem.into();
        let mut datasets = Vec::with_capacity(DATASETS.len());
        for name in DATASETS {
            let file_name = if stem.is_empty() {
                format!("{name}.tsv")
            } else {
                format!("{stem}_{name}.tsv")
            };
            datasets.push(MappedDataset::open(&root.join(file_name), name)?);
        }
        let mut corpus = Self {
            root,
            stem,
            datasets,
            report: RawReport {
                contract: String::new(),
                run_key: String::new(),
                byte_sha256: String::new(),
                canonical_sha256: String::new(),
                rows: HashMap::new(),
                balanced: false,
            },
        };
        corpus.report = corpus.validate()?;
        Ok(corpus)
    }

    fn dataset(&self, name: &str) -> &MappedDataset {
        &self.datasets[DATASETS
            .iter()
            .position(|candidate| *candidate == name)
            .expect("known dataset")]
    }

    fn validate(&self) -> Result<RawReport, RawError> {
        let runs = self.dataset("measurement_runs");
        let run_rows: Vec<_> = runs.rows().collect();
        if run_rows.len() != 2
            || run_rows[0].field("record_type")? != "START"
            || run_rows[1].field("record_type")? != "END"
        {
            return Err(RawError::Invariant(
                "measurement_runs must contain exactly START then END".into(),
            ));
        }
        let contract = run_rows[1].field("contract")?.to_owned();
        if contract != "NORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1"
            || run_rows[1].field("research_generation")? != "3"
        {
            return Err(RawError::Invariant(
                "wrong RG3 contract or generation".into(),
            ));
        }
        let run_key = run_rows[1].field("run_key")?.to_owned();
        if run_rows[0].field("run_key")? != run_key || run_rows[1].field("balanced")? != "1" {
            return Err(RawError::Invariant(
                "run identity mismatch or unbalanced END".into(),
            ));
        }
        for field in [
            "invocation_id",
            "raw_schema_version",
            "compression_grammar_version",
            "expansion_grammar_version",
            "canonical_instrument",
            "broker_symbol",
            "timeframe",
            "window_start",
            "window_end",
            "config_hash",
            "data_source_id",
            "data_fingerprint",
        ] {
            if run_rows[0].field(field)? != run_rows[1].field(field)? {
                return Err(RawError::Invariant(format!(
                    "run identity field changed: {field}"
                )));
            }
        }
        for forbidden in [
            "raw_return_ratio",
            "raw_path_efficiency",
            "raw_tortuosity",
            "raw_oriented_efficiency",
        ] {
            if self
                .dataset("expansion_samples")
                .header
                .contains_key(forbidden)
            {
                return Err(RawError::Invariant(format!(
                    "derived column leaked into raw expansion samples: {forbidden}"
                )));
            }
        }
        for forbidden in [
            "price_from_median_atr",
            "price_from_median_sigma",
            "price_from_cog_atr",
            "price_from_cog_sigma",
        ] {
            if self
                .dataset("structural_samples")
                .header
                .contains_key(forbidden)
            {
                return Err(RawError::Invariant(format!(
                    "derived column leaked into raw structural samples: {forbidden}"
                )));
            }
        }
        #[derive(Clone, Copy)]
        struct CompressionMeta {
            terminal_time: i64,
            censored: bool,
            sample_count: usize,
            top: f64,
            bottom: f64,
            mid: f64,
        }
        let mut compression_meta = HashMap::new();
        for row in self.dataset("compression_objects").rows() {
            let id = row.i64("compression_id")?;
            let meta = CompressionMeta {
                terminal_time: row.i64("terminal_time")?,
                censored: (5..=8).contains(&row.i64("terminal_reason_code")?),
                sample_count: row.usize("sample_count")?,
                top: row.f64("terminal_contain_top")?,
                bottom: row.f64("terminal_contain_bottom")?,
                mid: row.f64("terminal_contain_mid")?,
            };
            if compression_meta.insert(id, meta).is_some() {
                return Err(RawError::Invariant(
                    "duplicate compression primary key".into(),
                ));
            }
        }
        let compression_ids: HashSet<i64> = compression_meta.keys().copied().collect();
        let expansion_rows: Vec<_> = self.dataset("expansion_objects").rows().collect();
        #[derive(Clone, Copy)]
        struct ExpansionMeta {
            terminal_time: i64,
            censored: bool,
            sample_count: usize,
            terminal_price: f64,
            max: f64,
            opposite: f64,
            return_depth: f64,
            path: f64,
        }
        let mut expansion_meta = HashMap::new();
        for row in &expansion_rows {
            let id = row.i64("expansion_id")?;
            let summary = ExpansionMeta {
                terminal_time: row.i64("terminal_time")?,
                censored: (5..=8).contains(&row.i64("terminal_reason_code")?),
                sample_count: row.usize("sample_count")?,
                terminal_price: row.f64("terminal_price")?,
                max: row.f64("max_displacement")?,
                opposite: row.f64("opposite_displacement")?,
                return_depth: row.f64("return_depth")?,
                path: row.f64("close_path_length")?,
            };
            if expansion_meta.insert(id, summary).is_some() {
                return Err(RawError::Invariant(
                    "duplicate expansion primary key".into(),
                ));
            }
        }
        let expansion_ids: HashSet<i64> = expansion_meta.keys().copied().collect();

        type CompressionTail = (usize, i64, f64, f64, f64);
        let mut compression_tail: HashMap<i64, CompressionTail> = HashMap::new();
        for row in self.dataset("compression_samples").rows() {
            let id = row.i64("compression_id")?;
            let Some(meta) = compression_meta.get(&id) else {
                return Err(RawError::Invariant(
                    "compression sample references missing object".into(),
                ));
            };
            let time = row.i64("bar_time")?;
            let previous = compression_tail.get(&id).map(|tail| tail.1);
            if time > meta.terminal_time || previous.is_some_and(|last| time <= last) {
                return Err(RawError::Invariant(
                    "compression samples not strictly ordered or after terminal".into(),
                ));
            }
            let count = compression_tail.get(&id).map_or(1, |tail| tail.0 + 1);
            compression_tail.insert(
                id,
                (
                    count,
                    time,
                    row.f64("contain_top")?,
                    row.f64("contain_bottom")?,
                    row.f64("contain_mid")?,
                ),
            );
        }
        for (&id, meta) in &compression_meta {
            let Some(&(count, time, top, bottom, mid)) = compression_tail.get(&id) else {
                if meta.sample_count == 0 {
                    continue;
                }
                return Err(RawError::Invariant(format!(
                    "compression {id} has no samples"
                )));
            };
            if count != meta.sample_count
                || (!meta.censored && time != meta.terminal_time)
                || (top - meta.top).abs() > 1e-9
                || (bottom - meta.bottom).abs() > 1e-9
                || (mid - meta.mid).abs() > 1e-9
            {
                return Err(RawError::Invariant(format!(
                    "compression {id} terminal reduction mismatch"
                )));
            }
        }

        let mut event_sequences = HashSet::new();
        for name in ["compression_events", "expansion_events"] {
            for row in self.dataset(name).rows() {
                let id_column = if name == "compression_events" {
                    "compression_id"
                } else {
                    "expansion_id"
                };
                let valid = if name == "compression_events" {
                    compression_ids.contains(&row.i64(id_column)?)
                } else {
                    expansion_ids.contains(&row.i64(id_column)?)
                };
                if !valid {
                    return Err(RawError::Invariant(format!(
                        "{name} references missing object"
                    )));
                }
                if !event_sequences.insert(row.i64("sequence")?) {
                    return Err(RawError::Invariant(
                        "duplicate cross-stream event sequence".into(),
                    ));
                }
            }
        }
        for row in &expansion_rows {
            if !compression_ids.contains(&row.i64("origin_compression_id")?) {
                return Err(RawError::Invariant(
                    "expansion origin compression missing".into(),
                ));
            }
            let destination = row.field("destination_compression_id")?;
            if destination != "\\N"
                && !compression_ids.contains(&row.i64("destination_compression_id")?)
            {
                return Err(RawError::Invariant(
                    "expansion destination compression missing".into(),
                ));
            }
        }
        type ExpansionTail = (usize, i64, f64, f64, f64, f64, f64);
        let mut expansion_tail: HashMap<i64, ExpansionTail> = HashMap::new();
        for row in self.dataset("expansion_samples").rows() {
            let id = row.i64("expansion_id")?;
            let Some(meta) = expansion_meta.get(&id) else {
                return Err(RawError::Invariant(
                    "expansion sample references missing object".into(),
                ));
            };
            let time = row.i64("bar_time")?;
            let previous = expansion_tail.get(&id).map(|tail| tail.1);
            if time > meta.terminal_time || previous.is_some_and(|last| time <= last) {
                return Err(RawError::Invariant(
                    "expansion samples not strictly ordered or after terminal".into(),
                ));
            }
            let count = expansion_tail.get(&id).map_or(1, |tail| tail.0 + 1);
            expansion_tail.insert(
                id,
                (
                    count,
                    time,
                    row.f64("close")?,
                    row.f64("raw_max_displacement")?,
                    row.f64("raw_opposite_displacement")?,
                    row.f64("raw_return_depth")?,
                    row.f64("raw_close_path_length")?,
                ),
            );
        }
        for (&id, meta) in &expansion_meta {
            let Some(&(
                count,
                sample_time,
                sample_price,
                sample_max,
                sample_opposite,
                sample_return,
                sample_path,
            )) = expansion_tail.get(&id)
            else {
                return Err(RawError::Invariant(format!(
                    "expansion {id} has no samples"
                )));
            };
            if count != meta.sample_count
                || (!meta.censored && sample_time != meta.terminal_time)
                || (sample_price - meta.terminal_price).abs() > 1e-9
                || (sample_max - meta.max).abs() > 1e-9
                || (sample_opposite - meta.opposite).abs() > 1e-9
                || (sample_return - meta.return_depth).abs() > 1e-9
                || (sample_path - meta.path).abs() > 1e-9
            {
                return Err(RawError::Invariant(format!(
                    "expansion {id} terminal reduction mismatch"
                )));
            }
        }
        let relation_rows: Vec<_> = self.dataset("object_relations").rows().collect();
        let mut relation_ids = HashSet::new();
        for row in &relation_rows {
            if !relation_ids.insert(row.i64("relation_id")?) {
                return Err(RawError::Invariant("duplicate relation primary key".into()));
            }
            if row.field("source_kind")? == "EXPANSION"
                && !expansion_ids.contains(&row.i64("source_id")?)
            {
                return Err(RawError::Invariant(
                    "relation source expansion missing".into(),
                ));
            }
        }
        let mut structural_times = HashSet::new();
        for row in self.dataset("structural_samples").rows() {
            if !structural_times.insert(row.i64("bar_time")?) {
                return Err(RawError::Invariant(
                    "duplicate structural sample time".into(),
                ));
            }
        }
        let end = &run_rows[1];
        let compression_completed = end.usize("compression_completed")?;
        let compression_censored = end.usize("compression_censored")?;
        let expansion_completed = end.usize("expansion_completed")?;
        let expansion_censored = end.usize("expansion_censored")?;
        if end.usize("compression_started")? != compression_meta.len()
            || compression_completed + compression_censored + end.usize("compression_active")?
                != compression_meta.len()
            || end.usize("expansion_started")? != expansion_meta.len()
            || expansion_completed + expansion_censored + end.usize("expansion_active")?
                != expansion_meta.len()
            || end.usize("compression_samples")?
                != self.dataset("compression_samples").lines.len() - 1
            || end.usize("expansion_samples")? != self.dataset("expansion_samples").lines.len() - 1
            || end.usize("relations")? != relation_rows.len()
        {
            return Err(RawError::Invariant(
                "END balance does not match admitted rows".into(),
            ));
        }
        for dataset in &self.datasets {
            for row in dataset.rows() {
                if dataset.name != "measurement_runs" && row.field("run_key")? != run_key {
                    return Err(RawError::Invariant(format!(
                        "{} contains foreign run_key",
                        dataset.name
                    )));
                }
            }
        }
        let mut byte_hasher = Sha256::new();
        let mut canonical_hasher = Sha256::new();
        let mut rows = HashMap::with_capacity(DATASETS.len());
        for dataset in &self.datasets {
            byte_hasher.update(dataset.name.as_bytes());
            byte_hasher.update([0]);
            byte_hasher.update(dataset.raw());
            byte_hasher.update([0xff]);

            canonical_hasher.update(dataset.name.as_bytes());
            canonical_hasher.update([0]);
            let invocation_column = (dataset.name == "measurement_runs")
                .then(|| dataset.header.get("invocation_id").copied())
                .flatten();
            for &(start, end) in &dataset.lines {
                for (column, value) in dataset.mmap[start..end]
                    .split(|byte| *byte == b'\t')
                    .enumerate()
                {
                    if Some(column) == invocation_column {
                        continue;
                    }
                    canonical_hasher.update((value.len() as u64).to_le_bytes());
                    canonical_hasher.update(value);
                }
                canonical_hasher.update([0xfe]);
            }
            canonical_hasher.update([0xff]);
            rows.insert(
                dataset.name.to_owned(),
                dataset.lines.len().saturating_sub(1),
            );
        }
        Ok(RawReport {
            contract,
            run_key,
            byte_sha256: hex(byte_hasher.finalize()),
            canonical_sha256: hex(canonical_hasher.finalize()),
            rows,
            balanced: true,
        })
    }

    pub fn report(&self) -> &RawReport {
        &self.report
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn stem(&self) -> &str {
        &self.stem
    }
    pub(crate) fn raw_dataset(&self, name: &str) -> &[u8] {
        self.dataset(name).raw()
    }
    pub(crate) fn rows(&self, name: &str) -> impl Iterator<Item = Row<'_>> {
        self.dataset(name).rows()
    }

    pub fn missing_counts(&self) -> Result<HashMap<String, usize>, RawError> {
        let mut counts = HashMap::new();
        for dataset in &self.datasets {
            for row in dataset.rows() {
                for column in dataset.header.keys() {
                    if row.field(column)? == "\\N" {
                        *counts
                            .entry(format!("{}.{}", dataset.name, column))
                            .or_insert(0) += 1;
                    }
                }
            }
        }
        Ok(counts)
    }
}

#[cfg(test)]
pub(crate) fn write_fixture(root: &Path, stem: &str) {
    let files = [
        (
            "measurement_runs",
            "contract\trecord_type\trun_key\tinvocation_id\tresearch_generation\traw_schema_version\tcompression_grammar_version\texpansion_grammar_version\tcanonical_instrument\tbroker_symbol\ttimeframe\twindow_start\twindow_end\tconfig_hash\tdata_source_id\tdata_fingerprint\tbalanced\tcompression_started\tcompression_completed\tcompression_censored\tcompression_active\texpansion_started\texpansion_completed\texpansion_censored\texpansion_active\tcompression_samples\texpansion_samples\trelations\nNORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1\tSTART\tR1\tI1\t3\t1\t2\t1\tUS30\tUS30\t5\t1\t200\t42\tFIXTURE\tF1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\nNORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1\tEND\tR1\tI1\t3\t1\t2\t1\tUS30\tUS30\t5\t1\t200\t42\tFIXTURE\tF1\t1\t2\t2\t0\t0\t1\t1\t0\t0\t1\t1\t1\n",
        ),
        (
            "compression_objects",
            "run_key\tcompression_id\tterminal_time\tterminal_reason_code\tsample_count\tterminal_contain_top\tterminal_contain_bottom\tterminal_contain_mid\nR1\t1\t100\t1\t1\t101\t99\t100\nR1\t2\t200\t2\t0\t102\t98\t100\n",
        ),
        (
            "compression_samples",
            "run_key\tcompression_id\tbar_time\tcontain_top\tcontain_bottom\tcontain_mid\nR1\t1\t100\t101\t99\t100\n",
        ),
        (
            "compression_events",
            "run_key\tcompression_id\tsequence\nR1\t1\t1\n",
        ),
        (
            "expansion_objects",
            "run_key\texpansion_id\torigin_compression_id\tdestination_compression_id\tterminal_time\tterminal_reason_code\tsample_count\tterminal_price\tmax_displacement\topposite_displacement\treturn_depth\tclose_path_length\nR1\t1\t1\t2\t100\t4\t1\t103.0\t3.5\t0.0\t0.5\t4.0\n",
        ),
        (
            "expansion_samples",
            "run_key\texpansion_id\tclose\torigin_atr\traw_displacement\traw_max_displacement\traw_opposite_displacement\traw_return_depth\traw_close_path_length\traw_velocity_per_bar\tbar_time\nR1\t1\t103.0\t2.0\t3.0\t3.5\t0.0\t0.5\t4.0\t0.5\t100\n",
        ),
        (
            "expansion_events",
            "run_key\texpansion_id\tsequence\nR1\t1\t2\n",
        ),
        (
            "object_relations",
            "run_key\trelation_id\tsource_kind\tsource_id\nR1\t1\tEXPANSION\t1\n",
        ),
        ("structural_samples", "run_key\tbar_time\nR1\t100\n"),
    ];
    for (name, body) in files {
        std::fs::write(root.join(format!("{stem}_{name}.tsv")), body).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fmt::Write as _, time::Instant};
    #[test]
    fn verifies_relational_fixture() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(dir.path(), "fixture");
        let corpus = RawCorpus::open(dir.path(), "fixture").unwrap();
        assert_eq!(corpus.report.rows["expansion_samples"], 1);
        assert!(corpus.report.balanced);
        drop(corpus);
        for name in DATASETS {
            std::fs::rename(
                dir.path().join(format!("fixture_{name}.tsv")),
                dir.path().join(format!("{name}.tsv")),
            )
            .unwrap();
        }
        assert!(RawCorpus::open(dir.path(), "").unwrap().report.balanced);
    }

    #[test]
    fn canonical_hash_excludes_unique_invocation_identity() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        write_fixture(first.path(), "fixture");
        write_fixture(second.path(), "fixture");
        let run_path = second.path().join("fixture_measurement_runs.tsv");
        let changed = std::fs::read_to_string(&run_path)
            .unwrap()
            .replace("\tI1\t", "\tI2\t");
        std::fs::write(run_path, changed).unwrap();
        let first = RawCorpus::open(first.path(), "fixture").unwrap();
        let second = RawCorpus::open(second.path(), "fixture").unwrap();
        assert_ne!(first.report.byte_sha256, second.report.byte_sha256);
        assert_eq!(
            first.report.canonical_sha256,
            second.report.canonical_sha256
        );
    }

    #[test]
    #[ignore = "explicit 100k-row mmap performance lane"]
    fn validates_one_hundred_thousand_samples() {
        const N: usize = 100_000;
        let dir = tempfile::tempdir().unwrap();
        let stem = "perf";
        let runs = format!(
            "contract\trecord_type\trun_key\tinvocation_id\tresearch_generation\traw_schema_version\tcompression_grammar_version\texpansion_grammar_version\tcanonical_instrument\tbroker_symbol\ttimeframe\twindow_start\twindow_end\tconfig_hash\tdata_source_id\tdata_fingerprint\tbalanced\tcompression_started\tcompression_completed\tcompression_censored\tcompression_active\texpansion_started\texpansion_completed\texpansion_censored\texpansion_active\tcompression_samples\texpansion_samples\trelations\nNORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1\tSTART\tR1\tI1\t3\t1\t2\t1\tUS30\tUS30\t5\t1\t200\t42\tFIXTURE\tF1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\nNORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1\tEND\tR1\tI1\t3\t1\t2\t1\tUS30\tUS30\t5\t1\t200\t42\tFIXTURE\tF1\t1\t1\t1\t0\t0\t1\t1\t0\t0\t1\t{N}\t1\n"
        );
        std::fs::write(
            dir.path().join(format!("{stem}_measurement_runs.tsv")),
            runs,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_compression_objects.tsv")),
            "run_key\tcompression_id\tterminal_time\tterminal_reason_code\tsample_count\tterminal_contain_top\tterminal_contain_bottom\tterminal_contain_mid\nR1\t1\t100\t1\t1\t101\t99\t100\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_compression_samples.tsv")),
            "run_key\tcompression_id\tbar_time\tcontain_top\tcontain_bottom\tcontain_mid\nR1\t1\t100\t101\t99\t100\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_compression_events.tsv")),
            "run_key\tcompression_id\tsequence\nR1\t1\t1\n",
        )
        .unwrap();
        let terminal = 100 + N as i64 - 1;
        let final_displacement = (N - 1) as f64;
        std::fs::write(dir.path().join(format!("{stem}_expansion_objects.tsv")), format!(
            "run_key\texpansion_id\torigin_compression_id\tdestination_compression_id\tterminal_time\tterminal_reason_code\tsample_count\tterminal_price\tmax_displacement\topposite_displacement\treturn_depth\tclose_path_length\nR1\t1\t1\t\\N\t{terminal}\t4\t{N}\t{}\t{}\t0\t0\t{}\n",
            100.0 + final_displacement, final_displacement, final_displacement)).unwrap();
        let mut samples = String::with_capacity(N * 80);
        samples.push_str("run_key\texpansion_id\tclose\torigin_atr\traw_displacement\traw_max_displacement\traw_opposite_displacement\traw_return_depth\traw_close_path_length\traw_velocity_per_bar\tbar_time\n");
        for i in 0..N {
            let displacement = i as f64;
            writeln!(
                samples,
                "R1\t1\t{}\t2\t{displacement}\t{displacement}\t0\t0\t{displacement}\t{}\t{}",
                100.0 + displacement,
                usize::from(i > 0),
                100 + i
            )
            .unwrap();
        }
        std::fs::write(
            dir.path().join(format!("{stem}_expansion_samples.tsv")),
            samples,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_expansion_events.tsv")),
            "run_key\texpansion_id\tsequence\nR1\t1\t2\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_object_relations.tsv")),
            "run_key\trelation_id\tsource_kind\tsource_id\nR1\t1\tEXPANSION\t1\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(format!("{stem}_structural_samples.tsv")),
            "run_key\tbar_time\nR1\t100\n",
        )
        .unwrap();
        let started = Instant::now();
        let corpus = RawCorpus::open(dir.path(), stem).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(corpus.report.rows["expansion_samples"], N);
        eprintln!("RG3_PERF rows={N} elapsed_us={}", elapsed.as_micros());
    }
}

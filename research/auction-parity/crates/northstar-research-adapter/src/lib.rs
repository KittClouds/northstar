//! Read-only Phase 10.5 target adapter over packed RG2 bytes.

use std::{
    fs,
    path::{Path, PathBuf},
};

use hashbrown::HashMap;
use northstar_auction_contract::{AuctionResolution, CompletionStatus, NULL_TOKEN};
use northstar_packed_corpus::{PackedCorpus, PackedError, PackedRow, PackedSection, RelationKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const ADAPTER_CONTRACT: &str = "NORTHSTAR_RG2_PACKED_RESEARCH_ADAPTER_V1";

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error(transparent)]
    Packed(#[from] PackedError),
    #[error("adapter contract failed: {0}")]
    Contract(String),
    #[error("I/O failure at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("JSON failure at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

type Result<T> = std::result::Result<T, AdapterError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetUniverse {
    pub target: String,
    pub identity_kind: String,
    pub eligible_count: u64,
    pub observed_count: u64,
    pub censored_count: u64,
    pub analyzable_count: u64,
    pub eligible_ids_sha256: String,
    pub observed_ids_sha256: String,
    pub censored_ids_sha256: String,
    pub analyzable_ids_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetRegistry {
    pub contract: String,
    pub interface_version: String,
    pub source_corpus_sha256: String,
    pub targets: Vec<TargetUniverse>,
}

#[derive(Debug, Serialize)]
pub struct AdapterReceipt {
    pub contract: &'static str,
    pub status: &'static str,
    pub source_corpus_sha256: String,
    pub packed_semantic_sha256: String,
    pub views: ViewCounts,
    pub targets: Vec<TargetUniverse>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ViewCounts {
    pub runs: u64,
    pub attempts: u64,
    pub attempt_chains: u64,
    pub episodes: u64,
    pub event_timeline: u64,
    pub transits: u64,
    pub contributors: u64,
    pub features: u64,
}

#[derive(Clone, Copy)]
struct Attempt<'a> {
    run: &'a [u8],
    attempt: u64,
    episode: u64,
    is_retest: bool,
    contact: bool,
    broken: bool,
    end: Option<u64>,
    resolution: AuctionResolution,
    completion: CompletionStatus,
}

#[derive(Clone, Copy)]
struct Episode<'a> {
    run: &'a [u8],
    episode: u64,
    end: Option<u64>,
    resolution: AuctionResolution,
    completion: CompletionStatus,
}

#[derive(Clone, Copy)]
struct Transit<'a> {
    run: &'a [u8],
    episode: u64,
    start: Option<u64>,
    resolution: AuctionResolution,
}

pub struct ResearchAdapter {
    packed: PackedCorpus,
}

impl ResearchAdapter {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let packed = PackedCorpus::open(path)?;
        packed.verify_all_sections()?;
        Ok(Self { packed })
    }

    pub fn verify_registry(&self, registry_path: &Path) -> Result<AdapterReceipt> {
        let bytes = fs::read(registry_path).map_err(|source| AdapterError::Io {
            path: registry_path.to_path_buf(),
            source,
        })?;
        self.verify_registry_bytes(registry_path, &bytes)
    }

    pub fn verify_registry_with_sha(
        &self,
        registry_path: &Path,
        expected_sha256: &str,
    ) -> Result<AdapterReceipt> {
        let bytes = fs::read(registry_path).map_err(|source| AdapterError::Io {
            path: registry_path.to_path_buf(),
            source,
        })?;
        let actual = hex(&Sha256::digest(&bytes));
        if actual != expected_sha256 {
            return Err(AdapterError::Contract(
                "target registry SHA-256 mismatch".into(),
            ));
        }
        self.verify_registry_bytes(registry_path, &bytes)
    }

    fn verify_registry_bytes(&self, registry_path: &Path, bytes: &[u8]) -> Result<AdapterReceipt> {
        let expected: TargetRegistry =
            serde_json::from_slice(bytes).map_err(|source| AdapterError::Json {
                path: registry_path.to_path_buf(),
                source,
            })?;
        if expected.contract != "MST_RG2_TARGET_ID_REGISTRY_V1"
            || expected.source_corpus_sha256 != self.packed.source_corpus_sha256()
        {
            return Err(AdapterError::Contract(
                "target registry identity mismatch".into(),
            ));
        }
        let targets = self.target_universes()?;
        if targets != expected.targets {
            let actual = targets
                .iter()
                .map(|row| (row.target.as_str(), row))
                .collect::<HashMap<_, _>>();
            let mismatch = expected
                .targets
                .iter()
                .find(|row| actual.get(row.target.as_str()) != Some(row));
            return Err(AdapterError::Contract(format!(
                "target ID-set mismatch: {:?}",
                mismatch.map(|row| &row.target)
            )));
        }
        Ok(AdapterReceipt {
            contract: ADAPTER_CONTRACT,
            status: "PASS",
            source_corpus_sha256: self.packed.source_corpus_sha256(),
            packed_semantic_sha256: self.packed.packed_semantic_sha256(),
            views: self.view_counts(),
            targets,
        })
    }

    pub fn view_counts(&self) -> ViewCounts {
        ViewCounts {
            runs: self.packed.section(RelationKind::Runs).len() as u64,
            attempts: self.packed.section(RelationKind::Attempts).len() as u64,
            attempt_chains: self.packed.section(RelationKind::Attempts).len() as u64,
            episodes: self.packed.section(RelationKind::Episodes).len() as u64,
            event_timeline: self.packed.section(RelationKind::Events).len() as u64,
            transits: self.packed.section(RelationKind::Transits).len() as u64,
            contributors: self.packed.section(RelationKind::Context).len() as u64,
            features: self.packed.section(RelationKind::Features).len() as u64,
        }
    }

    pub fn target_universes(&self) -> Result<Vec<TargetUniverse>> {
        let attempts = load_attempts(&self.packed.section(RelationKind::Attempts))?;
        let episodes = load_episodes(&self.packed.section(RelationKind::Episodes))?;
        let transits = load_transits(&self.packed.section(RelationKind::Transits))?;
        Ok(vec![
            attempt_target(
                "reclaim_given_break",
                &attempts,
                |row| row.broken,
                AuctionResolution::ReclaimAfterBreak,
            ),
            attempt_target(
                "initial_acceptance_given_initial_contact",
                &attempts,
                |row| row.contact && !row.is_retest,
                AuctionResolution::AcceptThroughNode,
            ),
            attempt_target(
                "rejection_given_initial_contact",
                &attempts,
                |row| row.contact && !row.is_retest,
                AuctionResolution::RejectToOrigin,
            ),
            attempt_target(
                "retest_hold_given_retest_contact",
                &attempts,
                |row| row.contact && row.is_retest,
                AuctionResolution::AcceptAndHoldRetest,
            ),
            transit_target(&attempts, &episodes, &transits),
        ])
    }
}

fn load_attempts<'a>(table: &PackedSection<'a>) -> Result<Vec<Attempt<'a>>> {
    let c = Columns::new(
        table,
        &[
            "run_key",
            "attempt_id",
            "episode_id",
            "is_retest",
            "contact",
            "break",
            "end",
            "resolution_code",
            "completion_status_code",
        ],
    )?;
    table
        .rows()
        .map(|row| {
            Ok(Attempt {
                run: c.required(row, 0)?,
                attempt: c.u64(row, 1)?,
                episode: c.u64(row, 2)?,
                is_retest: c.u64(row, 3)? == 1,
                contact: !c.null(row, 4),
                broken: !c.null(row, 5),
                end: c.optional_u64(row, 6),
                resolution: parse_enum(c.i16(row, 7)?, "resolution")?,
                completion: parse_enum(c.i16(row, 8)?, "completion")?,
            })
        })
        .collect()
}

fn load_episodes<'a>(table: &PackedSection<'a>) -> Result<Vec<Episode<'a>>> {
    let c = Columns::new(
        table,
        &[
            "run_key",
            "episode_id",
            "end",
            "resolution_code",
            "completion_status_code",
        ],
    )?;
    table
        .rows()
        .map(|row| {
            Ok(Episode {
                run: c.required(row, 0)?,
                episode: c.u64(row, 1)?,
                end: c.optional_u64(row, 2),
                resolution: parse_enum(c.i16(row, 3)?, "resolution")?,
                completion: parse_enum(c.i16(row, 4)?, "completion")?,
            })
        })
        .collect()
}

fn load_transits<'a>(table: &PackedSection<'a>) -> Result<Vec<Transit<'a>>> {
    let c = Columns::new(
        table,
        &["run_key", "episode_id", "start", "resolution_code"],
    )?;
    table
        .rows()
        .map(|row| {
            Ok(Transit {
                run: c.required(row, 0)?,
                episode: c.u64(row, 1)?,
                start: c.optional_u64(row, 2),
                resolution: parse_enum(c.i16(row, 3)?, "resolution")?,
            })
        })
        .collect()
}

fn attempt_target<'a>(
    target: &str,
    attempts: &[Attempt<'a>],
    eligible: impl Fn(&Attempt<'a>) -> bool,
    positive: AuctionResolution,
) -> TargetUniverse {
    let mut eligible_ids = Vec::new();
    let mut observed_ids = Vec::new();
    let mut censored_ids = Vec::new();
    let mut analyzable_ids = Vec::new();
    for row in attempts.iter().filter(|row| eligible(row)) {
        let id = Id {
            run: row.run,
            prefix: b'A',
            value: row.attempt,
        };
        eligible_ids.push(id);
        let censored = behavior_censored(row);
        if row.resolution == positive {
            observed_ids.push(id);
        }
        if censored {
            censored_ids.push(id);
        } else {
            analyzable_ids.push(id);
        }
    }
    universe(
        target,
        "run_key+attempt_id",
        eligible_ids,
        observed_ids,
        censored_ids,
        analyzable_ids,
    )
}

fn transit_target<'a>(
    attempts: &[Attempt<'a>],
    episodes: &[Episode<'a>],
    transits: &[Transit<'a>],
) -> TargetUniverse {
    let mut acceptance = HashMap::<(&[u8], u64), u64>::new();
    for row in attempts.iter().filter(|row| {
        row.completion == CompletionStatus::Resolved
            && row.resolution == AuctionResolution::AcceptThroughNode
    }) {
        if let Some(end) = row.end {
            acceptance
                .entry((row.run, row.episode))
                .and_modify(|old| *old = (*old).min(end))
                .or_insert(end);
        }
    }
    let mut first_transit = HashMap::<(&[u8], u64), u64>::new();
    for row in transits
        .iter()
        .filter(|row| row.resolution == AuctionResolution::TransitToNextNode)
    {
        if let Some(start) = row.start {
            first_transit
                .entry((row.run, row.episode))
                .and_modify(|old| *old = (*old).min(start))
                .or_insert(start);
        }
    }
    let episode_map = episodes
        .iter()
        .map(|row| ((row.run, row.episode), *row))
        .collect::<HashMap<_, _>>();
    let mut eligible = Vec::new();
    let mut observed = Vec::new();
    let mut censored = Vec::new();
    let mut analyzable = Vec::new();
    for (&key, &accepted_at) in &acceptance {
        let id = Id {
            run: key.0,
            prefix: b'E',
            value: key.1,
        };
        eligible.push(id);
        let seen = first_transit
            .get(&key)
            .is_some_and(|time| *time >= accepted_at);
        if seen {
            observed.push(id);
        }
        let episode = episode_map[&key];
        let is_censored = (episode.completion != CompletionStatus::Resolved
            || matches!(
                episode.resolution,
                AuctionResolution::NodeRetired
                    | AuctionResolution::Timeout
                    | AuctionResolution::None
            ))
            && !seen;
        let _ = episode.end;
        if is_censored {
            censored.push(id);
        } else {
            analyzable.push(id);
        }
    }
    universe(
        "transit_given_episode_acceptance",
        "run_key+episode_id",
        eligible,
        observed,
        censored,
        analyzable,
    )
}

fn behavior_censored(row: &Attempt<'_>) -> bool {
    row.completion != CompletionStatus::Resolved
        || !matches!(
            row.resolution,
            AuctionResolution::RejectToOrigin
                | AuctionResolution::AcceptThroughNode
                | AuctionResolution::ReclaimAfterBreak
                | AuctionResolution::AcceptAndHoldRetest
                | AuctionResolution::AcceptAndFailRetest
        )
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Id<'a> {
    run: &'a [u8],
    prefix: u8,
    value: u64,
}

fn universe(
    target: &str,
    identity: &str,
    mut eligible: Vec<Id<'_>>,
    mut observed: Vec<Id<'_>>,
    mut censored: Vec<Id<'_>>,
    mut analyzable: Vec<Id<'_>>,
) -> TargetUniverse {
    TargetUniverse {
        target: target.into(),
        identity_kind: identity.into(),
        eligible_count: eligible.len() as u64,
        observed_count: observed.len() as u64,
        censored_count: censored.len() as u64,
        analyzable_count: analyzable.len() as u64,
        eligible_ids_sha256: id_hash(&mut eligible),
        observed_ids_sha256: id_hash(&mut observed),
        censored_ids_sha256: id_hash(&mut censored),
        analyzable_ids_sha256: id_hash(&mut analyzable),
    }
}

fn id_hash(ids: &mut [Id<'_>]) -> String {
    ids.sort_unstable_by(|left, right| {
        (left.run, left.prefix, left.value).cmp(&(right.run, right.prefix, right.value))
    });
    let mut hash = Sha256::new();
    hash.update(b"MST_RG2_TARGET_ID_SET_V1\0");
    for id in ids {
        hash.update(id.run);
        hash.update(b"\t");
        hash.update([id.prefix]);
        hash.update(b"\t");
        hash.update(id.value.to_string().as_bytes());
        hash.update(b"\n");
    }
    hex(hash.finalize().as_slice())
}

struct Columns {
    indexes: Vec<usize>,
}
impl Columns {
    fn new(table: &PackedSection<'_>, names: &[&str]) -> Result<Self> {
        let indexes = names
            .iter()
            .map(|name| {
                table.column(name).ok_or_else(|| {
                    AdapterError::Contract(format!("{} missing column {name}", table.kind().name()))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { indexes })
    }
    fn required<'a>(&self, row: PackedRow<'a>, index: usize) -> Result<&'a [u8]> {
        row.field(self.indexes[index])
            .filter(|value| !value.is_empty() && *value != NULL_TOKEN)
            .ok_or_else(|| AdapterError::Contract("missing required value".into()))
    }
    fn null(&self, row: PackedRow<'_>, index: usize) -> bool {
        row.field(self.indexes[index])
            .is_none_or(|value| value.is_empty() || value == NULL_TOKEN)
    }
    fn u64(&self, row: PackedRow<'_>, index: usize) -> Result<u64> {
        parse_u64(self.required(row, index)?)
            .ok_or_else(|| AdapterError::Contract("invalid u64".into()))
    }
    fn optional_u64(&self, row: PackedRow<'_>, index: usize) -> Option<u64> {
        row.field(self.indexes[index])
            .filter(|value| !value.is_empty() && *value != NULL_TOKEN)
            .and_then(parse_u64)
    }
    fn i16(&self, row: PackedRow<'_>, index: usize) -> Result<i16> {
        std::str::from_utf8(self.required(row, index)?)
            .ok()
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| AdapterError::Contract("invalid i16".into()))
    }
}

fn parse_u64(bytes: &[u8]) -> Option<u64> {
    let mut value = 0_u64;
    if bytes.is_empty() {
        return None;
    }
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(u64::from(*byte - b'0'))?;
    }
    Some(value)
}
fn parse_enum<T: TryFrom<i16>>(code: i16, kind: &str) -> Result<T> {
    T::try_from(code).map_err(|_| AdapterError::Contract(format!("unknown {kind} code {code}")))
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

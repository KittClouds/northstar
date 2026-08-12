use super::snapshot::Availability;
use crate::data_plane::ids::{InstrumentId, JournalSequence};
use crate::ledger::{
    ActorKind, LedgerCommit, LedgerEntryId, LedgerEventKind, LedgerEventStatus, LedgerMappedStore,
    TradeCaseId,
};
use hashbrown::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;

pub const LEDGER_VISIBLE_ROWS: usize = 256;

#[derive(Clone, Debug)]
pub struct LedgerRowSnapshot {
    pub entry_id: LedgerEntryId,
    pub case_id: TradeCaseId,
    pub parent_entry_id: LedgerEntryId,
    pub instrument_id: InstrumentId,
    pub ts_event_ns: i64,
    pub kind: LedgerEventKind,
    pub status: LedgerEventStatus,
    pub actor_kind: ActorKind,
    pub canonical_first_sequence: JournalSequence,
    pub canonical_last_sequence: JournalSequence,
    pub preview: Arc<str>,
}

#[derive(Clone, Debug)]
pub struct TradeCaseSummary {
    pub case_id: TradeCaseId,
    pub instrument_id: InstrumentId,
    pub last_entry_id: LedgerEntryId,
    pub last_event_ns: i64,
    pub entry_count: u32,
    pub last_kind: LedgerEventKind,
    pub preview: Arc<str>,
}

#[derive(Clone, Debug)]
pub struct LedgerSnapshot {
    pub generation: u64,
    pub published_ns: i64,
    pub last_sequence: JournalSequence,
    pub availability: Availability,
    pub entry_count: u64,
    pub case_count: u64,
    pub machine_count: u64,
    pub operator_count: u64,
    pub incident_count: u64,
    pub health: Arc<str>,
    pub cases: Arc<[TradeCaseSummary]>,
    /// Newest first, bounded independently from total durable history.
    pub rows: Arc<[LedgerRowSnapshot]>,
}

impl LedgerSnapshot {
    pub fn awaiting_store() -> Self {
        Self {
            generation: 0,
            published_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            availability: Availability::AwaitingFirstReceipt,
            entry_count: 0,
            case_count: 0,
            machine_count: 0,
            operator_count: 0,
            incident_count: 0,
            health: Arc::from("Ledger store is not attached"),
            cases: Arc::from([]),
            rows: Arc::from([]),
        }
    }
}

pub struct LedgerProjector {
    generation: u64,
    published_ns: i64,
    last_sequence: JournalSequence,
    entry_count: u64,
    case_index: HashMap<u64, usize>,
    correlation_cases: HashMap<u128, u64>,
    cases: Vec<TradeCaseSummary>,
    machine_count: u64,
    operator_count: u64,
    incident_count: u64,
    rows: VecDeque<LedgerRowSnapshot>,
}

impl LedgerProjector {
    pub fn empty() -> Self {
        Self {
            generation: 1,
            published_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            entry_count: 0,
            case_index: HashMap::new(),
            correlation_cases: HashMap::new(),
            cases: Vec::new(),
            machine_count: 0,
            operator_count: 0,
            incident_count: 0,
            rows: VecDeque::with_capacity(LEDGER_VISIBLE_ROWS),
        }
    }

    pub fn from_mapped(store: &LedgerMappedStore) -> Self {
        let mut projector = Self::empty();
        for entry in store.iter() {
            projector.apply_header(entry.header, entry.body);
        }
        projector
    }

    pub fn apply_commit(&mut self, commit: LedgerCommit, body: &[u8]) -> bool {
        if !commit.inserted {
            return false;
        }
        self.generation = self.generation.saturating_add(1);
        self.apply_header(&commit.header, body);
        true
    }

    pub fn case_for_correlation(
        &self,
        correlation: crate::ledger::CorrelationId,
    ) -> Option<TradeCaseId> {
        self.correlation_cases
            .get(&correlation.0)
            .copied()
            .map(TradeCaseId)
    }

    pub fn snapshot(&self) -> LedgerSnapshot {
        LedgerSnapshot {
            generation: self.generation,
            published_ns: self.published_ns,
            last_sequence: self.last_sequence,
            availability: Availability::Ready,
            entry_count: self.entry_count,
            case_count: self.cases.len() as u64,
            machine_count: self.machine_count,
            operator_count: self.operator_count,
            incident_count: self.incident_count,
            health: Arc::from("Durable hot journal and cold body pack ready"),
            cases: {
                let mut cases = self.cases.clone();
                cases.sort_unstable_by(|left, right| {
                    right
                        .last_event_ns
                        .cmp(&left.last_event_ns)
                        .then_with(|| right.case_id.cmp(&left.case_id))
                });
                cases.into()
            },
            rows: self.rows.iter().cloned().collect::<Vec<_>>().into(),
        }
    }

    fn apply_header(&mut self, header: &crate::ledger::LedgerEventHeader, body: &[u8]) {
        let Some(kind) = header.kind() else {
            return;
        };
        let Some(status) = LedgerEventStatus::from_raw(header.status) else {
            return;
        };
        let Some(actor_kind) = ActorKind::from_raw(header.actor_kind) else {
            return;
        };
        self.entry_count = self.entry_count.saturating_add(1);
        if kind.is_operator() {
            self.operator_count = self.operator_count.saturating_add(1);
        } else {
            self.machine_count = self.machine_count.saturating_add(1);
        }
        if kind == LedgerEventKind::DataIncident || status == LedgerEventStatus::Incident {
            self.incident_count = self.incident_count.saturating_add(1);
        }
        self.published_ns = self.published_ns.max(header.ts_recorded_ns);
        self.last_sequence = self
            .last_sequence
            .max(JournalSequence(header.canonical_last_sequence));
        let preview = body_preview(body);
        if header.case_id != 0 {
            let correlation = header.correlation_id();
            if correlation != crate::ledger::CorrelationId::UNKNOWN {
                self.correlation_cases
                    .entry(correlation.0)
                    .or_insert(header.case_id);
            }
            if let Some(index) = self.case_index.get(&header.case_id).copied() {
                let case = &mut self.cases[index];
                case.last_entry_id = LedgerEntryId(header.entry_id);
                case.last_event_ns = header.ts_event_ns;
                case.entry_count = case.entry_count.saturating_add(1);
                case.last_kind = kind;
                case.preview = Arc::clone(&preview);
                if header.instrument_id != 0 {
                    case.instrument_id = InstrumentId(header.instrument_id);
                }
            } else {
                let index = self.cases.len();
                self.case_index.insert(header.case_id, index);
                self.cases.push(TradeCaseSummary {
                    case_id: TradeCaseId(header.case_id),
                    instrument_id: InstrumentId(header.instrument_id),
                    last_entry_id: LedgerEntryId(header.entry_id),
                    last_event_ns: header.ts_event_ns,
                    entry_count: 1,
                    last_kind: kind,
                    preview: Arc::clone(&preview),
                });
            }
        }
        self.rows.push_front(LedgerRowSnapshot {
            entry_id: LedgerEntryId(header.entry_id),
            case_id: TradeCaseId(header.case_id),
            parent_entry_id: LedgerEntryId(header.parent_entry_id),
            instrument_id: InstrumentId(header.instrument_id),
            ts_event_ns: header.ts_event_ns,
            kind,
            status,
            actor_kind,
            canonical_first_sequence: JournalSequence(header.canonical_first_sequence),
            canonical_last_sequence: JournalSequence(header.canonical_last_sequence),
            preview,
        });
        if self.rows.len() > LEDGER_VISIBLE_ROWS {
            self.rows.pop_back();
        }
    }
}

impl Default for LedgerProjector {
    fn default() -> Self {
        Self::empty()
    }
}

fn body_preview(body: &[u8]) -> Arc<str> {
    let Ok(text) = std::str::from_utf8(body) else {
        return Arc::from("<binary evidence>");
    };
    let mut preview = String::with_capacity(text.len().min(180));
    let mut prior_space = false;
    for character in text.chars().take(180) {
        let character = if character.is_whitespace() {
            ' '
        } else {
            character
        };
        if character == ' ' && prior_space {
            continue;
        }
        prior_space = character == ' ';
        preview.push(character);
    }
    Arc::from(preview.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{instruments, SchemaVersion};
    use crate::ledger::{
        ActorId, CorrelationId, LedgerEventDraft, LedgerFlags, LedgerStore, SourceEventKey,
    };

    #[test]
    fn durable_entries_project_to_a_bounded_newest_first_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let hot = dir.path().join("ledger.hot");
        let cold = dir.path().join("ledger.body");
        let mut store = LedgerStore::create(&hot, &cold, 1).unwrap();
        for id in 1..=300u128 {
            let body = format!("note {id}");
            store
                .append(&LedgerEventDraft {
                    case_id: TradeCaseId((id % 3 + 1) as u64),
                    parent_entry_id: LedgerEntryId::UNKNOWN,
                    correlation_id: CorrelationId(id),
                    source_key: SourceEventKey(id),
                    actor_id: ActorId(1),
                    actor_kind: ActorKind::Operator,
                    kind: LedgerEventKind::OperatorNote,
                    status: LedgerEventStatus::Accepted,
                    flags: LedgerFlags::NONE,
                    instrument_id: instruments::US100,
                    account_id: 0,
                    ts_event_ns: id as i64,
                    ts_received_ns: id as i64,
                    ts_recorded_ns: id as i64,
                    canonical_sequences: None,
                    schema_version: SchemaVersion(1),
                    body: body.as_bytes(),
                })
                .unwrap();
        }
        drop(store);
        let mapped = LedgerMappedStore::open(&hot, &cold).unwrap();
        let snapshot = LedgerProjector::from_mapped(&mapped).snapshot();
        assert_eq!(snapshot.entry_count, 300);
        assert_eq!(snapshot.case_count, 3);
        assert_eq!(snapshot.rows.len(), LEDGER_VISIBLE_ROWS);
        assert_eq!(snapshot.rows[0].entry_id, LedgerEntryId(300));
        assert_eq!(&*snapshot.rows[0].preview, "note 300");
    }
}

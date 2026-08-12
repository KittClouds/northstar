use super::{LedgerProjector, MacroMaterialBatch, MacroMaterialKind};
use crate::data_plane::ids::{InstrumentId, JournalSequence, SchemaVersion, SourceId};
use crate::data_plane::providers::bea::BEA_SOURCE;
use crate::data_plane::providers::bls::BLS_SOURCE;
use crate::data_plane::providers::boe::BOE_SOURCE;
use crate::data_plane::providers::boj::BOJ_SOURCE;
use crate::data_plane::providers::census::CENSUS_SOURCE;
use crate::data_plane::providers::cftc::CFTC_SOURCE;
use crate::data_plane::providers::ecb::ECB_SOURCE;
use crate::data_plane::providers::eurostat::EUROSTAT_SOURCE;
use crate::data_plane::providers::fred::FRED_SOURCE;
use crate::data_plane::providers::ons::ONS_SOURCE;
use crate::ledger::{
    ActorId, ActorKind, CorrelationId, LedgerError, LedgerEventDraft, LedgerEventKind,
    LedgerEventStatus, LedgerFlags, LedgerStore, SourceEventKey, TradeCaseId,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomaticCasePolicy {
    NoCase,
    Existing(TradeCaseId),
    Correlate,
}

/// Provider-neutral material event accepted by Ledger's single writer.
///
/// TradeLocker, canonical-data reducers, reconciliation, and later Nautilus
/// adapters must normalize into this type. Provider payloads never cross this
/// boundary.
#[derive(Clone, Debug)]
pub struct AutomaticLedgerEventCommand {
    pub source_key: SourceEventKey,
    pub case_policy: AutomaticCasePolicy,
    pub correlation_id: CorrelationId,
    pub actor_id: ActorId,
    pub actor_kind: ActorKind,
    pub kind: LedgerEventKind,
    pub status: LedgerEventStatus,
    pub flags: LedgerFlags,
    pub instrument_id: InstrumentId,
    pub account_id: u64,
    pub ts_event_ns: i64,
    pub ts_received_ns: i64,
    pub canonical_sequences: Option<(JournalSequence, JournalSequence)>,
    pub schema_version: SchemaVersion,
    pub body: Arc<[u8]>,
}

impl AutomaticLedgerEventCommand {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.source_key == SourceEventKey::UNKNOWN {
            return Err("source event key is missing");
        }
        if self.actor_id == ActorId::UNKNOWN {
            return Err("actor identity is missing");
        }
        if self.actor_kind == ActorKind::Operator || self.kind.is_operator() {
            return Err("automatic event cannot claim operator authority");
        }
        if self.body.is_empty() {
            return Err("evidence body is empty");
        }
        if self.ts_received_ns < self.ts_event_ns {
            return Err("receive time precedes event time");
        }
        if self.schema_version.get() == 0 {
            return Err("schema version is missing");
        }
        if let Some((first, last)) = self.canonical_sequences {
            if first == JournalSequence::UNKNOWN || last < first {
                return Err("canonical sequence range is invalid");
            }
        }
        match self.case_policy {
            AutomaticCasePolicy::NoCase if self.kind.requires_trade_case() => {
                return Err("trade lifecycle event requires a case")
            }
            AutomaticCasePolicy::Existing(case_id) if case_id == TradeCaseId::UNKNOWN => {
                return Err("existing case identity is missing")
            }
            AutomaticCasePolicy::Correlate if self.correlation_id == CorrelationId::UNKNOWN => {
                return Err("correlated event has no correlation identity")
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn from_macro_batch(batch: MacroMaterialBatch) -> Self {
        let source_name = source_name(batch.source);
        let corrections = if batch.correction_count == 0 {
            String::new()
        } else {
            format!("; {} correction(s)", batch.correction_count)
        };
        let body = format!(
            "{source_name} committed {} {} to canonical replay (sequence {}..={}{corrections}).",
            batch.event_count,
            batch.kind.ledger_label(),
            batch.first_sequence.get(),
            batch.last_sequence.get(),
        );
        Self {
            source_key: macro_source_key(batch),
            case_policy: AutomaticCasePolicy::NoCase,
            correlation_id: CorrelationId::UNKNOWN,
            actor_id: ActorId(10_000 + u64::from(batch.source.0)),
            actor_kind: ActorKind::Provider,
            kind: match batch.kind {
                MacroMaterialKind::Observations => LedgerEventKind::MacroRelease,
                MacroMaterialKind::Positioning => LedgerEventKind::Positioning,
                MacroMaterialKind::Provenance => LedgerEventKind::MacroRelease,
            },
            status: LedgerEventStatus::Observed,
            flags: LedgerFlags::MATERIAL,
            instrument_id: InstrumentId::UNKNOWN,
            account_id: 0,
            ts_event_ns: batch.ts_received_ns,
            ts_received_ns: batch.ts_received_ns,
            canonical_sequences: Some((batch.first_sequence, batch.last_sequence)),
            schema_version: SchemaVersion(1),
            body: Arc::from(body.into_bytes()),
        }
    }
}

pub(crate) fn append_automatic_event(
    store: &mut LedgerStore,
    projector: &mut LedgerProjector,
    command: &AutomaticLedgerEventCommand,
    recorded_ns: i64,
) -> Result<bool, AutomaticLedgerApplyError> {
    command
        .validate()
        .map_err(AutomaticLedgerApplyError::Validation)?;
    let case_id = match command.case_policy {
        AutomaticCasePolicy::NoCase => TradeCaseId::UNKNOWN,
        AutomaticCasePolicy::Existing(case_id) => {
            if command.correlation_id != CorrelationId::UNKNOWN {
                if let Some(existing) = projector.case_for_correlation(command.correlation_id) {
                    if existing != case_id {
                        return Err(AutomaticLedgerApplyError::CorrelationConflict {
                            correlation: command.correlation_id,
                            existing,
                            requested: case_id,
                        });
                    }
                }
            }
            case_id
        }
        AutomaticCasePolicy::Correlate => projector
            .case_for_correlation(command.correlation_id)
            .unwrap_or_else(|| store.next_case_id()),
    };
    let recorded_ns = recorded_ns.max(command.ts_received_ns);
    let draft = LedgerEventDraft {
        case_id,
        parent_entry_id: crate::ledger::LedgerEntryId::UNKNOWN,
        correlation_id: command.correlation_id,
        source_key: command.source_key,
        actor_id: command.actor_id,
        actor_kind: command.actor_kind,
        kind: command.kind,
        status: command.status,
        flags: command.flags,
        instrument_id: command.instrument_id,
        account_id: command.account_id,
        ts_event_ns: command.ts_event_ns,
        ts_received_ns: command.ts_received_ns,
        ts_recorded_ns: recorded_ns,
        canonical_sequences: command.canonical_sequences,
        schema_version: command.schema_version,
        body: command.body.as_ref(),
    };
    let commit = store.append(&draft)?;
    Ok(projector.apply_commit(commit, draft.body))
}

#[derive(Debug, Error)]
pub(crate) enum AutomaticLedgerApplyError {
    #[error("automatic Ledger event is invalid: {0}")]
    Validation(&'static str),
    #[error(
        "correlation {correlation:?} belongs to case {existing:?}, not requested case {requested:?}"
    )]
    CorrelationConflict {
        correlation: CorrelationId,
        existing: TradeCaseId,
        requested: TradeCaseId,
    },
    #[error(transparent)]
    Store(#[from] LedgerError),
}

fn source_name(source: SourceId) -> &'static str {
    if source == BLS_SOURCE {
        "BLS"
    } else if source == CFTC_SOURCE {
        "CFTC TFF"
    } else if source == EUROSTAT_SOURCE {
        "Eurostat"
    } else if source == FRED_SOURCE {
        "FRED"
    } else if source == BEA_SOURCE {
        "BEA"
    } else if source == CENSUS_SOURCE {
        "Census MARTS"
    } else if source == ECB_SOURCE {
        "ECB"
    } else if source == ONS_SOURCE {
        "ONS"
    } else if source == BOE_SOURCE {
        "Bank of England"
    } else if source == BOJ_SOURCE {
        "Bank of Japan"
    } else {
        "Official source"
    }
}

fn macro_source_key(batch: MacroMaterialBatch) -> SourceEventKey {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-ledger-macro-material-v1");
    hasher.update(&batch.source.0.to_le_bytes());
    hasher.update(&[match batch.kind {
        MacroMaterialKind::Observations => 1,
        MacroMaterialKind::Positioning => 2,
        MacroMaterialKind::Provenance => 3,
    }]);
    hasher.update(&batch.first_sequence.get().to_le_bytes());
    hasher.update(&batch.last_sequence.get().to_le_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&hash.as_bytes()[..16]);
    SourceEventKey(u128::from_le_bytes(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_command_is_stable_material_and_canonical_linked() {
        let batch = MacroMaterialBatch {
            source: EUROSTAT_SOURCE,
            kind: MacroMaterialKind::Observations,
            first_sequence: JournalSequence(7),
            last_sequence: JournalSequence(9),
            ts_received_ns: 100,
            event_count: 3,
            correction_count: 1,
        };
        let first = AutomaticLedgerEventCommand::from_macro_batch(batch);
        let second = AutomaticLedgerEventCommand::from_macro_batch(batch);
        assert_eq!(first.source_key, second.source_key);
        assert_eq!(first.kind, LedgerEventKind::MacroRelease);
        assert_eq!(first.case_policy, AutomaticCasePolicy::NoCase);
        assert_eq!(
            first.canonical_sequences,
            Some((JournalSequence(7), JournalSequence(9)))
        );
        first.validate().unwrap();
        let positioning = AutomaticLedgerEventCommand::from_macro_batch(MacroMaterialBatch {
            kind: MacroMaterialKind::Positioning,
            ..batch
        });
        assert_eq!(positioning.kind, LedgerEventKind::Positioning);
    }

    #[test]
    fn trade_lifecycle_cannot_escape_case_correlation() {
        let mut command = AutomaticLedgerEventCommand::from_macro_batch(MacroMaterialBatch {
            source: BLS_SOURCE,
            kind: MacroMaterialKind::Observations,
            first_sequence: JournalSequence(1),
            last_sequence: JournalSequence(1),
            ts_received_ns: 1,
            event_count: 1,
            correction_count: 0,
        });
        command.kind = LedgerEventKind::Fill;
        assert_eq!(
            command.validate(),
            Err("trade lifecycle event requires a case")
        );
    }
}

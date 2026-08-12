use super::LedgerProjector;
use crate::data_plane::ids::{InstrumentId, SchemaVersion};
use crate::ledger::{
    ActorId, ActorKind, CorrelationId, LedgerEntryId, LedgerError, LedgerEventDraft,
    LedgerEventKind, LedgerEventStatus, LedgerFlags, LedgerStore, SourceEventKey, TradeCaseId,
};
use std::sync::Arc;
use thiserror::Error;

const OPERATOR_ACTOR: ActorId = ActorId(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorEntryKind {
    Plan,
    Observation,
    Intervention,
    Review,
}

impl OperatorEntryKind {
    pub const ALL: [Self; 4] = [
        Self::Plan,
        Self::Observation,
        Self::Intervention,
        Self::Review,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Plan => "Plan",
            Self::Observation => "Observe",
            Self::Intervention => "Intervene",
            Self::Review => "Review",
        }
    }

    pub const fn event_kind(self) -> LedgerEventKind {
        match self {
            Self::Plan => LedgerEventKind::OperatorPlan,
            Self::Observation => LedgerEventKind::OperatorObservation,
            Self::Intervention => LedgerEventKind::OperatorIntervention,
            Self::Review => LedgerEventKind::OperatorReview,
        }
    }

    pub const fn requires_existing_case(self) -> bool {
        matches!(self, Self::Intervention | Self::Review)
    }

    pub const fn key_byte(self) -> u8 {
        match self {
            Self::Plan => 1,
            Self::Observation => 2,
            Self::Intervention => 3,
            Self::Review => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorMutation {
    Append,
    Amend(LedgerEntryId),
    Redact(LedgerEntryId),
}

impl OperatorMutation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Append => "Append",
            Self::Amend(_) => "Amend",
            Self::Redact(_) => "Redact",
        }
    }

    pub const fn parent(self) -> LedgerEntryId {
        match self {
            Self::Append => LedgerEntryId::UNKNOWN,
            Self::Amend(parent) | Self::Redact(parent) => parent,
        }
    }

    pub const fn key_byte(self) -> u8 {
        match self {
            Self::Append => 1,
            Self::Amend(_) => 2,
            Self::Redact(_) => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OperatorLedgerCommand {
    pub command_key: SourceEventKey,
    pub case_id: TradeCaseId,
    pub correlation_id: CorrelationId,
    pub instrument_id: InstrumentId,
    pub submitted_ns: i64,
    pub entry_kind: OperatorEntryKind,
    pub mutation: OperatorMutation,
    pub body: Arc<str>,
}

impl OperatorLedgerCommand {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.command_key == SourceEventKey::UNKNOWN {
            return Err("operator command key is missing");
        }
        if self.body.trim().is_empty() {
            return Err("operator entry body is empty");
        }
        if self.submitted_ns < 0 {
            return Err("operator entry timestamp is invalid");
        }
        if self.entry_kind.requires_existing_case() && self.case_id == TradeCaseId::UNKNOWN {
            return Err("interventions and reviews require an existing case");
        }
        if self.mutation != OperatorMutation::Append
            && (self.case_id == TradeCaseId::UNKNOWN
                || self.mutation.parent() == LedgerEntryId::UNKNOWN)
        {
            return Err("amendments and redactions require a case and parent entry");
        }
        Ok(())
    }
}

pub(crate) fn append_operator_event(
    store: &mut LedgerStore,
    projector: &mut LedgerProjector,
    command: &OperatorLedgerCommand,
    recorded_ns: i64,
) -> Result<bool, OperatorLedgerError> {
    command
        .validate()
        .map_err(OperatorLedgerError::InvalidCommand)?;
    let parent_entry_id = command.mutation.parent();
    if parent_entry_id != LedgerEntryId::UNKNOWN {
        let parent = store.entry_header(parent_entry_id)?;
        if ActorKind::from_raw(parent.actor_kind) != Some(ActorKind::Operator) {
            return Err(OperatorLedgerError::MachineEvidenceIsImmutable(
                parent_entry_id,
            ));
        }
    }
    let case_id = if command.case_id == TradeCaseId::UNKNOWN {
        store.next_case_id()
    } else {
        command.case_id
    };
    let (kind, status) = match command.mutation {
        OperatorMutation::Append => (command.entry_kind.event_kind(), LedgerEventStatus::Accepted),
        OperatorMutation::Amend(_) => (LedgerEventKind::Amendment, LedgerEventStatus::Accepted),
        OperatorMutation::Redact(_) => (LedgerEventKind::Redaction, LedgerEventStatus::Redacted),
    };
    let correlation_id = if command.correlation_id == CorrelationId::UNKNOWN {
        CorrelationId(command.command_key.0)
    } else {
        command.correlation_id
    };
    let recorded_ns = recorded_ns.max(command.submitted_ns);
    let draft = LedgerEventDraft {
        case_id,
        parent_entry_id,
        correlation_id,
        source_key: command.command_key,
        actor_id: OPERATOR_ACTOR,
        actor_kind: ActorKind::Operator,
        kind,
        status,
        flags: LedgerFlags::MATERIAL,
        instrument_id: command.instrument_id,
        account_id: 0,
        ts_event_ns: command.submitted_ns,
        ts_received_ns: command.submitted_ns,
        ts_recorded_ns: recorded_ns,
        canonical_sequences: None,
        schema_version: SchemaVersion(1),
        body: command.body.as_bytes(),
    };
    let commit = store.append(&draft)?;
    Ok(projector.apply_commit(commit, draft.body))
}

#[derive(Debug, Error)]
pub enum OperatorLedgerError {
    #[error("invalid operator command: {0}")]
    InvalidCommand(&'static str),
    #[error("machine-owned entry {0:?} cannot be amended or redacted by an operator")]
    MachineEvidenceIsImmutable(LedgerEntryId),
    #[error(transparent)]
    Store(#[from] LedgerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::instruments;

    fn command(kind: OperatorEntryKind, mutation: OperatorMutation) -> OperatorLedgerCommand {
        OperatorLedgerCommand {
            command_key: SourceEventKey(11),
            case_id: TradeCaseId::UNKNOWN,
            correlation_id: CorrelationId::UNKNOWN,
            instrument_id: instruments::US100,
            submitted_ns: 10,
            entry_kind: kind,
            mutation,
            body: Arc::from("typed operator evidence"),
        }
    }

    #[test]
    fn interventions_and_reviews_cannot_create_context_free_cases() {
        for kind in [OperatorEntryKind::Intervention, OperatorEntryKind::Review] {
            assert!(command(kind, OperatorMutation::Append).validate().is_err());
        }
        assert!(command(OperatorEntryKind::Plan, OperatorMutation::Append)
            .validate()
            .is_ok());
    }

    #[test]
    fn mutations_require_an_explicit_case_and_parent() {
        let amendment = command(
            OperatorEntryKind::Observation,
            OperatorMutation::Amend(LedgerEntryId(1)),
        );
        assert!(amendment.validate().is_err());
    }

    #[test]
    fn operator_cannot_mutate_machine_owned_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let hot = dir.path().join("ledger.hot");
        let cold = dir.path().join("ledger.body");
        let mut store = LedgerStore::create(&hot, &cold, 1).unwrap();
        let machine = LedgerEventDraft {
            case_id: TradeCaseId(1),
            parent_entry_id: LedgerEntryId::UNKNOWN,
            correlation_id: CorrelationId(9),
            source_key: SourceEventKey(9),
            actor_id: ActorId(77),
            actor_kind: ActorKind::Venue,
            kind: LedgerEventKind::Order,
            status: LedgerEventStatus::Observed,
            flags: LedgerFlags::MATERIAL,
            instrument_id: instruments::US100,
            account_id: 1,
            ts_event_ns: 9,
            ts_received_ns: 9,
            ts_recorded_ns: 9,
            canonical_sequences: None,
            schema_version: SchemaVersion(1),
            body: b"venue order",
        };
        let commit = store.append(&machine).unwrap();
        let mut projector = LedgerProjector::empty();
        assert!(projector.apply_commit(commit, machine.body));

        let mut redaction = command(
            OperatorEntryKind::Observation,
            OperatorMutation::Redact(LedgerEntryId(1)),
        );
        redaction.command_key = SourceEventKey(10);
        redaction.case_id = TradeCaseId(1);
        assert!(matches!(
            append_operator_event(&mut store, &mut projector, &redaction, 10),
            Err(OperatorLedgerError::MachineEvidenceIsImmutable(
                LedgerEntryId(1)
            ))
        ));
        assert_eq!(store.len(), 1);
    }
}

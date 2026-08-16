use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleState {
    Declared,
    Eligible,
    Selected,
    ExecutionAuthorized,
    Executing,
    Executed,
    ResultSealed,
    ConsumabilityEstablished,
    DagRecomputed,
    NotEvaluable,
    Failed,
    Blocked,
    Fossilized,
    Forked,
    Rebased,
    Superseded,
    RevokedCapability,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("invalid lifecycle transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
}

impl LifecycleState {
    pub fn transition(self, to: Self) -> Result<Self, LifecycleError> {
        use LifecycleState::*;
        let valid = matches!(
            (self, to),
            (Declared, Eligible)
                | (Eligible, Selected)
                | (Selected, ExecutionAuthorized)
                | (ExecutionAuthorized, Executing)
                | (Executing, Executed)
                | (Executed, ResultSealed)
                | (ResultSealed, ConsumabilityEstablished)
                | (ConsumabilityEstablished, DagRecomputed)
                | (_, NotEvaluable | Failed | Blocked | Fossilized | Superseded)
                | (Blocked, Forked | Rebased)
                | (ExecutionAuthorized, RevokedCapability)
        );
        valid
            .then_some(to)
            .ok_or(LifecycleError::InvalidTransition { from: self, to })
    }
}

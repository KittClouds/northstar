use crate::{ExecutionBinding, Root};
use serde::Serialize;
use std::sync::atomic::{AtomicU8, Ordering};
use thiserror::Error;

const LIVE: u8 = 0;
const CONSUMED: u8 = 1;
const REVOKED: u8 = 2;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("execution identity does not match capability")]
    BindingMismatch,
    #[error("execution capability already consumed")]
    AlreadyConsumed,
    #[error("execution capability revoked")]
    Revoked,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ExecutionPermit {
    pub capability_id: Root,
    pub execution_id: Root,
    pub binding_root: Root,
}

pub struct ExecutionCapability {
    id: Root,
    binding: ExecutionBinding,
    state: AtomicU8,
}

impl ExecutionCapability {
    pub fn new(binding: ExecutionBinding) -> Self {
        let binding_root = binding.identity_root();
        let id = Root::canonical(&[b"NORTHSTAR:EXECUTION-CAPABILITY:V1", &binding_root.0]);
        Self {
            id,
            binding,
            state: AtomicU8::new(LIVE),
        }
    }

    #[inline]
    pub fn id(&self) -> Root {
        self.id
    }

    pub fn consume(
        &self,
        requested: &ExecutionBinding,
        execution_id: Root,
    ) -> Result<ExecutionPermit, CapabilityError> {
        if &self.binding != requested {
            return Err(CapabilityError::BindingMismatch);
        }
        match self
            .state
            .compare_exchange(LIVE, CONSUMED, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(ExecutionPermit {
                capability_id: self.id,
                execution_id,
                binding_root: self.binding.identity_root(),
            }),
            Err(CONSUMED) => Err(CapabilityError::AlreadyConsumed),
            Err(REVOKED) => Err(CapabilityError::Revoked),
            Err(_) => unreachable!("closed capability state"),
        }
    }

    pub fn revoke(&self) -> Result<(), CapabilityError> {
        self.state
            .compare_exchange(LIVE, REVOKED, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|state| {
                if state == CONSUMED {
                    CapabilityError::AlreadyConsumed
                } else {
                    CapabilityError::Revoked
                }
            })
    }
}

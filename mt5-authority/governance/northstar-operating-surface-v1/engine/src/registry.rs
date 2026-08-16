use crate::{ArtifactClass, Root};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactChronology {
    pub creation_authority_sealed_at: u64,
    pub inputs_frozen_at: u64,
    pub scheduler_selected_at: Option<u64>,
    pub execution_authorized_at: u64,
    pub execution_started_at: u64,
    pub result_sealed_at: u64,
    pub first_consumed_at: Option<u64>,
}

impl ArtifactChronology {
    pub fn valid(&self, scheduler_controlled: bool) -> bool {
        let base = self.creation_authority_sealed_at <= self.inputs_frozen_at
            && self.inputs_frozen_at <= self.execution_authorized_at
            && self.execution_authorized_at <= self.execution_started_at
            && self.execution_started_at < self.result_sealed_at
            && self
                .first_consumed_at
                .is_none_or(|position| self.result_sealed_at <= position);
        let selection = match (scheduler_controlled, self.scheduler_selected_at) {
            (true, Some(position)) => {
                self.inputs_frozen_at <= position && position <= self.execution_authorized_at
            }
            (true, None) => false,
            (false, _) => true,
        };
        base && selection
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub root: Root,
    pub class: ArtifactClass,
    pub parent_roots: SmallVec<[Root; 4]>,
    pub input_roots: SmallVec<[Root; 8]>,
    pub result_sealed: bool,
    pub creation_authorized: bool,
    pub execution_authorized: bool,
    pub inputs_valid: bool,
    pub scope_valid: bool,
    pub blocked: bool,
    pub scheduler_controlled: bool,
    pub chronology: ArtifactChronology,
    pub authority_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Consumability {
    pub artifact_root: Root,
    pub consumable: bool,
    pub blocked_ancestors: Vec<Root>,
    pub derivation_root: Root,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("authoritative insertion requires a matching operation receipt")]
    UnauthorizedInsertion,
    #[error("artifact already registered")]
    Duplicate,
    #[error("artifact not found")]
    NotFound,
    #[error("authority ceiling mismatch")]
    AuthorityCeilingMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertionReceipt {
    pub target_root: Root,
    pub target_class: ArtifactClass,
    pub authority_ceiling: String,
    pub receipt_root: Root,
}

#[derive(Default)]
pub struct ArtifactRegistry {
    records: HashMap<Root, ArtifactRecord>,
}

impl ArtifactRegistry {
    #[inline]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn insert_authoritative(
        &mut self,
        record: ArtifactRecord,
        receipt: &InsertionReceipt,
    ) -> Result<(), RegistryError> {
        if record.root != receipt.target_root || record.class != receipt.target_class {
            return Err(RegistryError::UnauthorizedInsertion);
        }
        if record.authority_ceiling != receipt.authority_ceiling {
            return Err(RegistryError::AuthorityCeilingMismatch);
        }
        if self.records.contains_key(&record.root) {
            return Err(RegistryError::Duplicate);
        }
        self.records.insert(record.root, record);
        Ok(())
    }

    pub fn get(&self, root: Root) -> Option<&ArtifactRecord> {
        self.records.get(&root)
    }

    pub fn derive_consumability(&self, root: Root) -> Result<Consumability, RegistryError> {
        let mut visiting = HashSet::new();
        let mut blocked = Vec::new();
        let consumable = self.derive_inner(root, &mut visiting, &mut blocked)?;
        blocked.sort_unstable_by_key(|root| root.0);
        blocked.dedup();
        let mut bytes = Vec::with_capacity(33 + blocked.len() * 32);
        bytes.extend_from_slice(&root.0);
        bytes.push(u8::from(consumable));
        for ancestor in &blocked {
            bytes.extend_from_slice(&ancestor.0);
        }
        Ok(Consumability {
            artifact_root: root,
            consumable,
            blocked_ancestors: blocked,
            derivation_root: Root::canonical(&[b"NORTHSTAR:CONSUMABILITY:V1", &bytes]),
        })
    }

    fn derive_inner(
        &self,
        root: Root,
        visiting: &mut HashSet<Root>,
        blocked: &mut Vec<Root>,
    ) -> Result<bool, RegistryError> {
        let record = self.records.get(&root).ok_or(RegistryError::NotFound)?;
        if !visiting.insert(root) {
            blocked.push(root);
            return Ok(false);
        }
        let local = record.result_sealed
            && record.creation_authorized
            && record.execution_authorized
            && record.inputs_valid
            && record.scope_valid
            && record.chronology.valid(record.scheduler_controlled)
            && !record.blocked;
        if !local {
            blocked.push(root);
        }
        let mut parents_ok = true;
        for parent in &record.parent_roots {
            if !self.derive_inner(*parent, visiting, blocked)? {
                parents_ok = false;
            }
        }
        visiting.remove(&root);
        Ok(local && parents_ok)
    }
}

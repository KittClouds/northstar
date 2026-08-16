use crate::registry::InsertionReceipt;
use crate::{
    ArtifactClass, ArtifactRegistry, DataCapability, OperationClass, OperationRequest, Root,
};
use hashbrown::HashSet;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MonitorError {
    #[error("unknown operation and artifact-class pair")]
    UnknownOperationPair,
    #[error("actor authority is absent or nonconsumable")]
    ActorUnauthorized,
    #[error("a parent, input, or authority is absent or nonconsumable")]
    DependencyUnavailable,
    #[error("protected data capability is forbidden")]
    ProtectedDataForbidden,
    #[error("output authority exceeds actor ceiling")]
    AuthorityCeilingExceeded,
    #[error("invalid target identifier")]
    InvalidTargetId,
}

pub struct AuthorityKernel {
    allowed_pairs: HashSet<(OperationClass, ArtifactClass)>,
    allowed_ceilings: HashSet<String>,
    allowed_data_capabilities: HashSet<DataCapability>,
}

impl AuthorityKernel {
    pub fn new(
        allowed_pairs: impl IntoIterator<Item = (OperationClass, ArtifactClass)>,
        allowed_ceilings: impl IntoIterator<Item = String>,
        allowed_data_capabilities: impl IntoIterator<Item = DataCapability>,
    ) -> Self {
        Self {
            allowed_pairs: allowed_pairs.into_iter().collect(),
            allowed_ceilings: allowed_ceilings.into_iter().collect(),
            allowed_data_capabilities: allowed_data_capabilities.into_iter().collect(),
        }
    }

    pub fn authorize(
        &self,
        request: &OperationRequest,
        registry: &ArtifactRegistry,
    ) -> Result<InsertionReceipt, MonitorError> {
        if !self
            .allowed_pairs
            .contains(&(request.operation, request.target_class))
        {
            return Err(MonitorError::UnknownOperationPair);
        }
        if request.target_id.is_empty()
            || request.target_id.len() > 160
            || request.target_id.bytes().any(|byte| {
                !(byte.is_ascii_uppercase() || byte.is_ascii_digit() || b"-_.".contains(&byte))
            })
        {
            return Err(MonitorError::InvalidTargetId);
        }
        let actor = registry
            .derive_consumability(request.actor_authority_root)
            .map_err(|_| MonitorError::ActorUnauthorized)?;
        if !actor.consumable {
            return Err(MonitorError::ActorUnauthorized);
        }
        for root in request
            .parent_roots
            .iter()
            .chain(request.input_roots.iter())
            .chain(request.authority_roots.iter())
        {
            let status = registry
                .derive_consumability(*root)
                .map_err(|_| MonitorError::DependencyUnavailable)?;
            if !status.consumable {
                return Err(MonitorError::DependencyUnavailable);
            }
        }
        if request
            .data_capabilities
            .iter()
            .any(|capability| !self.allowed_data_capabilities.contains(capability))
        {
            return Err(MonitorError::ProtectedDataForbidden);
        }
        if !self
            .allowed_ceilings
            .contains(&request.expected_authority_ceiling)
        {
            return Err(MonitorError::AuthorityCeilingExceeded);
        }
        let target_root = Root::canonical(&[
            b"NORTHSTAR:AUTHORIZED-OUTPUT:V1",
            request.target_id.as_bytes(),
            &request.actor_authority_root.0,
        ]);
        let receipt_root = Root::canonical(&[
            b"NORTHSTAR:AUTHORIZED-OPERATION:V1",
            &target_root.0,
            request.expected_authority_ceiling.as_bytes(),
        ]);
        Ok(InsertionReceipt {
            target_root,
            target_class: request.expected_output_class,
            authority_ceiling: request.expected_authority_ceiling.clone(),
            receipt_root,
        })
    }
}

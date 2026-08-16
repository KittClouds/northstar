//! Deny-by-default operating substrate for authoritative Northstar transitions.

mod artifact_io;
mod canonical;
mod capability;
mod lifecycle;
mod monitor;
mod registry;
mod types;

pub use artifact_io::{mmap_hash, ArtifactIoError};
pub use canonical::{canonical_json_bytes, CanonicalError};
pub use capability::{CapabilityError, ExecutionCapability, ExecutionPermit};
pub use lifecycle::{LifecycleError, LifecycleState};
pub use monitor::{AuthorityKernel, MonitorError};
pub use registry::{
    ArtifactChronology, ArtifactRecord, ArtifactRegistry, Consumability, InsertionReceipt,
    RegistryError,
};
pub use types::{
    ArtifactClass, DataCapability, ExecutionBinding, OperationClass, OperationRequest, Root,
};

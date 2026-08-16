use bytemuck::{Pod, Zeroable};
use memchr::memchr;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct Root(pub [u8; 32]);

impl Root {
    pub const ZERO: Self = Self([0; 32]);

    #[inline]
    pub fn hash(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    pub fn canonical(parts: &[&[u8]]) -> Self {
        let mut hasher = blake3::Hasher::new();
        for part in parts {
            hasher.update(&(part.len() as u64).to_le_bytes());
            hasher.update(part);
        }
        Self(*hasher.finalize().as_bytes())
    }

    pub fn to_hex(self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut out, byte| {
                use fmt::Write;
                let _ = write!(out, "{byte:02X}");
                out
            })
    }
}

impl fmt::Debug for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Root {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Root {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        if text.len() != 64 || memchr(b'\0', text.as_bytes()).is_some() {
            return Err(serde::de::Error::custom(
                "root must be 64 hexadecimal characters",
            ));
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
            let pair = std::str::from_utf8(pair).map_err(serde::de::Error::custom)?;
            bytes[index] = u8::from_str_radix(pair, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(Self(bytes))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationClass {
    Create,
    IssueGrant,
    Qualify,
    Select,
    AuthorizeExecution,
    Execute,
    SealResult,
    EstablishConsumability,
    Materialize,
    Derive,
    Bind,
    Transport,
    Project,
    Assemble,
    Verify,
    Consume,
    Fork,
    Rebase,
    Block,
    Revoke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactClass {
    GovernanceConstitution,
    AuthorityGrant,
    GateSpecification,
    SchedulerReceipt,
    ExecutionCapability,
    AuthorizedOperationReceipt,
    Result,
    AuditFinding,
    Registry,
    OperatingSurface,
    FutureProtocolQuestion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataCapability {
    SealedGovernance,
    SealedAuthorityMetadata,
    OperatingCode,
    Population,
    Real04a,
    Target,
    Outcome,
    ExplorerResult,
    G8Result,
    RuntimeTelemetry,
}

impl DataCapability {
    #[inline]
    pub fn is_protected(self) -> bool {
        matches!(
            self,
            Self::Population
                | Self::Real04a
                | Self::Target
                | Self::Outcome
                | Self::ExplorerResult
                | Self::G8Result
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionBinding {
    pub scheduler_receipt_root: Root,
    pub gate_id: String,
    pub gate_spec_root: Root,
    pub parent_roots: SmallVec<[Root; 4]>,
    pub input_roots: SmallVec<[Root; 8]>,
    pub authority_roots: SmallVec<[Root; 4]>,
}

impl ExecutionBinding {
    pub fn identity_root(&self) -> Root {
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(&self.scheduler_receipt_root.0);
        bytes.extend_from_slice(&(self.gate_id.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.gate_id.as_bytes());
        bytes.extend_from_slice(&self.gate_spec_root.0);
        for roots in [
            &self.parent_roots[..],
            &self.input_roots[..],
            &self.authority_roots[..],
        ] {
            bytes.extend_from_slice(&(roots.len() as u64).to_le_bytes());
            for root in roots {
                bytes.extend_from_slice(&root.0);
            }
        }
        Root::hash(&bytes)
    }
}

#[derive(Clone, Debug)]
pub struct OperationRequest {
    pub operation: OperationClass,
    pub actor_authority_root: Root,
    pub target_class: ArtifactClass,
    pub target_id: String,
    pub parent_roots: SmallVec<[Root; 4]>,
    pub input_roots: SmallVec<[Root; 8]>,
    pub authority_roots: SmallVec<[Root; 4]>,
    pub data_capabilities: SmallVec<[DataCapability; 4]>,
    pub expected_output_class: ArtifactClass,
    pub expected_authority_ceiling: String,
}

use northstar_operating_surface::{ExecutionBinding, ExecutionCapability, ExecutionPermit, Root};
use obs_open_04a::authority::BoundAuthority;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const G9_GATE_ID: &str = "G9_DYNAMIC_LIFT_04A_PRIMARY_CHAMBER_V1";
pub const G0_ROOT: &str = "e317cabef33114edf47c50f4628a210ae8f1bd0b59feb436dc37e7df9cfe38bf";
pub const G1_AUTHORITY: &str = "OBS_OPEN_G1_SENTINEL_SEMANTIC_KERNEL_V1";
pub const G4_ROOT: &str = "37ed98b4ebe447ef3c2152e550c99652d0157aea2c77b8a886379d1ba9e08e15";
pub const G5_ROOT: &str = "808f4089ada22e7736460a90efd71b1dec0726dece460ca4dc1df8cbccac554c";
pub const G6_AUTHORITY: &str =
    "OBS_OPEN_G6_FIBERWISE_BEHAVIORAL_EQUIVALENCE_CONTRACT_WITH_RESTRICTIONS_V1";
pub const G8_ROOT: &str = "556cba86bf75a76d67c411db5a62229cb35ec84c92bd0bfbb6fd086971496ece";
pub const O4A_ROOT: &str = "48102bd529abf01591842f85676b86eb66b0600dc8d702bd0dbb3d6b729b4c41";

const CONSTITUTION: &str = "constitution/G9_SCIENTIFIC_EXECUTION_CONSTITUTION_V1.json";
const PRECOMMIT: &str = "constitution/G9_SEARCH_PRECOMMIT_V1.json";
const DATA_CAPABILITY: &str = "constitution/G9_DATA_CAPABILITY_V1.json";
const INCIDENT: &str = "constitution/PREAUTHORITY_DISCOVERY_ACCESS_INCIDENT_V1.json";
const REPAIR_LEDGER: &str = "constitution/G9_APPARATUS_REPAIR_LEDGER_V1.json";

#[derive(Debug, Clone)]
pub struct AuthorityRoots {
    pub constitution: Root,
    pub precommit: Root,
    pub data_capability: Root,
    pub incident: Root,
    pub repair_ledger: Root,
    pub gate_spec: Root,
    pub execution_id: Root,
    pub sha256_roots: Vec<(String, String)>,
}

pub struct AuthorizedOpen {
    pub authority: BoundAuthority,
    pub roots: AuthorityRoots,
    pub permit: ExecutionPermit,
}

pub fn bind_and_open(
    crate_root: &Path,
    authority_repo: &Path,
) -> Result<AuthorizedOpen, Box<dyn std::error::Error>> {
    let roots = load_roots(crate_root)?;
    let binding = ExecutionBinding {
        scheduler_receipt_root: roots.constitution,
        gate_id: G9_GATE_ID.into(),
        gate_spec_root: roots.gate_spec,
        parent_roots: [root_from_hex(O4A_ROOT)?, root_from_hex(G4_ROOT)?]
            .into_iter()
            .collect(),
        input_roots: [roots.precommit, roots.data_capability]
            .into_iter()
            .collect(),
        authority_roots: [roots.constitution, Root::hash(G6_AUTHORITY.as_bytes())]
            .into_iter()
            .collect(),
    };
    let capability = ExecutionCapability::new(binding.clone());
    let permit = capability.consume(&binding, roots.execution_id)?;

    // The only authoritative raw-data open occurs after the exact single-use
    // execution identity has been frozen and consumed.
    let authority = obs_open_04a::authority::open(authority_repo)?;
    if authority.raw_source_hash
        != "125b768ad87ba578c5498deffe50462909a6a1be422953a0f7488f036aac9d5b"
    {
        return Err("G9_RAW_DA_HASH_DRIFT".into());
    }
    if authority.access.d_a_sessions_decoded != 154
        || authority.access.d_b_outcome_registry_applications != 0
        || authority.access.d_b_derived_outcomes_inspected != 0
        || authority.access.d_b_formal_scores != 0
        || authority.access.d_c_membership_rows_decoded != 0
        || authority.access.d_c_observations_read != 0
        || authority.access.d_c_outcomes_computed != 0
    {
        return Err("G9_ACCESS_FIREWALL_FAILURE".into());
    }
    Ok(AuthorizedOpen {
        authority,
        roots,
        permit,
    })
}

fn load_roots(crate_root: &Path) -> Result<AuthorityRoots, Box<dyn std::error::Error>> {
    let files = [
        CONSTITUTION,
        PRECOMMIT,
        DATA_CAPABILITY,
        INCIDENT,
        REPAIR_LEDGER,
    ];
    let mut bytes = Vec::with_capacity(files.len());
    let mut sha256_roots = Vec::with_capacity(files.len());
    for relative in files {
        let path = crate_root.join(relative);
        let content = fs::read(&path)?;
        let _: serde_json::Value = serde_json::from_slice(&content)?;
        sha256_roots.push((relative.into(), sha256_hex(&content)));
        bytes.push(content);
    }
    let constitution = Root::hash(&bytes[0]);
    let precommit = Root::hash(&bytes[1]);
    let data_capability = Root::hash(&bytes[2]);
    let incident = Root::hash(&bytes[3]);
    let repair_ledger = Root::hash(&bytes[4]);
    let gate_spec = Root::canonical(&[
        b"NORTHSTAR:G9-GATE-SPEC:V1",
        &constitution.0,
        &precommit.0,
        &data_capability.0,
        &repair_ledger.0,
    ]);
    let execution_id = Root::canonical(&[
        b"NORTHSTAR:G9-EXECUTION:V1",
        &gate_spec.0,
        O4A_ROOT.as_bytes(),
        G0_ROOT.as_bytes(),
        G1_AUTHORITY.as_bytes(),
        G5_ROOT.as_bytes(),
        G6_AUTHORITY.as_bytes(),
        G8_ROOT.as_bytes(),
    ]);
    Ok(AuthorityRoots {
        constitution,
        precommit,
        data_capability,
        incident,
        repair_ledger,
        gate_spec,
        execution_id,
        sha256_roots,
    })
}

pub fn crate_root_from_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn root_from_hex(value: &str) -> Result<Root, Box<dyn std::error::Error>> {
    if value.len() != 64 {
        return Err("ROOT_HEX_LENGTH".into());
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(Root(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_are_bound_before_execution() {
        let roots = load_roots(&crate_root_from_manifest()).expect("load roots");
        assert_ne!(roots.constitution, Root::ZERO);
        assert_ne!(roots.precommit, Root::ZERO);
        assert_ne!(roots.data_capability, Root::ZERO);
        assert_ne!(roots.gate_spec, Root::ZERO);
        assert_eq!(roots.sha256_roots.len(), 5);
    }

    #[test]
    fn capability_is_single_use_and_exactly_bound() {
        let roots = load_roots(&crate_root_from_manifest()).expect("load roots");
        let binding = ExecutionBinding {
            scheduler_receipt_root: roots.constitution,
            gate_id: G9_GATE_ID.into(),
            gate_spec_root: roots.gate_spec,
            parent_roots: Default::default(),
            input_roots: [roots.precommit].into_iter().collect(),
            authority_roots: [roots.constitution].into_iter().collect(),
        };
        let capability = ExecutionCapability::new(binding.clone());
        assert!(capability.consume(&binding, roots.execution_id).is_ok());
        assert!(capability.consume(&binding, roots.execution_id).is_err());
    }
}

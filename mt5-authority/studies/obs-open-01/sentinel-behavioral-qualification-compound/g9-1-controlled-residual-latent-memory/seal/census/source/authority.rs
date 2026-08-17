use northstar_operating_surface::{ExecutionBinding, ExecutionCapability, ExecutionPermit, Root};
use obs_open_04a::authority::BoundAuthority;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const G9_QUALIFICATION_ROOT: &str =
    "2e65b1032d43e843129fc6a57fd16d572329cc2c6925265e8dd85d3660ee6ee5";
pub const G9_PRIMARY_SCIENCE_ROOT: &str =
    "3844cc9dafe1f54aa54fb501381f16faf3b5c67958bdafc2e54d50325b55b5db";
pub const G4_ROOT: &str = "37ed98b4ebe447ef3c2152e550c99652d0157aea2c77b8a886379d1ba9e08e15";
pub const G5_ROOT: &str = "808f4089ada22e7736460a90efd71b1dec0726dece460ca4dc1df8cbccac554c";
pub const RAW_DA_SHA256: &str = "125b768ad87ba578c5498deffe50462909a6a1be422953a0f7488f036aac9d5b";

const CONSTITUTION: &str = "constitution/G9_1_SCIENTIFIC_EXECUTION_CONSTITUTION_V1.json";
const CENSUS_PRECOMMIT: &str = "constitution/G9_1_CENSUS_PRECOMMIT_V1.json";
const DISCOVERY_PRECOMMIT: &str = "constitution/G9_1_DISCOVERY_PRECOMMIT_V1.json";

#[derive(Debug, Clone)]
pub struct AuthorityRoots {
    pub constitution: Root,
    pub census_precommit: Root,
    pub discovery_precommit: Root,
    pub gate_spec: Root,
    pub execution_id: Root,
    pub input_sha256: Vec<(String, String)>,
}

pub struct AuthorizedOpen {
    pub authority: BoundAuthority,
    pub roots: AuthorityRoots,
    pub permit: ExecutionPermit,
}

pub fn bind_and_open(
    crate_root: &Path,
    authority_repo: &Path,
    phase: &str,
    census_root: Option<&str>,
) -> Result<AuthorizedOpen, Box<dyn std::error::Error>> {
    let roots = load_roots(crate_root, phase, census_root)?;
    let selected_precommit = if phase == "census" {
        roots.census_precommit
    } else {
        roots.discovery_precommit
    };
    let mut inputs = vec![selected_precommit];
    if let Some(root) = census_root {
        inputs.push(root_from_hex(root)?);
    }
    let binding = ExecutionBinding {
        scheduler_receipt_root: roots.constitution,
        gate_id: format!(
            "G9_1_CONTROLLED_RESIDUAL_LATENT_MEMORY_{}_V1",
            phase.to_uppercase()
        ),
        gate_spec_root: roots.gate_spec,
        parent_roots: [
            root_from_hex(G9_QUALIFICATION_ROOT)?,
            root_from_hex(G9_PRIMARY_SCIENCE_ROOT)?,
        ]
        .into_iter()
        .collect(),
        input_roots: inputs.into_iter().collect(),
        authority_roots: [
            roots.constitution,
            root_from_hex(G4_ROOT)?,
            root_from_hex(G5_ROOT)?,
        ]
        .into_iter()
        .collect(),
    };
    let capability = ExecutionCapability::new(binding.clone());
    let permit = capability.consume(&binding, roots.execution_id)?;
    let authority = obs_open_04a::authority::open(authority_repo)?;
    if authority.raw_source_hash != RAW_DA_SHA256 {
        return Err("G9_1_RAW_DA_HASH_DRIFT".into());
    }
    if authority.access.d_a_sessions_decoded != 154
        || authority.access.d_b_outcome_registry_applications != 0
        || authority.access.d_b_derived_outcomes_inspected != 0
        || authority.access.d_c_membership_rows_decoded != 0
        || authority.access.d_c_observations_read != 0
        || authority.access.d_c_outcomes_computed != 0
    {
        return Err("G9_1_ACCESS_FIREWALL_FAILURE".into());
    }
    Ok(AuthorizedOpen {
        authority,
        roots,
        permit,
    })
}

fn load_roots(
    crate_root: &Path,
    phase: &str,
    census_root: Option<&str>,
) -> Result<AuthorityRoots, Box<dyn std::error::Error>> {
    let names = [CONSTITUTION, CENSUS_PRECOMMIT, DISCOVERY_PRECOMMIT];
    let mut roots = Vec::with_capacity(3);
    let mut input_sha256 = Vec::with_capacity(3);
    for name in names {
        let bytes = fs::read(crate_root.join(name))?;
        let _: serde_json::Value = serde_json::from_slice(&bytes)?;
        roots.push(Root::hash(&bytes));
        input_sha256.push((name.into(), sha256_hex(&bytes)));
    }
    let gate_spec = Root::canonical(&[
        b"NORTHSTAR:G9.1-GATE-SPEC:V1",
        &roots[0].0,
        &roots[1].0,
        &roots[2].0,
    ]);
    let execution_id = Root::canonical(&[
        b"NORTHSTAR:G9.1-EXECUTION:V1",
        phase.as_bytes(),
        &gate_spec.0,
        census_root.unwrap_or("NO_CENSUS_PARENT").as_bytes(),
    ]);
    Ok(AuthorityRoots {
        constitution: roots[0],
        census_precommit: roots[1],
        discovery_precommit: roots[2],
        gate_spec,
        execution_id,
        input_sha256,
    })
}

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn root_from_hex(value: &str) -> Result<Root, Box<dyn std::error::Error>> {
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
    fn phase_changes_execution_identity() {
        let a = load_roots(&crate_root(), "census", None).unwrap();
        let b = load_roots(&crate_root(), "discover", Some(G9_PRIMARY_SCIENCE_ROOT)).unwrap();
        assert_ne!(a.execution_id, b.execution_id);
    }
}

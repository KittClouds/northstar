use crate::{
    census::{self, CAPTURE_EA_REL, CAPTURE_REL, OBSERVER_EX5_REL, OBSERVER_REL, PARENT_ROOT},
    source::{MappedSource, sha256_bytes},
};
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const EXPECTED_SOURCE_SHA: &str =
    "a133ef773531a1599d1cef1e5124fcacc24b037f84cca777205c67ee0a785c61";
const EXPECTED_EX5_SHA: &str = "a9e25979dc85a760b15499a13552216f19646e43274c93c4d032adc671f415dd";
const EXPECTED_CAPTURE_SHA: &str =
    "eecc387b450b6490ea493cf8b7784f1810ef41a2e7f612b335beb26ad876a944";
const EXPECTED_CAPTURE_EA_SHA: &str =
    "d89de0b1f912fc04b4a64bfce1c46ce5b264eca249f2c7d08e94e603d9db2815";

fn read_mapped(repo: &Path, rel: &str) -> Result<MappedSource, Box<dyn std::error::Error>> {
    Ok(MappedSource::open(&repo.join(rel))?)
}

fn require_hash(label: &str, got: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    if got != expected {
        return Err(format!("{label} hash mismatch: expected {expected}, got {got}").into());
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn artifact_entry(path: &Path, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(path.join(name))?;
    Ok(json!({"path":name,"bytes":bytes.len(),"sha256":sha256_bytes(&bytes)}))
}

pub fn build(repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!(
            "output directory must not already contain files: {}",
            out.display()
        )
        .into());
    }
    fs::create_dir_all(out)?;

    let observer = read_mapped(repo, OBSERVER_REL)?;
    let ex5 = read_mapped(repo, OBSERVER_EX5_REL)?;
    let capture = read_mapped(repo, CAPTURE_REL)?;
    let capture_ea = read_mapped(repo, CAPTURE_EA_REL)?;
    require_hash("observer source", &observer.sha256(), EXPECTED_SOURCE_SHA)?;
    require_hash("observer EX5", &ex5.sha256(), EXPECTED_EX5_SHA)?;
    require_hash("capture indicator", &capture.sha256(), EXPECTED_CAPTURE_SHA)?;
    require_hash("capture EA", &capture_ea.sha256(), EXPECTED_CAPTURE_EA_SHA)?;

    let specimen = json!({
        "schema":"OBSERVER_SPECIMEN_BINDING_V1",
        "parent_inst01_root":PARENT_ROOT,
        "parent_authority":"CAUSAL_RANGE_EXTREME_INSTRUMENT_V1",
        "specimens":[
            {"role":"OBSERVER_SOURCE","path":OBSERVER_REL,"bytes":observer.len(),"sha256":observer.sha256(),"lines":observer.line_count()},
            {"role":"RUNTIME_EX5","path":OBSERVER_EX5_REL,"bytes":ex5.len(),"sha256":ex5.sha256()},
            {"role":"INST01_CAPTURE_INDICATOR_FOSSIL","path":CAPTURE_REL,"bytes":capture.len(),"sha256":capture.sha256(),"lines":capture.line_count()},
            {"role":"INST01_CAPTURE_EA_FOSSIL","path":CAPTURE_EA_REL,"bytes":capture_ea.len(),"sha256":capture_ea.sha256(),"lines":capture_ea.line_count()}
        ],
        "mutation":"NONE"
    });

    let artifacts: [(&str, Value); 11] = [
        ("OTO_PROTOCOL.json", census::protocol()),
        ("OBSERVER_SPECIMEN_BINDING.json", specimen),
        ("OBSERVER_ANATOMY_CENSUS.json", census::anatomy(&observer)?),
        (
            "EXPLICIT_DOF_CENSUS.json",
            census::explicit_dofs(&observer)?,
        ),
        (
            "BUFFER_PUBLICATION_CENSUS.json",
            census::buffers(&observer)?,
        ),
        (
            "LATENT_POLICY_CENSUS.json",
            census::latent_policies(&observer)?,
        ),
        (
            "AMBIENT_COORDINATE_CENSUS.json",
            census::ambient(&observer)?,
        ),
        (
            "INSTRUMENTATION_AUTHORITY_CENSUS.json",
            census::instrumentation(&observer, &capture)?,
        ),
        (
            "PARAMETER_INTROSPECTION_REQUIREMENT.json",
            census::parameter_requirement(&capture)?,
        ),
        ("OUTCOME_ACCESS_AUDIT.json", census::access_audit()),
        ("OTO_TYPED_FINDINGS.json", census::typed_findings()),
    ];
    for (name, value) in &artifacts {
        write_json(&out.join(name), value)?;
    }

    let entries = artifacts
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({
        "schema":"OTO_STATIC_ROOT_PAYLOAD_V1",
        "authority":"OBS_OPEN_OBSERVE_THE_OBSERVER_STATIC_METROLOGY_V1",
        "parent_inst01_root":PARENT_ROOT,
        "artifacts":entries
    });
    let logical_root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_ROOT_RECEIPT.json"),
        &json!({
            "schema":"OTO_STATIC_ROOT_RECEIPT_V1","logical_root":logical_root,"payload":payload
        }),
    )?;
    Ok(logical_root)
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|p| p.is_file());
    paths.sort();
    Ok(paths)
}

pub fn finalize(a: &Path, b: &Path, seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if seal.exists() {
        return Err(format!("seal destination already exists: {}", seal.display()).into());
    }
    let af = files(a)?;
    let bf = files(b)?;
    let an = af
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    let bn = bf
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    if an != bn {
        return Err("independent build artifact names differ".into());
    }
    let mut mismatches = Vec::new();
    for (left, right) in af.iter().zip(&bf) {
        if fs::read(left)? != fs::read(right)? {
            mismatches.push(left.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    if !mismatches.is_empty() {
        return Err(format!("independent build mismatch: {mismatches:?}").into());
    }
    fs::create_dir_all(seal)?;
    for path in &af {
        fs::copy(path, seal.join(path.file_name().unwrap()))?;
    }
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| {
        let bytes = fs::read(p).unwrap();
        json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()})
    }).collect::<Vec<_>>();
    let artifact_set_hash = sha256_bytes(&serde_json::to_vec(&set_payload)?);
    write_json(
        &seal.join("DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({
            "schema":"OTO_DETERMINISTIC_REBUILD_RECEIPT_V1",
            "build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),
            "mismatch_count":0,"byte_identical":true,"artifact_set_hash":artifact_set_hash,
            "logical_root":logical_root
        }),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_ROOT_RECEIPT.json"))?)?;
    let payload = receipt
        .get("payload")
        .ok_or("root receipt missing payload")?;
    let expected_root = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let actual_root = sha256_bytes(&serde_json::to_vec(payload)?);
    if actual_root != expected_root {
        return Err("logical root mismatch".into());
    }
    let mut names = HashSet::new();
    for artifact in payload["artifacts"]
        .as_array()
        .ok_or("artifacts not array")?
    {
        let name = artifact["path"].as_str().ok_or("artifact path missing")?;
        if !names.insert(name) {
            return Err(format!("duplicate artifact: {name}").into());
        }
        let bytes = fs::read(seal.join(name))?;
        if bytes.len() as u64 != artifact["bytes"].as_u64().ok_or("artifact bytes missing")? {
            return Err(format!("artifact size mismatch: {name}").into());
        }
        if sha256_bytes(&bytes) != artifact["sha256"].as_str().ok_or("artifact hash missing")? {
            return Err(format!("artifact hash mismatch: {name}").into());
        }
    }
    Ok(expected_root.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_authority_hashes_are_sha256() {
        for hash in [
            EXPECTED_SOURCE_SHA,
            EXPECTED_EX5_SHA,
            EXPECTED_CAPTURE_SHA,
            EXPECTED_CAPTURE_EA_SHA,
            PARENT_ROOT,
        ] {
            assert_eq!(hash.len(), 64);
            assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}

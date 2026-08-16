use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const PARENT_REL: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/seal";

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn explicit_effects(id: &str) -> Vec<&'static str> {
    if id.contains("Color")
        || id.contains("Style")
        || id.contains("Width")
        || id == "InpShowLegacyLines"
        || id == "InpShowExtremeSentinels"
    {
        return vec!["PUBLICATION_ONLY", "RENDER_ONLY"];
    }
    match id {
        "InpUniqueId" => vec!["PUBLICATION_ONLY", "IDENTITY_ONLY"],
        "InpHistoryDays" => vec!["AVAILABLE_DOMAIN", "INITIALIZATION", "PUBLICATION"],
        "InpRequireExactStartAlignment" | "InpRequireRangeContinuity" => {
            vec!["CHANGES_ADMISSION", "AVAILABLE_DOMAIN", "PUBLICATION"]
        }
        "InpRequireChartContinuity" => vec![
            "CHANGES_STATE_TRANSITION",
            "CHANGES_ADMISSION",
            "PUBLICATION",
        ],
        "InpEnableExtremeSentinels" => vec![
            "CHANGES_STATE_TRANSITION",
            "AVAILABLE_DOMAIN",
            "PUBLICATION",
        ],
        _ => vec![
            "CHANGES_TIME_SEMANTICS",
            "INITIALIZATION",
            "AVAILABLE_DOMAIN",
            "CHANGES_STATE_TRANSITION",
            "PUBLICATION",
        ],
    }
}

fn ambient_effects(id: &str) -> Vec<&'static str> {
    match id {
        "DIGITS" => vec!["PUBLICATION_ONLY"],
        "UNUSED_RUNTIME_ARRAYS" => vec!["NO_READ_IDENTIFIED"],
        "STOP_SIGNAL" => vec!["AVAILABLE_DOMAIN", "EXECUTION_COMPLETENESS"],
        "PLATFORM_EMPTY_VALUE" => vec!["PUBLICATION", "MISSINGNESS_SEMANTICS"],
        "PLATFORM_FLOATING_ARITHMETIC" => vec![
            "CHANGES_STATE_TRANSITION",
            "PUBLICATION",
            "NUMERICAL_SEMANTICS",
        ],
        "PREV_CALCULATED" => vec!["INITIALIZATION", "UPDATE_PATH_SELECTION"],
        "FORMING_VS_COMPLETED" => vec!["CHANGES_STATE_TRANSITION", "KNOWLEDGE_TIME", "PUBLICATION"],
        "CIVIL_DAY_CONVERSION" => vec!["CHANGES_TIME_SEMANTICS", "SESSION_OWNERSHIP"],
        _ => vec![
            "AVAILABLE_DOMAIN",
            "INITIALIZATION",
            "CHANGES_STATE_TRANSITION",
            "PUBLICATION",
        ],
    }
}

fn policy_effects(dimension: &str) -> Vec<&'static str> {
    match dimension {
        "INPUT_NORMALIZATION" | "CLOCK_NORMALIZATION" => vec![
            "INITIALIZATION",
            "CHANGES_TIME_SEMANTICS",
            "AVAILABLE_DOMAIN",
        ],
        "ENDPOINT_POLICY" => vec![
            "CHANGES_TIME_SEMANTICS",
            "CHANGES_ADMISSION",
            "CHANGES_STATE_TRANSITION",
        ],
        "KNOWLEDGE_TIME" => vec!["CHANGES_STATE_TRANSITION", "PUBLICATION", "KNOWLEDGE_TIME"],
        "RESOLUTION_POLICY" | "SOURCE_GEOMETRY" | "SESSION_OWNERSHIP" | "HISTORY_SCOPE" => {
            vec!["AVAILABLE_DOMAIN", "INITIALIZATION", "CHANGES_ADMISSION"]
        }
        "CLASSIFICATION_STRICTNESS" | "OBSERVED_PRICE_FIELD" | "RENEWAL_STRICTNESS" => {
            vec!["CHANGES_STATE_TRANSITION", "PUBLICATION"]
        }
        "IDENTITY_POLICY" => vec!["CHANGES_STATE_TRANSITION", "IDENTITY", "PUBLICATION"],
        "PROVISIONAL_COMMITTED" => {
            vec!["CHANGES_STATE_TRANSITION", "KNOWLEDGE_TIME", "PUBLICATION"]
        }
        "GAP_POLICY" => vec![
            "CHANGES_ADMISSION",
            "CHANGES_STATE_TRANSITION",
            "AVAILABLE_DOMAIN",
        ],
        "UPDATE_TIMING" => vec![
            "UPDATE_PATH_SELECTION",
            "CHANGES_STATE_TRANSITION",
            "PUBLICATION",
        ],
        _ => vec!["STRUCTURAL_EFFECT_UNKNOWN"],
    }
}

fn target_for(effect: &str) -> &'static str {
    match effect {
        "RENDER_ONLY"
        | "PUBLICATION_ONLY"
        | "PUBLICATION"
        | "IDENTITY_ONLY"
        | "MISSINGNESS_SEMANTICS" => "ARCH_PUBLICATION_SURFACE",
        "CHANGES_TIME_SEMANTICS" | "SESSION_OWNERSHIP" => "ARCH_CLOCK_SESSION_RESOLUTION",
        "CHANGES_ADMISSION" | "AVAILABLE_DOMAIN" | "EXECUTION_COMPLETENESS" => {
            "ARCH_COVERAGE_EVALUABILITY"
        }
        "INITIALIZATION" => "ARCH_INITIALIZATION",
        "KNOWLEDGE_TIME" => "ARCH_PROVISIONAL_COMMITTED_SPLIT",
        "UPDATE_PATH_SELECTION" => "ARCH_UPDATE_DISPATCH",
        "IDENTITY" => "ARCH_CANDIDATE_GENEALOGY",
        "NUMERICAL_SEMANTICS" => "ARCH_NUMERICAL_DOMAIN",
        "NO_READ_IDENTIFIED" => "ARCH_UNUSED_INPUT_SURFACE",
        _ => "ARCH_CAUSAL_STATE_TRANSITION",
    }
}

pub fn build(repo: &Path, out: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err(format!("output directory must be empty: {}", out.display()).into());
    }
    fs::create_dir_all(out)?;
    let parent = repo.join(PARENT_REL);
    let parent_receipt = read_json(&parent.join("OTO_ROOT_RECEIPT.json"))?;
    if parent_receipt["logical_root"] != STATIC_ROOT {
        return Err("static parent root mismatch".into());
    }
    let explicit = read_json(&parent.join("EXPLICIT_DOF_CENSUS.json"))?;
    let ambient = read_json(&parent.join("AMBIENT_COORDINATE_CENSUS.json"))?;
    let policies = read_json(&parent.join("LATENT_POLICY_CENSUS.json"))?;

    let mut nodes = Vec::with_capacity(72);
    let mut edges = Vec::with_capacity(240);
    for group in explicit["groups"]
        .as_array()
        .ok_or("explicit groups absent")?
    {
        for field in group["fields"].as_array().ok_or("explicit fields absent")? {
            let id = field.as_str().ok_or("explicit field not string")?;
            let effects = explicit_effects(id);
            nodes.push(json!({"id":id,"class":"EXPLICIT_INPUT","group":group["id"],"primitive_status":"DECLARED_INPUT","effects":effects,"independence_authority":"NOT_EARNED","dynamic_authority":"NONE"}));
            for effect in effects {
                edges.push(json!({"source":id,"target":target_for(effect),"kind":"STRUCTURALLY_AFFECTS","effect":effect,"authority":"SOURCE_DERIVED"}));
            }
        }
    }
    for item in ambient["coordinates"]
        .as_array()
        .ok_or("ambient coordinates absent")?
    {
        let id = item["id"].as_str().ok_or("ambient id absent")?;
        let effects = ambient_effects(id);
        nodes.push(json!({"id":id,"class":"AMBIENT_COORDINATE","primitive_status":"RUNTIME_OR_PLATFORM_SUPPLIED","effects":effects,"independence_authority":"NOT_EARNED","dynamic_authority":"NONE"}));
        for effect in effects {
            edges.push(json!({"source":id,"target":target_for(effect),"kind":"STRUCTURALLY_AFFECTS","effect":effect,"authority":"SOURCE_DERIVED"}));
        }
    }
    for item in policies["policies"].as_array().ok_or("policies absent")? {
        let id = item["id"].as_str().ok_or("policy id absent")?;
        let dimension = item["dimension"]
            .as_str()
            .ok_or("policy dimension absent")?;
        let effects = policy_effects(dimension);
        nodes.push(json!({"id":id,"class":"LATENT_POLICY","dimension":dimension,"primitive_status":"HARD_CODED_CHOICE","effects":effects,"independence_authority":"NOT_EARNED","dynamic_authority":"NONE","evidence":item["evidence"]}));
        for effect in effects {
            edges.push(json!({"source":id,"target":target_for(effect),"kind":"STRUCTURALLY_AFFECTS","effect":effect,"authority":"SOURCE_DERIVED"}));
        }
    }

    let architecture = [
        ("ARCH_INITIALIZATION", "STATE_ARCHITECTURE"),
        ("ARCH_CLOCK_SESSION_RESOLUTION", "STATE_ARCHITECTURE"),
        ("ARCH_COVERAGE_EVALUABILITY", "STATE_ARCHITECTURE"),
        ("ARCH_CAUSAL_STATE_TRANSITION", "STATE_ARCHITECTURE"),
        ("ARCH_PROVISIONAL_COMMITTED_SPLIT", "STATE_ARCHITECTURE"),
        ("ARCH_CANDIDATE_GENEALOGY", "STATE_ARCHITECTURE"),
        ("ARCH_UPDATE_DISPATCH", "STATE_ARCHITECTURE"),
        ("ARCH_NUMERICAL_DOMAIN", "SEMANTIC_DOMAIN"),
        ("ARCH_UNUSED_INPUT_SURFACE", "DEPENDENCY_BOUNDARY"),
        ("ARCH_PUBLICATION_SURFACE", "PUBLICATION_SURFACE"),
    ];
    for (id, class) in architecture {
        nodes.push(json!({"id":id,"class":class,"counted_in_58_joint_census":false}));
    }
    edges.extend([
        json!({"source":"TIMEFRAME","target":"DERIVED_PERIOD_SECONDS","kind":"DERIVES"}),
        json!({"source":"SYMBOL","target":"HISTORY_AVAILABILITY","kind":"PARAMETERIZES"}),
        json!({"source":"TIMEFRAME","target":"HISTORY_AVAILABILITY","kind":"PARAMETERIZES"}),
        json!({"source":"FORMING_VS_COMPLETED","target":"PROVISIONAL_SENTINEL_COMMITTED_IDENTITY_SPLIT","kind":"COUPLES_TO"}),
        json!({"source":"InpRequireChartContinuity","target":"SENTINEL_GAP_FAIL_CLOSED","kind":"ACTIVATES_POLICY"}),
        json!({"source":"InpRequireChartContinuity","target":"GRAMMAR_GAP_RESUME","kind":"ACTIVATES_POLICY"}),
        json!({"source":"TIMEFRAME","target":"FREEZE_AT_END_BAR_CLOSE","kind":"PARAMETERIZES_VIA_PERIOD_SECONDS"}),
        json!({"source":"InpEnableExtremeSentinels","target":"ARCH_CANDIDATE_GENEALOGY","kind":"GATES"})
    ]);

    let joint_count = nodes
        .iter()
        .filter(|n| {
            matches!(
                n["class"].as_str(),
                Some("EXPLICIT_INPUT" | "AMBIENT_COORDINATE" | "LATENT_POLICY")
            )
        })
        .count();
    if joint_count != 58 {
        return Err(format!("expected 58 identified joints, got {joint_count}").into());
    }
    let graph = json!({
        "schema":"OTO_DOF_DEPENDENCY_GRAPH_V1","parent_static_root":STATIC_ROOT,
        "joint_count":joint_count,"nodes":nodes,"edges":edges,
        "graph_authority":"STRUCTURAL_SOURCE_DERIVED_ONLY",
        "independence_authority":"NOT_EARNED","dynamic_dof_authority":"NONE",
        "governing_warning":"CENSUS_COUNT_IS_NOT_DIMENSIONALITY"
    });
    write_json(&out.join("OTO_DOF_DEPENDENCY_GRAPH.json"), &graph)?;
    let summary = json!({
        "schema":"OTO_DOF_TOPOLOGY_FINDINGS_V1","parent_static_root":STATIC_ROOT,
        "explicit_input_nodes":24,"ambient_coordinate_nodes":12,"latent_policy_nodes":22,
        "identified_joint_count":58,"independent_dimension_count":"NOT_EVALUABLE",
        "render_only_nodes":explicit["groups"].as_array().unwrap().iter().filter(|g|g["effect_class"]=="PUBLICATION_RENDERING_ONLY").map(|g|g["count"].as_u64().unwrap()).sum::<u64>() + 5,
        "dynamic_experiments":0,"market_rows_read":0,"outcome_rows_read":0,
        "result":"STATIC_DOF_DEPENDENCY_TOPOLOGY_SEALED",
        "maximum_authority":"OBS_OPEN_OTO_STATIC_DOF_TOPOLOGY_V1"
    });
    write_json(&out.join("OTO_DOF_TOPOLOGY_FINDINGS.json"), &summary)?;
    let names = [
        "OTO_DOF_DEPENDENCY_GRAPH.json",
        "OTO_DOF_TOPOLOGY_FINDINGS.json",
    ];
    let artifacts = names
        .iter()
        .map(|name| {
            let bytes = fs::read(out.join(name)).unwrap();
            json!({"path":name,"bytes":bytes.len(),"sha256":sha256_bytes(&bytes)})
        })
        .collect::<Vec<_>>();
    let payload = json!({"schema":"OTO_DOF_TOPOLOGY_ROOT_PAYLOAD_V1","parent_static_root":STATIC_ROOT,"authority":"OBS_OPEN_OTO_STATIC_DOF_TOPOLOGY_V1","artifacts":artifacts});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_DOF_TOPOLOGY_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_DOF_TOPOLOGY_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
    )?;
    Ok(root)
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut v = fs::read_dir(dir)?
        .map(|e| e.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    v.retain(|p| p.is_file());
    v.sort();
    Ok(v)
}

pub fn finalize(a: &Path, b: &Path, seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if seal.exists() {
        return Err("topology seal destination exists".into());
    }
    let af = files(a)?;
    let bf = files(b)?;
    if af.len() != bf.len() {
        return Err("topology build counts differ".into());
    }
    for (x, y) in af.iter().zip(&bf) {
        if x.file_name() != y.file_name() || fs::read(x)? != fs::read(y)? {
            return Err("topology independent builds differ".into());
        }
    }
    fs::create_dir_all(seal)?;
    for p in &af {
        fs::copy(p, seal.join(p.file_name().unwrap()))?;
    }
    let root = verify(seal)?;
    write_json(
        &seal.join("DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_DOF_TOPOLOGY_REBUILD_RECEIPT_V1","artifact_count":af.len(),"mismatch_count":0,"byte_identical":true,"logical_root":root}),
    )?;
    Ok(root)
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt = read_json(&seal.join("OTO_DOF_TOPOLOGY_ROOT_RECEIPT.json"))?;
    let payload = &receipt["payload"];
    let root = receipt["logical_root"]
        .as_str()
        .ok_or("topology root absent")?;
    if sha256_bytes(&serde_json::to_vec(payload)?) != root {
        return Err("topology logical root mismatch".into());
    }
    let mut seen = HashSet::new();
    for a in payload["artifacts"]
        .as_array()
        .ok_or("topology artifacts absent")?
    {
        let name = a["path"].as_str().ok_or("topology artifact name absent")?;
        if !seen.insert(name) {
            return Err("duplicate topology artifact".into());
        }
        let bytes = fs::read(seal.join(name))?;
        if sha256_bytes(&bytes) != a["sha256"] {
            return Err(format!("topology artifact mismatch: {name}").into());
        }
    }
    Ok(root.to_owned())
}

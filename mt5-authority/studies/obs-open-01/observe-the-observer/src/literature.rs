use crate::source::sha256_bytes;
use hashbrown::HashSet;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const STATIC_ROOT: &str = "145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d";
const OTO_A_ROOT: &str = "a6a8dc1b01dcb97fcd75dde29594d472fbcc526b44add2263c89c4a89a1accc0";
const OTO_A1_ROOT: &str = "6f5209a690d0a860bf9916741d5298275a7b18d1c7fe984ab2d5aefb962a0157";
const TOPOLOGY_ROOT: &str = "5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1";
const CLOSURE_ROOT: &str = "b96ad31eb43f30ec5026a71a1d607d95e0041543f67418dd20180fabf3c7db71";
const ACCESS_ROOT: &str = "mt5-authority/studies/obs-open-01/observe-the-observer/authority-surface/seal/OTO_A1_ACCESS_AUDIT.json";

fn read_bytes(repo: &Path, rel: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(fs::read(repo.join(rel))?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn artifact_entry(dir: &Path, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = fs::read(dir.join(name))?;
    Ok(json!({"path": name, "bytes": bytes.len(), "sha256": sha256_bytes(&bytes)}))
}

fn source(
    title: &str,
    authors: &[&str],
    year: u16,
    venue: &str,
    doi: Option<&str>,
    url: &str,
) -> Value {
    json!({"title":title,"authors":authors,"year":year,"venue":venue,"doi":doi,"url":url})
}

#[allow(clippy::too_many_arguments)]
fn record(
    id: &str,
    track: &str,
    source_obj: Value,
    mechanism: &str,
    formalism: &str,
    problem: &str,
    relevance: &str,
    membership: &str,
    transfer: &str,
    term_alignment: &str,
    missing: &[&str],
    conflicts: &[&str],
    note: &str,
) -> Value {
    json!({
        "schema":"OTO_L_RECORD_V1", "record_id":id, "track":track, "mechanism_id":mechanism,
        "observer_source_basis":["OTO_STATIC_ROOT","OTO_A1_SURFACE_ALIGNMENT"],
        "oto_static_roots":[STATIC_ROOT,OTO_A_ROOT,OTO_A1_ROOT,TOPOLOGY_ROOT,CLOSURE_ROOT],
        "related_gates":["G0","G1","G2","G3","G4","G5","G6","G7","G8","OTO-A1"],
        "literature_object":source_obj, "formalism":formalism, "problem_class":problem,
        "relevance_kind":relevance, "formalism_membership":membership, "theorem_transfer":transfer,
        "missing_hypotheses":missing, "conflicting_hypotheses":conflicts,
        "term_alignment":term_alignment, "relevance_note":note,
        "authority_gain":"NONE", "membership_scope":"ADJACENCY_ONLY", "economic_authority":"NONE"
    })
}

fn records() -> Vec<Value> {
    vec![
        record(
            "OTO_L_M01",
            "MECHANISM",
            source(
                "Runtime Verification for LTL and TLTL",
                &["Andreas Bauer", "Martin Leucker", "Christian Schallhart"],
                2011,
                "ACM TOSEM",
                Some("10.1145/2000799.2000800"),
                "https://doi.org/10.1145/2000799.2000800",
            ),
            "CAUSAL_EVENT_MONITORING",
            "runtime verification / temporal monitors",
            "finite-trace monitoring",
            "ANALYSIS_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a formal mapping from the sentinel event grammar to LTL/TLTL",
                "a declared verdict semantics for source gaps and provisional bars",
            ],
            &["04A has a qualified causal kernel and typed emissions, not an asserted LTL monitor"],
            "Useful vocabulary for monitoring ordered traces; no membership or theorem transfer is claimed.",
        ),
        record(
            "OTO_L_M02",
            "MECHANISM",
            source(
                "A Theory of Timed Automata",
                &["Rajeev Alur", "David L. Dill"],
                1994,
                "Theoretical Computer Science",
                Some("10.1016/0304-3975(94)90010-8"),
                "https://doi.org/10.1016/0304-3975(94)90010-8",
            ),
            "SESSION_TIME_AND_COMMIT",
            "timed automata",
            "timed words and transition constraints",
            "ANALYSIS_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a proved encoding of integer-tick bars, session resets, and knowledge time",
                "a reachable timed-language correspondence",
            ],
            &[
                "the paper's timed-word model does not establish 04A's coverage or platform semantics",
            ],
            "Relevant for timing vocabulary and possible future reachability work; integer ticks are not automatically timed-automaton membership.",
        ),
        record(
            "OTO_L_M03",
            "MECHANISM",
            source(
                "Processing Flows of Information: From Data Stream to Complex Event Processing",
                &["Gianpaolo Cugola", "Alessandro Margara"],
                2012,
                "ACM Computing Surveys",
                Some("10.1145/2187671.2187677"),
                "https://doi.org/10.1145/2187671.2187677",
            ),
            "STREAMS_GAPS_AND_EVENT_GRAMMAR",
            "information-flow processing / complex event processing",
            "ordered streams, event patterns, and processing semantics",
            "TERMINOLOGY",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a common stream contract for 04A source gaps and session ownership",
                "proof that the observer's gap policies satisfy a surveyed model",
            ],
            &[
                "surveyed systems have heterogeneous data/rule models; 04A's grammar is not thereby a CEP engine",
            ],
            "Strong vocabulary adjacency for streams and event ordering; source gaps remain observer-specific.",
        ),
        record(
            "OTO_L_M04",
            "MECHANISM",
            source(
                "Records: Mathematical Theory",
                &["Valery B. Nevzorov"],
                2001,
                "American Mathematical Society, Translations of Mathematical Monographs 194",
                None,
                "https://bookstore.ams.org/MMONO/194",
            ),
            "RUNNING_EXTREMA_AND_RENEWAL",
            "record-value theory",
            "record times, record values, and extrema",
            "ALGORITHM",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a stochastic law for the actual source path",
                "a proof that strict-renewal genealogy has the assumptions of a record process",
            ],
            &[
                "04A's sentinels are a deterministic causal observer; no iid or distributional model is authorized",
            ],
            "The record-process neighborhood names a mathematical shape but does not certify the observer as a record process.",
        ),
        record(
            "OTO_L_M05",
            "MECHANISM",
            source(
                "Runtime Verification for LTL and TLTL",
                &["Andreas Bauer", "Martin Leucker", "Christian Schallhart"],
                2011,
                "ACM TOSEM",
                Some("10.1145/2000799.2000800"),
                "https://doi.org/10.1145/2000799.2000800",
            ),
            "ORDERED_EMISSIONS",
            "finite-state monitor / transduction",
            "event emission and trace semantics",
            "TERMINOLOGY",
            "NOT_PROVEN",
            "NOT_ATTEMPTED",
            "HOMONYM_ONLY",
            &["an explicit output alphabet and transducer construction for 04A"],
            &["the word observer/monitor is used in different semantic senses"],
            "Negative adjacency is intentional: similar monitor language does not establish a transducer model.",
        ),
        record(
            "OTO_L_M06",
            "MECHANISM",
            source(
                "A Theory of Timed Automata",
                &["Rajeev Alur", "David L. Dill"],
                1994,
                "Theoretical Computer Science",
                Some("10.1016/0304-3975(94)90010-8"),
                "https://doi.org/10.1016/0304-3975(94)90010-8",
            ),
            "PROVISIONAL_COMMITTED_KNOWLEDGE",
            "timed transition systems",
            "event time versus state-commit timing",
            "ANALYSIS_METHOD",
            "NOT_PROVEN",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a formal context carrying knowledge time and provisional state",
                "a timed transition relation extracted from G1",
            ],
            &[
                "04A's provisional/committed split is source-qualified but not yet mapped to timed automata",
            ],
            "May guide a later formalization; no current membership claim.",
        ),
        record(
            "OTO_L_Q01",
            "QUALIFICATION",
            source(
                "Program Slicing",
                &["Mark Weiser"],
                1981,
                "ICSE 1981",
                None,
                "https://dblp.org/rec/conf/icse/Weiser81",
            ),
            "DEPENDENCY_AND_IMPACT",
            "program slicing / dependence analysis",
            "change-impact and dependency closure",
            "ANALYSIS_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a sound slice criterion for the semantic observer rather than implementation text alone",
                "a proved mapping from OTO dependency edges to protected observables",
            ],
            &["source-level dependence is not itself a G4 equivalence proof"],
            "Relevant to OTO-I/C/A dependency maps; explicitly not a certificate of removability.",
        ),
        record(
            "OTO_L_Q02",
            "QUALIFICATION",
            source(
                "Abstract interpretation: A unified lattice model for static analysis of programs by construction or approximation of fixpoints",
                &["Patrick Cousot", "Radhia Cousot"],
                1977,
                "POPL 1977",
                Some("10.1145/512950.512973"),
                "https://doi.org/10.1145/512950.512973",
            ),
            "SOUND_OVERAPPROXIMATION",
            "abstract interpretation",
            "sound abstraction and fixpoint approximation",
            "PROOF_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "an abstract domain for the G3 context-indexed reachable machine",
                "soundness and completeness conditions for any proposed abstract domain",
            ],
            &["an overapproximation is not an exact 04A reachability characterization"],
            "Provides a candidate proof language for sound bounds; no G3 theorem is imported.",
        ),
        record(
            "OTO_L_Q03",
            "QUALIFICATION",
            source(
                "Counterexample-Guided Abstraction Refinement",
                &[
                    "Edmund M. Clarke",
                    "Orna Grumberg",
                    "Somesh Jha",
                    "Yuan Lu",
                    "Helmut Veith",
                ],
                2000,
                "CAV 2000",
                Some("10.1007/10722167_15"),
                "https://doi.org/10.1007/10722167_15",
            ),
            "WITNESS_AND_REFINEMENT",
            "CEGAR",
            "spurious counterexamples and abstraction refinement",
            "COUNTEREXAMPLE_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a qualified abstraction and a G7-authorized property",
                "an exact validation path for 04A witness candidates",
            ],
            &["a solver's counterexample is not automatically a reachable observer history"],
            "Directly adjacent to future G3/G8 work; no current verifier authority.",
        ),
        record(
            "OTO_L_Q04",
            "QUALIFICATION",
            source(
                "Proof-Carrying Code",
                &["George C. Necula"],
                1997,
                "POPL 1997",
                Some("10.1145/263699.263712"),
                "https://doi.org/10.1145/263699.263712",
            ),
            "CERTIFICATE_LINEAGE",
            "proof-carrying code / proof checking",
            "certificate evidence versus verifier soundness",
            "VERIFICATION_METHOD",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a named proof calculus for the exact G7 properties",
                "an independently justified verifier TCB",
            ],
            &["a certificate format alone does not prove the semantic theorem required by G8"],
            "Terminology and architecture adjacency only; reinforces the certificate/verifier separation.",
        ),
        record(
            "OTO_L_Q05",
            "QUALIFICATION",
            source(
                "Abstract interpretation: A unified lattice model for static analysis of programs by construction or approximation of fixpoints",
                &["Patrick Cousot", "Radhia Cousot"],
                1977,
                "POPL 1977",
                Some("10.1145/512950.512973"),
                "https://doi.org/10.1145/512950.512973",
            ),
            "REACHABILITY_BOUNDS",
            "abstract interpretation",
            "under- and over-approximation with known direction",
            "PROOF_METHOD",
            "NOT_PROVEN",
            "NOT_ATTEMPTED",
            "HOMONYM_ONLY",
            &[
                "a G3 sound upper envelope and constructive lower witnesses",
                "a transfer theorem connecting bounds to the observer's admissible traces",
            ],
            &["absence of an abstract state does not prove unreachability without completeness"],
            "Negative adjacency preserves the G3 sandwich discipline.",
        ),
        record(
            "OTO_L_Q06",
            "QUALIFICATION",
            source(
                "Program Slicing",
                &["Mark Weiser"],
                1981,
                "ICSE 1981",
                None,
                "https://dblp.org/rec/conf/icse/Weiser81",
            ),
            "DISPLAY_VS_COB_SURFACE",
            "program slicing",
            "separating data dependence from display-only configuration",
            "ANALYSIS_METHOD",
            "NOT_PROVEN",
            "NOT_ATTEMPTED",
            "HOMONYM_ONLY",
            &[
                "a slice criterion that names G4 protected observables rather than pixels",
                "an instrument-capture model",
            ],
            &["a display attribute can be observable to a human but absent from the COB surface"],
            "This is the explicit negative-adjacency record for the A1 four-way surface separation.",
        ),
        record(
            "OTO_L_Q07",
            "QUALIFICATION",
            source(
                "Proof-Carrying Code",
                &["George C. Necula"],
                1997,
                "POPL 1997",
                Some("10.1145/263699.263712"),
                "https://doi.org/10.1145/263699.263712",
            ),
            "AUTHORITY_REUSE",
            "proof reuse / refinement",
            "when an earlier certificate remains valid after a change",
            "PROOF_METHOD",
            "NOT_PROVEN",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a requalification theorem for the exact OTO gate lineage",
                "a proof that the changed surface is outside the verifier's observations",
            ],
            &["the original OTO-A seal is immutable and cannot be retroactively relabeled"],
            "Supports the campaign doctrine: corrective authority is appended, not history-rewriting.",
        ),
        record(
            "OTO_L_Q08",
            "QUALIFICATION",
            source(
                "A Theory of Timed Automata",
                &["Rajeev Alur", "David L. Dill"],
                1994,
                "Theoretical Computer Science",
                Some("10.1016/0304-3975(94)90010-8"),
                "https://doi.org/10.1016/0304-3975(94)90010-8",
            ),
            "FORMAL_DECISION_BOUNDARY",
            "timed automata",
            "decidability and complexity boundaries",
            "TERMINOLOGY",
            "PARTIAL_INTERFACE_MATCH",
            "NOT_ATTEMPTED",
            "PARTIAL_MATCH",
            &[
                "a formal encoding of the observer's actual arithmetic and input language",
                "a proof that the desired property lies in the paper's decision fragment",
            ],
            &["G7 already permits NOT_DECIDABLE; resemblance does not change that boundary"],
            "Useful for naming a future formalism boundary, not for claiming decidability now.",
        ),
    ]
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.is_file());
    paths.sort();
    Ok(paths)
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
    let access_bytes = read_bytes(repo, ACCESS_ROOT)?;
    let mut entries = records();
    entries.sort_by(|a, b| a["record_id"].as_str().cmp(&b["record_id"].as_str()));
    let mut track_counts = BTreeMap::<String, usize>::new();
    let mut membership_counts = BTreeMap::<String, usize>::new();
    let mut transfer_counts = BTreeMap::<String, usize>::new();
    for entry in &entries {
        *track_counts
            .entry(entry["track"].as_str().unwrap_or("UNKNOWN").to_owned())
            .or_insert(0) += 1;
        *membership_counts
            .entry(
                entry["formalism_membership"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_owned(),
            )
            .or_insert(0) += 1;
        *transfer_counts
            .entry(
                entry["theorem_transfer"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_owned(),
            )
            .or_insert(0) += 1;
    }
    let negative = entries
        .iter()
        .filter(|entry| {
            entry["formalism_membership"] == "NOT_PROVEN"
                || entry["formalism_membership"] == "HOMONYM_ONLY"
        })
        .cloned()
        .collect::<Vec<_>>();
    let protocol = json!({
        "schema":"OTO_L_PROTOCOL_V1", "gate":"OTO-L_LITERATURE_FORMALISM_ADJACENCY_ATLAS", "execution_state":"SEALED", "question_status":"CLOSED",
        "result":"LITERATURE_ADJACENCY_ATLAS_SEALED", "disposition":"NONE",
        "question":"Which established methods and formal neighborhoods are adjacent to the already-qualified observer mechanisms and qualification problems?",
        "tracks":["MECHANISM","QUALIFICATION"], "authority_gain":"NONE", "formalism_membership_default":"NOT_PROVEN",
        "theorem_transfer_default":"NOT_ATTEMPTED", "negative_adjacency_required":true, "term_alignment_required":true,
        "prohibitions":["NO_OBSERVER_REDESIGN","NO_FORMALISM_MEMBERSHIP_BY_RESEMBLANCE","NO_THEOREM_TRANSFER","NO_MARKET_OR_OUTCOME_READS"]
    });
    let registry =
        json!({"schema":"OTO_L_RECORD_REGISTRY_V1","record_count":entries.len(),"records":entries});
    let mechanism_index = json!({"schema":"OTO_L_MECHANISM_INDEX_V1","track":"MECHANISM","record_ids":registry["records"].as_array().unwrap().iter().filter(|x| x["track"] == "MECHANISM").map(|x| x["record_id"].clone()).collect::<Vec<_>>()});
    let qualification_index = json!({"schema":"OTO_L_QUALIFICATION_INDEX_V1","track":"QUALIFICATION","record_ids":registry["records"].as_array().unwrap().iter().filter(|x| x["track"] == "QUALIFICATION").map(|x| x["record_id"].clone()).collect::<Vec<_>>()});
    let negative_artifact = json!({"schema":"OTO_L_NEGATIVE_ADJACENCY_V1","count":negative.len(),"records":negative,"interpretation":"negative adjacency is a valid atlas result and carries no authority loss"});
    let access = json!({"schema":"OTO_L_ACCESS_AUDIT_V1","static_roots_read":true,"a1_access_audit_read":true,"source_bytes_read":0,"market_rows_read":0,"outcome_rows_read":0,"indicator_buffers_read":0,"runtime_probe_executed":false,"d_b_reads":0,"d_c_reads":0,"d_d_reads":0,"trading_com_editor_invoked":false,"authority_gain":"NONE","a1_access_audit_sha256":sha256_bytes(&access_bytes)});
    let findings = json!({"schema":"OTO_L_TYPED_FINDINGS_V1","execution_state":"SEALED","question_status":"CLOSED","result":"LITERATURE_ADJACENCY_ATLAS_SEALED","record_count":registry["record_count"],"track_counts":track_counts,"membership_counts":membership_counts,"theorem_transfer_counts":transfer_counts,"negative_adjacency_count":negative.len(),"authority_gain":"NONE","maximum_authority":"OBS_OPEN_OTO_STATIC_LITERATURE_FORMALISM_ADJACENCY_V1"});
    let artifact_values = [
        ("OTO_L_PROTOCOL_V1.json", protocol),
        ("OTO_L_RECORD_REGISTRY.json", registry),
        ("OTO_L_MECHANISM_INDEX.json", mechanism_index),
        ("OTO_L_QUALIFICATION_INDEX.json", qualification_index),
        ("OTO_L_NEGATIVE_ADJACENCY.json", negative_artifact),
        ("OTO_L_ACCESS_AUDIT.json", access),
        ("OTO_L_TYPED_FINDINGS.json", findings),
    ];
    for (name, value) in &artifact_values {
        write_json(&out.join(name), value)?;
    }
    let artifacts = artifact_values
        .iter()
        .map(|(name, _)| artifact_entry(out, name))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({"schema":"OTO_L_ROOT_PAYLOAD_V1","authority":"OBS_OPEN_OTO_STATIC_LITERATURE_FORMALISM_ADJACENCY_V1","parent_static_root":STATIC_ROOT,"parent_oto_a_root":OTO_A_ROOT,"parent_oto_a1_root":OTO_A1_ROOT,"parent_topology_root":TOPOLOGY_ROOT,"parent_closure_root":CLOSURE_ROOT,"literature_record_count":entries.len(),"artifact_count":artifacts.len(),"artifacts":artifacts});
    let root = sha256_bytes(&serde_json::to_vec(&payload)?);
    write_json(
        &out.join("OTO_L_ROOT_RECEIPT.json"),
        &json!({"schema":"OTO_L_ROOT_RECEIPT_V1","logical_root":root,"payload":payload}),
    )?;
    Ok(root)
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
    for (left, right) in af.iter().zip(&bf) {
        if fs::read(left)? != fs::read(right)? {
            return Err(format!("independent build mismatch: {}", left.display()).into());
        }
    }
    fs::create_dir_all(seal)?;
    for path in &af {
        fs::copy(path, seal.join(path.file_name().unwrap()))?;
    }
    let root: Value = serde_json::from_slice(&fs::read(seal.join("OTO_L_ROOT_RECEIPT.json"))?)?;
    let logical_root = root["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    let set_payload = af.iter().map(|p| { let bytes = fs::read(p).unwrap(); json!({"path":p.file_name().unwrap().to_string_lossy(),"sha256":sha256_bytes(&bytes),"bytes":bytes.len()}) }).collect::<Vec<_>>();
    write_json(
        &seal.join("OTO_L_DETERMINISTIC_REBUILD_RECEIPT.json"),
        &json!({"schema":"OTO_L_DETERMINISTIC_REBUILD_RECEIPT_V1","build_a_artifact_count":af.len(),"build_b_artifact_count":bf.len(),"mismatch_count":0,"byte_identical":true,"artifact_set_hash":sha256_bytes(&serde_json::to_vec(&set_payload)?),"logical_root":logical_root}),
    )?;
    Ok(logical_root.to_owned())
}

pub fn verify(seal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(seal.join("OTO_L_ROOT_RECEIPT.json"))?)?;
    let payload = receipt
        .get("payload")
        .ok_or("root receipt missing payload")?;
    let expected = receipt["logical_root"]
        .as_str()
        .ok_or("root receipt missing logical_root")?;
    if sha256_bytes(&serde_json::to_vec(payload)?) != expected {
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
        if bytes.len() != artifact["bytes"].as_u64().ok_or("artifact bytes missing")? as usize {
            return Err(format!("artifact size mismatch: {name}").into());
        }
        if sha256_bytes(&bytes) != artifact["sha256"].as_str().ok_or("artifact hash missing")? {
            return Err(format!("artifact hash mismatch: {name}").into());
        }
    }
    Ok(expected.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_are_sha256() {
        for value in [
            STATIC_ROOT,
            OTO_A_ROOT,
            OTO_A1_ROOT,
            TOPOLOGY_ROOT,
            CLOSURE_ROOT,
        ] {
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
    #[test]
    fn two_tracks_are_present() {
        let xs = records();
        assert!(xs.iter().any(|x| x["track"] == "MECHANISM"));
        assert!(xs.iter().any(|x| x["track"] == "QUALIFICATION"));
    }
}

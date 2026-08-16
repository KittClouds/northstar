use obs_open_04a_g1::kernel::{init, step};
use obs_open_04a_g1::model::{CompletedObservation, KernelContext, KernelError, StepResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

const G1_ROOT: &str = "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd";
const DESCENDANT_ID: &str = "G1_EXECUTABLE_DESCENDANT_V1";

#[derive(Debug, Deserialize)]
struct Corpus {
    schema: String,
    g1_root: String,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    case_id: String,
    context: KernelContext,
    observations: Vec<CompletedObservation>,
    expected_terminal: ExpectedTerminal,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
enum ExpectedTerminal {
    Applied,
    Rejected { error: String },
}

#[derive(Debug, Serialize)]
struct InvocationEnvelope {
    schema: &'static str,
    descendant_id: &'static str,
    g1_root: &'static str,
    cases: Vec<CaseResult>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    applied_steps: usize,
    terminal: &'static str,
    error: Option<String>,
    expected_match: bool,
    outputs: Vec<StepResult>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 || args[1] != "run" {
        return Err("usage: obs-open-04a-g1-executable-descendant run CORPUS.json OUTPUT.json".into());
    }
    let corpus: Corpus = serde_json::from_slice(&fs::read(&args[2])?)?;
    if corpus.schema != "G1_R_SYNTHETIC_QUALIFICATION_CORPUS_V1" {
        return Err("UNRECOGNIZED_CORPUS_SCHEMA".into());
    }
    if corpus.g1_root != G1_ROOT {
        return Err("G1_ROOT_MISMATCH".into());
    }

    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in corpus.cases {
        cases.push(run_case(case)?);
    }
    let envelope = InvocationEnvelope {
        schema: "G1_R_EXECUTION_ENVELOPE_V1",
        descendant_id: DESCENDANT_ID,
        g1_root: G1_ROOT,
        cases,
    };
    let bytes = canonical_json(&envelope)?;
    fs::write(&args[3], &bytes)?;
    println!("{}", hex_sha256(&bytes));
    Ok(())
}

fn run_case(case: Case) -> Result<CaseResult, Box<dyn std::error::Error>> {
    let mut state = init(&case.context).map_err(|error| format_kernel_error("init", error))?;
    let mut outputs = Vec::with_capacity(case.observations.len());
    let mut error = None;
    for observation in case.observations {
        match step(&state, &observation, &case.context) {
            Ok(result) => {
                state = result.state.clone();
                outputs.push(result);
            }
            Err(value) => {
                error = Some(value.to_string());
                break;
            }
        }
    }
    let (terminal, expected_match) = match &case.expected_terminal {
        ExpectedTerminal::Applied => ("APPLIED", error.is_none()),
        ExpectedTerminal::Rejected { error: expected } => (
            "REJECTED",
            error.as_deref() == Some(expected.as_str()),
        ),
    };
    Ok(CaseResult {
        case_id: case.case_id,
        applied_steps: outputs.len(),
        terminal,
        error,
        expected_match,
        outputs,
    })
}

fn format_kernel_error(stage: &str, error: KernelError) -> Box<dyn std::error::Error> {
    format!("{stage}:{error}").into()
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
    path.exists()
}

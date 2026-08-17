use obs_open_03a::{load_atlas_authority, raw_path};
use obs_open_04a_g1::{
    audit::{context_from_session, observation_from_bar},
    kernel::{init, step},
    model::{Emission, InputAuthority, KernelState, StepResult},
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    env,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

const DA_PROTOCOL_ROOT: &str = "3d1da3657154d5a4e11eca99cab021c71c9c8a5d6d4458469133f3d2b8e3a402";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arm {
    P1,
    P2,
    P3,
    P4,
    P5,
}

impl Arm {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "SOL-P1" => Ok(Self::P1),
            "SOL-P2" => Ok(Self::P2),
            "SOL-P3" => Ok(Self::P3),
            "SOL-P4" => Ok(Self::P4),
            "SOL-P5" => Ok(Self::P5),
            _ => Err(format!("UNKNOWN_ARM:{value}")),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::P1 => "SOL-P1",
            Self::P2 => "SOL-P2",
            Self::P3 => "SOL-P3",
            Self::P4 => "SOL-P4",
            Self::P5 => "SOL-P5",
        }
    }
}

#[derive(Debug)]
struct Args {
    authority_repo: PathBuf,
    raw: PathBuf,
    arm: Arm,
}

#[derive(Serialize)]
struct Packet<'a> {
    schema: &'static str,
    arm_id: &'static str,
    specimen_id: String,
    path_complete: bool,
    record_count: usize,
    records: &'a [Value],
}

#[derive(Serialize)]
struct AccessReceipt {
    schema: &'static str,
    arm_id: &'static str,
    source_authority: &'static str,
    raw_source_hash: String,
    firewall_manifest_hash: String,
    d_a_sessions_decoded: usize,
    d_a_retained_causal_bars: usize,
    d_a_path_gap_sessions: usize,
    d_b_ohlc_values_decoded: usize,
    d_c_observations_read: usize,
    d_d_observations_read: usize,
    target_reads: usize,
    outcome_reads: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    verify_parent_roots(&args.authority_repo)?;
    let loaded = load_atlas_authority(&args.authority_repo, &args.raw)?;
    let mut output = BufWriter::with_capacity(1 << 20, io::stdout().lock());
    let mut records = Vec::with_capacity(390);

    for session in &loaded.sessions {
        records.clear();
        let context = context_from_session(session)?;
        let mut state = init(&context)?;
        for bar in &session.bars {
            let observation =
                observation_from_bar(bar, InputAuthority::CanonicalIntegerM1BridgeV1)?;
            let result = step(&state, &observation, &context)?;
            records.push(project(args.arm, &observation, &result));
            state = result.state;
        }
        serde_json::to_writer(
            &mut output,
            &Packet {
                schema: "NATIVE_EYE_REAL_LIGHT_02_PACKET_V1",
                arm_id: args.arm.id(),
                specimen_id: specimen_id(&session.spec.session_id),
                path_complete: session.path_complete,
                record_count: records.len(),
                records: &records,
            },
        )?;
        output.write_all(b"\n")?;
    }

    let access = AccessReceipt {
        schema: "NATIVE_EYE_REAL_LIGHT_02_ACCESS_RECEIPT_V1",
        arm_id: args.arm.id(),
        source_authority: "D_A_CANONICAL_INTEGER_M1_G1_APPLIED_TRACE",
        raw_source_hash: loaded.raw_source_hash,
        firewall_manifest_hash: loaded.firewall_manifest_hash,
        d_a_sessions_decoded: loaded.access.d_a_sessions_decoded,
        d_a_retained_causal_bars: loaded.access.d_a_retained_causal_bars,
        d_a_path_gap_sessions: loaded.access.d_a_path_gap_sessions,
        d_b_ohlc_values_decoded: loaded.access.d_b_ohlc_values_decoded,
        d_c_observations_read: loaded.access.d_c_observations_read,
        d_d_observations_read: 0,
        target_reads: 0,
        outcome_reads: loaded.access.d_a_outcome_registry_applications
            + loaded.access.d_b_outcome_registry_applications
            + loaded.access.d_b_derived_outcomes_inspected,
    };
    eprintln!("ACCESS_RECEIPT={}", serde_json::to_string(&access)?);
    output.flush()?;
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut authority_repo = None;
    let mut raw = None;
    let mut arm = None;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{flag}"))?;
        match flag.as_str() {
            "--authority-repo" => authority_repo = Some(PathBuf::from(value)),
            "--raw" => raw = Some(PathBuf::from(value)),
            "--arm" => arm = Some(Arm::parse(&value)?),
            _ => return Err(format!("UNKNOWN_ARGUMENT:{flag}")),
        }
    }
    Ok(Args {
        authority_repo: authority_repo.ok_or("MISSING_AUTHORITY_REPO")?,
        raw: raw.unwrap_or_else(raw_path),
        arm: arm.ok_or("MISSING_ARM")?,
    })
}

fn verify_parent_roots(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let relative = Path::new("studies/obs-open-01/sentinel-behavioral-qualification-compound");
    let checks = [
        (
            "g0-authority-input-bridge/seal/G0_ROOT_RECEIPT.json",
            "G0_root",
            obs_open_04a_g1::G0_ROOT,
        ),
        (
            "g1-semantic-kernel/seal/G1_ROOT_RECEIPT.json",
            "G1_root",
            "65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd",
        ),
        (
            "g4-preservation-surface/seal/G4_ROOT_RECEIPT.json",
            "G4_root",
            "37ed98b4ebe447ef3c2152e550c99652d0157aea2c77b8a886379d1ba9e08e15",
        ),
    ];
    for (file, field, expected) in checks {
        let value: Value = serde_json::from_slice(&std::fs::read(repo.join(relative).join(file))?)?;
        if value.get(field).and_then(Value::as_str) != Some(expected)
            || value.get("status").and_then(Value::as_str) != Some("SEALED")
        {
            return Err(format!("PARENT_ROOT_MISMATCH:{file}").into());
        }
    }
    Ok(())
}

fn project(
    arm: Arm,
    observation: &obs_open_04a_g1::model::CompletedObservation,
    result: &StepResult,
) -> Value {
    match arm {
        Arm::P1 | Arm::P4 => temporal_record(observation, result),
        Arm::P2 => protected_record(result),
        Arm::P3 => scalar_record(observation, &result.state, true),
        Arm::P5 => scalar_record(observation, &result.state, false),
    }
}

fn temporal_record(
    observation: &obs_open_04a_g1::model::CompletedObservation,
    result: &StepResult,
) -> Value {
    let state = &result.state;
    let upper = state.upper.as_ref().expect("applied G1 state");
    let lower = state.lower.as_ref().expect("applied G1 state");
    json!({
        "causal_ordinal": state.bar_index,
        "input_knowledge_time_ns": observation.knowledge_time_ns,
        "state_knowledge_time_ns": state.knowledge_time_ns,
        "commit_knowledge_time_ns": observation.knowledge_time_ns,
        "upper_birth_knowledge_time_ns": upper.birth_knowledge_time_ns,
        "lower_birth_knowledge_time_ns": lower.birth_knowledge_time_ns,
    })
}

fn scalar_record(
    observation: &obs_open_04a_g1::model::CompletedObservation,
    state: &KernelState,
    include_discrete: bool,
) -> Value {
    let upper = state.upper.as_ref().expect("applied G1 state");
    let lower = state.lower.as_ref().expect("applied G1 state");
    let mut record = Map::new();
    record.insert("causal_ordinal".into(), json!(state.bar_index));
    record.insert(
        "input_knowledge_time_ns".into(),
        json!(observation.knowledge_time_ns),
    );
    record.insert(
        "state_knowledge_time_ns".into(),
        json!(state.knowledge_time_ns),
    );
    record.insert(
        "upper_birth_knowledge_time_ns".into(),
        json!(upper.birth_knowledge_time_ns),
    );
    record.insert(
        "lower_birth_knowledge_time_ns".into(),
        json!(lower.birth_knowledge_time_ns),
    );
    record.insert("upper_birth_bar_index".into(), json!(upper.birth_bar_index));
    record.insert("lower_birth_bar_index".into(), json!(lower.birth_bar_index));
    record.insert("upper_age_bars".into(), json!(upper.age_bars));
    record.insert("lower_age_bars".into(), json!(lower.age_bars));
    record.insert("upper_value_ticks".into(), json!(upper.value_ticks));
    record.insert("lower_value_ticks".into(), json!(lower.value_ticks));
    if include_discrete {
        record.insert("initialized".into(), json!(state.initialized));
        record.insert("upper_id".into(), json!(upper.id));
        record.insert("lower_id".into(), json!(lower.id));
    }
    Value::Object(record)
}

fn protected_record(result: &StepResult) -> Value {
    json!({
        "causal_ordinal": result.state.bar_index,
        "protected_state": result.state,
        "emissions": result.emissions.ordered.iter().map(emission_view).collect::<Vec<_>>(),
    })
}

fn emission_view(emission: &Emission) -> Value {
    match emission {
        Emission::ObservationCommit {
            knowledge_time_ns,
            coverage,
            ..
        } => {
            json!({"event":"OBSERVATION_COMMIT","knowledge_time_ns":knowledge_time_ns,"coverage":coverage})
        }
        Emission::NewUpperExtreme { candidate_id } => {
            json!({"event":"NEW_UPPER_EXTREME","candidate_id":candidate_id})
        }
        Emission::UpperCandidateIdChange { prior, current } => {
            json!({"event":"UPPER_CANDIDATE_ID_CHANGE","prior":prior,"current":current})
        }
        Emission::NewLowerExtreme { candidate_id } => {
            json!({"event":"NEW_LOWER_EXTREME","candidate_id":candidate_id})
        }
        Emission::LowerCandidateIdChange { prior, current } => {
            json!({"event":"LOWER_CANDIDATE_ID_CHANGE","prior":prior,"current":current})
        }
        Emission::LocationTransition {
            k,
            prior,
            current,
            knowledge_time_ns,
        } => {
            json!({"event":"LOCATION_TRANSITION","k":k,"prior":prior,"current":current,"knowledge_time_ns":knowledge_time_ns})
        }
    }
}

fn specimen_id(session_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(DA_PROTOCOL_ROOT.as_bytes());
    hash.update([0]);
    hash.update(session_id.as_bytes());
    format!("{:x}", hash.finalize())
}

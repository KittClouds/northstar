use std::{env, path::PathBuf, process::ExitCode};

use northstar_holdout_eval::{
    AuthorizeArgs, EvaluateArgs, FreezeArgs, authorize_once, evaluate_once, freeze_protocol,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("northstar-holdout: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or("expected freeze or evaluate")?;
    let values = pairs(args.collect())?;
    match command.as_str() {
        "freeze" => {
            let protocol = freeze_protocol(&FreezeArgs {
                model_registry: required(&values, "--model-registry")?,
                candidate_protocol: required(&values, "--candidate-protocol")?,
                model_metrics: required(&values, "--model-metrics")?,
                nonlinear_metrics: required(&values, "--nonlinear-metrics")?,
                phase11_seal: required(&values, "--phase11-seal")?,
                reservations: required(&values, "--reservations")?,
                packed_receipt: required(&values, "--packed-receipt")?,
                output_dir: required(&values, "--output")?,
            })?;
            println!("PREAUTHORIZED_NOT_AUTHORIZED {}", protocol.protocol_sha256);
        }
        "authorize" => {
            let authorization = authorize_once(&AuthorizeArgs {
                protocol: required(&values, "--protocol")?,
                output: required(&values, "--output")?,
                confirmation: values
                    .get("--confirmation")
                    .cloned()
                    .ok_or("missing --confirmation")?,
            })?;
            println!(
                "AUTHORIZED_UNCONSUMED {}",
                authorization.authorization_token_sha256
            );
        }
        "evaluate" => evaluate_once(&EvaluateArgs {
            protocol: required(&values, "--protocol")?,
            authorization: required(&values, "--authorization")?,
            bundle: required(&values, "--bundle")?,
            state_dir: required(&values, "--state")?,
            report: required(&values, "--report")?,
        })?,
        _ => return Err(format!("unknown command {command}").into()),
    }
    Ok(())
}

fn pairs(
    values: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
    if !values.len().is_multiple_of(2) {
        return Err("arguments must be --name value pairs".into());
    }
    let mut out = std::collections::HashMap::new();
    for pair in values.chunks_exact(2) {
        if !pair[0].starts_with("--") || out.insert(pair[0].clone(), pair[1].clone()).is_some() {
            return Err(format!("invalid or duplicate argument {}", pair[0]).into());
        }
    }
    Ok(out)
}

fn required(
    values: &std::collections::HashMap<String, String>,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    values
        .get(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}").into())
}

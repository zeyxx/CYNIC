use std::fs;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::{SourceReleaseObservation, SourceReleaseRequest, plan_source_release};

pub fn execute_source_admission_plan(request_path: &str, observation_path: &str) -> ExitCode {
    let request = match read_json::<SourceReleaseRequest>(request_path, "request") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let observation = match read_json::<SourceReleaseObservation>(observation_path, "observation") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64);
    let plan = plan_source_release(&request, &observation, now_unix_ms);
    let admitted = plan.status == "admitted";

    match serde_json::to_string_pretty(&plan) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("source admission plan serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    if admitted {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str, label: &str) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("{label}_read_failed:{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{label}_decode_failed:{error}"))
}

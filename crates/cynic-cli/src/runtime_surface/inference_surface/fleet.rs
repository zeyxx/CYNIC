//! Read-only projection of the protocol-owned fleet inference reconciliation.

use std::process::ExitCode;

use cynic_protocol::{
    FleetInferenceDeployment, FleetInferenceNodeObservation, reconcile_fleet_inference,
};

pub(crate) fn execute_inference_fleet_plan(
    deployment_path: &str,
    observations_path: &str,
    json: bool,
) -> ExitCode {
    if !json {
        eprintln!("fleet inference planning requires --json for proof consumption");
        return ExitCode::FAILURE;
    }
    let deployment = match load_deployment(deployment_path) {
        Ok(deployment) => deployment,
        Err(error) => return fail(error),
    };
    let observations = match load_observations(observations_path) {
        Ok(observations) => observations,
        Err(error) => return fail(error),
    };
    let plan = reconcile_fleet_inference(&deployment, &observations);
    crate::emit_json(&plan, plan.status != "ready")
}

fn load_deployment(path: &str) -> Result<FleetInferenceDeployment, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read_fleet_inference_deployment:{path}:{error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("parse_fleet_inference_deployment:{path}:{error}"))
}

fn load_observations(path: &str) -> Result<Vec<FleetInferenceNodeObservation>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read_fleet_inference_observations:{path}:{error}"))?;
    let observations: Vec<FleetInferenceNodeObservation> = serde_json::from_str(&raw)
        .map_err(|error| format!("parse_fleet_inference_observations:{path}:{error}"))?;
    if observations.is_empty() {
        return Err("fleet_inference_observations_empty".to_string());
    }
    Ok(observations)
}

fn fail(error: String) -> ExitCode {
    eprintln!("fleet inference plan blocked: {error}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_observation_array_is_explicitly_blocked() {
        let path = std::env::temp_dir().join("cynic-empty-fleet-inference-observations.json");
        std::fs::write(&path, "[]").expect("write test fixture");
        let result = load_observations(path.to_str().expect("utf8 path"));
        assert_eq!(result.unwrap_err(), "fleet_inference_observations_empty");
        let _ = std::fs::remove_file(path);
    }
}

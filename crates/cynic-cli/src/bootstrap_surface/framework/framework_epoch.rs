use std::fs;

use serde::de::DeserializeOwned;

use clap::Subcommand;
use cynic_host::framework_bootstrap;
use cynic_protocol::{DeploymentEpoch, DeploymentPhaseObservation};

use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum FrameworkEpochCommand {
    /// Compute the next deployment phase transition from signed epoch + observation.
    EpochAdvance {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        epoch: String,
        #[arg(long)]
        observation: String,
    },
    /// Compute a deployment epoch plan (no host mutation).
    EpochPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        desired_state_digest: String,
        #[arg(long)]
        target_scope: String,
        #[arg(long)]
        created_at_unix_ms: u64,
    },
}

pub(crate) fn execute_epoch(command: FrameworkEpochCommand) -> std::process::ExitCode {
    match command {
        FrameworkEpochCommand::EpochPlan {
            json,
            source_commit,
            desired_state_digest,
            target_scope,
            created_at_unix_ms,
        } => {
            if !json {
                return std::process::ExitCode::FAILURE;
            }
            match framework_bootstrap::plan_framework_epoch(
                &source_commit,
                &desired_state_digest,
                &target_scope,
                created_at_unix_ms,
            ) {
                Ok(plan) => emit_json(&plan, plan.status == "blocked"),
                Err(error) => fail(error),
            }
        }
        FrameworkEpochCommand::EpochAdvance {
            json,
            epoch,
            observation,
        } => {
            if !json {
                return std::process::ExitCode::FAILURE;
            }
            let epoch: DeploymentEpoch = match read_json_from_path(&epoch) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observation: DeploymentPhaseObservation = match read_json_from_path(&observation) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let transition = framework_bootstrap::advance_framework_epoch(
                &epoch,
                observation.phase,
                &observation,
            );
            emit_json(&transition, transition.status == "blocked")
        }
    }
}

fn read_json_from_path<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map_err(|error| error.to_string())
}

use std::path::PathBuf;

use clap::{Args, Subcommand};
use cynic_askesis::experiment::runner::ObserveReconcileReadBackPolicy;
use cynic_askesis::experiment::types::{ExperimentSpec, ServiceScenario};
use cynic_askesis::experiment::{plan_trials, run_reconciliation_episode};

#[derive(Debug, Args)]
pub(crate) struct ExperimentArgs {
    #[command(subcommand)]
    command: ExperimentCommand,
}

#[derive(Debug, Subcommand)]
enum ExperimentCommand {
    /// Expand a domain × dimension × scale specification without mutation.
    Plan {
        #[arg(value_name = "SPEC_JSON")]
        spec: PathBuf,
    },
    /// Run the deterministic service-reconciliation control policy in a simulator.
    ServiceLab {
        #[arg(value_name = "SCENARIO_JSON")]
        scenario: PathBuf,
        #[arg(long, default_value_t = 3)]
        max_actions: u32,
    },
}

pub(crate) fn run(args: ExperimentArgs) -> Result<(), Box<dyn std::error::Error>> {
    let output = match args.command {
        ExperimentCommand::Plan { spec } => {
            let spec: ExperimentSpec = serde_json::from_slice(&std::fs::read(spec)?)?;
            serde_json::to_value(plan_trials(&spec)?)?
        }
        ExperimentCommand::ServiceLab {
            scenario,
            max_actions,
        } => {
            let scenario: ServiceScenario = serde_json::from_slice(&std::fs::read(scenario)?)?;
            serde_json::to_value(run_reconciliation_episode(
                scenario,
                max_actions,
                &ObserveReconcileReadBackPolicy,
            ))?
        }
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

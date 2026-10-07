use std::process::ExitCode;

use clap::{Parser, Subcommand};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;

#[derive(Parser)]
#[command(
    name = "cynic-network-actuator",
    about = "Bounded root network actuator"
)]
struct ActuatorCli {
    #[command(subcommand)]
    command: ActuatorCommand,
}

#[derive(Subcommand)]
enum ActuatorCommand {
    Apply {
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> ExitCode {
    let cli = ActuatorCli::parse();
    match cli.command {
        ActuatorCommand::Apply {
            registry,
            profile,
            apply,
        } => {
            let observation = cynic_node::observe::observe_payload(Some(&registry));
            let report =
                cynic_node::network_actuator::apply_profile(&observation, &profile, apply);
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            if report.status == "completed" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

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
        } => cynic_node::execute_network_actuator_apply(&registry, &profile, apply),
    }
}

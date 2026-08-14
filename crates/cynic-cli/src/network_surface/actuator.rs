use std::process::ExitCode;

use cynic_node::NetworkCommands;

use crate::network_surface::{NetworkActuatorCommand, NetworkCommand};
use crate::registry_path;

pub(crate) fn try_execute_actuator(command: NetworkCommand) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Actuator { command } = command else {
        return Err(command);
    };

    let exit = match command {
        NetworkActuatorCommand::Status { registry } => {
            cynic_node::execute_network_command(&NetworkCommands::ActuatorStatus {
                json: true,
                registry: registry_path(registry),
            })
        }
        NetworkActuatorCommand::Bundle {
            registry,
            output_dir,
        } => cynic_node::execute_network_command(&NetworkCommands::ActuatorBundle {
            registry: registry_path(registry),
            output_dir,
        }),
        NetworkActuatorCommand::Plan {
            registry,
            bundle_dir,
        } => cynic_node::execute_network_command(&NetworkCommands::ActuatorPlan {
            registry: registry_path(registry),
            bundle_dir,
        }),
        NetworkActuatorCommand::Apply {
            registry,
            profile,
            apply,
        } => cynic_node::execute_network_command(&NetworkCommands::Apply {
            registry: registry_path(registry),
            profile,
            apply,
        }),
    };

    Ok(exit)
}

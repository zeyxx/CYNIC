use std::process::ExitCode;

use super::RuntimeCommand;

pub(crate) fn try_execute_compat_alias(
    command: RuntimeCommand,
) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::Memory { json } => Ok(crate::node_surface::host::execute_memory(json)),
        RuntimeCommand::Cpu { json } => Ok(crate::node_surface::host::execute_cpu(json)),
        RuntimeCommand::Disk { command } => {
            Ok(crate::node_surface::host::execute_disk_command(command))
        }
        RuntimeCommand::Network { command } => Ok(execute_network(command)),
        other => Err(other),
    }
}

fn execute_network(command: crate::runtime_surface::RuntimeNetworkCommand) -> ExitCode {
    use crate::runtime_surface::RuntimeNetworkCommand;
    match command {
        RuntimeNetworkCommand::Status { json } => super::network::execute_network(json),
        RuntimeNetworkCommand::Preflight {
            json,
            hosts,
            persist,
            endpoint,
        } => super::network_preflight::execute_network_preflight(
            json,
            &hosts,
            persist,
            endpoint.as_deref(),
        ),
    }
}

use std::process::ExitCode;

use super::RuntimeCommand;

pub(crate) fn try_execute_compat_alias(
    command: RuntimeCommand,
) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::Memory { json } => Ok(super::memory::execute_memory(json)),
        RuntimeCommand::Cpu { json } => Ok(super::cpu::execute_cpu(json)),
        RuntimeCommand::Disk { command } => Ok(execute_disk(command)),
        RuntimeCommand::Network { command } => Ok(execute_network(command)),
        other => Err(other),
    }
}

fn execute_disk(command: crate::runtime_surface::RuntimeDiskCommand) -> ExitCode {
    use crate::runtime_surface::RuntimeDiskCommand;
    match command {
        RuntimeDiskCommand::Status { json } => super::disk::execute_disk(json),
        RuntimeDiskCommand::Clean {
            json,
            apply,
            build_runner,
            registry,
            node,
            state_root,
        } => super::disk::execute_disk_clean(
            json,
            apply,
            build_runner,
            &registry,
            node.as_deref(),
            &state_root,
        ),
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

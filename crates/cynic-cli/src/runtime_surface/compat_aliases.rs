use std::process::ExitCode;

use super::RuntimeCommand;

pub(crate) fn try_execute_compat_alias(
    command: RuntimeCommand,
) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::Disk { json } => Ok(super::disk::execute_disk(json)),
        RuntimeCommand::Memory { json } => Ok(super::memory::execute_memory(json)),
        RuntimeCommand::Cpu { json } => Ok(super::cpu::execute_cpu(json)),
        RuntimeCommand::Network { json } => Ok(super::network::execute_network(json)),
        RuntimeCommand::DiskClean {
            json,
            apply,
            build_runner,
            registry,
            node,
            state_root,
        } => Ok(super::disk::execute_disk_clean(
            json,
            apply,
            build_runner,
            &registry,
            node.as_deref(),
            &state_root,
        )),
        other => Err(other),
    }
}

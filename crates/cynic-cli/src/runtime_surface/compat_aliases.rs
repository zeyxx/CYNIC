use std::process::ExitCode;

use super::RuntimeCommand;

pub(crate) fn try_execute_compat_alias(
    command: RuntimeCommand,
) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::Deps { json } => Ok(crate::deps_surface::execute_deps(json)),
        RuntimeCommand::DepsPlan { json } => Ok(crate::deps_surface::execute_deps_plan(json)),
        RuntimeCommand::DepsAudit { json } => Ok(crate::deps_surface::execute_deps_audit(json)),
        RuntimeCommand::DepsUpgrade {
            json,
            yes,
            apply,
            confirm,
            only,
        } => Ok(crate::deps_surface::execute_deps_upgrade(
            json,
            apply,
            yes,
            confirm.as_deref(),
            &only,
        )),
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

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
        RuntimeCommand::DiskClean { json, apply } => {
            Ok(super::disk::execute_disk_clean(json, apply))
        }
        other => Err(other),
    }
}

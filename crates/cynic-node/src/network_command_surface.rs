use serde::Serialize;

use cynic_host::command_surface::{
    HostCommandInvocation, HostCommandSurface, argv_command as host_argv_command,
    expectation as host_expectation, install_dir_command as host_install_dir_command,
    install_file_command as host_install_file_command,
    package_presence_command as host_package_presence_command, render_invocation,
    shell_command as host_shell_command,
};
use cynic_host::install_contract::InstallPathPolicy;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkCommandSurface {
    pub(crate) role: String,
    #[serde(flatten)]
    pub(crate) invocation: NetworkCommandInvocation,
}

pub(crate) type NetworkCommandInvocation = HostCommandInvocation;

pub(crate) fn from_host_command(command: HostCommandSurface) -> NetworkCommandSurface {
    NetworkCommandSurface {
        role: command.role,
        invocation: command.invocation,
    }
}

pub(crate) fn shell_command(
    role: &'static str,
    command: impl Into<String>,
) -> NetworkCommandSurface {
    from_host_command(host_shell_command(role, command))
}

pub(crate) fn argv_command(role: &'static str, argv: Vec<String>) -> NetworkCommandSurface {
    from_host_command(host_argv_command(role, argv))
}

pub(crate) fn expectation(role: &'static str, text: impl Into<String>) -> NetworkCommandSurface {
    from_host_command(host_expectation(role, text))
}

pub(crate) fn package_presence_command(
    role: &'static str,
    package: impl Into<String>,
    query_bin: impl Into<String>,
    rpm_query_bin: impl Into<String>,
) -> NetworkCommandSurface {
    from_host_command(host_package_presence_command(
        role,
        package,
        query_bin,
        rpm_query_bin,
    ))
}

pub(crate) fn install_dir_command(
    role: &'static str,
    path: impl Into<String>,
    policy: InstallPathPolicy,
) -> NetworkCommandSurface {
    from_host_command(host_install_dir_command(role, path, policy))
}

pub(crate) fn install_file_command(
    role: &'static str,
    source: impl Into<String>,
    target: impl Into<String>,
    policy: InstallPathPolicy,
) -> NetworkCommandSurface {
    from_host_command(host_install_file_command(role, source, target, policy))
}

pub(crate) fn render_command(command: &NetworkCommandSurface) -> String {
    render_invocation(&command.invocation)
}

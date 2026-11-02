pub(crate) use cynic_network::command_surface::{
    HostCommandInvocation as NetworkCommandInvocation,
    HostCommandSurface as NetworkCommandSurface,
    argv_command, expectation, install_dir_command, install_file_command,
    package_presence_command, render_command, render_invocation, shell_command,
    validate_sudoers_command,
};

pub(crate) fn from_host_command(command: NetworkCommandSurface) -> NetworkCommandSurface {
    command
}

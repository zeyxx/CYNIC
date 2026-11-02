use crate::network_actuator::CommandExecution;
use crate::network_command_surface::NetworkCommandSurface;
use cynic_host::execution::{HostArgvCommand, execute_argv_commands};

pub(crate) fn run_command(command_surface: &NetworkCommandSurface) -> CommandExecution {
    let (role, argv) = match expand_command(command_surface) {
        Ok(expanded) => expanded,
        Err(execution) => return execution,
    };

    let report = execute_argv_commands(
        &[HostArgvCommand {
            role: role.clone(),
            argv: argv.clone(),
        }],
        true,
        true,
    );

    match report.executions.into_iter().next() {
        Some(execution) => CommandExecution {
            role: execution.role,
            argv: execution.argv,
            exit_code: execution.exit_code,
            stdout: execution.stdout,
            stderr: execution.stderr,
        },
        None => CommandExecution {
            role,
            argv,
            exit_code: None,
            stdout: String::new(),
            stderr: report.blockers.join("; "),
        },
    }
}

fn expand_command(
    command_surface: &NetworkCommandSurface,
) -> Result<(String, Vec<String>), CommandExecution> {
    match cynic_network::actuator::command_to_argv(command_surface) {
        Ok(argv) => Ok((command_surface.role.clone(), argv)),
        Err(error) => Err(CommandExecution {
            role: command_surface.role.clone(),
            argv: vec![],
            exit_code: None,
            stdout: String::new(),
            stderr: error,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::expand_command;
    use crate::network_command_surface::{
        NetworkCommandInvocation, NetworkCommandSurface, argv_command, expectation,
        package_presence_command,
    };
    use cynic_protocol::install_contract::{MODE_0755, root_owned_path};

    #[test]
    fn expand_command_rejects_non_executable_surfaces() {
        let expectation_surface = expectation("check", "do not execute me");
        let result = expand_command(&expectation_surface).unwrap_err();

        assert_eq!(result.role, "check");
        assert!(result.stderr.contains("host execution supports only argv/install/visudo surfaces"));
    }

    #[test]
    fn expand_command_builds_install_argv_deterministically() {
        let surface = NetworkCommandSurface {
            role: "install_helper".to_string(),
            invocation: NetworkCommandInvocation::InstallFile {
                source: "/tmp/helper".to_string(),
                target: "/usr/local/bin/helper".to_string(),
                policy: root_owned_path(MODE_0755),
            },
        };

        let (_, argv) = expand_command(&surface).unwrap();
        assert_eq!(
            argv,
            vec![
                "install",
                "-o",
                "root",
                "-g",
                "root",
                "-m",
                "0755",
                "/tmp/helper",
                "/usr/local/bin/helper"
            ]
        );
    }

    #[test]
    fn expand_command_preserves_plain_argv_surfaces() {
        let surface = argv_command("status", vec!["/usr/bin/true".to_string()]);
        let (_, argv) = expand_command(&surface).unwrap();
        assert_eq!(argv, vec!["/usr/bin/true"]);
    }

    #[test]
    fn expand_command_rejects_package_presence_without_shell_escape() {
        let surface = package_presence_command("package_presence", "coredns", "dpkg-query", "rpm");

        let result = expand_command(&surface).unwrap_err();

        assert_eq!(result.role, "package_presence");
        assert_eq!(result.argv, vec![] as Vec<String>);
        assert!(result.stderr.contains("host execution supports only argv/install/visudo surfaces"));
    }
}

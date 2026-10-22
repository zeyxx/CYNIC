//! Couche B — the SSH transport that carries the deploy commands.
//!
//! WHY A TRANSPORT AND NOT A SCRIPT. Measured on 2026-10-19: the fleet has
//! three different reachability shapes. The builder is an ssh-config alias
//! (`cynic-builder-ssh`); agent-host's registry alias
//! (`cynic-agent-host.synapse.cynic`) resolves ONLY inside the mesh DNS and
//! fails from the operator host, where the IP form works; cortex is the
//! local machine. A hardcoded command line would be wrong on at least one
//! of them, so the endpoint comes from the registry and the fallback is a
//! declared second try, not a guess.
//!
//! SAFETY. Every remote command is `ssh -o BatchMode=yes` (no password
//! prompt, no hang) with a connect timeout, and the remote argv is passed as
//! arguments — never as a shell string. The remote side runs a fixed
//! `sh -c` script that the pure layer rendered; the scripts are built from
//! registry-declared paths and validated before they cross the boundary.
//!
//! NO SHELL INJECTION. The remote script is one argv element. It is
//! rendered by the pure layer from validated path components, and this
//! module refuses to run a script that contains a newline or a NUL — the two
//! things that would turn one argv element into several commands.

use cynic_protocol::argv::{ArgvOutput, ArgvRunner};

use crate::deploy_locations::DeployMachine;
use crate::deploy_remote::{RemoteCommand, RemoteExecutorPort, RemoteOutcome, TransportForm};

/// SSH options every remote command carries.
pub const SSH_BATCH_OPTIONS: [&str; 6] = [
    "-o",
    "BatchMode=yes",
    "-o",
    "StrictHostKeyChecking=yes",
    "-o",
    "ConnectTimeout=10",
];

/// The endpoints to try for a machine, in order.
pub fn endpoint_candidates(machine: &DeployMachine) -> Vec<String> {
    let mut candidates = Vec::new();
    if machine.ssh_target == "local" {
        return candidates;
    }
    candidates.push(machine.ssh_target.clone());
    if let Some(fallback) = machine
        .ssh_target_fallback
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if !candidates.contains(fallback) {
            candidates.push(fallback.clone());
        }
    }
    candidates
}

/// Whether a machine is this host (the deploy runs without a transport).
pub fn is_local(machine: &DeployMachine) -> bool {
    machine.ssh_target == "local"
}

/// Refuse a remote script that could execute more than one command.
pub fn validate_remote_script(script: &str) -> Result<(), String> {
    if script.contains('\n') || script.contains('\r') {
        return Err("remote script must be a single line".to_string());
    }
    if script.contains('\0') {
        return Err("remote script must not contain NUL".to_string());
    }
    Ok(())
}

/// The SSH argv for a remote command.
pub fn ssh_argv(endpoint: &str, command: &RemoteCommand) -> Result<Vec<String>, String> {
    if endpoint.trim().is_empty() {
        return Err("empty endpoint".to_string());
    }
    if endpoint.starts_with('-') {
        return Err(format!("endpoint {endpoint} could be read as an option"));
    }
    if endpoint.contains(char::is_whitespace) {
        return Err(format!("endpoint {endpoint} contains whitespace"));
    }
    if endpoint.contains(';') || endpoint.contains('&') || endpoint.contains('|') {
        return Err(format!("endpoint {endpoint} contains a shell metacharacter"));
    }
    if command.argv.is_empty() {
        return Err("empty remote command".to_string());
    }
    // The remote program is `sh -c <script>`: one argv element, so the script
    // is never re-parsed by the local shell.
    if command.argv[0] == "sh" && command.argv.len() >= 3 && command.argv[1] == "-c" {
        validate_remote_script(&command.argv[2])?;
    }
    let mut argv = vec!["ssh".to_string()];
    argv.extend(SSH_BATCH_OPTIONS.iter().map(|option| option.to_string()));
    argv.push(endpoint.to_string());
    argv.extend(command.argv.iter().cloned());
    Ok(argv)
}

/// The SSH executor: carries a command to a machine and reports what came
/// back.
pub struct SshRemoteExecutor<R: ArgvRunner> {
    runner: R,
    /// The endpoints tried per machine, in order (the registry's declaration).
    endpoints: Vec<String>,
}

impl<R: ArgvRunner> SshRemoteExecutor<R> {
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            endpoints: Vec::new(),
        }
    }

    /// Declare the endpoint order. Empty means "use the registry's".
    pub fn with_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.endpoints = endpoints;
        self
    }

    fn candidates(&self, machine: &DeployMachine) -> Vec<String> {
        if !self.endpoints.is_empty() {
            return self.endpoints.clone();
        }
        endpoint_candidates(machine)
    }
}

impl<R: ArgvRunner> RemoteExecutorPort for SshRemoteExecutor<R> {
    fn run(&self, target: &DeployMachine, command: &RemoteCommand) -> RemoteOutcome {
        let mut last = RemoteOutcome {
            exit_code: 255,
            stdout: String::new(),
            stderr: "no endpoint attempted".to_string(),
            transport_failed: true,
        };
        for endpoint in self.candidates(target) {
            let argv = match ssh_argv(&endpoint, command) {
                Ok(argv) => argv,
                Err(error) => {
                    return RemoteOutcome {
                        exit_code: 255,
                        stdout: String::new(),
                        stderr: error,
                        transport_failed: true,
                    }
                }
            };
            let borrowed: Vec<&str> = argv.iter().map(|value| value.as_str()).collect();
            let output: ArgvOutput = self.runner.run(&borrowed);
            if let Some(error) = output.spawn_error {
                last = RemoteOutcome {
                    exit_code: 255,
                    stdout: output.stdout,
                    stderr: error,
                    transport_failed: true,
                };
                continue;
            }
            let exit_code = output.exit_code.unwrap_or(255);
            // A transport-level failure (255) is the SSH "cannot connect"
            // answer: try the next declared endpoint. Anything else is the
            // remote command's own verdict and stops the search.
            if exit_code == 255 {
                last = RemoteOutcome {
                    exit_code,
                    stdout: output.stdout,
                    stderr: output.stderr,
                    transport_failed: true,
                };
                continue;
            }
            return RemoteOutcome {
                exit_code,
                stdout: output.stdout,
                stderr: output.stderr,
                transport_failed: false,
            };
        }
        last
    }
}

/// The local executor: runs the command on this host, no transport.
pub struct LocalRemoteExecutor<R: ArgvRunner> {
    runner: R,
}

impl<R: ArgvRunner> LocalRemoteExecutor<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: ArgvRunner> RemoteExecutorPort for LocalRemoteExecutor<R> {
    fn run(&self, _target: &DeployMachine, command: &RemoteCommand) -> RemoteOutcome {
        if command.argv.is_empty() {
            return RemoteOutcome {
                exit_code: 255,
                stdout: String::new(),
                stderr: "empty command".to_string(),
                transport_failed: true,
            };
        }
        let borrowed: Vec<&str> = command.argv.iter().map(|value| value.as_str()).collect();
        let output = self.runner.run(&borrowed);
        if let Some(error) = output.spawn_error {
            return RemoteOutcome {
                exit_code: 255,
                stdout: output.stdout,
                stderr: error,
                transport_failed: true,
            };
        }
        RemoteOutcome {
            exit_code: output.exit_code.unwrap_or(255),
            stdout: output.stdout,
            stderr: output.stderr,
            transport_failed: false,
        }
    }
}

/// The transport form a machine will be reached through.
pub fn transport_for(machine: &DeployMachine) -> TransportForm {
    if is_local(machine) {
        TransportForm::Local
    } else {
        TransportForm::Ssh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_locations::DeployLayout;

    fn machine(ssh_target: &str, fallback: Option<&str>) -> DeployMachine {
        DeployMachine {
            hostname: "builder".to_string(),
            registry_node_id: "cynic-builder".to_string(),
            os: "Ubuntu".to_string(),
            ssh_target: ssh_target.to_string(),
            ssh_target_fallback: fallback.map(str::to_string),
            principal: "cynic".to_string(),
            deploy_path: "${state_root}/bin/cynic".to_string(),
            deploy_path_resolved: "/var/lib/cynic/bin/cynic".to_string(),
            path_owner: "cynic:cynic".to_string(),
            layout: DeployLayout::Symlink,
            backup_pattern: "binary.bak-<sha>".to_string(),
            readback: crate::deploy_locations::DeployReadback {
                sha256_command: "sha256sum /var/lib/cynic/bin/cynic".to_string(),
                identity_command: "/var/lib/cynic/bin/cynic version --json".to_string(),
                smoke_command: "/var/lib/cynic/bin/cynic source status --json".to_string(),
                smoke_expect: "exit code 0".to_string(),
            },
            measured: None,
        }
    }

    #[test]
    fn a_local_machine_has_no_endpoint() {
        assert!(endpoint_candidates(&machine("local", None)).is_empty());
        assert!(is_local(&machine("local", None)));
        assert_eq!(transport_for(&machine("local", None)), TransportForm::Local);
    }

    #[test]
    fn the_fallback_is_the_second_try() {
        let candidates = endpoint_candidates(&machine(
            "cynic-agent-host.synapse.cynic",
            Some("cynic@10.10.10.102"),
        ));
        assert_eq!(
            candidates,
            vec![
                "cynic-agent-host.synapse.cynic".to_string(),
                "cynic@10.10.10.102".to_string()
            ]
        );
    }

    #[test]
    fn a_machine_without_fallback_has_one_candidate() {
        assert_eq!(
            endpoint_candidates(&machine("cynic-builder-ssh", None)),
            vec!["cynic-builder-ssh".to_string()]
        );
    }

    #[test]
    fn the_ssh_argv_carries_the_batch_options() {
        let command = RemoteCommand {
            argv: vec!["sha256sum".to_string(), "/x".to_string()],
        };
        let argv = ssh_argv("cynic-builder-ssh", &command).expect("valid endpoint");
        assert_eq!(argv[0], "ssh");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(argv.contains(&"StrictHostKeyChecking=yes".to_string()));
        assert_eq!(argv.last().unwrap(), "/x");
    }

    #[test]
    fn a_malicious_endpoint_is_refused() {
        let command = RemoteCommand {
            argv: vec!["true".to_string()],
        };
        assert!(ssh_argv("-oProxyCommand=evil", &command).is_err());
        assert!(ssh_argv("host; rm -rf /", &command).is_err());
        assert!(ssh_argv("host name", &command).is_err());
        assert!(ssh_argv("  ", &command).is_err());
    }

    #[test]
    fn a_multi_line_remote_script_is_refused() {
        let command = RemoteCommand {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                "rm -rf /\nrm -rf /".to_string(),
            ],
        };
        assert!(ssh_argv("host", &command).is_err());
    }

    #[test]
    fn a_single_line_script_passes() {
        let command = RemoteCommand {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                "mv -f /a /b".to_string(),
            ],
        };
        assert!(ssh_argv("host", &command).is_ok());
    }
}

use std::collections::BTreeSet;
use std::time::Duration;

use base64::Engine;
use cynic_host::network_actuation::{
    NetworkActuationExecutor, NetworkActuationObservation, NetworkActuationObserver,
};
use cynic_host::runtime_harness::{
    RuntimeHarnessContext, RuntimeHarnessTargetAttestation,
    validate_runtime_harness_target_attestation,
};
<<<<<<< HEAD
use cynic_lifecycle::{ArgvOutput, ArgvRunner};
=======
use cynic_host::update_lifecycle::{ArgvOutput, ArgvRunner};
>>>>>>> github/main
use serde_json::{Value, json};

use crate::domain::network_execution::NetworkExecutionBoundary;
use crate::network_actuator::CommandExecution;
use crate::network_command_surface::{NetworkCommandInvocation, NetworkCommandSurface};
use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

pub struct QemuGuestAgentNetworkRunner<R> {
    runner: R,
    context: RuntimeHarnessContext,
    attestation: RuntimeHarnessTargetAttestation,
    virsh_bin: String,
    allowed_argv: BTreeSet<Vec<String>>,
    timeout_seconds: u64,
    max_status_polls: usize,
    poll_interval: Duration,
}

pub struct QemuGuestAgentNetworkObserver<'a, R> {
    transport: &'a QemuGuestAgentNetworkRunner<R>,
    observation_argv: Vec<String>,
}

impl<'a, R> QemuGuestAgentNetworkObserver<'a, R> {
    pub fn new(
        transport: &'a QemuGuestAgentNetworkRunner<R>,
        observation_argv: Vec<String>,
    ) -> Self {
        Self {
            transport,
            observation_argv,
        }
    }
}

impl<R> QemuGuestAgentNetworkRunner<R> {
    pub fn new(
        runner: R,
        context: RuntimeHarnessContext,
        attestation: RuntimeHarnessTargetAttestation,
        virsh_bin: impl Into<String>,
        allowed_argv: impl IntoIterator<Item = Vec<String>>,
        timeout_seconds: u64,
        max_status_polls: usize,
        poll_interval: Duration,
    ) -> Result<Self, String> {
        if attestation.provider != "qemu-libvirt" {
            return Err(format!(
                "qemu guest-agent transport refuses provider {}",
                attestation.provider
            ));
        }
        if attestation.transport != "qemu-guest-agent" {
            return Err(format!(
                "qemu guest-agent transport refuses attested transport {}",
                attestation.transport
            ));
        }
        if attestation.transport_target != attestation.target_id {
            return Err(
                "qemu guest-agent transport target must equal the attested libvirt domain identity"
                    .to_string(),
            );
        }
        if timeout_seconds == 0 || max_status_polls == 0 {
            return Err("qemu guest-agent timeout and status poll limit must be non-zero".into());
        }

        Ok(Self {
            runner,
            context,
            attestation,
            virsh_bin: virsh_bin.into(),
            allowed_argv: allowed_argv.into_iter().collect(),
            timeout_seconds,
            max_status_polls,
            poll_interval,
        })
    }

    fn execute_guest_argv(&self, role: String, argv: Vec<String>) -> CommandExecution
    where
        R: ArgvRunner,
    {
        if argv.is_empty() {
            return failed_execution(role, argv, "guest argv cannot be empty");
        }
        if !self.allowed_argv.contains(&argv) {
            return failed_execution(
                role,
                argv,
                "guest argv is not an exact member of the selected mesh profile",
            );
        }
        let attestation_violations =
            validate_runtime_harness_target_attestation(&self.context, &self.attestation);
        if !attestation_violations.is_empty() {
            let codes = attestation_violations
                .iter()
                .map(|violation| violation.code)
                .collect::<Vec<_>>()
                .join(",");
            return failed_execution(
                role,
                argv,
                format!("target attestation is no longer valid: {codes}"),
            );
        }

        let request = json!({
            "execute": "guest-exec",
            "arguments": {
                "path": argv[0],
                "arg": argv[1..],
                "capture-output": true,
            }
        });
        let spawn = self.qga_call(&request);
        if !spawn.success() {
            return transport_failure(role, argv, "guest-exec", &spawn);
        }
        let pid = match response_return(&spawn)
            .and_then(|result| result.get("pid").cloned())
            .and_then(|pid| pid.as_u64())
        {
            Some(pid) => pid,
            None => {
                return failed_execution(
                    role,
                    argv,
                    format!("guest-exec returned no pid: {}", spawn.stdout.trim()),
                );
            }
        };

        for poll in 0..self.max_status_polls {
            if poll > 0 && !self.poll_interval.is_zero() {
                std::thread::sleep(self.poll_interval);
            }
            let status_request = json!({
                "execute": "guest-exec-status",
                "arguments": { "pid": pid },
            });
            let status = self.qga_call(&status_request);
            if !status.success() {
                return transport_failure(role, argv, "guest-exec-status", &status);
            }
            let Some(result) = response_return(&status) else {
                return failed_execution(
                    role,
                    argv,
                    format!(
                        "guest-exec-status returned invalid JSON: {}",
                        status.stdout.trim()
                    ),
                );
            };
            if result.get("exited").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let stdout = match decode_guest_output(result.get("out-data")) {
                Ok(output) => output,
                Err(error) => return failed_execution(role, argv, error),
            };
            let stderr = match decode_guest_output(result.get("err-data")) {
                Ok(output) => output,
                Err(error) => return failed_execution(role, argv, error),
            };
            return CommandExecution {
                role,
                argv,
                exit_code: result
                    .get("exitcode")
                    .and_then(Value::as_i64)
                    .and_then(|code| i32::try_from(code).ok()),
                stdout,
                stderr,
            };
        }

        failed_execution(
            role,
            argv,
            format!(
                "guest-exec-status did not observe exit within {} polls",
                self.max_status_polls
            ),
        )
    }

    fn qga_call(&self, request: &Value) -> ArgvOutput
    where
        R: ArgvRunner,
    {
        let timeout = self.timeout_seconds.to_string();
        let payload = request.to_string();
        self.runner.run(&[
            &self.virsh_bin,
            "qemu-agent-command",
            &self.attestation.transport_target,
            "--timeout",
            &timeout,
            &payload,
        ])
    }
}

impl<R: ArgvRunner> NetworkCommandRunnerPort for QemuGuestAgentNetworkRunner<R> {
    fn execution_boundary(&self) -> NetworkExecutionBoundary {
        NetworkExecutionBoundary::IsolatedHarnessTarget
    }

    fn target_attestation(&self) -> Option<&RuntimeHarnessTargetAttestation> {
        Some(&self.attestation)
    }

    fn run_command(&self, command: &NetworkCommandSurface) -> CommandExecution {
        match &command.invocation {
            NetworkCommandInvocation::Argv { argv } => {
                self.execute_guest_argv(command.role.clone(), argv.clone())
            }
            _ => failed_execution(
                command.role.clone(),
                Vec::new(),
                "qemu guest-agent transport accepts argv command surfaces only",
            ),
        }
    }
}

impl<R: ArgvRunner + Send + Sync> NetworkActuationExecutor for QemuGuestAgentNetworkRunner<R> {
    fn run_argv(&self, role: &str, argv: &[String]) -> CommandExecution {
        self.execute_guest_argv(role.to_string(), argv.to_vec())
    }
}

impl<R: ArgvRunner + Send + Sync> NetworkActuationObserver
    for QemuGuestAgentNetworkObserver<'_, R>
{
    fn observe(&self) -> Result<NetworkActuationObservation, String> {
        let execution = self.transport.execute_guest_argv(
            "network_actuation_postflight".to_string(),
            self.observation_argv.clone(),
        );
        if execution.exit_code != Some(0) {
            return Err(format!(
                "guest observation failed: {}",
                execution.stderr.trim()
            ));
        }
        serde_json::from_str(&execution.stdout)
            .map_err(|error| format!("guest observation returned invalid JSON: {error}"))
    }
}

fn response_return(output: &ArgvOutput) -> Option<Value> {
    serde_json::from_str::<Value>(&output.stdout)
        .ok()?
        .get("return")
        .cloned()
}

fn decode_guest_output(value: Option<&Value>) -> Result<String, String> {
    let Some(encoded) = value.and_then(Value::as_str) else {
        return Ok(String::new());
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|error| format!("guest-agent output is not valid base64: {error}"))
}

fn transport_failure(
    role: String,
    argv: Vec<String>,
    operation: &str,
    output: &ArgvOutput,
) -> CommandExecution {
    let detail = output
        .spawn_error
        .as_deref()
        .unwrap_or_else(|| output.stderr.trim());
    failed_execution(
        role,
        argv,
        format!("{operation} transport failed: {detail}"),
    )
}

fn failed_execution(role: String, argv: Vec<String>, error: impl Into<String>) -> CommandExecution {
    CommandExecution {
        role,
        argv,
        exit_code: None,
        stdout: String::new(),
        stderr: error.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

    use base64::Engine;
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };
<<<<<<< HEAD
    use cynic_lifecycle::{ArgvOutput, ArgvRunner};
=======
    use cynic_host::update_lifecycle::{ArgvOutput, ArgvRunner};
>>>>>>> github/main
    use serde_json::Value;

    use cynic_host::network_actuation::{
        NetworkActuationObservation, NetworkActuationObserver, NetworkActuationTarget,
    };

    use super::{QemuGuestAgentNetworkObserver, QemuGuestAgentNetworkRunner};
    use crate::network_command_surface::argv_command;
    use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

    struct SequenceRunner {
        calls: Mutex<Vec<Vec<String>>>,
        outputs: Mutex<VecDeque<ArgvOutput>>,
    }

    impl ArgvRunner for SequenceRunner {
        fn run(&self, argv: &[&str]) -> ArgvOutput {
            self.calls
                .lock()
                .expect("calls")
                .push(argv.iter().map(|item| (*item).to_string()).collect());
            self.outputs
                .lock()
                .expect("outputs")
                .pop_front()
                .expect("stubbed output")
        }
    }

    fn output(stdout: Value) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn attestation() -> RuntimeHarnessTargetAttestation {
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "lab-kernel".to_string(),
            declared_isolation_boundary: closed_local_lab_context().isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["sha256:test:proof".to_string()],
        }
    }

    fn command_argv() -> Vec<String> {
        vec![
            "/portable/ip".to_string(),
            "route".to_string(),
            "replace".to_string(),
            "10.10.10.0/24".to_string(),
            "dev".to_string(),
            "cynic-mesh0".to_string(),
        ]
    }

    fn observation_argv() -> Vec<String> {
        vec![
            "/portable/cynic-node".to_string(),
            "network".to_string(),
            "actuation-observation".to_string(),
            "--json".to_string(),
        ]
    }

    #[test]
    fn executes_exact_allowed_argv_without_shell_or_ssh() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("route installed\n");
        let runner = SequenceRunner {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::from([
                output(serde_json::json!({"return": {"pid": 41}})),
                output(serde_json::json!({
                    "return": {"exited": true, "exitcode": 0, "out-data": encoded}
                })),
            ])),
        };
        let transport = QemuGuestAgentNetworkRunner::new(
            runner,
            closed_local_lab_context(),
            attestation(),
            "/portable/virsh",
            [command_argv()],
            5,
            2,
            Duration::ZERO,
        )
        .expect("transport");

        let execution = transport.run_command(&argv_command("route_install", command_argv()));

        assert_eq!(execution.exit_code, Some(0));
        assert_eq!(execution.stdout, "route installed\n");
        let calls = transport.runner.calls.lock().expect("calls");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0][0], "/portable/virsh");
        assert_eq!(calls[0][1], "qemu-agent-command");
        let payload: Value = serde_json::from_str(calls[0].last().expect("payload")).expect("json");
        assert_eq!(payload["execute"], "guest-exec");
        assert_eq!(payload["arguments"]["path"], "/portable/ip");
        assert!(
            calls
                .iter()
                .flatten()
                .all(|token| token != "ssh" && token != "sh")
        );
    }

    #[test]
    fn refuses_argv_outside_exact_profile_without_transport_call() {
        let runner = SequenceRunner {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::new()),
        };
        let transport = QemuGuestAgentNetworkRunner::new(
            runner,
            closed_local_lab_context(),
            attestation(),
            "virsh",
            [command_argv()],
            5,
            2,
            Duration::ZERO,
        )
        .expect("transport");

        let execution = transport.run_command(&argv_command(
            "unexpected",
            vec!["/portable/ip".to_string(), "link".to_string()],
        ));

        assert_eq!(execution.exit_code, None);
        assert!(execution.stderr.contains("exact member"));
        assert!(transport.runner.calls.lock().expect("calls").is_empty());
    }

    #[test]
    fn revalidates_attestation_immediately_before_transport() {
        let runner = SequenceRunner {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::new()),
        };
        let mut expired = attestation();
        expired.observed_at_unix_ms = 1;
        expired.expires_at_unix_ms = 2;
        let transport = QemuGuestAgentNetworkRunner::new(
            runner,
            closed_local_lab_context(),
            expired,
            "virsh",
            [command_argv()],
            5,
            2,
            Duration::ZERO,
        )
        .expect("transport");

        let execution = transport.run_command(&argv_command("route_install", command_argv()));

        assert_eq!(execution.exit_code, None);
        assert!(
            execution
                .stderr
                .contains("harness_target_attestation_expired")
        );
        assert!(transport.runner.calls.lock().expect("calls").is_empty());
    }

    #[test]
    fn guest_status_polling_is_bounded_and_fails_closed() {
        let runner = SequenceRunner {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::from([
                output(serde_json::json!({"return": {"pid": 41}})),
                output(serde_json::json!({"return": {"exited": false}})),
                output(serde_json::json!({"return": {"exited": false}})),
            ])),
        };
        let transport = QemuGuestAgentNetworkRunner::new(
            runner,
            closed_local_lab_context(),
            attestation(),
            "virsh",
            [command_argv()],
            5,
            2,
            Duration::ZERO,
        )
        .expect("transport");

        let execution = transport.run_command(&argv_command("route_install", command_argv()));

        assert_eq!(execution.exit_code, None);
        assert!(execution.stderr.contains("within 2 polls"));
        assert_eq!(transport.runner.calls.lock().expect("calls").len(), 3);
    }

    #[test]
    fn postflight_observer_reads_typed_evidence_through_same_attested_transport() {
        let expected = NetworkActuationObservation {
            provider_fallback_active: false,
            synapse_route_active: true,
            wireguard_active: false,
            management_link_active: false,
            target: NetworkActuationTarget {
                provider_fallback_cidr: "192.0.2.0/24".to_string(),
                provider_fallback_gateway: Some("192.0.2.1".to_string()),
                synapse_cidr: "10.10.10.0/24".to_string(),
                mesh_interface: "cynic-mesh0".to_string(),
                management_interface: None,
                management_connection: None,
                management_address: None,
                management_gateway: None,
            },
        };
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&expected).expect("observation json"));
        let runner = SequenceRunner {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::from([
                output(serde_json::json!({"return": {"pid": 42}})),
                output(serde_json::json!({
                    "return": {"exited": true, "exitcode": 0, "out-data": encoded}
                })),
            ])),
        };
        let transport = QemuGuestAgentNetworkRunner::new(
            runner,
            closed_local_lab_context(),
            attestation(),
            "virsh",
            [observation_argv()],
            5,
            2,
            Duration::ZERO,
        )
        .expect("transport");
        let observer = QemuGuestAgentNetworkObserver::new(&transport, observation_argv());

        let observed = observer.observe().expect("postflight observation");

        assert_eq!(
            observed.target.mesh_interface,
            expected.target.mesh_interface
        );
        assert!(observed.synapse_route_active);
        assert!(!observed.wireguard_active);
        assert_eq!(transport.runner.calls.lock().expect("calls").len(), 2);
    }
}

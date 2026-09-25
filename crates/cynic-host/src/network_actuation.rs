//! Typed, shell-free network-actuation policy shapes.
//!
//! The host substrate owns the command contract consumed by privileged
//! helpers. Node runtimes may supply observations, but they must not invent a
//! second policy or command schema.

use std::collections::BTreeMap;

use cynic_protocol::{
    ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof,
};
use serde::{Deserialize, Serialize};

use crate::command_surface::{HostCommandSurface, argv_command};
<<<<<<< HEAD
use crate::execution::{HostCommandExecution, command_to_argv, digest_json};
=======
use crate::execution::{HostCommandExecution, command_to_argv};
>>>>>>> github/main
use crate::runtime_harness::{
    RuntimeHarnessContext, RuntimeHarnessTargetAttestation,
    validate_runtime_harness_target_attestation,
};

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationPolicyDocument {
    pub schema: &'static str,
    pub status: &'static str,
    pub helper_path: String,
    pub sudoers_subject: &'static str,
    pub max_timeout_seconds: u64,
    pub allowed_profiles: BTreeMap<String, NetworkActuationAllowedProfile>,
    pub forbidden_tokens: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationAllowedProfile {
    pub description: &'static str,
    pub command_prefixes: Vec<HostCommandSurface>,
}

/// Capability-selected command paths used to materialize a bounded network
/// policy. Provider and distro discovery stays in adapters.
#[derive(Debug, Clone)]
pub struct NetworkActuationRuntime {
    pub helper_path: String,
    pub sudoers_subject: &'static str,
    pub max_timeout_seconds: u64,
    pub service_manager_bin: String,
    pub wireguard_unit: String,
    pub route_bin: String,
    pub wireguard_bin: String,
    pub network_manager_bin: String,
}

/// Topology values observed or supplied by an adapter. This remains data, not
/// an executable command string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActuationTarget {
    pub provider_fallback_cidr: String,
    pub provider_fallback_gateway: Option<String>,
    pub synapse_cidr: String,
    pub mesh_interface: String,
    #[serde(default)]
    pub management_interface: Option<String>,
    #[serde(default)]
    pub management_connection: Option<String>,
    #[serde(default)]
    pub management_address: Option<String>,
    #[serde(default)]
    pub management_gateway: Option<String>,
}

/// Minimal, provider-neutral observation consumed by host policy and the
/// privileged helper. Higher layers project their richer observations here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActuationObservation {
    pub provider_fallback_active: bool,
    pub synapse_route_active: bool,
    pub wireguard_active: bool,
    #[serde(default)]
    pub management_link_active: bool,
    pub target: NetworkActuationTarget,
}

impl NetworkActuationObservation {
    pub fn allows_profile(&self, profile: &str) -> bool {
        match profile {
            "provider-fallback-route" => {
                !self.provider_fallback_active && self.target.provider_fallback_gateway.is_some()
            }
            "synapse-route" => !self.synapse_route_active,
            "wireguard-mesh" => !self.wireguard_active,
            "direct-management-link" => {
                !self.management_link_active
                    && self.target.management_interface.is_some()
                    && self.target.management_connection.is_some()
                    && self.target.management_address.is_some()
                    && self.target.management_gateway.is_some()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationPlan {
    pub schema: &'static str,
    pub id: String,
    pub profile: String,
    pub target: String,
    pub commands: Vec<HostCommandSurface>,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

pub trait NetworkActuationExecutor: Send + Sync {
    fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution;
}

pub trait NetworkActuationObserver: Send + Sync {
    fn observe(&self) -> Result<NetworkActuationObservation, String>;
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationExecutionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan_id: String,
    pub execution_count: usize,
    pub executions: Vec<HostCommandExecution>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationProofReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub execution: NetworkActuationExecutionReport,
    pub postflight: Option<NetworkActuationObservation>,
    #[serde(default)]
    pub postflight_controls: BTreeMap<String, serde_json::Value>,
    pub proof_refs: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn execute_admitted_network_actuation(
    plan: &NetworkActuationPlan,
    admission_blockers: Vec<String>,
    executor: &impl NetworkActuationExecutor,
) -> NetworkActuationExecutionReport {
    let mut blockers = admission_blockers;
    let mut executions = Vec::new();
    if blockers.is_empty() {
        for command in &plan.commands {
            let argv = match command_to_argv(command) {
                Ok(argv) => argv,
                Err(error) => {
                    blockers.push(format!("non_argv_command:{}:{error}", command.role));
                    break;
                }
            };
            let execution = executor.run_argv(&command.role, &argv);
            let succeeded = execution.exit_code == Some(0);
            executions.push(execution);
            if !succeeded {
                break;
            }
        }
    }
    let status = if !blockers.is_empty() {
        "blocked"
    } else if executions.len() == plan.commands.len()
        && executions
            .iter()
            .all(|execution| execution.exit_code == Some(0))
    {
        "completed"
    } else {
        "failed"
    };
    NetworkActuationExecutionReport {
        schema: "cynic.host.network_actuation_execution.v1",
        status,
        plan_id: plan.id.clone(),
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

pub fn execute_network_actuation_with_readback(
    plan: &NetworkActuationPlan,
    admission_blockers: Vec<String>,
    executor: &impl NetworkActuationExecutor,
    observer: &impl NetworkActuationObserver,
) -> NetworkActuationProofReport {
    let execution = execute_admitted_network_actuation(plan, admission_blockers, executor);
    let mut blockers = execution.blockers.clone();
    let mut proof_refs = Vec::new();
    let postflight = if execution.status == "completed" {
        match observer.observe() {
            Ok(observation) => {
                if profile_satisfied(&observation, &plan.profile) {
                    proof_refs.push(format!("postflight:{}:{}", plan.target, plan.profile));
                } else {
                    blockers.push("postflight_desired_state_unproven".to_string());
                }
                Some(observation)
            }
            Err(error) => {
                blockers.push(format!("postflight_observation_failed:{error}"));
                None
            }
        }
    } else {
        None
    };
    let status = if execution.status == "completed" && blockers.is_empty() {
        "proved"
    } else if execution.status == "blocked" {
        "blocked"
    } else {
        "failed"
    };
    NetworkActuationProofReport {
        schema: "cynic.host.network_actuation_proof.v1",
        status,
        execution,
        postflight,
        postflight_controls: BTreeMap::new(),
        proof_refs,
        blockers,
    }
}

fn profile_satisfied(observation: &NetworkActuationObservation, profile: &str) -> bool {
    match profile {
        "provider-fallback-route" => observation.provider_fallback_active,
        "synapse-route" => observation.synapse_route_active,
        "wireguard-mesh" => observation.wireguard_active,
        "direct-management-link" => observation.management_link_active,
        _ => false,
    }
}

pub fn gate_network_actuation_harness(
    plan: &NetworkActuationPlan,
    context: &RuntimeHarnessContext,
    attestation: &RuntimeHarnessTargetAttestation,
) -> Vec<String> {
    let mut blockers = validate_runtime_harness_target_attestation(context, attestation)
        .into_iter()
        .map(|violation| violation.code.to_string())
        .collect::<Vec<_>>();
    if attestation.target_id != plan.target {
        blockers.push("harness_target_plan_mismatch".to_string());
    }
    if context.mutates_host_network {
        blockers.push("harness_may_mutate_operator_host_network".to_string());
    }
    blockers
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationRequest {
    pub schema: String,
    pub plan: NetworkActuationPlan,
    pub admission: PolicyAdmissionReport,
    pub authorization: ExecutionAuthorization,
}

pub const NETWORK_ACTUATION_REQUEST_SCHEMA: &str = "cynic.host.network_actuation_request.v1";

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkActuationWirePlan {
    pub schema: String,
    pub id: String,
    pub profile: String,
    pub target: String,
    pub commands: Vec<HostCommandSurface>,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkActuationWireRequest {
    pub schema: String,
    pub plan: NetworkActuationWirePlan,
    pub admission: PolicyAdmissionReport,
    pub authorization: ExecutionAuthorization,
}

impl NetworkActuationWireRequest {
    pub fn into_request(self) -> Result<NetworkActuationRequest, String> {
        if self.schema != NETWORK_ACTUATION_REQUEST_SCHEMA {
            return Err("request_schema_invalid".to_string());
        }
        if self.plan.schema != "cynic.host.network_actuation_plan.v1" {
            return Err("plan_schema_invalid".to_string());
        }
        Ok(NetworkActuationRequest {
            schema: self.schema,
            plan: NetworkActuationPlan {
                schema: "cynic.host.network_actuation_plan.v1",
                id: self.plan.id,
                profile: self.plan.profile,
                target: self.plan.target,
                commands: self.plan.commands,
                preconditions: self.plan.preconditions,
                rollback_intent: self.plan.rollback_intent,
                expected_proofs: self.plan.expected_proofs,
            },
            admission: self.admission,
            authorization: self.authorization,
        })
    }
}

pub fn admit_network_actuation_plan(
    plan: &NetworkActuationPlan,
    actor: impl Into<String>,
) -> PolicyAdmissionReport {
    let digest = digest_json(plan);
    let mutation = MutationContract {
        kind: "network_actuation".to_string(),
        scope: PolicyScope {
            domain: "network".to_string(),
            target: plan.target.clone(),
        },
        mutates: true,
        requires_privilege: true,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::HumanOperator,
        preconditions: plan.preconditions.clone(),
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan.expected_proofs.clone(),
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: "cynic.host.network_actuation".to_string(),
            version: "v1".to_string(),
            digest: format!("sha256:{}", digest_json(&"cynic.host.network_actuation.v1")),
        },
        PolicyIntentRef {
            id: plan.id.clone(),
            kind: mutation.kind.clone(),
            actor: actor.into(),
            subject: plan.target.clone(),
            digest: digest.clone(),
            plan_digest: digest,
        },
        mutation.clone(),
        PolicyDecision::Allow {
            required_authority: mutation.required_authority,
            required_proofs: mutation.expected_proofs.clone(),
        },
    )
}

pub fn gate_network_actuation_plan(
    plan: &NetworkActuationPlan,
    admission: &PolicyAdmissionReport,
    authorization: &ExecutionAuthorization,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if admission.intent.plan_digest != digest_json(plan) {
        blockers.push("plan_digest_mismatch".to_string());
    }
    if !admission.admitted() {
        blockers.push("policy_decision_not_allow".to_string());
    }
    if authorization.authority != PolicyAuthority::HumanOperator {
        blockers.push("authority_insufficient".to_string());
    }
    if !authorization.explicit_apply {
        blockers.push("explicit_apply_required".to_string());
    }
    for required in &plan.preconditions {
        if !authorization.satisfied_preconditions.contains(required) {
            blockers.push(format!("precondition_unsatisfied:{required}"));
        }
    }
    blockers
}

pub fn gate_network_actuation_request(request: &NetworkActuationRequest) -> Vec<String> {
    let mut blockers = if request.schema == NETWORK_ACTUATION_REQUEST_SCHEMA {
        Vec::new()
    } else {
        vec!["request_schema_invalid".to_string()]
    };
    blockers.extend(gate_network_actuation_plan(
        &request.plan,
        &request.admission,
        &request.authorization,
    ));
    blockers
}

<<<<<<< HEAD
=======
fn digest_json(value: &impl Serialize) -> String {
    cynic_protocol::digest_json(value).unwrap_or_else(|_| "sha256:unavailable".to_string())
}

>>>>>>> github/main
pub fn build_network_actuation_policy<'a>(
    allowed_profiles: impl IntoIterator<Item = &'a str>,
    runtime: &NetworkActuationRuntime,
) -> NetworkActuationPolicyDocument {
    let mut profiles = BTreeMap::new();
    for profile in allowed_profiles {
        let Some(allowed) = allowed_profile(profile, runtime) else {
            continue;
        };
        profiles.insert(profile.to_string(), allowed);
    }
    NetworkActuationPolicyDocument {
        schema: "cynic.network_actuator.policy.v1",
        status: "planned",
        helper_path: runtime.helper_path.clone(),
        sudoers_subject: runtime.sudoers_subject,
        max_timeout_seconds: runtime.max_timeout_seconds,
        allowed_profiles: profiles,
        forbidden_tokens: vec![
            "sh", "bash", "-c", "curl", "wget", "nc", "python", "python3", "perl", "ruby", ">",
            ">>", "|", "&&", ";",
        ],
    }
}

fn allowed_profile(
    profile: &str,
    runtime: &NetworkActuationRuntime,
) -> Option<NetworkActuationAllowedProfile> {
    match profile {
        "provider-fallback-route" => Some(route_profile(
            "Restore temporary provider fallback route for resurrection only.",
            &runtime.route_bin,
        )),
        "synapse-route" => Some(route_profile(
            "Restore canonical Synapse route on the local node.",
            &runtime.route_bin,
        )),
        "wireguard-mesh" => Some(NetworkActuationAllowedProfile {
            description: "Activate or inspect canonical WireGuard mesh on the local node.",
            command_prefixes: vec![
                argv_command(
                    "start_service",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "start".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "restart_service",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "restart".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "service_status",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "is-active".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "inspect_wireguard",
                    vec![runtime.wireguard_bin.clone(), "show".to_string()],
                ),
            ],
        }),
        "direct-management-link" => Some(NetworkActuationAllowedProfile {
            description: "Enroll a directly attached management link without replacing the operator default route.",
            command_prefixes: vec![
                argv_command(
                    "configure_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "modify".to_string(),
                    ],
                ),
                argv_command(
                    "activate_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "up".to_string(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                    ],
                ),
            ],
        }),
        _ => None,
    }
}

fn route_profile(description: &'static str, route_bin: &str) -> NetworkActuationAllowedProfile {
    NetworkActuationAllowedProfile {
        description,
        command_prefixes: vec![
            argv_command(
                "mutate_route",
                vec![
                    route_bin.to_string(),
                    "route".to_string(),
                    "replace".to_string(),
                ],
            ),
            argv_command(
                "inspect_route",
                vec![
                    route_bin.to_string(),
                    "route".to_string(),
                    "show".to_string(),
                ],
            ),
        ],
    }
}

pub fn build_network_actuation_commands(
    profile: &str,
    target: &NetworkActuationTarget,
    runtime: &NetworkActuationRuntime,
) -> Option<Vec<HostCommandSurface>> {
    match profile {
        "provider-fallback-route" => target.provider_fallback_gateway.as_ref().map(|gateway| {
            vec![
                argv_command(
                    "mutate_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "replace".to_string(),
                        target.provider_fallback_cidr.clone(),
                        "via".to_string(),
                        gateway.clone(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                        target.provider_fallback_cidr.clone(),
                    ],
                ),
            ]
        }),
        "synapse-route" => Some(vec![
            argv_command(
                "mutate_route",
                vec![
                    runtime.route_bin.clone(),
                    "route".to_string(),
                    "replace".to_string(),
                    target.synapse_cidr.clone(),
                    "dev".to_string(),
                    target.mesh_interface.clone(),
                ],
            ),
            argv_command(
                "inspect_route",
                vec![
                    runtime.route_bin.clone(),
                    "route".to_string(),
                    "show".to_string(),
                    target.synapse_cidr.clone(),
                ],
            ),
        ]),
        "wireguard-mesh" => Some(vec![
            argv_command(
                "restart_service",
                vec![
                    runtime.service_manager_bin.clone(),
                    "restart".to_string(),
                    runtime.wireguard_unit.clone(),
                ],
            ),
            argv_command(
                "service_status",
                vec![
                    runtime.service_manager_bin.clone(),
                    "is-active".to_string(),
                    runtime.wireguard_unit.clone(),
                ],
            ),
            argv_command(
                "inspect_wireguard",
                vec![runtime.wireguard_bin.clone(), "show".to_string()],
            ),
        ]),
        "direct-management-link" => {
            let interface = target.management_interface.as_ref()?;
            let connection = target.management_connection.as_ref()?;
            let address = target.management_address.as_ref()?;
            let gateway = target.management_gateway.as_ref()?;
            Some(vec![
                argv_command(
                    "configure_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "modify".to_string(),
                        connection.clone(),
                        "connection.interface-name".to_string(),
                        interface.clone(),
                        "ipv4.method".to_string(),
                        "manual".to_string(),
                        "ipv4.addresses".to_string(),
                        address.clone(),
                        "ipv4.gateway".to_string(),
                        String::new(),
                        "ipv4.dns".to_string(),
                        String::new(),
                        "ipv4.never-default".to_string(),
                        "yes".to_string(),
                        "ipv4.routes".to_string(),
                        format!("{} {}", target.synapse_cidr, gateway),
                        "ipv4.dad-timeout".to_string(),
                        "3000".to_string(),
                        "ipv6.never-default".to_string(),
                        "yes".to_string(),
                    ],
                ),
                argv_command(
                    "activate_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "up".to_string(),
                        connection.clone(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                        target.synapse_cidr.clone(),
                    ],
                ),
            ])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuationAllowedProfile, NetworkActuationExecutor, NetworkActuationObservation,
        NetworkActuationObserver, NetworkActuationPlan, NetworkActuationPolicyDocument,
        NetworkActuationRuntime, NetworkActuationTarget, build_network_actuation_commands,
        build_network_actuation_policy, execute_admitted_network_actuation,
        execute_network_actuation_with_readback, gate_network_actuation_harness,
    };
    use crate::command_surface::argv_command;
    use crate::execution::HostCommandExecution;
    use crate::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };
    use cynic_protocol::RequiredProof;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FailSecond(AtomicUsize);

    struct SuccessfulExecutor;
    struct ActiveSynapseObserver;

    impl NetworkActuationExecutor for FailSecond {
        fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution {
            let call = self.0.fetch_add(1, Ordering::SeqCst);
            HostCommandExecution {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: Some(if call == 1 { 1 } else { 0 }),
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl NetworkActuationExecutor for SuccessfulExecutor {
        fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution {
            HostCommandExecution {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl NetworkActuationObserver for ActiveSynapseObserver {
        fn observe(&self) -> Result<NetworkActuationObservation, String> {
            Ok(NetworkActuationObservation {
                provider_fallback_active: true,
                synapse_route_active: true,
                wireguard_active: true,
                management_link_active: false,
                target: NetworkActuationTarget {
                    provider_fallback_cidr: "10.0.0.0/24".to_string(),
                    provider_fallback_gateway: None,
                    synapse_cidr: "10.10.10.0/24".to_string(),
                    mesh_interface: "wg0".to_string(),
                    management_interface: None,
                    management_connection: None,
                    management_address: None,
                    management_gateway: None,
                },
            })
        }
    }

    #[test]
    fn policy_document_keeps_commands_as_host_argv_surfaces() {
        let policy = NetworkActuationPolicyDocument {
            schema: "cynic.network_actuator.policy.v1",
            status: "planned",
            helper_path: "/portable/libexec/cynic-host-exec".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            allowed_profiles: BTreeMap::from([(
                "wireguard-mesh".to_string(),
                NetworkActuationAllowedProfile {
                    description: "activate mesh",
                    command_prefixes: vec![argv_command(
                        "inspect_wireguard",
                        vec!["/portable/bin/wg".to_string(), "show".to_string()],
                    )],
                },
            )]),
            forbidden_tokens: vec!["sh", "-c"],
        };

        assert_eq!(
            policy.allowed_profiles["wireguard-mesh"].command_prefixes[0].role,
            "inspect_wireguard"
        );
    }

    #[test]
    fn builder_uses_injected_capability_paths() {
        let runtime = NetworkActuationRuntime {
            helper_path: "/portable/libexec/cynic-host-exec".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let policy = build_network_actuation_policy(["wireguard-mesh", "unknown"], &runtime);

        assert_eq!(policy.allowed_profiles.len(), 1);
        assert_eq!(
            policy.allowed_profiles["wireguard-mesh"].command_prefixes[0].role,
            "start_service"
        );
    }

    #[test]
    fn command_builder_keeps_observed_topology_as_argv_data() {
        let runtime = NetworkActuationRuntime {
            helper_path: "/portable/libexec/cynic-host-exec".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let target = NetworkActuationTarget {
            provider_fallback_cidr: "10.0.0.0/24".to_string(),
            provider_fallback_gateway: Some("10.0.0.1".to_string()),
            synapse_cidr: "10.10.10.0/24".to_string(),
            mesh_interface: "wg-portable".to_string(),
            management_interface: None,
            management_connection: None,
            management_address: None,
            management_gateway: None,
        };
        let commands = build_network_actuation_commands("synapse-route", &target, &runtime)
            .expect("synapse route is supported");

        assert_eq!(commands[0].role, "mutate_route");
        assert_eq!(
            crate::command_surface::render_command(&commands[0]),
            "/portable/bin/ip route replace 10.10.10.0/24 dev wg-portable"
        );
    }

    #[test]
    fn direct_management_link_preserves_default_route_and_uses_injected_adapter() {
        let runtime = NetworkActuationRuntime {
            helper_path: "/portable/libexec/cynic-host-exec".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let target = NetworkActuationTarget {
            provider_fallback_cidr: "192.168.0.0/24".to_string(),
            provider_fallback_gateway: Some("192.168.0.254".to_string()),
            synapse_cidr: "10.10.10.0/24".to_string(),
            mesh_interface: "wg-portable".to_string(),
            management_interface: Some("eth-test".to_string()),
            management_connection: Some("cynic-management".to_string()),
            management_address: Some("192.168.100.2/24".to_string()),
            management_gateway: Some("192.168.100.1".to_string()),
        };

        let commands =
            build_network_actuation_commands("direct-management-link", &target, &runtime)
                .expect("complete topology admits the profile");
        let rendered = crate::command_surface::render_command(&commands[0]);

        assert!(rendered.starts_with("/portable/bin/nmcli connection modify"));
        assert!(rendered.contains("ipv4.never-default yes"));
        assert!(rendered.contains("10.10.10.0/24 192.168.100.1"));
        assert!(!rendered.contains("192.168.0.254"));
    }

    #[test]
    fn admitted_execution_is_fail_fast() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "test-plan".to_string(),
            profile: "test".to_string(),
            target: "local-host".to_string(),
            commands: vec![
                argv_command("one", vec!["true".to_string()]),
                argv_command("two", vec!["false".to_string()]),
                argv_command("three", vec!["true".to_string()]),
            ],
            preconditions: Vec::new(),
            rollback_intent: "none needed for fixture".to_string(),
            expected_proofs: vec![RequiredProof {
                gate: "fixture".to_string(),
                subject: "local-host".to_string(),
            }],
        };
        let executor = FailSecond(AtomicUsize::new(0));
        let report = execute_admitted_network_actuation(&plan, Vec::new(), &executor);
        assert_eq!(report.status, "failed");
        assert_eq!(report.execution_count, 2);
        assert_eq!(executor.0.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn harness_gate_binds_plan_to_isolated_target() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "lab-plan".to_string(),
            profile: "synapse-route".to_string(),
            target: "guest-1".to_string(),
            commands: vec![argv_command("probe", vec!["true".to_string()])],
            preconditions: Vec::new(),
            rollback_intent: "restore route".to_string(),
            expected_proofs: Vec::new(),
        };
        let context = closed_local_lab_context();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let attestation = RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: now,
            expires_at_unix_ms: now + 60_000,
            target_id: "guest-1".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "guest-1".to_string(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "isolated-vm-network".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["proof:guest-1".to_string()],
        };
        assert!(gate_network_actuation_harness(&plan, &context, &attestation).is_empty());
    }

    #[test]
    fn completed_execution_requires_postflight_proof() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "proof-plan".to_string(),
            profile: "synapse-route".to_string(),
            target: "guest-1".to_string(),
            commands: vec![argv_command("route", vec!["true".to_string()])],
            preconditions: Vec::new(),
            rollback_intent: "restore route".to_string(),
            expected_proofs: Vec::new(),
        };
        let report = execute_network_actuation_with_readback(
            &plan,
            Vec::new(),
            &SuccessfulExecutor,
            &ActiveSynapseObserver,
        );
        assert_eq!(report.status, "proved");
        assert_eq!(report.proof_refs, vec!["postflight:guest-1:synapse-route"]);
    }
}

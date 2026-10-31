//! Network actuation execution and read-back proof.
//!
//! Pure orchestration over admitted plans: run each command through an injected
//! executor, then (on success) observe the post-flight state and prove the
//! desired profile is satisfied.

use std::collections::BTreeMap;

use cynic_process::ArgvOutput;
use serde::{Deserialize, Serialize};

use crate::actuator::{NetworkActuationObservation, NetworkActuationPlan};
use crate::command_surface::{HostCommandInvocation, HostCommandSurface};

pub trait NetworkActuationExecutor: Send + Sync {
    fn run_argv(&self, role: &str, argv: &[String]) -> ArgvOutput;
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
    pub executions: Vec<ArgvOutput>,
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

pub fn command_to_argv(command: &HostCommandSurface) -> Result<Vec<String>, String> {
    invocation_to_argv(&command.invocation)
}

fn invocation_to_argv(invocation: &HostCommandInvocation) -> Result<Vec<String>, String> {
    match invocation {
        HostCommandInvocation::Argv { argv } => Ok(argv.clone()),
        HostCommandInvocation::InstallDir { path, policy } => Ok(vec![
            "install".to_string(),
            "-d".to_string(),
            "-o".to_string(),
            policy.owner.user.to_string(),
            "-g".to_string(),
            policy.owner.group.to_string(),
            "-m".to_string(),
            policy.mode.octal.to_string(),
            path.clone(),
        ]),
        HostCommandInvocation::InstallFile {
            source,
            target,
            policy,
        } => Ok(vec![
            "install".to_string(),
            "-o".to_string(),
            policy.owner.user.to_string(),
            "-g".to_string(),
            policy.owner.group.to_string(),
            "-m".to_string(),
            policy.mode.octal.to_string(),
            source.clone(),
            target.clone(),
        ]),
        HostCommandInvocation::ValidateSudoers { path } => Ok(vec![
            std::env::var("CYNIC_VISUDO_BIN").unwrap_or_else(|_| "visudo".to_string()),
            "-cf".to_string(),
            path.clone(),
        ]),
        HostCommandInvocation::Shell { .. }
        | HostCommandInvocation::Expectation { .. }
        | HostCommandInvocation::PackagePresence { .. } => {
            Err("host execution supports only argv/install/visudo surfaces".to_string())
        }
    }
}
//! Linux process identity adapter for externally supervised agent runtimes.

use std::{fs, path::PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessIdentityState {
    Observed,
    ProcessMissing,
    PrincipalMismatch,
    Unknown,
}

/// Ownership classification derived from a process observation, never from a
/// machine-local username or executable path.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeOwnershipState {
    RuntimeOwned,
    OperatorLocal,
    ExternalUnadopted,
    Unknown,
}

/// Explicitly separates a locally installed operator tool from a runtime that
/// CYNIC is allowed to supervise. `operator_local` is a safe stop condition,
/// not an invitation to take over the process.
#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeOwnership {
    pub state: AgentRuntimeOwnershipState,
    pub adoption_required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProcessIdentityObservation {
    pub pid: u32,
    pub state: ProcessIdentityState,
    pub observed_uid: Option<u32>,
    pub executable: Option<String>,
    pub reason: String,
}

pub fn classify_runtime_ownership(
    observation: Option<&AgentProcessIdentityObservation>,
) -> AgentRuntimeOwnership {
    match observation.map(|observation| observation.state) {
        Some(ProcessIdentityState::Observed) => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::RuntimeOwned,
            adoption_required: false,
            reason: "process_principal_matches_runtime_authority".into(),
        },
        Some(ProcessIdentityState::PrincipalMismatch) => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::OperatorLocal,
            adoption_required: true,
            reason: "process_principal_differs_from_runtime_authority".into(),
        },
        Some(ProcessIdentityState::ProcessMissing) => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::Unknown,
            adoption_required: false,
            reason: "runtime_process_missing".into(),
        },
        Some(ProcessIdentityState::Unknown) | None => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::Unknown,
            adoption_required: false,
            reason: "runtime_process_identity_unavailable".into(),
        },
    }
}

/// A running gateway discovered through an external CLI is not evidence that
/// its process belongs to CYNIC. Keep it explicitly unadopted until a selected
/// PID proves the runtime principal.
pub fn classify_runtime_ownership_with_external_gateway(
    observation: Option<&AgentProcessIdentityObservation>,
    external_gateway_observed: bool,
) -> AgentRuntimeOwnership {
    let ownership = classify_runtime_ownership(observation);
    if matches!(ownership.state, AgentRuntimeOwnershipState::Unknown) && external_gateway_observed {
        AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::ExternalUnadopted,
            adoption_required: true,
            reason: "external_gateway_observed_without_runtime_owned_process".into(),
        }
    } else {
        ownership
    }
}

/// Observe a process selected by a higher-level adapter. This does not choose
/// a process by name and therefore cannot accidentally adopt a sibling agent.
pub fn observe_linux_process_identity(
    pid: u32,
    expected_uid: u32,
) -> AgentProcessIdentityObservation {
    let root = PathBuf::from(format!("/proc/{pid}"));
    let executable = root.join("exe");
    match fs::metadata(&executable) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return AgentProcessIdentityObservation {
                pid,
                state: ProcessIdentityState::ProcessMissing,
                observed_uid: None,
                executable: None,
                reason: "process_or_executable_missing".into(),
            };
        }
        Err(_) => {
            return AgentProcessIdentityObservation {
                pid,
                state: ProcessIdentityState::Unknown,
                observed_uid: None,
                executable: None,
                reason: "process_identity_unreadable".into(),
            };
        }
    }
    let uid = match fs::read_to_string(root.join("status"))
        .ok()
        .and_then(|status| parse_effective_uid(&status))
    {
        Some(uid) => uid,
        None => {
            return AgentProcessIdentityObservation {
                pid,
                state: ProcessIdentityState::Unknown,
                observed_uid: None,
                executable: None,
                reason: "process_uid_unreadable".into(),
            };
        }
    };
    let path = fs::read_link(executable)
        .ok()
        .map(|path| path.display().to_string());
    AgentProcessIdentityObservation {
        pid,
        state: if uid == expected_uid {
            ProcessIdentityState::Observed
        } else {
            ProcessIdentityState::PrincipalMismatch
        },
        observed_uid: Some(uid),
        executable: path,
        reason: if uid == expected_uid {
            "process_principal_matches".into()
        } else {
            "process_principal_mismatch".into()
        },
    }
}

fn parse_effective_uid(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        let values = line
            .strip_prefix("Uid:")?
            .split_whitespace()
            .collect::<Vec<_>>();
        values.get(1)?.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(state: ProcessIdentityState) -> AgentProcessIdentityObservation {
        AgentProcessIdentityObservation {
            pid: 42,
            state,
            observed_uid: Some(1000),
            executable: None,
            reason: "fixture".into(),
        }
    }

    #[test]
    fn principal_mismatch_requires_non_mutating_adoption() {
        let ownership =
            classify_runtime_ownership(Some(&observation(ProcessIdentityState::PrincipalMismatch)));

        assert!(matches!(
            ownership.state,
            AgentRuntimeOwnershipState::OperatorLocal
        ));
        assert!(ownership.adoption_required);
    }

    #[test]
    fn matching_runtime_principal_is_owned() {
        let ownership =
            classify_runtime_ownership(Some(&observation(ProcessIdentityState::Observed)));

        assert!(matches!(
            ownership.state,
            AgentRuntimeOwnershipState::RuntimeOwned
        ));
        assert!(!ownership.adoption_required);
    }

    #[test]
    fn external_gateway_without_selected_pid_remains_unadopted() {
        let ownership = classify_runtime_ownership_with_external_gateway(None, true);

        assert!(matches!(
            ownership.state,
            AgentRuntimeOwnershipState::ExternalUnadopted
        ));
        assert!(ownership.adoption_required);
    }
}

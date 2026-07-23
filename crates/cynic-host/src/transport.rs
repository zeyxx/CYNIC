use serde::Serialize;

use cynic_protocol::{
    SecurityEvidenceState, TransportScope, TransportSecurityReport, TransportSecuritySurface,
};

use crate::bridge;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutableObservation {
    pub binary: String,
    pub status: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TmuxSessionObservation {
    pub name: String,
    pub windows: u32,
    pub attached_clients: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TmuxHarnessObservation {
    pub executable: ExecutableObservation,
    pub status: String,
    pub sessions: Vec<TmuxSessionObservation>,
    pub detail: Option<String>,
}

/// Read-only observation boundary for operator-side bootstrap and continuity tools.
///
/// SSH and tmux are deliberately not runtime mutation authority. The host adapter
/// owns process invocation; this module only projects evidence and policy.
pub trait OperatorTransportObservationPort {
    fn observe_ssh(&self) -> ExecutableObservation;
    fn observe_tmux(&self) -> TmuxHarnessObservation;
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshHostKeyObservation {
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshTargetInspectionReport {
    pub schema: &'static str,
    pub target: String,
    pub status: String,
    pub ssh: ExecutableObservation,
    pub ssh_keyscan: ExecutableObservation,
    pub host_keys: Vec<SshHostKeyObservation>,
    pub trust_state: String,
    pub mutation: bool,
    pub next_action: String,
    pub detail: Option<String>,
}

/// Explicit read-only boundary for bootstrap SSH target trust evidence.
pub trait SshTargetInspectionPort {
    fn inspect_ssh_target(&self, target: &str) -> SshTargetInspectionReport;
}

pub fn inspect_ssh_target(
    observer: &impl SshTargetInspectionPort,
    target: &str,
) -> SshTargetInspectionReport {
    if !valid_ssh_target(target) {
        return SshTargetInspectionReport {
            schema: "cynic.ssh_target_inspection.v1",
            target: target.to_string(),
            status: "invalid_target".to_string(),
            ssh: ExecutableObservation {
                binary: "ssh".to_string(),
                status: "unknown".to_string(),
                path: None,
            },
            ssh_keyscan: ExecutableObservation {
                binary: "ssh-keyscan".to_string(),
                status: "unknown".to_string(),
                path: None,
            },
            host_keys: Vec::new(),
            trust_state: "not_observed".to_string(),
            mutation: false,
            next_action: "provide a DNS name or IP literal without shell syntax".to_string(),
            detail: Some("target contains unsupported characters".to_string()),
        };
    }
    observer.inspect_ssh_target(target)
}

fn valid_ssh_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 253
        && target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-'))
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportMatrixReport {
    schema: &'static str,
    status: &'static str,
    transports: Vec<TransportSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportSurface {
    id: &'static str,
    class: &'static str,
    status: &'static str,
    authority_boundary: &'static str,
    canonical_use: &'static str,
    industrial_grade: &'static str,
    command_surface: &'static str,
    notes: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executable: Option<ExecutableObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tmux_harness: Option<TmuxHarnessObservation>,
}

pub fn build_transport_security_report() -> TransportSecurityReport {
    use SecurityEvidenceState::{Absent, Declared, NotApplicable, Proven, Unknown};
    use TransportScope::{
        ExternalProvider, HypervisorGuest, LocalPrivilegeBoundary, LocalProcess, RemoteNetwork,
    };

    let bridge_active = bridge::bridge_status().status == "active";
    TransportSecurityReport::from_surfaces(vec![
        TransportSecuritySurface {
            id: "host_exec".to_string(),
            scope: LocalPrivilegeBoundary,
            mutates: true,
            confidentiality: NotApplicable,
            peer_identity: if bridge_active { Proven } else { Unknown },
            mutual_authentication: NotApplicable,
            authorization_binding: if bridge_active { Proven } else { Unknown },
            evidence_refs: if bridge_active {
                vec!["cynic.bridge_status.v1".to_string()]
            } else {
                Vec::new()
            },
            residual_risks: if bridge_active {
                Vec::new()
            } else {
                vec!["bounded root bridge is not active".to_string()]
            },
        },
        TransportSecuritySurface {
            id: "qemu_guest_agent".to_string(),
            scope: HypervisorGuest,
            mutates: true,
            confidentiality: NotApplicable,
            peer_identity: Declared,
            mutual_authentication: NotApplicable,
            authorization_binding: Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec![
                "identity becomes proven only for a live unexpired target attestation".to_string(),
            ],
        },
        TransportSecuritySurface {
            id: "fleet_node_exec".to_string(),
            scope: RemoteNetwork,
            mutates: true,
            confidentiality: Unknown,
            peer_identity: Unknown,
            mutual_authentication: Unknown,
            authorization_binding: Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec![
                "runtime machine identity and secure channel are not proven".to_string(),
            ],
        },
        TransportSecuritySurface {
            id: "kernel_rest_websocket".to_string(),
            scope: RemoteNetwork,
            mutates: true,
            confidentiality: Absent,
            peer_identity: Absent,
            mutual_authentication: Absent,
            authorization_binding: Unknown,
            evidence_refs: vec!["default:http+ws".to_string()],
            residual_risks: vec![
                "default listeners expose plaintext HTTP/WebSocket semantics".to_string(),
            ],
        },
        TransportSecuritySurface {
            id: "wireguard_mesh".to_string(),
            scope: RemoteNetwork,
            mutates: false,
            confidentiality: Declared,
            peer_identity: Declared,
            mutual_authentication: Declared,
            authorization_binding: Unknown,
            evidence_refs: Vec::new(),
            residual_risks: vec!["live peer and key evidence not supplied".to_string()],
        },
        TransportSecuritySurface {
            id: "ssh_bootstrap".to_string(),
            scope: RemoteNetwork,
            mutates: true,
            confidentiality: Declared,
            peer_identity: Unknown,
            mutual_authentication: Unknown,
            authorization_binding: Absent,
            evidence_refs: Vec::new(),
            residual_risks: vec![
                "breakglass shell is not canonical runtime mutation authority".to_string(),
            ],
        },
        TransportSecuritySurface {
            id: "mcp_stdio".to_string(),
            scope: LocalProcess,
            mutates: true,
            confidentiality: NotApplicable,
            peer_identity: Declared,
            mutual_authentication: NotApplicable,
            authorization_binding: Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec!["caller process identity is not yet emitted as proof".to_string()],
        },
        TransportSecuritySurface {
            id: "forgejo_http".to_string(),
            scope: RemoteNetwork,
            mutates: true,
            confidentiality: Absent,
            peer_identity: Absent,
            mutual_authentication: Absent,
            authorization_binding: Declared,
            evidence_refs: vec!["infra/repo-remotes.json:http".to_string()],
            residual_risks: vec!["sovereign candidate remote is plaintext HTTP".to_string()],
        },
        TransportSecuritySurface {
            id: "proxmox_api".to_string(),
            scope: RemoteNetwork,
            mutates: true,
            confidentiality: Declared,
            peer_identity: Unknown,
            mutual_authentication: Unknown,
            authorization_binding: Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec!["provider TLS and machine identity proof unavailable".to_string()],
        },
        TransportSecuritySurface {
            id: "external_https_apis".to_string(),
            scope: ExternalProvider,
            mutates: false,
            confidentiality: Declared,
            peer_identity: Declared,
            mutual_authentication: Absent,
            authorization_binding: Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec![
                "API keys authenticate CYNIC without mTLS client identity".to_string(),
            ],
        },
    ])
}

pub fn build_transport_matrix_report(
    observer: &impl OperatorTransportObservationPort,
) -> TransportMatrixReport {
    let bridge_status = bridge::bridge_status();
    let ssh = observer.observe_ssh();
    let tmux = observer.observe_tmux();
    let host_exec_status = if bridge_status.status == "active" {
        "active"
    } else {
        "missing_activation"
    };

    TransportMatrixReport {
        schema: "cynic.host_transport_matrix.v1",
        status: if host_exec_status == "active" {
            "bounded_transport_ready"
        } else {
            "bootstrap_required"
        },
        transports: vec![
            TransportSurface {
                id: "host_exec",
                class: "local_bounded_privilege_bridge",
                status: host_exec_status,
                authority_boundary: "cynic user -> fixed root helper",
                canonical_use: "local root-owned host bootstrap and bounded OS mutation",
                industrial_grade: "yes",
                command_surface: "cynic bootstrap bridge plan",
                notes: vec![
                    "single helper path and sudoers rule",
                    "no shell escape and no raw package manager/systemctl sudo surface",
                    "proof available through bridge-status and bootstrap-chain",
                ],
                executable: None,
                tmux_harness: None,
            },
            TransportSurface {
                id: "fleet_node_exec",
                class: "remote_bounded_node_exec",
                status: "declared_contract",
                authority_boundary: "kernel/MCP tool -> bounded node action",
                canonical_use: "remote bounded execution on registered nodes",
                industrial_grade: "target",
                command_surface: "fleet_node_exec via cynic MCP",
                notes: vec![
                    "preferred remote transport in registry contracts",
                    "must stay bounded and evidence-producing",
                    "should not degrade into raw ssh command tunneling",
                ],
                executable: None,
                tmux_harness: None,
            },
            TransportSurface {
                id: "ssh",
                class: "bootstrap_breakglass_transport",
                status: "degraded_fallback_only",
                authority_boundary: "operator shell on remote host",
                canonical_use: "initial reachability, emergency diagnostics, repo transport fallback",
                industrial_grade: "no",
                command_surface: "ssh <host>",
                notes: vec![
                    "not acceptable as the canonical runtime mutation plane",
                    "weak evidence boundary compared to bounded helper surfaces",
                    "must shrink as host_exec and fleet_node_exec coverage expands",
                ],
                executable: Some(ssh),
                tmux_harness: None,
            },
            TransportSurface {
                id: "tmux",
                class: "local_operator_harness",
                status: if tmux.status == "unavailable" {
                    "unavailable"
                } else {
                    "operator_harness_only"
                },
                authority_boundary: "operator terminal -> explicit local session",
                canonical_use: "preserve and inspect operator work continuity; never grant runtime mutation authority",
                industrial_grade: "harness_only",
                command_surface: "cynic transport matrix",
                notes: vec![
                    "session evidence is read-only and machine-readable",
                    "tmux is a controlled execution context, not a capability or authority",
                    "remote mutations still require fleet_node_exec or a bounded helper",
                ],
                executable: None,
                tmux_harness: Some(tmux),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeObserver;

    #[test]
    fn remote_mutation_surfaces_fail_closed_without_identity_proof() {
        let report = build_transport_security_report();

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .remote_mutation_blockers
                .contains(&"fleet_node_exec:remote_mutation_identity_unproven".to_string())
        );
        assert!(
            report
                .remote_mutation_blockers
                .contains(&"kernel_rest_websocket:remote_mutation_identity_unproven".to_string())
        );
    }

    impl OperatorTransportObservationPort for FakeObserver {
        fn observe_ssh(&self) -> ExecutableObservation {
            ExecutableObservation {
                binary: "ssh".to_string(),
                status: "available".to_string(),
                path: Some("/usr/bin/ssh".to_string()),
            }
        }

        fn observe_tmux(&self) -> TmuxHarnessObservation {
            TmuxHarnessObservation {
                executable: ExecutableObservation {
                    binary: "tmux".to_string(),
                    status: "available".to_string(),
                    path: Some("/usr/bin/tmux".to_string()),
                },
                status: "active".to_string(),
                sessions: vec![TmuxSessionObservation {
                    name: "cynic".to_string(),
                    windows: 1,
                    attached_clients: 0,
                }],
                detail: None,
            }
        }
    }

    struct FakeSshTargetInspector;

    impl SshTargetInspectionPort for FakeSshTargetInspector {
        fn inspect_ssh_target(&self, target: &str) -> SshTargetInspectionReport {
            SshTargetInspectionReport {
                schema: "cynic.ssh_target_inspection.v1",
                target: target.to_string(),
                status: "observed".to_string(),
                ssh: ExecutableObservation {
                    binary: "ssh".to_string(),
                    status: "available".to_string(),
                    path: Some("/usr/bin/ssh".to_string()),
                },
                ssh_keyscan: ExecutableObservation {
                    binary: "ssh-keyscan".to_string(),
                    status: "available".to_string(),
                    path: Some("/usr/bin/ssh-keyscan".to_string()),
                },
                host_keys: vec![SshHostKeyObservation {
                    algorithm: "ed25519".to_string(),
                    fingerprint: "SHA256:fixture".to_string(),
                }],
                trust_state: "untrusted_observed".to_string(),
                mutation: false,
                next_action: "compare host key fingerprints before trust acceptance".to_string(),
                detail: None,
            }
        }
    }

    #[test]
    fn transport_matrix_classifies_ssh_as_fallback_only() {
        let report = build_transport_matrix_report(&FakeObserver);
        assert_eq!(report.schema, "cynic.host_transport_matrix.v1");
        assert!(
            report
                .transports
                .iter()
                .any(|item| item.id == "ssh" && item.status == "degraded_fallback_only")
        );
        assert!(
            report
                .transports
                .iter()
                .any(|item| item.id == "host_exec" && item.industrial_grade == "yes")
        );
        assert!(report.transports.iter().any(|item| {
            item.id == "tmux"
                && item
                    .tmux_harness
                    .as_ref()
                    .is_some_and(|harness| harness.status == "active")
        }));
    }

    #[test]
    fn ssh_target_inspection_rejects_shell_syntax_before_adapter_execution() {
        let report = inspect_ssh_target(&FakeSshTargetInspector, "pve.home;whoami");

        assert_eq!(report.status, "invalid_target");
        assert!(report.host_keys.is_empty());
    }

    #[test]
    fn ssh_target_inspection_projects_untrusted_host_keys() {
        let report = inspect_ssh_target(&FakeSshTargetInspector, "pve.home");

        assert_eq!(report.status, "observed");
        assert_eq!(report.trust_state, "untrusted_observed");
        assert!(!report.mutation);
    }
}

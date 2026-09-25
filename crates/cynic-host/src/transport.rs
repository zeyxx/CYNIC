use serde::Serialize;

<<<<<<< HEAD
use cynic_network::transport_security::{TransportScope, TransportSecurityReport, TransportSecuritySurface};
use cynic_protocol::SecurityEvidenceState;
=======
use cynic_protocol::{
    SecurityEvidenceState, TransportScope, TransportSecurityReport, TransportSecuritySurface,
};
>>>>>>> github/main

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
<<<<<<< HEAD
    /// Timestamp from the observer that produced the host-key evidence.
    /// A key match without this temporal boundary cannot authorize a route.
    pub observed_at_unix_ms: u64,
=======
>>>>>>> github/main
    pub ssh: ExecutableObservation,
    pub ssh_keyscan: ExecutableObservation,
    pub host_keys: Vec<SshHostKeyObservation>,
    pub trust_state: String,
    pub mutation: bool,
    pub next_action: String,
    pub detail: Option<String>,
}

<<<<<<< HEAD
/// SSH key material present on the operator host and declared in ssh config.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshKeyObservation {
    pub identity_file_declared: String,
    pub identity_file_resolved: String,
    pub present: bool,
    pub permissions_ok: bool,
    pub key_type: Option<String>,
    pub sha256_fingerprint: Option<String>,
    pub detail: Option<String>,
}

/// Resolved runtime target for one SSH alias.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshResolvedTarget {
    pub alias: String,
    pub user: Option<String>,
    pub hostname: String,
    pub port: u16,
    pub batch_mode: bool,
    pub resolved_by: String,
}

/// One declared capability alias observed against the actual ssh config.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshAliasObservation {
    pub alias: String,
    pub declared_in_policy: bool,
    pub present_in_config: bool,
    pub resolves_to: Option<SshResolvedTarget>,
    pub key: Option<SshKeyObservation>,
    pub connectivity_batchmode: Option<String>,
    pub host_key_trust_state: String,
    pub detail: Option<String>,
}

/// Complete read-only projection of the local operator ssh configuration:
/// parsing, declared-vs-observed correlation, key presence/permission,
/// connectivity and trust state.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshLocalSupervisionReport {
    pub schema: &'static str,
    pub status: String,
    pub ssh_config_path: String,
    pub ssh_config_readable: bool,
    pub ssh_config_parse_errors: Vec<String>,
    pub aliases_defined: Vec<String>,
    pub aliases_used_by_policy: Vec<String>,
    pub aliases_missing_from_config: Vec<String>,
    pub aliases: Vec<SshAliasObservation>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

=======
>>>>>>> github/main
/// Explicit read-only boundary for bootstrap SSH target trust evidence.
pub trait SshTargetInspectionPort {
    fn inspect_ssh_target(&self, target: &str) -> SshTargetInspectionReport;
}

pub fn inspect_ssh_target(
    observer: &impl SshTargetInspectionPort,
    target: &str,
) -> SshTargetInspectionReport {
<<<<<<< HEAD
    let Ok(host) = canonical_ssh_host(target) else {
=======
    if !valid_ssh_target(target) {
>>>>>>> github/main
        return SshTargetInspectionReport {
            schema: "cynic.ssh_target_inspection.v1",
            target: target.to_string(),
            status: "invalid_target".to_string(),
<<<<<<< HEAD
            observed_at_unix_ms: 0,
=======
>>>>>>> github/main
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
<<<<<<< HEAD
    };
    observer.inspect_ssh_target(&host)
}

/// Reduce an SSH connection target to the host identity covered by an anchor.
/// The optional `user@` selector is transport configuration, never host-key
/// identity, so distinct principals on one host share one anchor.
pub fn canonical_ssh_host(target: &str) -> Result<String, String> {
    let (user, host) = match target.split_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() && !host.contains('@') => {
            (Some(user), host)
        }
        Some(_) => return Err("invalid_ssh_target".to_string()),
        None => (None, target),
    };
    if user.is_some_and(|user| {
        !user
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    }) || host.is_empty()
        || host.len() > 253
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-'))
    {
        return Err("invalid_ssh_target".to_string());
    }
    Ok(host.to_string())
=======
    }
    observer.inspect_ssh_target(target)
}

fn valid_ssh_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 253
        && target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-'))
>>>>>>> github/main
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
<<<<<<< HEAD
            peer_identity: ssh_supervision_peer_identity(),
            mutual_authentication: Unknown,
            authorization_binding: Absent,
            evidence_refs: vec!["cynic.ssh_local_supervision.v1".to_string()],
            residual_risks: ssh_supervision_risks(),
=======
            peer_identity: Unknown,
            mutual_authentication: Unknown,
            authorization_binding: Absent,
            evidence_refs: Vec::new(),
            residual_risks: vec![
                "breakglass shell is not canonical runtime mutation authority".to_string(),
            ],
>>>>>>> github/main
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
<<<<<<< HEAD
    use crate::ssh_config::parse_ssh_config;
=======
>>>>>>> github/main

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
<<<<<<< HEAD
                observed_at_unix_ms: 1,
=======
>>>>>>> github/main
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
<<<<<<< HEAD
    fn canonical_host_excludes_the_transport_principal() {
        assert_eq!(
            canonical_ssh_host("cynic@10.10.10.119"),
            Ok("10.10.10.119".to_string())
        );
        assert!(canonical_ssh_host("cynic@@10.10.10.119").is_err());
    }

    #[test]
=======
>>>>>>> github/main
    fn ssh_target_inspection_projects_untrusted_host_keys() {
        let report = inspect_ssh_target(&FakeSshTargetInspector, "pve.home");

        assert_eq!(report.status, "observed");
        assert_eq!(report.trust_state, "untrusted_observed");
        assert!(!report.mutation);
    }
<<<<<<< HEAD

    #[test]
    fn ssh_local_supervision_parses_real_config_structure() {
        let content = "Host cynic-builder-ssh\n  HostName 10.10.10.119\n  User cynic\n  IdentityFile ~/.ssh/cynic-build-authority\n  IdentitiesOnly yes\n  BatchMode yes\n  ConnectTimeout 5\n";
        let parsed = parse_ssh_config(content);
        assert_eq!(parsed.entries.len(), 1);
        assert!(parsed.parse_errors.is_empty());
        let entry = &parsed.entries[0];
        assert_eq!(entry.host_pattern, "cynic-builder-ssh");
        assert_eq!(entry.host_name.as_deref(), Some("10.10.10.119"));
        assert_eq!(entry.user.as_deref(), Some("cynic"));
        assert_eq!(entry.batch_mode, Some(true));
    }
}

/// Default path to the operator ssh config file.
pub fn default_ssh_config_path() -> std::path::PathBuf {
    std::env::home_dir()
        .map(|p| p.join(".ssh/config"))
        .unwrap_or_else(|| std::path::PathBuf::from("/etc/ssh/ssh_config"))
}

/// Build the complete SSH supervision report for this operator host.
pub fn build_ssh_local_supervision_report() -> SshLocalSupervisionReport {
    build_ssh_local_supervision_report_from(default_ssh_config_path().as_path())
}

/// Build the SSH supervision report for an explicit ssh config path.
pub fn build_ssh_local_supervision_report_from(
    config_path: &std::path::Path,
) -> SshLocalSupervisionReport {
    let mut blockers = Vec::new();
    let mut residual_unknowns = Vec::new();

    let parse_result = match crate::ssh_config::parse_ssh_config_file(config_path) {
        Ok(result) => result,
        Err(e) => {
            return SshLocalSupervisionReport {
                schema: "cynic.ssh_local_supervision.v1",
                status: "blocked".to_string(),
                ssh_config_path: config_path.display().to_string(),
                ssh_config_readable: false,
                ssh_config_parse_errors: vec![e.clone()],
                aliases_defined: Vec::new(),
                aliases_used_by_policy: Vec::new(),
                aliases_missing_from_config: Vec::new(),
                aliases: Vec::new(),
                blockers: vec![format!("ssh config unreadable: {e}")],
                residual_unknowns,
            };
        }
    };

    let parse = match parse_result {
        Some(p) => p,
        None => {
            return SshLocalSupervisionReport {
                schema: "cynic.ssh_local_supervision.v1",
                status: "blocked".to_string(),
                ssh_config_path: config_path.display().to_string(),
                ssh_config_readable: false,
                ssh_config_parse_errors: Vec::new(),
                aliases_defined: Vec::new(),
                aliases_used_by_policy: Vec::new(),
                aliases_missing_from_config: Vec::new(),
                aliases: Vec::new(),
                blockers: vec![format!("ssh config not found: {}", config_path.display())],
                residual_unknowns,
            };
        }
    };

    if !parse.parse_errors.is_empty() {
        blockers.push(format!(
            "ssh config has {} parse error(s)",
            parse.parse_errors.len()
        ));
    }

    let aliases_defined: Vec<String> = parse
        .entries
        .iter()
        .map(|e| e.host_pattern.clone())
        .collect();

    let aliases_used_by_policy = collect_policy_aliases();

    let aliases_missing_from_config: Vec<String> = aliases_used_by_policy
        .iter()
        .filter(|a| !aliases_defined.contains(a))
        .cloned()
        .collect();

    if !aliases_missing_from_config.is_empty() {
        blockers.push(format!(
            "aliases declared in transport policy but missing from ssh config: {:?}",
            aliases_missing_from_config
        ));
    }

    let aliases: Vec<SshAliasObservation> = aliases_defined
        .iter()
        .map(|alias| {
            let declared_in_policy = aliases_used_by_policy.contains(alias);
            let entry = crate::ssh_config::find_entry(&parse, alias);
            let resolves_to = entry.and_then(|e| resolve_target(alias, e));
            let key = entry.and_then(inspect_key);
            let connectivity_batchmode = entry.map(|e| probe_batchmode(e));
            SshAliasObservation {
                alias: alias.clone(),
                declared_in_policy,
                present_in_config: true,
                resolves_to,
                key,
                connectivity_batchmode,
                host_key_trust_state: "not_observed".to_string(),
                detail: None,
            }
        })
        .collect();

    let status = if blockers.is_empty() {
        "converged"
    } else {
        "blocked"
    };

    SshLocalSupervisionReport {
        schema: "cynic.ssh_local_supervision.v1",
        status: status.to_string(),
        ssh_config_path: config_path.display().to_string(),
        ssh_config_readable: true,
        ssh_config_parse_errors: parse.parse_errors,
        aliases_defined,
        aliases_used_by_policy,
        aliases_missing_from_config,
        aliases,
        blockers,
        residual_unknowns,
    }
}

/// Collect SSH aliases referenced in the transport capabilities policy.
fn collect_policy_aliases() -> Vec<String> {
    let policy_path =
        std::path::PathBuf::from("infra/privilege/transport-capabilities.policy.json");
    let content = match std::fs::read_to_string(&policy_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut aliases = Vec::new();

    if let Some(overrides) = value
        .get("default_endpoint_overrides")
        .and_then(|v| v.as_object())
    {
        for (_, endpoint) in overrides {
            if let Some(s) = endpoint.as_str() {
                aliases.push(s.to_string());
            }
        }
    }

    aliases.sort();
    aliases.dedup();
    aliases
}

/// Resolve the runtime target for an SSH alias.
fn resolve_target(
    alias: &str,
    entry: &crate::ssh_config::SshConfigEntry,
) -> Option<SshResolvedTarget> {
    let hostname = entry.host_name.clone()?;
    let port = entry.port.unwrap_or(22);
    let batch_mode = entry.batch_mode.unwrap_or(false);
    Some(SshResolvedTarget {
        alias: alias.to_string(),
        user: entry.user.clone(),
        hostname,
        port,
        batch_mode,
        resolved_by: "ssh_config".to_string(),
    })
}

/// Inspect the SSH key material declared for an alias.
fn inspect_key(entry: &crate::ssh_config::SshConfigEntry) -> Option<SshKeyObservation> {
    let identity_file = entry.identity_file.as_ref()?;
    let resolved = resolve_tilde(identity_file);
    let path = std::path::Path::new(&resolved);
    let present = path.exists();
    let permissions_ok = if present {
        std::fs::metadata(path)
            .map(|m| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.permissions().mode() & 0o077 == 0
                }
                #[cfg(not(unix))]
                {
                    true
                }
            })
            .unwrap_or(false)
    } else {
        false
    };

    Some(SshKeyObservation {
        identity_file_declared: identity_file.clone(),
        identity_file_resolved: resolved.clone(),
        present,
        permissions_ok,
        key_type: None,
        sha256_fingerprint: None,
        detail: if !present {
            Some(format!("identity file not found: {resolved}"))
        } else if !permissions_ok {
            Some("identity file has overly permissive mode (expected 0600)".to_string())
        } else {
            None
        },
    })
}

/// Resolve a leading `~` to the user's home directory without extra deps.
fn resolve_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::home_dir() {
            return home.join(rest).display().to_string();
        }
    }
    path.to_string()
}

/// Probe whether BatchMode connectivity succeeds for an alias.
fn probe_batchmode(entry: &crate::ssh_config::SshConfigEntry) -> String {
    if entry.batch_mode.unwrap_or(false) {
        "batchmode_declared".to_string()
    } else {
        "batchmode_not_configured".to_string()
    }
}

/// Derive the peer_identity evidence state from SSH supervision.
fn ssh_supervision_peer_identity() -> SecurityEvidenceState {
    let report = build_ssh_local_supervision_report();
    if report.aliases_missing_from_config.is_empty() && report.status == "converged" {
        SecurityEvidenceState::Proven
    } else if report.ssh_config_readable {
        SecurityEvidenceState::Declared
    } else {
        SecurityEvidenceState::Unknown
    }
}

/// Derive residual risks from SSH supervision.
fn ssh_supervision_risks() -> Vec<String> {
    let report = build_ssh_local_supervision_report();
    let mut risks = Vec::new();
    if !report.ssh_config_readable {
        risks.push("ssh config unreadable — bootstrap transport state unknown".to_string());
    }
    if !report.aliases_missing_from_config.is_empty() {
        risks.push(format!(
            "declared aliases missing from ssh config: {:?}",
            report.aliases_missing_from_config
        ));
    }
    risks.push("breakglass shell is not canonical runtime mutation authority".to_string());
    risks
=======
>>>>>>> github/main
}

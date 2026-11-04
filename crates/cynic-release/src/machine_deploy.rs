//! Pure layout-aware per-machine deploy planning, read-back parsing, and gate
//! evaluation for `infra/deploy-locations.json`.
//!
//! Zero I/O: this module decides exact typed argv vectors for staging, backup,
//! atomic swap (`Symlink` and `Flat`), read-back probes, and rollback, then
//! evaluates observed command outputs into typed gate verdicts.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::deploy_locations::{
    decide_deploy_target, identity_matches, resolve_deploy_path, DeployLayout,
    DeployLocationsContract, DeployTargetRefusal,
};

/// Schema identifier emitted by `cynic build deploy --apply --json`.
pub const REMOTE_DEPLOY_REPORT_SCHEMA: &str = "cynic.release.remote_deploy_report.v1";

/// Machine filesystem roots used to expand `${state_root}` and `${home}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineRoots {
    pub state_root: Option<String>,
    pub home: Option<String>,
}

/// Pure, shell-free execution specification for deploying onto one machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineDeploySpec {
    pub machine: String,
    pub registry_node_id: String,
    pub primary_endpoint: String,
    pub fallback_endpoint: Option<String>,
    pub layout: DeployLayout,
    pub deploy_path: String,
    pub staging_dir: String,
    pub staged_binary_path: String,
    pub backup_pattern: String,
    pub swap_commands: Vec<Vec<String>>,
    pub sha256_argv: Vec<String>,
    pub identity_argv: Vec<String>,
    pub smoke_argv: Vec<String>,
}

/// Typed failure gate when a machine deploy step does not converge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeployGateFailure {
    Refused { reason: String },
    Transport { detail: String },
    Stage { detail: String },
    Backup { detail: String },
    Swap { detail: String },
    BytesMismatch { expected: String, observed: String },
    IdentityMismatch { expected: String, observed: String },
    Smoke { exit_code: i32, stderr: String },
}

impl DeployGateFailure {
    pub fn gate_name(&self) -> &'static str {
        match self {
            Self::Refused { .. } => "target_admission",
            Self::Transport { .. } => "transport",
            Self::Stage { .. } => "stage",
            Self::Backup { .. } => "backup",
            Self::Swap { .. } => "swap",
            Self::BytesMismatch { .. } => "bytes_match",
            Self::IdentityMismatch { .. } => "identity_match",
            Self::Smoke { .. } => "smoke",
        }
    }
}

/// Three independent post-swap observations proving the live binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployReadbackObservation {
    pub sha256: String,
    pub git_sha: String,
    pub smoke_exit_code: i32,
    pub smoke_stdout: String,
}

/// Parse `sha256sum` stdout into a validated 64-character hex digest.
pub fn parse_sha256(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|d| d.to_ascii_lowercase())
}

/// Parse `cynic version --json` stdout into its baked `git_sha`.
pub fn parse_identity_git_sha(stdout: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let sha = value
        .get("git_sha")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            value
                .get("build")
                .and_then(|b| b.get("git_sha"))
                .and_then(serde_json::Value::as_str)
        })?;
    let trimmed = sha.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Compute the self-describing backup path `<dir>/<name>.bak-<short_sha>`.
pub fn backup_path_for(pattern: &str, deploy_path: &Path, live_sha256: &str) -> PathBuf {
    let bin_name = deploy_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cynic");
    let short = if live_sha256.len() >= 12 {
        &live_sha256[..12]
    } else {
        live_sha256
    };
    let rendered = pattern.replace("<sha>", short);
    let file_name = if rendered.contains("binary") {
        rendered.replace("binary", bin_name)
    } else {
        rendered
    };
    deploy_path
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .join(file_name)
}

/// Build the shell-free backup argv (`cp -pL` dereferences symlinks so the
/// backup always holds concrete ELF bytes rather than a retargeted link).
pub fn build_backup_argv(deploy_path: &str, backup_path: &Path) -> Vec<String> {
    vec![
        "cp".to_string(),
        "-pL".to_string(),
        deploy_path.to_string(),
        backup_path.display().to_string(),
    ]
}

/// Build shell-free rollback commands restoring `backup_path` without breaking
/// the machine's declared layout (`Flat` vs `Symlink`).
pub fn build_rollback_commands(
    spec: &MachineDeploySpec,
    backup_path: &Path,
    roots: &MachineRoots,
    live_sha256: &str,
) -> Vec<Vec<String>> {
    let backup_str = backup_path.display().to_string();
    let bin_name = Path::new(&spec.deploy_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cynic");
    let short = if live_sha256.len() >= 12 {
        &live_sha256[..12]
    } else {
        "prev"
    };
    match spec.layout {
        DeployLayout::Flat => {
            let restore_tmp = format!("{}.rollback-{}", spec.deploy_path, short);
            vec![
                vec!["cp".to_string(), "-p".to_string(), backup_str, restore_tmp.clone()],
                vec!["mv".to_string(), "-f".to_string(), restore_tmp, spec.deploy_path.clone()],
            ]
        }
        DeployLayout::Symlink => {
            let Some(root) = roots
                .state_root
                .as_deref()
                .map(|r| r.trim().trim_end_matches('/'))
                .filter(|r| !r.is_empty())
            else {
                return Vec::new();
            };
            let rollback_dir = format!("{root}/releases/rollback-{short}");
            let rollback_bin = format!("{rollback_dir}/{bin_name}");
            let current_link = format!("{root}/current");
            let tmp_link = format!("{root}/current.rollback-{short}");
            vec![
                vec!["mkdir".to_string(), "-p".to_string(), rollback_dir.clone()],
                vec!["cp".to_string(), "-p".to_string(), backup_str, rollback_bin],
                vec!["ln".to_string(), "-sfn".to_string(), rollback_dir, tmp_link.clone()],
                vec!["mv".to_string(), "-Tf".to_string(), tmp_link, current_link.clone()],
                vec![
                    "ln".to_string(),
                    "-sfn".to_string(),
                    format!("{current_link}/{bin_name}"),
                    spec.deploy_path.clone(),
                ],
            ]
        }
    }
}

fn validate_clean_segment(label: &str, machine: &str, value: &str) -> Result<(), DeployTargetRefusal> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.starts_with('-')
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(DeployTargetRefusal {
            machine: machine.to_string(),
            reason: format!("invalid {label} {value:?}"),
        });
    }
    Ok(())
}

fn expand_smoke_argv(
    machine_name: &str,
    smoke_template: &str,
    roots: &MachineRoots,
) -> Result<Vec<String>, DeployTargetRefusal> {
    let tokens: Vec<&str> = smoke_template.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(DeployTargetRefusal {
            machine: machine_name.to_string(),
            reason: "smoke command is empty".to_string(),
        });
    }
    let mut argv = Vec::with_capacity(tokens.len());
    for (idx, token) in tokens.iter().enumerate() {
        if token.chars().any(|c| "`$|;&<>()\\".contains(c)) {
            return Err(DeployTargetRefusal {
                machine: machine_name.to_string(),
                reason: format!("smoke command token {token:?} contains shell metacharacters"),
            });
        }
        if idx == 0 && (token.contains("${") || token.starts_with('/')) {
            argv.push(resolve_deploy_path(
                token,
                roots.state_root.as_deref(),
                roots.home.as_deref(),
            )?);
        } else if token.contains("${") {
            return Err(DeployTargetRefusal {
                machine: machine_name.to_string(),
                reason: format!("smoke argument {token:?} carries an unsupported expansion"),
            });
        } else {
            argv.push((*token).to_string());
        }
    }
    Ok(argv)
}

/// Plan the exact shell-free commands for deploying `commit` onto `machine_key`.
pub fn plan_machine_deploy(
    locations: &DeployLocationsContract,
    machine_key: &str,
    commit: &str,
    tag: &str,
    roots: &MachineRoots,
) -> Result<MachineDeploySpec, DeployTargetRefusal> {
    if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(DeployTargetRefusal {
            machine: machine_key.to_string(),
            reason: "commit must be an exact 40-character hexadecimal SHA".to_string(),
        });
    }
    validate_clean_segment("release tag", machine_key, tag)?;
    let target = decide_deploy_target(locations, machine_key)?;
    let deploy_path = resolve_deploy_path(
        &target.deploy_path,
        roots.state_root.as_deref(),
        roots.home.as_deref(),
    )?;
    let deploy_ref = Path::new(&deploy_path);
    let deploy_parent = deploy_ref
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| DeployTargetRefusal {
            machine: target.hostname.clone(),
            reason: format!("deploy path {deploy_path} has no parent directory"),
        })?;
    let bin_name = deploy_ref
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| DeployTargetRefusal {
            machine: target.hostname.clone(),
            reason: format!("deploy path {deploy_path} has no binary file name"),
        })?;
    let short_commit = &commit[..12];

    let (staging_dir, staged_binary_path, swap_commands) = match target.layout {
        DeployLayout::Flat => {
            let staged = format!("{deploy_parent}/.{bin_name}.staged-{short_commit}");
            let commands = vec![
                vec!["chmod".to_string(), "0755".to_string(), staged.clone()],
                vec!["mv".to_string(), "-f".to_string(), staged.clone(), deploy_path.clone()],
            ];
            (deploy_parent.to_string(), staged, commands)
        }
        DeployLayout::Symlink => {
            let root = roots
                .state_root
                .as_deref()
                .map(|r| r.trim().trim_end_matches('/'))
                .filter(|r| !r.is_empty() && r.starts_with('/'))
                .ok_or_else(|| DeployTargetRefusal {
                    machine: target.hostname.clone(),
                    reason: "symlink layout requires an absolute state_root".to_string(),
                })?;
            let release_dir = format!("{root}/releases/{tag}");
            let staged = format!("{release_dir}/{bin_name}");
            let current_link = format!("{root}/current");
            let tmp_link = format!("{root}/current.new-{short_commit}");
            let commands = vec![
                vec!["chmod".to_string(), "0755".to_string(), staged.clone()],
                vec!["ln".to_string(), "-sfn".to_string(), release_dir.clone(), tmp_link.clone()],
                vec!["mv".to_string(), "-Tf".to_string(), tmp_link, current_link.clone()],
                vec!["mkdir".to_string(), "-p".to_string(), deploy_parent.to_string()],
                vec![
                    "ln".to_string(),
                    "-sfn".to_string(),
                    format!("{current_link}/{bin_name}"),
                    deploy_path.clone(),
                ],
            ];
            (release_dir, staged, commands)
        }
    };

    let smoke_argv = expand_smoke_argv(&target.hostname, &target.readback.smoke_command, roots)?;

    Ok(MachineDeploySpec {
        machine: target.hostname,
        registry_node_id: target.registry_node_id,
        primary_endpoint: target.ssh_target,
        fallback_endpoint: target.ssh_target_fallback,
        layout: target.layout,
        deploy_path: deploy_path.clone(),
        staging_dir,
        staged_binary_path,
        backup_pattern: target.backup_pattern,
        swap_commands,
        sha256_argv: vec!["sha256sum".to_string(), deploy_path.clone()],
        identity_argv: vec![deploy_path, "version".to_string(), "--json".to_string()],
        smoke_argv,
    })
}

/// Evaluate post-swap read-back observations against expected digest & commit.
pub fn evaluate_readback(
    expected_sha256: &str,
    expected_commit: &str,
    sha256_stdout: &str,
    identity_stdout: &str,
    smoke_exit_code: i32,
    smoke_stdout: &str,
    smoke_stderr: &str,
) -> Result<DeployReadbackObservation, DeployGateFailure> {
    let observed_sha = parse_sha256(sha256_stdout).unwrap_or_default();
    if observed_sha != expected_sha256.trim().to_ascii_lowercase() {
        return Err(DeployGateFailure::BytesMismatch {
            expected: expected_sha256.to_string(),
            observed: observed_sha,
        });
    }
    let observed_git_sha = parse_identity_git_sha(identity_stdout).unwrap_or_default();
    if !identity_matches(&observed_git_sha, expected_commit) {
        return Err(DeployGateFailure::IdentityMismatch {
            expected: expected_commit.to_string(),
            observed: observed_git_sha,
        });
    }
    if smoke_exit_code != 0 {
        return Err(DeployGateFailure::Smoke {
            exit_code: smoke_exit_code,
            stderr: smoke_stderr.trim().to_string(),
        });
    }
    Ok(DeployReadbackObservation {
        sha256: observed_sha,
        git_sha: observed_git_sha,
        smoke_exit_code,
        smoke_stdout: smoke_stdout.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy_locations::{DeployMachine, DeployReadback, DEPLOY_LOCATIONS_SCHEMA};

    const COMMIT: &str = "42e56f5f1f19b7cd7ef0560255129bf7a439f82f";
    const DIGEST: &str = "998ddf12236bb715cd307618a134f38381debfefadba0d5054989a03c7c13f7e";

    fn sample_contract() -> DeployLocationsContract {
        DeployLocationsContract {
            schema: DEPLOY_LOCATIONS_SCHEMA.to_string(),
            status: "normative".to_string(),
            machines: vec![
                DeployMachine {
                    hostname: "builder".to_string(),
                    registry_node_id: "cynic-builder".to_string(),
                    os: "Ubuntu".to_string(),
                    ssh_target: "cynic-builder-ssh".to_string(),
                    ssh_target_fallback: None,
                    principal: "cynic".to_string(),
                    deploy_path: "${state_root}/bin/cynic".to_string(),
                    deploy_path_resolved: "/srv/fleet-state/bin/cynic".to_string(),
                    path_owner: "cynic:cynic".to_string(),
                    layout: DeployLayout::Symlink,
                    backup_pattern: "binary.bak-<sha>".to_string(),
                    readback: DeployReadback {
                        sha256_command: "sha256sum ${state_root}/bin/cynic".to_string(),
                        identity_command: "${state_root}/bin/cynic version --json".to_string(),
                        smoke_command: "${state_root}/bin/cynic source status --json".to_string(),
                        smoke_expect: "exit code 0".to_string(),
                    },
                    measured: None,
                },
                DeployMachine {
                    hostname: "workstation".to_string(),
                    registry_node_id: "cynic-workstation".to_string(),
                    os: "Ubuntu".to_string(),
                    ssh_target: "local".to_string(),
                    ssh_target_fallback: None,
                    principal: "op".to_string(),
                    deploy_path: "${home}/.local/bin/cynic".to_string(),
                    deploy_path_resolved: "${home}/.local/bin/cynic".to_string(),
                    path_owner: "op:op".to_string(),
                    layout: DeployLayout::Flat,
                    backup_pattern: "cynic.bak-<sha>".to_string(),
                    readback: DeployReadback {
                        sha256_command: "sha256sum ${home}/.local/bin/cynic".to_string(),
                        identity_command: "${home}/.local/bin/cynic version --json".to_string(),
                        smoke_command: "${home}/.local/bin/cynic source status --json".to_string(),
                        smoke_expect: "exit code 0".to_string(),
                    },
                    measured: None,
                },
            ],
        }
    }

    #[test]
    fn flat_layout_plans_same_dir_atomic_rename_without_state_root() {
        let roots = MachineRoots {
            state_root: None,
            home: Some("/tmp/op-home".to_string()),
        };
        let spec = plan_machine_deploy(&sample_contract(), "workstation", COMMIT, "git-42e56f5f", &roots)
            .expect("flat workstation must plan without state_root");
        assert_eq!(spec.deploy_path, "/tmp/op-home/.local/bin/cynic");
        assert_eq!(spec.staging_dir, "/tmp/op-home/.local/bin");
        assert!(spec.staged_binary_path.starts_with("/tmp/op-home/.local/bin/.cynic.staged-"));
        assert_eq!(spec.swap_commands[1][0], "mv");
        assert_eq!(spec.swap_commands[1][1], "-f");
    }

    #[test]
    fn symlink_layout_plans_atomic_current_link_swap_and_preserves_rollback_layout() {
        let roots = MachineRoots {
            state_root: Some("/srv/fleet-state".to_string()),
            home: None,
        };
        let spec = plan_machine_deploy(&sample_contract(), "builder", COMMIT, "v0.9.0", &roots)
            .expect("symlink machine must plan");
        assert_eq!(spec.staging_dir, "/srv/fleet-state/releases/v0.9.0");
        assert!(spec.swap_commands.iter().any(|cmd| cmd.first().map(String::as_str) == Some("mv") && cmd.get(1).map(String::as_str) == Some("-Tf")));
        let backup = backup_path_for(&spec.backup_pattern, Path::new(&spec.deploy_path), DIGEST);
        assert_eq!(backup, PathBuf::from("/srv/fleet-state/bin/cynic.bak-998ddf12236b"));
        let rollback = build_rollback_commands(&spec, &backup, &roots, DIGEST);
        assert_eq!(rollback.len(), 5);
        assert_eq!(rollback[3][0], "mv");
    }

    #[test]
    fn readback_gates_reject_bytes_identity_and_smoke_mismatches() {
        let ok = evaluate_readback(
            DIGEST,
            COMMIT,
            &format!("{DIGEST}  /srv/fleet-state/bin/cynic\n"),
            &format!(r#"{{"git_sha":"{COMMIT}"}}"#),
            0,
            "{}",
            "",
        )
        .expect("matching readback passes");
        assert_eq!(ok.git_sha, COMMIT);

        assert!(matches!(
            evaluate_readback("0000000000000000000000000000000000000000000000000000000000000000", COMMIT, &format!("{DIGEST} /x"), "{}", 0, "", ""),
            Err(DeployGateFailure::BytesMismatch { .. })
        ));
        assert!(matches!(
            evaluate_readback(DIGEST, COMMIT, &format!("{DIGEST} /x"), r#"{"git_sha":"deadbeef"}"#, 0, "", ""),
            Err(DeployGateFailure::IdentityMismatch { .. })
        ));
        assert!(matches!(
            evaluate_readback(DIGEST, COMMIT, &format!("{DIGEST} /x"), &format!(r#"{{"git_sha":"{COMMIT}"}}"#), 2, "", "boom"),
            Err(DeployGateFailure::Smoke { exit_code: 2, .. })
        ));
    }
}

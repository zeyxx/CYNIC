//! Typed build-runner bootstrap boundary.
//!
//! This is the Rust owner for replacing the legacy shell bootstrap entrypoint.
//! It deliberately plans only; argv execution and receipt persistence must be
//! injected by the host execution layer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::update_lifecycle::ArgvRunner;
use crate::{
    adapters::local_build_cache::LocalBuildCacheAdapter,
    domain::disk::build_runner::{
        BuildCacheObservationPort, BuildCachePlan, load_build_cache_desired_state,
        reconcile_build_cache,
    },
};

pub const BUILD_RUNNER_BOOTSTRAP_PLAN_SCHEMA: &str = "cynic.build_runner.bootstrap_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapRequest {
    pub node_id: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub target_phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapPlan {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub target_phase: String,
    pub mutates: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapExecutionPlan {
    pub workspace: String,
    pub argv: Vec<Vec<String>>,
    pub rollback_intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapBuildPlan {
    pub workspace: String,
    pub receipt_path: String,
    pub artifact_path: String,
    pub argv: Vec<String>,
    pub rollback_intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapBuildProof {
    pub schema: String,
    pub status: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub artifact_path: String,
    pub artifact_digest: Option<String>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

/// Canonical provision plan — native substrate provisioning via the
/// workspace-built `cynic-host-exec` binary. No SSH; uses principal_manager
/// (ensure_system_user) inside the bootstrap artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapProvisionPlan {
    pub scheme: String,
    pub node_id: String,
    pub principal: String,
    pub argv: Vec<String>,
    pub rollback_intent: String,
}

pub fn build_runner_bootstrap_provision_plan(
    plan: &BuildRunnerBootstrapPlan,
    workspace: &Path,
) -> BuildRunnerBootstrapProvisionPlan {
    let target_dir = workspace.join(".cynic-target");
    let host_exec = target_dir.join("debug").join("cynic-host-bootstrap");
    BuildRunnerBootstrapProvisionPlan {
        scheme: "cynic.build_runner_bootstrap_provision.v1".to_string(),
        node_id: plan.node_id.clone(),
        principal: "cynic".to_string(),
        argv: vec![
            "env".to_string(),
            format!("CARGO_TARGET_DIR={}", target_dir.display()),
            host_exec.display().to_string(),
            "provision-substrate".to_string(),
            "--node".to_string(),
            plan.node_id.clone(),
            "--principal".to_string(),
            "cynic".to_string(),
        ],
        rollback_intent: "identity provisioning is idempotent; no rollback needed"
            .to_string(),
    }
}

pub fn apply_build_runner_bootstrap_provision(
    provision: &BuildRunnerBootstrapProvisionPlan,
    runner: &dyn ArgvRunner,
) -> Result<String, String> {
    let argv: Vec<&str> = provision.argv.iter().map(String::as_str).collect();
    let out = runner.run(&argv);
    if out.exit_code != Some(0) {
        return Err(format!(
            "build_runner_bootstrap_provision_failed: exit={:?} stderr={}",
            out.exit_code,
            out.stderr.trim()
        ));
    }
    if !out.stdout.contains(&format!("principal={} provisioned", provision.principal)) {
        return Err(format!(
            "build_runner_bootstrap_provision_read_back_failed: {}",
            out.stdout.trim()
        ));
    }
    Ok(format!(
        "machine_enrollment_user_provisioned:{}",
        out.stdout.trim()
    ))
}

pub fn build_runner_bootstrap_execution_plan(
    plan: &BuildRunnerBootstrapPlan,
    workspace: &Path,
    source_bundle: &Path,
) -> Option<BuildRunnerBootstrapExecutionPlan> {
    if plan.status != "planned" || plan.target_phase == "preflight" {
        return None;
    }
    let workspace = workspace.display().to_string();
    Some(BuildRunnerBootstrapExecutionPlan {
        argv: vec![
            vec![
                "git".to_string(),
                "-C".to_string(),
                workspace.clone(),
                "fetch".to_string(),
                source_bundle.display().to_string(),
                plan.source_commit.clone(),
            ],
            vec![
                "git".to_string(),
                "-C".to_string(),
                workspace.clone(),
                "checkout".to_string(),
                "--detach".to_string(),
                plan.source_commit.clone(),
            ],
        ],
        workspace,
        rollback_intent:
            "leave prior checkout recoverable through git reflog; do not activate runtime artifacts"
                .to_string(),
    })
}

pub fn apply_build_runner_bootstrap(
    execution: &BuildRunnerBootstrapExecutionPlan,
    expected_commit: &str,
    expected_registry_digest: &str,
    apply: bool,
    runner: &dyn ArgvRunner,
) -> Result<String, String> {
    if !apply {
        return Err("build_runner_bootstrap_explicit_apply_required".to_string());
    }
    for command in &execution.argv {
        let argv = command.iter().map(String::as_str).collect::<Vec<_>>();
        if !runner.run(&argv).success() {
            return Err("build_runner_bootstrap_argv_failed".to_string());
        }
    }
    let read_back = runner.run(&["git", "-C", &execution.workspace, "rev-parse", "HEAD"]);
    if !read_back.success() || read_back.stdout.trim() != expected_commit {
        return Err("build_runner_bootstrap_commit_read_back_failed".to_string());
    }
    if registry_file_digest(&Path::new(&execution.workspace).join("infra/registry.json")).as_deref()
        != Some(expected_registry_digest)
    {
        return Err("build_runner_bootstrap_source_registry_read_back_failed".to_string());
    }
    Ok(format!("git-commit:{}", expected_commit))
}

/// The target directory is inside the detached workspace: source and artifact
/// read-back cannot fall through to the invoker's mutable worktree.
pub fn build_runner_bootstrap_build_plan(
    workspace: &Path,
    receipt_path: &Path,
) -> BuildRunnerBootstrapBuildPlan {
    let target_dir = workspace.join(".cynic-target");
    BuildRunnerBootstrapBuildPlan {
        workspace: workspace.display().to_string(),
        receipt_path: receipt_path.display().to_string(),
        artifact_path: target_dir.join("debug/cynic-host-exec").display().to_string(),
        argv: vec![
            "env".to_string(),
            format!("CARGO_TARGET_DIR={}", target_dir.display()),
            format!("CYNIC_BUILD_BOOTSTRAP_RECEIPT={}", receipt_path.display()),
            "make".to_string(),
            "-C".to_string(),
            workspace.display().to_string(),
            "build-host-exec".to_string(),
        ],
        rollback_intent: "retain the immutable source checkout and failed target directory for inspection; do not activate the artifact".to_string(),
    }
}

pub fn apply_build_runner_bootstrap_build(
    plan: &BuildRunnerBootstrapBuildPlan,
    runner: &dyn ArgvRunner,
) -> Result<String, String> {
    let argv = plan.argv.iter().map(String::as_str).collect::<Vec<_>>();
    if !runner.run(&argv).success() {
        return Err("build_runner_bootstrap_build_argv_failed".to_string());
    }
    let metadata = std::fs::symlink_metadata(&plan.artifact_path)
        .map_err(|_| "build_runner_bootstrap_artifact_read_back_missing".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("build_runner_bootstrap_artifact_read_back_unsafe".to_string());
    }
    let bytes = std::fs::read(&plan.artifact_path)
        .map_err(|_| "build_runner_bootstrap_artifact_read_back_missing".to_string())?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

pub fn bootstrap_build_proof(
    plan: &BuildRunnerBootstrapPlan,
    source_proof: String,
    build_plan: &BuildRunnerBootstrapBuildPlan,
    artifact_digest: String,
) -> BootstrapBuildProof {
    BootstrapBuildProof {
        schema: "cynic.host.bootstrap_build_proof.v1".to_string(),
        status: "proved".to_string(),
        source_commit: plan.source_commit.clone(),
        source_bundle_digest: plan.source_bundle_digest.clone(),
        artifact_path: build_plan.artifact_path.clone(),
        artifact_digest: Some(artifact_digest.clone()),
        evidence_refs: vec![
            source_proof,
            format!("artifact:{}:{}", build_plan.artifact_path, artifact_digest),
        ],
        blockers: Vec::new(),
    }
}

fn registry_file_digest(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

pub fn write_bootstrap_admission_receipt(
    path: &Path,
    receipt: &cynic_protocol::BuildRunnerBootstrapAdmission,
) -> Result<(), String> {
    if !receipt.validate().is_empty() {
        return Err("build_runner_bootstrap_receipt_contract_invalid".to_string());
    }
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|_| "build_runner_bootstrap_receipt_encode_failed".to_string())?;
    write_new_json_durable(path, &bytes, "build_runner_bootstrap_receipt")?;
    let read_back: cynic_protocol::BuildRunnerBootstrapAdmission = serde_json::from_slice(
        &std::fs::read(path)
            .map_err(|_| "build_runner_bootstrap_receipt_read_back_failed".to_string())?,
    )
    .map_err(|_| "build_runner_bootstrap_receipt_read_back_failed".to_string())?;
    if &read_back != receipt {
        return Err("build_runner_bootstrap_receipt_read_back_mismatch".to_string());
    }
    Ok(())
}

pub fn write_bootstrap_build_proof(path: &Path, proof: &BootstrapBuildProof) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(proof)
        .map_err(|_| "build_runner_bootstrap_proof_encode_failed".to_string())?;
    write_new_json_durable(path, &bytes, "build_runner_bootstrap_proof")?;
    let read_back: BootstrapBuildProof = serde_json::from_slice(
        &fs::read(path).map_err(|_| "build_runner_bootstrap_proof_read_back_failed".to_string())?,
    )
    .map_err(|_| "build_runner_bootstrap_proof_read_back_failed".to_string())?;
    if &read_back != proof {
        return Err("build_runner_bootstrap_proof_read_back_mismatch".to_string());
    }
    Ok(())
}

fn write_new_json_durable(path: &Path, bytes: &[u8], label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label}_path_parent_missing"))?;
    fs::create_dir_all(parent).map_err(|_| format!("{label}_directory_create_failed"))?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| format!("{label}_temporary_create_failed"))?;
    file.write_all(bytes)
        .map_err(|_| format!("{label}_write_failed"))?;
    file.sync_all()
        .map_err(|_| format!("{label}_sync_failed"))?;
    fs::hard_link(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            format!("{label}_already_recorded")
        } else {
            format!("{label}_activate_failed")
        }
    })?;
    fs::remove_file(&temporary).map_err(|_| format!("{label}_temporary_remove_failed"))?;
    Ok(())
}

pub fn bootstrap_admission_from_proof(
    plan: &BuildRunnerBootstrapPlan,
    registry_path: &Path,
    registry_digest: String,
) -> cynic_protocol::BuildRunnerBootstrapAdmission {
    let allowed_actions = match plan.target_phase.as_str() {
        "check" => vec![
            "make:check".to_string(),
            "cargo:check".to_string(),
            "cargo:run".to_string(),
            "cargo:test".to_string(),
        ],
        "build" => vec!["make:build".to_string(), "cargo:build".to_string()],
        _ => Vec::new(),
    };
    cynic_protocol::BuildRunnerBootstrapAdmission {
        schema: cynic_protocol::BUILD_RUNNER_BOOTSTRAP_ADMISSION_SCHEMA.to_string(),
        status: "admitted".to_string(),
        authority_class: "MachineEnrollment".to_string(),
        node_id: plan.node_id.clone(),
        source_commit: plan.source_commit.clone(),
        registry_path: registry_path.display().to_string(),
        registry_digest,
        source_bundle_digest: Some(plan.source_bundle_digest.clone()),
        target_phase: plan.target_phase.clone(),
        allowed_actions,
        proofs: vec![
            "source_commit_read_back".to_string(),
            "build_resource_preflight_read_back".to_string(),
            "bootstrap_preflight_proved".to_string(),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildRunnerBootstrapObservation {
    pub workspace_clean: bool,
    pub registry_readable: bool,
    pub source_bundle_staged: bool,
    pub source_bundle_digest_matches: bool,
    pub registry_digest: Option<String>,
    pub resource_plan: Option<BuildCachePlan>,
    pub blockers: Vec<String>,
}

pub fn observe_build_runner_bootstrap(
    runner: &dyn ArgvRunner,
    workspace: &Path,
    registry: &Path,
    source_bundle: &Path,
    expected_bundle_digest: &str,
    node_id: &str,
    state_root: &Path,
    temporary_root: &Path,
) -> BuildRunnerBootstrapObservation {
    let mut blockers = Vec::new();
    let workspace_path = workspace.display().to_string();
    let git = runner.run(&["git", "-C", &workspace_path, "status", "--porcelain"]);
    let workspace_clean = workspace.join(".git").is_dir() && git.success() && git.stdout.is_empty();
    if !workspace_clean {
        blockers.push("build_runner_bootstrap_workspace_unclean_or_unobservable".to_string());
    }
    let registry_bytes = std::fs::read(registry).ok();
    let registry_readable = registry_bytes
        .as_ref()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some();
    let registry_digest = registry_file_digest(registry);
    if !registry_readable {
        blockers.push("build_runner_bootstrap_registry_unreadable".to_string());
    }
    let source_bundle_staged = source_bundle
        .strip_prefix(state_root)
        .ok()
        .is_some_and(|relative| relative.starts_with("bootstrap"))
        && std::fs::symlink_metadata(source_bundle)
            .ok()
            .is_some_and(|metadata| !metadata.file_type().is_symlink() && metadata.is_file());
    if !source_bundle_staged {
        blockers.push("build_runner_bootstrap_source_bundle_not_staged".to_string());
    }
    let source_bundle_digest_matches = source_bundle_staged
        .then(|| std::fs::read(source_bundle).ok())
        .flatten()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
        .as_deref()
        == Some(expected_bundle_digest);
    if !source_bundle_digest_matches {
        blockers.push("build_runner_bootstrap_source_bundle_digest_mismatch".to_string());
    }
    let resource_plan =
        match load_build_cache_desired_state(registry, node_id, state_root, temporary_root) {
            Ok(desired) => {
                let observed = LocalBuildCacheAdapter.observe(&desired);
                let plan = reconcile_build_cache(&desired, &observed);
                if plan.status != "satisfied" {
                    blockers.extend(plan.blockers.iter().cloned());
                    blockers
                        .push("build_runner_bootstrap_resource_preflight_unsatisfied".to_string());
                }
                Some(plan)
            }
            Err(blocker) => {
                blockers.push(blocker);
                None
            }
        };
    // Role observation: le nœud exécutant le bootstrap doit avoir le rôle
    // build_runner dans le registry. Empêche un agent de builder localement
    // sur un nœud kernel_primary (cortex) en forgeant un node_id erroné.
    let local_role = std::fs::read_to_string("/etc/cynic/node-role").ok();
    let local_role_matches = local_role
        .as_deref()
        .is_some_and(|r| r.trim() == "build_runner");
    if !local_role_matches {
        blockers.push("build_runner_bootstrap_role_mismatch".to_string());
    }
    BuildRunnerBootstrapObservation {
        workspace_clean,
        registry_readable,
        source_bundle_staged,
        source_bundle_digest_matches,
        registry_digest,
        resource_plan,
        blockers,
    }
}

pub fn plan_build_runner_bootstrap(
    request: &BuildRunnerBootstrapRequest,
) -> BuildRunnerBootstrapPlan {
    let mut blockers = Vec::new();
    if request.node_id.trim().is_empty() {
        blockers.push("build_runner_bootstrap_node_missing".to_string());
    }
    if request.source_commit.len() != 40
        || !request
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("build_runner_bootstrap_source_commit_invalid".to_string());
    }
    if !request.source_bundle_digest.starts_with("sha256:") {
        blockers.push("build_runner_bootstrap_source_bundle_digest_invalid".to_string());
    }
    if !matches!(
        request.target_phase.as_str(),
        "preflight" | "check" | "build"
    ) {
        blockers.push("build_runner_bootstrap_phase_forbidden".to_string());
    }
    BuildRunnerBootstrapPlan {
        schema: BUILD_RUNNER_BOOTSTRAP_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "planned"
        } else {
            "blocked"
        }
        .to_string(),
        node_id: request.node_id.clone(),
        source_commit: request.source_commit.clone(),
        source_bundle_digest: request.source_bundle_digest.clone(),
        target_phase: request.target_phase.clone(),
        mutates: request.target_phase != "preflight",
        blockers,
    }
}

pub fn reconcile_build_runner_bootstrap(
    request: &BuildRunnerBootstrapRequest,
    observation: &BuildRunnerBootstrapObservation,
) -> BuildRunnerBootstrapPlan {
    let mut plan = plan_build_runner_bootstrap(request);
    plan.blockers.extend(observation.blockers.iter().cloned());
    plan.blockers.sort();
    plan.blockers.dedup();
    if !plan.blockers.is_empty() {
        plan.status = "blocked".to_string();
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::ArgvOutput;
    #[test]
    fn forbids_activation_in_the_rust_bootstrap_boundary() {
        let plan = plan_build_runner_bootstrap(&BuildRunnerBootstrapRequest {
            node_id: "builder".to_string(),
            source_commit: "a".repeat(40),
            source_bundle_digest: format!("sha256:{}", "b".repeat(64)),
            target_phase: "activate".to_string(),
        });
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"build_runner_bootstrap_phase_forbidden".to_string())
        );
    }

    #[test]
    fn unreadable_bundle_blocks_an_otherwise_valid_plan() {
        let request = BuildRunnerBootstrapRequest {
            node_id: "builder".to_string(),
            source_commit: "a".repeat(40),
            source_bundle_digest: format!("sha256:{}", "b".repeat(64)),
            target_phase: "check".to_string(),
        };
        let plan = reconcile_build_runner_bootstrap(
            &request,
            &BuildRunnerBootstrapObservation {
                workspace_clean: true,
                registry_readable: true,
                source_bundle_staged: true,
                source_bundle_digest_matches: false,
                registry_digest: Some(format!("sha256:{}", "c".repeat(64))),
                resource_plan: None,
                blockers: vec!["build_runner_bootstrap_source_bundle_digest_mismatch".to_string()],
            },
        );
        assert_eq!(plan.status, "blocked");
    }

    #[test]
    fn build_plan_targets_only_the_bootstrap_host_artifact() {
        let workspace = tempfile::tempdir().expect("workspace");
        let plan = build_runner_bootstrap_build_plan(
            workspace.path(),
            &workspace.path().join("receipt.json"),
        );

        assert_eq!(plan.argv.last(), Some(&"build-host-exec".to_string()));
        assert_eq!(
            plan.artifact_path,
            workspace
                .path()
                .join(".cynic-target/debug/cynic-host-exec")
                .display()
                .to_string()
        );
    }

    #[test]
    fn registry_digest_binds_the_exact_file_bytes_consumed_by_the_gate() {
        let root = std::env::temp_dir().join(format!(
            "cynic-build-runner-registry-digest-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create temporary registry root");
        let registry = root.join("registry.json");
        let bytes = b"{\n  \"node\": \"builder\"\n}\n";
        std::fs::write(&registry, bytes).expect("write registry");

        assert_eq!(
            registry_file_digest(&registry),
            Some(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
        );

        std::fs::remove_dir_all(root).expect("remove temporary registry root");
    }

    struct PanicRunner;
    impl ArgvRunner for PanicRunner {
        fn run(&self, _: &[&str]) -> ArgvOutput {
            panic!("must not execute")
        }
    }

    #[test]
    fn apply_requires_explicit_consent_before_running_argv() {
        let execution = BuildRunnerBootstrapExecutionPlan {
            workspace: "/workspace".to_string(),
            argv: vec![vec!["git".to_string()]],
            rollback_intent: String::new(),
        };
        assert_eq!(
            apply_build_runner_bootstrap(
                &execution,
                &"a".repeat(40),
                &"b".repeat(64),
                false,
                &PanicRunner
            ),
            Err("build_runner_bootstrap_explicit_apply_required".to_string())
        );
    }

    #[test]
    fn fetch_failure_stops_before_checkout() {
        struct FailingRunner(std::sync::atomic::AtomicU8);
        impl ArgvRunner for FailingRunner {
            fn run(&self, _: &[&str]) -> ArgvOutput {
                self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                ArgvOutput {
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                }
            }
        }
        let runner = FailingRunner(std::sync::atomic::AtomicU8::new(0));
        let execution = BuildRunnerBootstrapExecutionPlan {
            workspace: "/workspace".to_string(),
            argv: vec![vec!["git".to_string()], vec!["git".to_string()]],
            rollback_intent: String::new(),
        };
        assert_eq!(
            apply_build_runner_bootstrap(
                &execution,
                &"a".repeat(40),
                &"b".repeat(64),
                true,
                &runner
            ),
            Err("build_runner_bootstrap_argv_failed".to_string())
        );
        assert_eq!(runner.0.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn divergent_head_refuses_proof() {
        struct DivergentRunner(std::sync::atomic::AtomicU8);
        impl ArgvRunner for DivergentRunner {
            fn run(&self, _: &[&str]) -> ArgvOutput {
                let call = self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: if call == 2 {
                        "b\n".to_string()
                    } else {
                        String::new()
                    },
                    stderr: String::new(),
                    spawn_error: None,
                }
            }
        }
        let execution = BuildRunnerBootstrapExecutionPlan {
            workspace: "/workspace".to_string(),
            argv: vec![vec!["git".to_string()], vec!["git".to_string()]],
            rollback_intent: String::new(),
        };
        assert_eq!(
            apply_build_runner_bootstrap(
                &execution,
                &"a".repeat(40),
                &"b".repeat(64),
                true,
                &DivergentRunner(std::sync::atomic::AtomicU8::new(0))
            ),
            Err("build_runner_bootstrap_commit_read_back_failed".to_string())
        );
    }

    #[test]
    fn source_registry_mismatch_refuses_receipt_proof() {
        struct CommitRunner;
        impl ArgvRunner for CommitRunner {
            fn run(&self, _: &[&str]) -> ArgvOutput {
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: format!("{}\n", "a".repeat(40)),
                    stderr: String::new(),
                    spawn_error: None,
                }
            }
        }
        let directory = tempfile::tempdir().expect("workspace");
        std::fs::create_dir_all(directory.path().join("infra")).expect("infra");
        std::fs::write(
            directory.path().join("infra/registry.json"),
            br#"{"nodes":[]}"#,
        )
        .expect("registry");
        let execution = BuildRunnerBootstrapExecutionPlan {
            workspace: directory.path().display().to_string(),
            argv: vec![vec!["git".to_string()], vec!["git".to_string()]],
            rollback_intent: String::new(),
        };
        assert_eq!(
            apply_build_runner_bootstrap(
                &execution,
                &"a".repeat(40),
                &format!("sha256:{}", "b".repeat(64)),
                true,
                &CommitRunner,
            ),
            Err("build_runner_bootstrap_source_registry_read_back_failed".to_string())
        );
    }
}

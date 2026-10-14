use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cynic_build::build_source_reference_from_workload;
use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::{LeaseGuard, digest_json};
use cynic_workload::{DurableAgentWorkloadRecord, consume_agent_workload_action};

use crate::{
    adapters::{
        ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner,
        http_runtime_artifact_transport::HttpRuntimeArtifactTransport,
        local_agent_workload_store::LocalAgentWorkloadStore,
        local_build_workload_grant_store::LocalBuildWorkloadGrantStore,
        local_build_workspace::LocalBuildWorkspace, registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::{
        agent_workload_store_port::AgentWorkloadStorePort,
        agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspaceLayoutPort},
        build_scheduler_port::{BuildExecutionRequest, BuildWorkspacePort, BuildWorkspaceReadBack},
        build_workload_grant_store_port::BuildWorkloadGrantStorePort,
    },
    usecases::build_workload_grant_uc::issue_signed_build_workload_grant,
};

use super::source_prepare::prepare_build_source;

pub(super) struct PreparedBuild {
    pub durable: DurableAgentWorkloadRecord,
    pub workspace: BuildWorkspaceReadBack,
    pub execution: BuildExecutionRequest,
    pub provenance: Vec<String>,
}

pub(super) async fn prepare(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    claimed: &DurableAgentWorkloadRecord,
    guard: &LeaseGuard,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<PreparedBuild, String> {
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_prepare".into());
    }
    let source = build_source_reference_from_workload(&claimed.record.envelope)
        .map_err(|blockers| blockers.join(","))?;
    if source.repository_id != scheduler.repository_id {
        return Err("build_scheduler_repository_id_mismatch".into());
    }
    let layout_port = RegistryAgentWorkspaceLayoutAdapter::new(
        &scheduler.registry_path,
        &control.agent_workload_state_root,
    );
    let layout = layout_port
        .resolve(
            &source.repository_id,
            &claimed.record.envelope.workload_id,
            &control.node_id,
        )
        .map_err(|error| error.code)?;
    let layout = apply_stable_workspace(
        layout,
        scheduler,
        &claimed.record.envelope.action.argv,
    )?;
    let _desired_working_directory = resolve_working_directory(
        claimed.record.envelope.action.working_directory.as_deref(),
        &layout.workspace_root,
    )?;

    let runner = LocalArgvRunner;
    let snapshot_root = control.agent_workload_state_root.join("source/snapshots");
    let artifact_transport = HttpRuntimeArtifactTransport::new(
        snapshot_root,
        control.runtime_artifact_max_bytes,
        Duration::from_secs(control.runtime_artifact_timeout_secs),
    )?;
    let (source_layout, mut source_provenance) = prepare_build_source(
        control,
        scheduler,
        &source,
        &claimed.record.envelope.workload_id,
        layout,
        &runner,
        &artifact_transport,
        cancellation,
    )
    .await?;
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_workspace".into());
    }
    let workspace = LocalBuildWorkspace::new(&runner, "git")
        .prepare(&source_layout, &source.revision)
        .map_err(|error| error.code)?;

    let signer = Ed25519BuildWorkloadGrantSigner::load_hex_file(&control.signing_key_path)
        .map_err(|_| "build_grant_machine_signer_unavailable".to_string())?;
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_action_reservation".into());
    }
    let (consumed, action_now_ms) =
        reserve_action(store, &claimed.record.envelope.workload_id, guard)?;
    let signed = issue_signed_build_workload_grant(
        &consumed,
        &control.node_id,
        action_now_ms / 1_000,
        &signer,
    )
    .map_err(|blockers| blockers.join(","))?;
    let grant_store = LocalBuildWorkloadGrantStore::new(
        control
            .agent_workload_state_root
            .join("agent-scheduler/build-grants"),
    );
    let grant = grant_store.persist(&signed).map_err(|error| error.code)?;
    let public_key_path = control
        .agent_workload_state_root
        .join("agent-scheduler/build-authority.pub");
    signer
        .persist_public_key(&public_key_path)
        .map_err(|_| "build_grant_public_key_persist_failed".to_string())?;
    let workload_record_path = store.durable_path(&consumed).map_err(|error| error.code)?;
    let verifier = std::env::current_exe()
        .map_err(|_| "build_scheduler_verifier_path_unobservable".to_string())?;
    let mut environment = BTreeMap::new();
    environment.insert(
        "CYNIC_BUILD_ADMISSION_GRANT".into(),
        grant.path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_WORKLOAD_RECORD".into(),
        workload_record_path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_AUTHORITY_PUBLIC_KEY".into(),
        public_key_path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_ADMISSION_VERIFIER".into(),
        verifier.to_string_lossy().into_owned(),
    );
    if let Some(root) = scheduler.shared_target_root.as_ref() {
        // Warm shared cargo cache: consecutive builds reuse <root>/<profile>
        // instead of recompiling cold in a throwaway directory.
        let target_dir = shared_target_dir_for(root, &claimed.record.envelope.action.argv);
        environment.insert(
            "CARGO_TARGET_DIR".into(),
            target_dir.to_string_lossy().into_owned(),
        );
    }
    source_provenance.extend([
        consumed.record_digest.clone(),
        digest_json(&signed).map_err(|_| "build_grant_digest_failed".to_string())?,
        grant.evidence_ref.clone(),
        source_layout.evidence_digest,
    ]);
    Ok(PreparedBuild {
        durable: consumed,
        execution: BuildExecutionRequest {
            argv: claimed.record.envelope.action.argv.clone(),
            working_directory: workspace.workspace_root.clone(),
            environment,
            timeout: Duration::from_secs(scheduler.command_timeout_secs),
            sandbox: sandbox_mode_for(
                scheduler,
                &workspace.workspace_root,
                &claimed.record.envelope.action.argv,
            )?,
        },
        workspace,
        provenance: source_provenance,
    })
}

/// Derive the cargo profile name from argv: `release` when `--release` is
/// present, `debug` otherwise. Pure.
fn profile_for_argv(argv: &[String]) -> &'static str {
    if argv.iter().any(|arg| arg == "--release") {
        "release"
    } else {
        "debug"
    }
}

/// Resolve the shared cargo target directory for a build: `<root>/<profile>`
/// with `release` when `--release` is in argv, `debug` otherwise. Pure.
fn shared_target_dir_for(root: &std::path::Path, argv: &[String]) -> std::path::PathBuf {
    root.join(profile_for_argv(argv))
}

/// Resolve the stable build workspace path for a profile:
/// `<root>/<profile>`. Pure: the same profile always maps to the same path,
/// so absolute-path fingerprints embedded in build artifacts survive across
/// builds.
fn stable_workspace_for(root: &std::path::Path, profile: &str) -> std::path::PathBuf {
    root.join(profile)
}

/// Pre-build clean for a stable workspace: remove a previous workspace at
/// the exact stable path so the next build starts from a clean tree, while
/// the path itself stays stable so fingerprints survive. A git-registered
/// worktree is removed through `git worktree remove` so the repository
/// worktree registry keeps no stale entry that would block re-creation at
/// the same path; anything else falls back to a plain recursive removal.
fn clean_stable_workspace(
    repository_root: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<(), String> {
    if !workspace.exists() {
        return Ok(());
    }
    let git_removed = std::process::Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["worktree", "remove", "--force"])
        .arg(workspace)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if git_removed {
        return Ok(());
    }
    fs::remove_dir_all(workspace)
        .map_err(|_| "build_workspace_stable_clean_failed".to_string())
}

/// Apply the stable-workspace override when the scheduler configures one:
/// the build workspace becomes `<stable_root>/<profile>` (profile derived
/// from argv exactly like `shared_target_root`) instead of the
/// registry-resolved per-workload path, and the previous tree at that path
/// is cleaned first. Without the setting the registry layout passes
/// through unchanged. The registry layout evidence digest keeps describing
/// the registry-declared authority inputs; the workspace path actually
/// used is recorded in the execution request and its read-back.
fn apply_stable_workspace(
    mut layout: AgentWorkspaceLayout,
    scheduler: &BuildSchedulerConfig,
    argv: &[String],
) -> Result<AgentWorkspaceLayout, String> {
    let root = match scheduler.stable_workspace_root.as_ref() {
        None => return Ok(layout),
        Some(root) => root,
    };
    let workspace_root = stable_workspace_for(root, profile_for_argv(argv));
    clean_stable_workspace(&layout.repository_root, &workspace_root)?;
    layout.workspace_root = workspace_root;
    Ok(layout)
}

/// Decide the execution isolation mode from scheduler config.
///
/// Returns `Bwrap` with a host-resolved spec when `sandbox_bwrap` is set,
/// `Off` otherwise. When sandboxing is demanded but the spec cannot be
/// resolved (no `PATH`, relative workspace), preparation fails closed with
/// `build_sandbox_spec_unresolvable` — never silently downgraded to `Off`.
///
/// When `shared_target_root` is set, the spec reuses `<root>/<profile>`
/// (profile from argv) so consecutive builds share warm artifacts instead
/// of recompiling cold in a throwaway directory.
fn sandbox_mode_for(
    scheduler: &BuildSchedulerConfig,
    workspace_root: &std::path::Path,
    argv: &[String],
) -> Result<crate::ports::build_scheduler_port::BuildSandboxMode, String> {
    use crate::ports::build_scheduler_port::BuildSandboxMode;
    use cynic_sandbox::cargo::CargoSandboxSpec;
    if !scheduler.sandbox_bwrap {
        return Ok(BuildSandboxMode::Off);
    }
    let target_dir = scheduler
        .shared_target_root
        .as_ref()
        .map(|root| shared_target_dir_for(root, argv));
    CargoSandboxSpec::resolve_from_host_env(workspace_root.to_path_buf(), target_dir)
        .map(BuildSandboxMode::Bwrap)
        .ok_or_else(|| "build_sandbox_spec_unresolvable".to_string())
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn reserve_action(
    store: &LocalAgentWorkloadStore,
    workload_id: &str,
    guard: &LeaseGuard,
) -> Result<(DurableAgentWorkloadRecord, u64), String> {
    for _ in 0..3 {
        let latest = store
            .read(workload_id)
            .map_err(|error| error.code)?
            .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
        let now_ms = current_unix_ms();
        let consumed = consume_agent_workload_action(&latest.record, guard, now_ms)
            .map_err(|error| error.code)?;
        match store.compare_and_swap(workload_id, Some(latest.revision), &consumed, now_ms) {
            Ok(durable) => return Ok((durable, now_ms)),
            Err(error) if error.code == "workload_store_revision_conflict" => continue,
            Err(error) => return Err(error.code),
        }
    }
    Err("build_scheduler_action_reservation_contended".into())
}

/// Resolve the execution working directory: an absent envelope directory
/// means the node assigns its registry-resolved layout root; an explicit
/// directory must equal it, otherwise the workload targets a layout this
/// node did not prepare.
pub(super) fn resolve_working_directory(
    desired: Option<&str>,
    layout_root: &PathBuf,
) -> Result<PathBuf, String> {
    match desired.filter(|value| !value.trim().is_empty()) {
        None => Ok(layout_root.clone()),
        Some(explicit) if PathBuf::from(explicit) == *layout_root => Ok(layout_root.clone()),
        Some(_) => Err("build_scheduler_working_directory_layout_mismatch".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_directory_assigns_layout_root() {
        let root = PathBuf::from("/var/lib/cynic/workspaces/w1");
        assert_eq!(resolve_working_directory(None, &root), Ok(root.clone()));
        assert_eq!(resolve_working_directory(Some("  "), &root), Ok(root));
    }

    #[test]
    fn explicit_matching_directory_passes_through() {
        let root = PathBuf::from("/var/lib/cynic/workspaces/w1");
        assert_eq!(
            resolve_working_directory(Some("/var/lib/cynic/workspaces/w1"), &root),
            Ok(root)
        );
    }

    #[test]
    fn foreign_directory_fails_closed() {
        let root = PathBuf::from("/var/lib/cynic/workspaces/w1");
        assert_eq!(
            resolve_working_directory(Some("/tmp/elsewhere"), &root),
            Err("build_scheduler_working_directory_layout_mismatch".to_string())
        );
    }

    #[test]
    fn shared_target_dir_maps_release_and_debug_profiles() {
        let root = std::path::Path::new("/var/lib/cynic/build-targets");
        let release_argv = vec![
            "cargo".to_string(),
            "build".to_string(),
            "--release".to_string(),
        ];
        let debug_argv = vec!["cargo".to_string(), "build".to_string()];
        assert_eq!(
            shared_target_dir_for(root, &release_argv),
            PathBuf::from("/var/lib/cynic/build-targets/release")
        );
        assert_eq!(
            shared_target_dir_for(root, &debug_argv),
            PathBuf::from("/var/lib/cynic/build-targets/debug")
        );
    }

    fn scheduler_config() -> crate::config::BuildSchedulerConfig {
        crate::config::BuildSchedulerConfig {
            registry_path: PathBuf::from("/tmp/x"),
            repository_id: String::new(),
            transport_uri: None,
            interval_secs: 0,
            claim_ttl_secs: 60,
            command_timeout_secs: 900,
            sandbox_bwrap: false,
            shared_target_root: None,
            stable_workspace_root: None,
            release_publication: None,
        }
    }

    fn registry_layout() -> AgentWorkspaceLayout {
        AgentWorkspaceLayout {
            repository_root: PathBuf::from("/var/lib/cynic/state/source/repositories/CYNIC"),
            workspace_root: PathBuf::from(
                "/var/lib/cynic/state/source/workspaces/cynic/workload-1",
            ),
            dedicated: true,
            required_principal: "cynic".to_string(),
            evidence_digest: "sha256:registry".to_string(),
            transport_uri: None,
        }
    }

    fn git(args: &[&str], dir: &std::path::Path) -> bool {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn stable_workspace_reuses_profile_dir() {
        let root = std::path::Path::new("/var/lib/cynic/build-workspaces");
        assert_eq!(
            stable_workspace_for(root, "debug"),
            stable_workspace_for(root, "debug"),
            "the same profile must map to the same stable workspace path"
        );
        assert_eq!(
            stable_workspace_for(root, "debug"),
            PathBuf::from("/var/lib/cynic/build-workspaces/debug")
        );
        assert_eq!(
            stable_workspace_for(root, "release"),
            PathBuf::from("/var/lib/cynic/build-workspaces/release")
        );
        assert_ne!(
            stable_workspace_for(root, "debug"),
            stable_workspace_for(root, "release"),
            "different profiles must not share a workspace path"
        );
    }

    #[test]
    fn stable_workspace_profile_follows_release_argv() {
        let release_argv = vec![
            "cargo".to_string(),
            "build".to_string(),
            "--release".to_string(),
        ];
        let debug_argv = vec!["cargo".to_string(), "build".to_string()];
        assert_eq!(profile_for_argv(&release_argv), "release");
        assert_eq!(profile_for_argv(&debug_argv), "debug");
    }

    #[test]
    fn stable_workspace_clean_removes_previous_workspace() {
        let case = tempfile::tempdir().unwrap();
        let workspace = case.path().join("stable").join("debug");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("stale-artifact.bin"), b"stale").unwrap();
        clean_stable_workspace(case.path(), &workspace).unwrap();
        assert!(!workspace.exists());
    }

    #[test]
    fn stable_workspace_clean_absent_path_is_noop() {
        let case = tempfile::tempdir().unwrap();
        clean_stable_workspace(case.path(), &case.path().join("absent")).unwrap();
    }

    #[test]
    fn stable_workspace_clean_removes_registered_git_worktree() {
        let case = tempfile::tempdir().unwrap();
        let repository = case.path().join("repository");
        let workspace = case.path().join("stable").join("debug");
        std::fs::create_dir_all(repository.join(".git")).unwrap();
        git(&["init", "-q"], &repository);
        git(&["config", "user.email", "t@t"], &repository);
        git(&["config", "user.name", "t"], &repository);
        std::fs::write(repository.join("f"), b"x").unwrap();
        git(&["add", "f"], &repository);
        git(&["commit", "-qm", "init"], &repository);
        std::fs::create_dir_all(workspace.parent().unwrap()).unwrap();
        assert!(
            git(
                &[
                    "worktree",
                    "add",
                    "--detach",
                    "-q",
                    workspace.to_str().unwrap(),
                    "HEAD"
                ],
                &repository
            ),
            "worktree creation must succeed"
        );
        assert!(workspace.exists());
        clean_stable_workspace(&repository, &workspace).unwrap();
        assert!(!workspace.exists());
        // The clean must leave no stale registration that blocks re-creation
        // at the same stable path.
        assert!(
            git(
                &[
                    "worktree",
                    "add",
                    "--detach",
                    "-q",
                    workspace.to_str().unwrap(),
                    "HEAD"
                ],
                &repository
            ),
            "re-add after clean must succeed"
        );
    }

    #[test]
    fn stable_workspace_allocation_overrides_registry_workspace() {
        let case = tempfile::tempdir().unwrap();
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        scheduler.stable_workspace_root = Some(case.path().join("stable"));
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let applied = apply_stable_workspace(layout, &scheduler, &argv).unwrap();
        assert_eq!(applied.workspace_root, case.path().join("stable/debug"));
        assert_eq!(
            applied.repository_root,
            PathBuf::from("/var/lib/cynic/state/source/repositories/CYNIC")
        );
    }

    #[test]
    fn stable_workspace_allocation_unset_preserves_registry_workspace() {
        let layout = registry_layout();
        let scheduler = scheduler_config();
        let applied = apply_stable_workspace(layout.clone(), &scheduler, &[]).unwrap();
        assert_eq!(applied, layout);
    }
}

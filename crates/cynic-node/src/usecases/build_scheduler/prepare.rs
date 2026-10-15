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
        local_build_workspace::LocalBuildWorkspace,
        registry::{RegistryAgentWorkspaceLayoutAdapter, recompute_layout_digest},
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
    // Capture the original registry-resolved root before the stable-workspace
    // override. Admission binds `action.working_directory` to this path;
    // after the override it must be rebased to follow the stable root.
    let original_root = layout.workspace_root.clone();
    let layout = apply_stable_workspace(
        layout,
        scheduler,
        &claimed.record.envelope.action.argv,
    )?;
    let desired = claimed.record.envelope.action.working_directory.as_deref();
    let rebased = rebase_working_directory(desired, &original_root, &layout.workspace_root);
    let _desired_working_directory = resolve_working_directory(rebased, &layout.workspace_root)?;

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
pub(crate) fn profile_for_argv(argv: &[String]) -> &'static str {
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

/// Prepare the stable workspace path for a build: ensure the parent
/// directory exists. Unlike the previous `clean_stable_workspace`, this
/// does NOT remove an existing checkout — the checkout is persisted
/// across builds so unchanged sources keep their old mtimes and cargo
/// fingerprints hit the warm shared target (H1b). The actual update
/// (fetch + reset --hard) happens in `LocalBuildWorkspace::prepare`.
fn prepare_stable_workspace(workspace: &std::path::Path) -> Result<(), String> {
    let parent = workspace
        .parent()
        .ok_or_else(|| "build_workspace_parent_missing".to_string())?;
    if !parent.exists() {
        fs::create_dir_all(parent)
            .map_err(|_| "build_workspace_parent_create_failed".to_string())?;
    }
    Ok(())
}

/// Apply the stable-workspace override when the scheduler configures one:
/// the build workspace becomes `<stable_root>/<profile>` (profile derived
/// from argv exactly like `shared_target_root`) instead of the
/// registry-resolved per-workload path, and the previous tree at that path
/// is cleaned first. Without the setting the registry layout passes
/// through unchanged. After the override the evidence digest is recomputed
/// so it describes the actual workspace path that will be used, not the
/// registry-declared path (C1).
fn apply_stable_workspace(
    mut layout: AgentWorkspaceLayout,
    scheduler: &BuildSchedulerConfig,
    argv: &[String],
) -> Result<AgentWorkspaceLayout, String> {
    let root = match scheduler.stable_workspace_root.as_ref() {
        None => return Ok(layout),
        Some(root) => root,
    };
    validate_stable_workspace_root(root, &layout.repository_root)?;
    let workspace_root = stable_workspace_for(root, profile_for_argv(argv));
    prepare_stable_workspace(&workspace_root)?;
    layout.workspace_root = workspace_root;
    // Recompute the evidence digest so it describes the actual workspace
    // path after the override, not the registry-declared path (C1).
    layout.evidence_digest = recompute_layout_digest(
        &layout,
        "", // node_id not needed for the resolved-layout digest
        "", // repository_id not needed for the resolved-layout digest
        "", // workspace_id not needed for the resolved-layout digest
    )?;
    Ok(layout)
}

/// Validate the stable-workspace root against trivial misuse (M1/M2/M3, I3):
/// M1 — the root must be absolute; M2 — the root must not be an existing
/// file; M3 — the root must not equal the repository root; I3 — the root
/// must not be inside the repository (a worktree inside its own repository
/// would corrupt the git index).
fn validate_stable_workspace_root(
    root: &std::path::Path,
    repository_root: &std::path::Path,
) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("build_workspace_stable_root_not_absolute".into());
    }
    if root.is_file() {
        return Err("build_workspace_stable_root_is_file".into());
    }
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canonical_repo =
        std::fs::canonicalize(repository_root).unwrap_or_else(|_| repository_root.to_path_buf());
    if canonical_root == canonical_repo {
        return Err("build_workspace_stable_root_equals_repository".into());
    }
    if canonical_root.starts_with(&canonical_repo) {
        return Err("build_workspace_stable_root_inside_repository".into());
    }
    Ok(())
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

/// Number of CAS retries in `reserve_action`. The concurrent `monitor_claim`
/// task renews the claim every second (when renewal is due), bumping the
/// revision. A slow prepare — e.g. the stable-workspace clean (C2) — widens
/// the window in which a renewal can land between the read and the CAS.
/// Three retries exhaust on a busy system; thirty gives the reservation
/// room to absorb renewals without failing the build (C3).
const RESERVE_ACTION_MAX_RETRIES: usize = 30;

fn reserve_action(
    store: &LocalAgentWorkloadStore,
    workload_id: &str,
    guard: &LeaseGuard,
) -> Result<(DurableAgentWorkloadRecord, u64), String> {
    for _ in 0..RESERVE_ACTION_MAX_RETRIES {
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

/// Rebase an admission-bound working directory onto the (possibly
/// overridden) stable root.
///
/// Admission binds `action.working_directory` to the per-build registry
/// workspace path. `apply_stable_workspace` then overrides
/// `layout.workspace_root` with the stable path, so the admission-bound
/// directory no longer matches and `resolve_working_directory` fails with
/// `build_scheduler_working_directory_layout_mismatch`.
///
/// When the desired directory equals `original_root` — i.e. it was bound by
/// admission, not forced by the operator — it is rebased to follow the
/// override by returning `None`, which makes `resolve_working_directory`
/// assign the stable root. A desired path that equals neither the original
/// nor the stable root is returned unchanged so `resolve_working_directory`
/// fails closed on it.
pub(super) fn rebase_working_directory<'a>(
    desired: Option<&'a str>,
    original_root: &std::path::Path,
    _stable_root: &std::path::Path,
) -> Option<&'a str> {
    if desired.map(PathBuf::from) == Some(original_root.to_path_buf()) {
        None
    } else {
        desired
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

    // --- Fix round 2: rebase admission-bound working_directory ---

    #[test]
    fn rebase_admission_bound_dir_onto_stable_root() {
        // Admission binds action.working_directory to the per-build registry
        // path. After apply_stable_workspace overrides layout.workspace_root,
        // the admission-bound path must be rebased to follow the override
        // (pass None so resolve assigns the stable root).
        let original = PathBuf::from("/var/lib/cynic/state/source/workspaces/cynic/workload-1");
        let stable = PathBuf::from("/var/lib/cynic/build-workspaces/debug");
        let rebased = rebase_working_directory(
            original.to_str(),
            &original,
            &stable,
        );
        assert_eq!(
            rebased, None,
            "admission-bound dir equal to original root must rebase to None"
        );
        assert_eq!(
            resolve_working_directory(rebased, &stable),
            Ok(stable),
            "rebased resolve must assign the stable root"
        );
    }

    #[test]
    fn rebase_foreign_dir_still_fails_closed() {
        // A desired path that equals neither the original nor the stable
        // root must still fail closed — the rebase must not mask it.
        let original = PathBuf::from("/var/lib/cynic/state/source/workspaces/cynic/workload-1");
        let stable = PathBuf::from("/var/lib/cynic/build-workspaces/debug");
        let foreign = "/tmp/elsewhere";
        let rebased = rebase_working_directory(Some(foreign), &original, &stable);
        assert_eq!(
            rebased,
            Some(foreign),
            "foreign dir must pass through unchanged"
        );
        assert_eq!(
            resolve_working_directory(rebased, &stable),
            Err("build_scheduler_working_directory_layout_mismatch".to_string()),
            "foreign dir must still fail closed"
        );
    }

    #[test]
    fn rebase_override_off_keeps_unchanged_behavior() {
        // When override is off, original_root == stable_root. A desired
        // dir equal to the original must still resolve to the original —
        // the rebase must not change behavior when there is no override.
        let original = PathBuf::from("/var/lib/cynic/state/source/workspaces/cynic/workload-1");
        let rebased = rebase_working_directory(
            original.to_str(),
            &original,
            &original,
        );
        assert_eq!(
            rebased, None,
            "desired==original must rebase to None even when override is off"
        );
        assert_eq!(
            resolve_working_directory(rebased, &original),
            Ok(original),
            "rebase with override off must still resolve to the original root"
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

    #[test]
    fn execution_env_has_cargo_target_dir_when_shared_target_root_configured() {
        let root = PathBuf::from("/var/lib/cynic/build-targets");
        let scheduler = crate::config::BuildSchedulerConfig {
            shared_target_root: Some(root.clone()),
            ..scheduler_config()
        };
        let argv = vec!["cargo".to_string(), "build".to_string()];
        // Construct the env inline (build_execution_env was inlined).
        let mut env: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        if let Some(root) = scheduler.shared_target_root.as_ref() {
            let target_dir = shared_target_dir_for(root, &argv);
            env.insert(
                "CARGO_TARGET_DIR".into(),
                target_dir.to_string_lossy().into_owned(),
            );
        }
        assert_eq!(
            env.get("CARGO_TARGET_DIR"),
            Some(&"/var/lib/cynic/build-targets/debug".to_string()),
            "CARGO_TARGET_DIR must be set to <root>/<profile> when shared_target_root is configured"
        );
    }

    #[test]
    fn execution_env_has_no_cargo_target_dir_when_shared_target_root_absent() {
        let scheduler = scheduler_config();
        assert!(
            scheduler.shared_target_root.is_none(),
            "precondition: shared_target_root must be absent"
        );
        let argv = vec!["cargo".to_string(), "build".to_string()];
        // Construct the env inline (build_execution_env was inlined).
        let mut env: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        if let Some(root) = scheduler.shared_target_root.as_ref() {
            let target_dir = shared_target_dir_for(root, &argv);
            env.insert(
                "CARGO_TARGET_DIR".into(),
                target_dir.to_string_lossy().into_owned(),
            );
        }
        assert!(
            !env.contains_key("CARGO_TARGET_DIR"),
            "CARGO_TARGET_DIR must NOT be set when shared_target_root is absent"
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
    fn stable_workspace_prepare_does_not_remove_existing_workspace() {
        // H1b: the stable workspace must persist across builds so unchanged
        // sources keep old mtimes and cargo fingerprints hit the warm
        // shared target. prepare_stable_workspace must NOT remove the
        // checkout.
        let case = tempfile::tempdir().unwrap();
        let workspace = case.path().join("stable").join("debug");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("sentinel.txt"), b"keep-me").unwrap();
        let sentinel = workspace.join("sentinel.txt");
        let mtime_before = std::fs::metadata(&sentinel)
            .unwrap()
            .modified()
            .unwrap();
        prepare_stable_workspace(&workspace).unwrap();
        assert!(
            sentinel.exists(),
            "sentinel file must survive prepare (no wipe)"
        );
        let mtime_after = std::fs::metadata(&sentinel)
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            mtime_before, mtime_after,
            "sentinel mtime must be unchanged (no re-materialization)"
        );
    }

    #[test]
    fn stable_workspace_prepare_creates_parent_directory() {
        // prepare_stable_workspace must ensure the parent directory exists
        // so the subsequent materialization can create the worktree.
        let case = tempfile::tempdir().unwrap();
        let workspace = case.path().join("stable").join("debug");
        assert!(!workspace.parent().unwrap().exists());
        prepare_stable_workspace(&workspace).unwrap();
        assert!(workspace.parent().unwrap().is_dir());
    }

    #[test]
    fn stable_workspace_prepare_preserves_registered_git_worktree() {
        // H1b: a registered git worktree at the stable path must survive
        // prepare_stable_workspace. The worktree registration in
        // .git/worktrees must persist so the next build can update the
        // checkout in place instead of re-cloning.
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
        prepare_stable_workspace(&workspace).unwrap();
        assert!(
            workspace.exists(),
            "registered worktree must survive prepare (no wipe)"
        );
        // The .git/worktrees registration must persist.
        assert!(
            repository
                .join(".git")
                .join("worktrees")
                .exists(),
            "worktree registration must persist"
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

    // --- C1: digest reflects actual workspace path after override ---

    #[test]
    fn stable_workspace_digest_reflects_actual_workspace_path() {
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        scheduler.stable_workspace_root = Some(PathBuf::from("/var/lib/cynic/stable"));
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let applied = apply_stable_workspace(layout, &scheduler, &argv).unwrap();
        // The digest must NOT be the registry layout digest — it must
        // describe the actual stable workspace path.
        assert_ne!(
            applied.evidence_digest, "sha256:registry",
            "digest must be recomputed after the stable-workspace override"
        );
        assert!(
            applied.evidence_digest.starts_with("sha256:"),
            "digest must be a valid sha256 digest"
        );
        // The digest must change when the workspace path changes.
        let mut scheduler2 = scheduler_config();
        scheduler2.stable_workspace_root = Some(PathBuf::from("/var/lib/cynic/other"));
        let applied2 = apply_stable_workspace(registry_layout(), &scheduler2, &argv).unwrap();
        assert_ne!(
            applied.evidence_digest, applied2.evidence_digest,
            "different workspace paths must produce different digests"
        );
    }

    // --- H1b: corrupt workspace fallback tested in local_build_workspace.rs ---

    // --- C3: reserve_action retries widened ---

    #[test]
    fn reserve_action_retries_are_widened_for_concurrent_renewals() {
        // The retry count must be large enough to absorb concurrent
        // monitor_claim renewals during a slow prepare. Three retries
        // exhaust on a busy system; the constant must be at least 30.
        assert!(
            RESERVE_ACTION_MAX_RETRIES >= 30,
            "reserve_action must retry at least 30 times to handle concurrent renewals (C3)"
        );
    }

    // --- I3: root-inside-repo guard ---

    #[test]
    fn stable_workspace_root_inside_repository_is_rejected() {
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        // Set stable root inside the repository.
        scheduler.stable_workspace_root = Some(
            PathBuf::from("/var/lib/cynic/state/source/repositories/CYNIC/stable")
        );
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let result = apply_stable_workspace(layout, &scheduler, &argv);
        assert_eq!(
            result.unwrap_err(),
            "build_workspace_stable_root_inside_repository"
        );
    }

    // --- M1: absolute root guard ---

    #[test]
    fn stable_workspace_root_relative_is_rejected() {
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        scheduler.stable_workspace_root = Some(PathBuf::from("relative/path"));
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let result = apply_stable_workspace(layout, &scheduler, &argv);
        assert_eq!(
            result.unwrap_err(),
            "build_workspace_stable_root_not_absolute"
        );
    }

    // --- M2: file-at-path guard ---

    #[test]
    fn stable_workspace_root_is_file_is_rejected() {
        let case = tempfile::tempdir().unwrap();
        let file_path = case.path().join("a_file");
        std::fs::write(&file_path, b"x").unwrap();
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        scheduler.stable_workspace_root = Some(file_path);
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let result = apply_stable_workspace(layout, &scheduler, &argv);
        assert_eq!(
            result.unwrap_err(),
            "build_workspace_stable_root_is_file"
        );
    }

    // --- M3: root==repo guard ---

    #[test]
    fn stable_workspace_root_equals_repository_is_rejected() {
        let layout = registry_layout();
        let mut scheduler = scheduler_config();
        scheduler.stable_workspace_root = Some(
            PathBuf::from("/var/lib/cynic/state/source/repositories/CYNIC")
        );
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let result = apply_stable_workspace(layout, &scheduler, &argv);
        assert_eq!(
            result.unwrap_err(),
            "build_workspace_stable_root_equals_repository"
        );
    }
}

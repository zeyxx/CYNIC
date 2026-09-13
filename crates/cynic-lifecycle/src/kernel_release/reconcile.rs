use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use super::proof::{append_event, sha256_file};
use super::types::{
    KERNEL_RELEASE_EVENT_SCHEMA, KERNEL_RELEASE_REPORT_SCHEMA, KERNEL_RELEASE_SPEC_SCHEMA,
    KernelIdentityEvidence, KernelReleaseAction, KernelReleaseEvent, KernelReleaseReport,
    KernelReleaseSpec,
};
use crate::{ArgvRunner, MutationClass, load_state, phase_policy};

type ReconcileError = (Option<Box<KernelReleaseSpec>>, String);

pub fn reconcile_kernel_release(
    lifecycle_state_path: &Path,
    spec_path: &Path,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> KernelReleaseReport {
    match reconcile(lifecycle_state_path, spec_path, yes, runner) {
        Ok(report) => report,
        Err((spec, error)) => failure_report(spec.as_deref(), error),
    }
}

fn reconcile(
    lifecycle_state_path: &Path,
    spec_path: &Path,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> Result<KernelReleaseReport, ReconcileError> {
    let raw = fs::read_to_string(spec_path)
        .map_err(|error| (None, format!("read {}: {error}", spec_path.display())))?;
    let spec: KernelReleaseSpec = serde_json::from_str(&raw)
        .map_err(|error| (None, format!("parse {}: {error}", spec_path.display())))?;
    validate_spec(&spec).map_err(|error| (Some(Box::new(spec.clone())), error))?;

    let lifecycle = load_state(lifecycle_state_path);
    let phase = lifecycle.current_phase.as_str();
    if spec.phase != phase {
        return Err((
            Some(Box::new(spec)),
            format!("release spec phase does not match lifecycle phase {phase}"),
        ));
    }
    if !phase_policy(&lifecycle.current_phase)
        .allowed_mutations
        .contains(&MutationClass::KernelRuntime)
    {
        return Err((
            Some(Box::new(spec)),
            format!("kernel runtime mutation is not allowed in {phase}"),
        ));
    }

    let artifact_path = PathBuf::from(&spec.artifact.path);
    let expected_sha = spec.artifact.sha256.clone();
    let actual_sha =
        sha256_file(&artifact_path).map_err(|error| (Some(Box::new(spec.clone())), error))?;
    if actual_sha != expected_sha {
        return Err((
            Some(Box::new(spec)),
            format!("artifact sha256 mismatch: expected={expected_sha} observed={actual_sha}"),
        ));
    }

    let observed_before = observe_identity(runner, &spec.identity_url).ok();
    let release_binary = PathBuf::from(&spec.install_root)
        .join("releases")
        .join(&spec.artifact.sha256)
        .join(&spec.artifact.executable_name);
    let current_link = PathBuf::from(&spec.install_root).join("current");
    let proof_path = PathBuf::from(&spec.install_root).join("proof/kernel-release.jsonl");
    let previous_target = fs::read_link(&current_link)
        .ok()
        .map(|path| path.display().to_string());
    let actions = planned_actions(&spec, &release_binary, &current_link);

    if !yes {
        return Ok(KernelReleaseReport {
            schema: KERNEL_RELEASE_REPORT_SCHEMA,
            status: "planned".to_string(),
            artifact_id: spec.artifact.id.clone(),
            service_id: spec.service_id.clone(),
            node_id: spec.node_id,
            lifecycle_phase: phase.to_string(),
            mutates: false,
            desired_sha256: spec.artifact.sha256,
            desired_version: spec.artifact.version,
            desired_git_sha: spec.artifact.git_sha,
            previous_target,
            activated_target: Some(release_binary.display().to_string()),
            observed_before,
            observed_after: None,
            rolled_back: false,
            actions,
            proof_event: None,
            error: None,
        });
    }

    stage_artifact(&artifact_path, &release_binary)
        .map_err(|error| (Some(Box::new(spec.clone())), error))?;
    activate_link(&release_binary, &current_link)
        .map_err(|error| (Some(Box::new(spec.clone())), error))?;

    let restart = run_restart(runner, &spec.restart_argv);
    let observed_after = restart
        .and_then(|_| observe_identity(runner, &spec.identity_url))
        .and_then(|identity| verify_identity(&spec, identity));

    match observed_after {
        Ok(identity) => {
            let event = event_for(
                &spec,
                phase,
                "deployment_succeeded",
                "ok",
                previous_target.clone(),
                Some(release_binary.display().to_string()),
                Some(&identity),
            );
            let proof_event = match append_event(&proof_path, event) {
                Ok(event) => event,
                Err(proof_error) => {
                    let rollback_result = rollback(
                        &current_link,
                        previous_target.as_deref(),
                        runner,
                        &spec.restart_argv,
                    );
                    let rolled_back = rollback_result.is_ok();
                    let error = match rollback_result {
                        Ok(()) => format!(
                            "release proof append failed and activation was rolled back: {proof_error}"
                        ),
                        Err(rollback_error) => format!(
                            "release proof append failed: {proof_error}; rollback failed: {rollback_error}"
                        ),
                    };
                    return Ok(KernelReleaseReport {
                        schema: KERNEL_RELEASE_REPORT_SCHEMA,
                        status: if rolled_back {
                            "rolled_back"
                        } else {
                            "critical"
                        }
                        .to_string(),
                        artifact_id: spec.artifact.id.clone(),
                        service_id: spec.service_id.clone(),
                        node_id: spec.node_id,
                        lifecycle_phase: phase.to_string(),
                        mutates: true,
                        desired_sha256: spec.artifact.sha256,
                        desired_version: spec.artifact.version,
                        desired_git_sha: spec.artifact.git_sha,
                        previous_target,
                        activated_target: Some(release_binary.display().to_string()),
                        observed_before,
                        observed_after: Some(identity),
                        rolled_back,
                        actions,
                        proof_event: None,
                        error: Some(error),
                    });
                }
            };
            Ok(KernelReleaseReport {
                schema: KERNEL_RELEASE_REPORT_SCHEMA,
                status: "ok".to_string(),
                artifact_id: spec.artifact.id.clone(),
                service_id: spec.service_id.clone(),
                node_id: spec.node_id,
                lifecycle_phase: phase.to_string(),
                mutates: true,
                desired_sha256: spec.artifact.sha256,
                desired_version: spec.artifact.version,
                desired_git_sha: spec.artifact.git_sha,
                previous_target,
                activated_target: Some(release_binary.display().to_string()),
                observed_before,
                observed_after: Some(identity),
                rolled_back: false,
                actions,
                proof_event: Some(proof_event),
                error: None,
            })
        }
        Err(deploy_error) => {
            let rollback_result = rollback(
                &current_link,
                previous_target.as_deref(),
                runner,
                &spec.restart_argv,
            );
            let rolled_back = rollback_result.is_ok();
            let combined_error = match rollback_result {
                Ok(()) => deploy_error,
                Err(rollback_error) => format!("{deploy_error}; rollback failed: {rollback_error}"),
            };
            let event = event_for(
                &spec,
                phase,
                if rolled_back {
                    "deployment_rolled_back"
                } else {
                    "deployment_failed"
                },
                if rolled_back {
                    "rolled_back"
                } else {
                    "critical"
                },
                previous_target.clone(),
                Some(release_binary.display().to_string()),
                None,
            );
            let proof_event = append_event(&proof_path, event).ok();
            Ok(KernelReleaseReport {
                schema: KERNEL_RELEASE_REPORT_SCHEMA,
                status: if rolled_back {
                    "rolled_back"
                } else {
                    "critical"
                }
                .to_string(),
                artifact_id: spec.artifact.id.clone(),
                service_id: spec.service_id.clone(),
                node_id: spec.node_id,
                lifecycle_phase: phase.to_string(),
                mutates: true,
                desired_sha256: spec.artifact.sha256,
                desired_version: spec.artifact.version,
                desired_git_sha: spec.artifact.git_sha,
                previous_target,
                activated_target: Some(release_binary.display().to_string()),
                observed_before,
                observed_after: None,
                rolled_back,
                actions,
                proof_event,
                error: Some(combined_error),
            })
        }
    }
}

fn validate_spec(spec: &KernelReleaseSpec) -> Result<(), String> {
    if spec.schema != KERNEL_RELEASE_SPEC_SCHEMA {
        return Err(format!("unsupported release spec schema {}", spec.schema));
    }
    if spec.node_id.is_empty() || spec.node_id.chars().any(char::is_control) {
        return Err("node_id must be non-empty and contain no control characters".to_string());
    }
    if spec.artifact.id.is_empty() || spec.service_id.is_empty() || spec.identity_schema.is_empty()
    {
        return Err("artifact id, service id, and identity schema must be non-empty".to_string());
    }
    let executable = Path::new(&spec.artifact.executable_name);
    if spec.artifact.executable_name.is_empty()
        || executable.components().count() != 1
        || executable.is_absolute()
    {
        return Err("artifact.executable_name must be one relative path component".to_string());
    }
    if spec.artifact.sha256.len() != 64
        || !spec
            .artifact
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("artifact.sha256 must be a 64-character hexadecimal digest".to_string());
    }
    if spec.restart_argv.is_empty() || spec.restart_argv.iter().any(|arg| arg.is_empty()) {
        return Err("restart_argv must contain a program and non-empty arguments".to_string());
    }
    if spec.identity_url.contains(char::is_whitespace) || !spec.identity_url.ends_with("/identity")
    {
        return Err(
            "identity_url must be an absolute /identity endpoint without whitespace".to_string(),
        );
    }
    Ok(())
}

fn planned_actions(
    spec: &KernelReleaseSpec,
    release: &Path,
    current: &Path,
) -> Vec<KernelReleaseAction> {
    vec![
        KernelReleaseAction {
            phase: "preflight",
            description: "verify desired artifact SHA-256 and lifecycle phase".to_string(),
            mutates: false,
            requires_privilege: false,
            proof: spec.artifact.sha256.clone(),
        },
        KernelReleaseAction {
            phase: "stage",
            description: format!("materialize immutable artifact {}", release.display()),
            mutates: true,
            requires_privilege: spec.requires_privilege,
            proof: "staged artifact hash equals desired hash".to_string(),
        },
        KernelReleaseAction {
            phase: "activate",
            description: format!(
                "atomically replace {} and execute restart argv",
                current.display()
            ),
            mutates: true,
            requires_privilege: spec.requires_privilege,
            proof: "atomic link target + restart exit status".to_string(),
        },
        KernelReleaseAction {
            phase: "observe",
            description: format!(
                "verify {} returns desired service/node/version/git",
                spec.identity_url
            ),
            mutates: false,
            requires_privilege: false,
            proof: spec.identity_schema.clone(),
        },
        KernelReleaseAction {
            phase: "proof",
            description: "append and fsync hash-chained release event".to_string(),
            mutates: true,
            requires_privilege: spec.requires_privilege,
            proof: KERNEL_RELEASE_EVENT_SCHEMA.to_string(),
        },
    ]
}

fn stage_artifact(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or("release destination has no parent")?;
    fs::create_dir_all(parent).map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
    if destination.exists() {
        return Ok(());
    }
    let temporary = parent.join(format!(".cynic-kernel.tmp-{}", std::process::id()));
    fs::copy(source, &temporary)
        .map_err(|error| format!("stage {}: {error}", temporary.display()))?;
    let mode = fs::metadata(source)
        .map_err(|error| error.to_string())?
        .permissions()
        .mode();
    fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("chmod {}: {error}", temporary.display()))?;
    fs::File::open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("sync {}: {error}", temporary.display()))?;
    fs::rename(&temporary, destination).map_err(|error| {
        format!(
            "activate immutable artifact {}: {error}",
            destination.display()
        )
    })
}

fn activate_link(release: &Path, current: &Path) -> Result<(), String> {
    let parent = current.parent().ok_or("current link has no parent")?;
    fs::create_dir_all(parent).map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
    let next = parent.join(format!(".current.next-{}", std::process::id()));
    let _ = fs::remove_file(&next);
    symlink(release, &next).map_err(|error| format!("symlink {}: {error}", next.display()))?;
    fs::rename(&next, current).map_err(|error| format!("swap {}: {error}", current.display()))
}

fn run_restart(runner: &dyn ArgvRunner, argv: &[String]) -> Result<(), String> {
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let output = runner.run(&refs);
    if output.success() {
        Ok(())
    } else {
        Err(output
            .spawn_error
            .unwrap_or_else(|| format!("restart exited {:?}: {}", output.exit_code, output.stderr)))
    }
}

fn observe_identity(runner: &dyn ArgvRunner, url: &str) -> Result<KernelIdentityEvidence, String> {
    let argv = [
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        url,
    ];
    let output = runner.run(&argv);
    if !output.success() {
        return Err(output.spawn_error.unwrap_or_else(|| {
            format!(
                "identity observation exited {:?}: {}",
                output.exit_code, output.stderr
            )
        }));
    }
    serde_json::from_str(&output.stdout)
        .map_err(|error| format!("invalid kernel identity: {error}"))
}

fn verify_identity(
    spec: &KernelReleaseSpec,
    identity: KernelIdentityEvidence,
) -> Result<KernelIdentityEvidence, String> {
    if identity.schema != spec.identity_schema
        || identity.service != spec.service_id
        || identity.node_id != spec.node_id
        || identity.build.version != spec.artifact.version
        || identity.build.git_sha != spec.artifact.git_sha
    {
        return Err(format!(
            "activated kernel identity mismatch: node={} version={} git_sha={}",
            identity.node_id, identity.build.version, identity.build.git_sha
        ));
    }
    Ok(identity)
}

fn rollback(
    current: &Path,
    previous: Option<&str>,
    runner: &dyn ArgvRunner,
    restart: &[String],
) -> Result<(), String> {
    let previous = previous.ok_or("no previous release target is available")?;
    activate_link(Path::new(previous), current)?;
    run_restart(runner, restart)
}

fn event_for(
    spec: &KernelReleaseSpec,
    phase: &str,
    event: &str,
    status: &str,
    previous_target: Option<String>,
    activated_target: Option<String>,
    identity: Option<&KernelIdentityEvidence>,
) -> KernelReleaseEvent {
    KernelReleaseEvent {
        schema: KERNEL_RELEASE_EVENT_SCHEMA.to_string(),
        seq: 0,
        timestamp_unix_ms: super::super::time::now_ms(),
        artifact_id: spec.artifact.id.clone(),
        service_id: spec.service_id.clone(),
        node_id: spec.node_id.clone(),
        lifecycle_phase: phase.to_string(),
        event: event.to_string(),
        desired_sha256: spec.artifact.sha256.clone(),
        previous_target,
        activated_target,
        observed_version: identity.map(|value| value.build.version.clone()),
        observed_git_sha: identity.map(|value| value.build.git_sha.clone()),
        status: status.to_string(),
        prev_hash: String::new(),
        hash: String::new(),
    }
}

fn failure_report(spec: Option<&KernelReleaseSpec>, error: String) -> KernelReleaseReport {
    KernelReleaseReport {
        schema: KERNEL_RELEASE_REPORT_SCHEMA,
        status: "blocked".to_string(),
        artifact_id: spec
            .map(|value| value.artifact.id.clone())
            .unwrap_or_default(),
        service_id: spec
            .map(|value| value.service_id.clone())
            .unwrap_or_default(),
        node_id: spec.map(|value| value.node_id.clone()).unwrap_or_default(),
        lifecycle_phase: spec.map(|value| value.phase.clone()).unwrap_or_default(),
        mutates: false,
        desired_sha256: spec
            .map(|value| value.artifact.sha256.clone())
            .unwrap_or_default(),
        desired_version: spec
            .map(|value| value.artifact.version.clone())
            .unwrap_or_default(),
        desired_git_sha: spec
            .map(|value| value.artifact.git_sha.clone())
            .unwrap_or_default(),
        previous_target: None,
        activated_target: None,
        observed_before: None,
        observed_after: None,
        rolled_back: false,
        actions: Vec::new(),
        proof_event: None,
        error: Some(error),
    }
}

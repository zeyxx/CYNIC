use std::{fs, path::Path};

use cynic_protocol::{
    RuntimeSeedAction, RuntimeSeedRequest, SecurityEvidenceState, reconcile_runtime_seed,
};

use cynic_lifecycle::ArgvRunner;

use super::{
    RuntimeSeedApplyReport,
    filesystem::{
        activate_link, ensure_root, extract_archive, inspect_archive, persist_release_marker,
        rollback_staging, verify_apply_preconditions, verify_bundle_marker,
    },
    observe_runtime_seed,
    registry::{
        install_current_registry, install_registry, rollback_registry, verify_staged_registry,
    },
    signature::RuntimeSeedSignatureVerifier,
};

pub fn apply_runtime_seed<R: ArgvRunner>(
    request: &RuntimeSeedRequest,
    verifier: &dyn RuntimeSeedSignatureVerifier,
    runner: &R,
) -> RuntimeSeedApplyReport {
    let before = observe_runtime_seed(request, verifier);
    let mut report = RuntimeSeedApplyReport {
        schema: "cynic.host.runtime_seed.apply_report.v3",
        status: "blocked".to_string(),
        before,
        after: None,
        activated: false,
        registry_activated: false,
        rolled_back: false,
        error: None,
    };
    let admission = reconcile_runtime_seed(request, &report.before);
    if admission.status != "planned" {
        report.error = Some(format!(
            "runtime_seed_apply_not_admitted:{}",
            admission
                .blockers
                .into_iter()
                .chain(admission.unknowns)
                .collect::<Vec<_>>()
                .join(",")
        ));
        return report;
    }
    if let Err(error) = verify_apply_preconditions(request, verifier) {
        report.error = Some(error);
        return report;
    }
    let root = Path::new(&request.install_root);
    if let Err(error) = ensure_root(root) {
        report.error = Some(error);
        return report;
    }
    if admission
        .actions
        .contains(&RuntimeSeedAction::RestoreInstalledRegistry)
    {
        return repair_installed_registry(request, verifier, runner, root, report);
    }

    let staging = root.join(&request.staged_slot);
    if staging.exists() {
        report.error = Some("runtime_seed_staged_slot_exists".to_string());
        return report;
    }
    if let Err(error) = inspect_archive(request, runner)
        .and_then(|()| extract_archive(request, &staging, runner))
        .and_then(|()| verify_bundle_marker(request, &staging))
        .and_then(|()| verify_staged_registry(request, &staging))
        .and_then(|()| persist_release_marker(request, &staging))
        .and_then(|()| verify_apply_preconditions(request, verifier))
    {
        rollback_staging(&staging);
        report.error = Some(error);
        return report;
    }

    let refreshed = observe_runtime_seed(request, verifier);
    if reconcile_runtime_seed(request, &refreshed).status != "planned" {
        rollback_staging(&staging);
        report.error = Some("runtime_seed_admission_changed_before_activation".to_string());
        report.after = Some(refreshed);
        return report;
    }

    let current_link = root.join(&request.current_link);
    let previous_link = root.join(&request.previous_link);
    let old_target = fs::read_link(&current_link).ok();
    if let Err(error) = activate_link(root, &current_link, Path::new(&request.staged_slot)) {
        rollback_staging(&staging);
        report.error = Some(error);
        return report;
    }
    report.activated = true;
    let registry_rollback = match install_registry(request, &staging) {
        Ok(rollback) => {
            report.registry_activated = true;
            rollback
        }
        Err(error) => {
            report.rolled_back = true;
            report.status = "rolled_back".to_string();
            report.error = Some(error);
            if let Err(rollback_error) =
                rollback_current_link(root, &current_link, old_target.as_deref())
            {
                report.status = "unknown".to_string();
                append_error(&mut report, rollback_error);
            } else {
                rollback_staging(&staging);
            }
            report.after = Some(observe_runtime_seed(request, verifier));
            return report;
        }
    };
    if let Err(error) = run_postflight(request, runner) {
        report.rolled_back = true;
        report.status = "rolled_back".to_string();
        report.error = Some(error);
        if let Err(error) = rollback_registry(&registry_rollback) {
            report.status = "unknown".to_string();
            append_error(&mut report, error);
        }
        if let Err(error) = rollback_current_link(root, &current_link, old_target.as_deref()) {
            report.status = "unknown".to_string();
            append_error(&mut report, error);
        }
        report.after = Some(observe_runtime_seed(request, verifier));
        return report;
    }
    if let Some(old_target) = old_target
        && let Err(error) = activate_link(root, &previous_link, &old_target)
    {
        report.status = "unknown".to_string();
        report.error = Some(error);
        report.after = Some(observe_runtime_seed(request, verifier));
        return report;
    }
    prove_after(request, verifier, report)
}

fn repair_installed_registry<R: ArgvRunner>(
    request: &RuntimeSeedRequest,
    verifier: &dyn RuntimeSeedSignatureVerifier,
    runner: &R,
    root: &Path,
    mut report: RuntimeSeedApplyReport,
) -> RuntimeSeedApplyReport {
    let rollback = match install_current_registry(request, root) {
        Ok(rollback) => {
            report.registry_activated = true;
            rollback
        }
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    if let Err(error) = run_postflight(request, runner) {
        report.rolled_back = true;
        report.status = "rolled_back".to_string();
        report.error = Some(error);
        if let Err(error) = rollback_registry(&rollback) {
            report.status = "unknown".to_string();
            append_error(&mut report, error);
        }
        report.after = Some(observe_runtime_seed(request, verifier));
        return report;
    }
    prove_after(request, verifier, report)
}

fn prove_after(
    request: &RuntimeSeedRequest,
    verifier: &dyn RuntimeSeedSignatureVerifier,
    mut report: RuntimeSeedApplyReport,
) -> RuntimeSeedApplyReport {
    let mut after = observe_runtime_seed(request, verifier);
    let converged = after.current_release_id.as_deref() == Some(&request.release_id)
        && after.current_release_epoch == Some(request.release_epoch)
        && after.current_source_commit.as_deref() == Some(&request.source_commit)
        && after.current_build_proof_digest.as_deref() == Some(&request.build_proof_digest)
        && after.current_basket_digest.as_deref() == Some(&request.basket_digest)
        && after.current_registry_digest.as_deref() == Some(&request.registry_digest)
        && after.installed_registry_digest.as_deref() == Some(&request.registry_digest);
    after.postflight = if converged {
        SecurityEvidenceState::Proven
    } else {
        SecurityEvidenceState::Absent
    };
    report.status = if converged { "proved" } else { "unknown" }.to_string();
    report.after = Some(after);
    report
}

fn run_postflight<R: ArgvRunner>(request: &RuntimeSeedRequest, runner: &R) -> Result<(), String> {
    let argv = request
        .postflight_argv
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let postflight = runner.run(&argv);
    if postflight.success() {
        Ok(())
    } else {
        Err(format!(
            "runtime_seed_postflight_failed:{}",
            postflight.stderr.trim()
        ))
    }
}

fn rollback_current_link(
    root: &Path,
    current_link: &Path,
    old_target: Option<&Path>,
) -> Result<(), String> {
    if let Some(target) = old_target {
        activate_link(root, current_link, target)
            .map_err(|error| format!("runtime_seed_rollback_failed:{error}"))
    } else {
        fs::remove_file(current_link)
            .map_err(|error| format!("runtime_seed_rollback_remove_failed:{error}"))
    }
}

fn append_error(report: &mut RuntimeSeedApplyReport, error: String) {
    report.error = Some(format!(
        "{};{error}",
        report.error.as_deref().unwrap_or_default()
    ));
}

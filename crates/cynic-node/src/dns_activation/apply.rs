use std::path::Path;

use cynic_host::update_lifecycle::ArgvRunner;

use super::decision::{activation_argv, plan_dns_activation};
use super::filesystem::{
    atomic_install, capture_backups, missing_parent_directories, rollback_files_and_directories,
};
use super::observe::observe_dns_activation;
use super::rollback::{rollback_after_activation, rollback_matches};
use super::types::{
    DnsActivationApplyReport, DnsActivationObservation, DnsActivationPlan, DnsActivationSpec,
};

pub(crate) fn apply_dns_activation(
    spec: &DnsActivationSpec,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> DnsActivationApplyReport {
    let before = observe_dns_activation(spec, runner);
    let plan = plan_dns_activation(spec, &before);
    if plan.status == "converged" {
        return report(
            plan,
            before,
            Some(observe_dns_activation(spec, runner)),
            None,
            false,
            yes,
            Vec::new(),
            false,
            Vec::new(),
            "converged",
        );
    }
    let mut blockers = plan.blockers.clone();
    blockers.extend(plan.unknowns.clone());
    if plan.status != "planned" {
        return report(
            plan,
            before,
            None,
            None,
            false,
            yes,
            Vec::new(),
            false,
            blockers,
            "blocked",
        );
    }
    if !yes {
        blockers.push("explicit_yes_required".to_string());
    }
    if unsafe { libc::geteuid() } != 0 {
        blockers.push("dns_activation_root_authority_required".to_string());
    }
    if !blockers.is_empty() {
        return report(
            plan,
            before,
            None,
            None,
            false,
            yes,
            Vec::new(),
            false,
            blockers,
            "blocked",
        );
    }
    let backups = match capture_backups(spec) {
        Ok(backups) => backups,
        Err(error) => {
            return report(
                plan,
                before,
                None,
                None,
                false,
                yes,
                Vec::new(),
                false,
                vec![error],
                "blocked",
            );
        }
    };
    let created_directories = match missing_parent_directories(spec) {
        Ok(paths) => paths,
        Err(error) => {
            return report(
                plan,
                before,
                None,
                None,
                false,
                yes,
                Vec::new(),
                false,
                vec![error],
                "blocked",
            );
        }
    };
    for file in &spec.files {
        if let Err(error) = atomic_install(
            Path::new(&file.source),
            Path::new(&file.target),
            file.mode,
            &file.sha256,
        ) {
            let rollback = rollback_files_and_directories(&backups, &created_directories);
            let rollback_after = observe_dns_activation(spec, runner);
            let rolled_back = rollback.is_ok() && rollback_matches(&before, &rollback_after);
            let mut failure = vec![error];
            append_rollback_failure(&mut failure, rollback, rolled_back);
            return report(
                plan,
                before,
                None,
                Some(rollback_after),
                true,
                yes,
                Vec::new(),
                rolled_back,
                failure,
                rollback_status(rolled_back),
            );
        }
    }
    let mut executed_argv = Vec::new();
    for argv in activation_argv(spec) {
        let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
        executed_argv.push(argv);
        if !output.success() {
            let rollback = rollback_after_activation(
                spec,
                &before,
                &backups,
                &created_directories,
                runner,
                &mut executed_argv,
            );
            let rollback_after = observe_dns_activation(spec, runner);
            let rolled_back = rollback.is_ok() && rollback_matches(&before, &rollback_after);
            let mut failure = vec![format!(
                "dns_activation_command_failed:{}",
                output.stderr.trim()
            )];
            append_rollback_failure(&mut failure, rollback, rolled_back);
            return report(
                plan,
                before,
                None,
                Some(rollback_after),
                true,
                yes,
                executed_argv,
                rolled_back,
                failure,
                rollback_status(rolled_back),
            );
        }
    }
    let after = observe_dns_activation(spec, runner);
    let read_back = plan_dns_activation(spec, &after);
    if read_back.status == "converged" {
        return report(
            plan,
            before,
            Some(after),
            None,
            true,
            yes,
            executed_argv,
            false,
            Vec::new(),
            "proved",
        );
    }

    let mut residual = read_back.blockers;
    residual.extend(read_back.unknowns);
    residual.push("dns_activation_read_back_not_converged".to_string());
    let rollback = rollback_after_activation(
        spec,
        &before,
        &backups,
        &created_directories,
        runner,
        &mut executed_argv,
    );
    let rollback_after = observe_dns_activation(spec, runner);
    let rolled_back = rollback.is_ok() && rollback_matches(&before, &rollback_after);
    append_rollback_failure(&mut residual, rollback, rolled_back);
    report(
        plan,
        before,
        Some(after),
        Some(rollback_after),
        true,
        yes,
        executed_argv,
        rolled_back,
        residual,
        rollback_status(rolled_back),
    )
}

fn append_rollback_failure(
    blockers: &mut Vec<String>,
    rollback: Result<(), String>,
    rolled_back: bool,
) {
    if let Err(error) = rollback {
        blockers.push(error);
    }
    if !rolled_back {
        blockers.push("dns_activation_rollback_read_back_not_converged".to_string());
    }
}

fn rollback_status(rolled_back: bool) -> &'static str {
    if rolled_back {
        "rolled_back"
    } else {
        "unknown"
    }
}

#[allow(clippy::too_many_arguments)]
fn report(
    plan: DnsActivationPlan,
    before: DnsActivationObservation,
    after: Option<DnsActivationObservation>,
    rollback_after: Option<DnsActivationObservation>,
    mutates: bool,
    requested_apply: bool,
    executed_argv: Vec<Vec<String>>,
    rolled_back: bool,
    blockers: Vec<String>,
    status: &'static str,
) -> DnsActivationApplyReport {
    DnsActivationApplyReport {
        schema: "cynic.dns_activation.apply_report.v1",
        status,
        mutates,
        requested_apply,
        plan,
        before,
        after,
        rollback_after,
        executed_argv,
        rolled_back,
        blockers,
        rollback_intent: "restore previous files, created directories, service enablement, and service activity; then read back or retain residual unknowns",
    }
}

use std::path::PathBuf;

use cynic_lifecycle::ArgvRunner;

use super::filesystem::{Backup, rollback_files_and_directories};
use super::types::{DnsActivationObservation, DnsActivationSpec};

pub(super) fn rollback_after_activation(
    spec: &DnsActivationSpec,
    before: &DnsActivationObservation,
    backups: &[Backup],
    created_directories: &[PathBuf],
    runner: &dyn ArgvRunner,
    executed_argv: &mut Vec<Vec<String>>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if before.service_enabled == Some(false) {
        retain_command_error(
            &mut errors,
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "disable".to_string(),
                spec.service_unit.clone(),
            ],
        );
    }
    if before.service_active == Some(false) {
        retain_command_error(
            &mut errors,
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "stop".to_string(),
                spec.service_unit.clone(),
            ],
        );
    }
    if let Err(error) = rollback_files_and_directories(backups, created_directories) {
        errors.push(error);
    }
    retain_command_error(
        &mut errors,
        runner,
        executed_argv,
        vec![
            spec.service_manager_bin.clone(),
            "daemon-reload".to_string(),
        ],
    );
    if before.service_enabled == Some(true) {
        retain_command_error(
            &mut errors,
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "enable".to_string(),
                spec.service_unit.clone(),
            ],
        );
    }
    if before.service_active == Some(true) {
        retain_command_error(
            &mut errors,
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "start".to_string(),
                spec.service_unit.clone(),
            ],
        );
    }
    retain_command_error(
        &mut errors,
        runner,
        executed_argv,
        vec![
            spec.service_manager_bin.clone(),
            "restart".to_string(),
            spec.resolver_unit.clone(),
        ],
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(";"))
    }
}

pub(super) fn rollback_matches(
    before: &DnsActivationObservation,
    after: &DnsActivationObservation,
) -> bool {
    after.unknowns.is_empty()
        && before.files == after.files
        && before.service_active == after.service_active
        && before.service_enabled == after.service_enabled
        && before.probe_resolves == after.probe_resolves
}

fn retain_command_error(
    errors: &mut Vec<String>,
    runner: &dyn ArgvRunner,
    executed_argv: &mut Vec<Vec<String>>,
    argv: Vec<String>,
) {
    let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed_argv.push(argv);
    if !output.success() {
        errors.push(format!(
            "dns_activation_rollback_command_failed:{}",
            output.stderr.trim()
        ));
    }
}

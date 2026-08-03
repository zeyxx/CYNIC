use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use cynic_host::update_lifecycle::ArgvRunner;
use sha2::Digest;

use super::decision::{activation_argv, plan_dns_activation};
use super::observe::observe_dns_activation;
use super::types::{DnsActivationApplyReport, DnsActivationSpec};

struct Backup {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    mode: Option<u32>,
}

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
            if let Err(rollback_error) = rollback {
                failure.push(rollback_error);
            }
            if !rolled_back {
                failure.push("dns_activation_rollback_read_back_not_converged".to_string());
            }
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
                if rolled_back {
                    "rolled_back"
                } else {
                    "unknown"
                },
            );
        }
    }
    let mut executed_argv = Vec::new();
    for argv in activation_argv(spec) {
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = runner.run(&refs);
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
            if let Err(rollback_error) = rollback {
                failure.push(rollback_error);
            }
            if !rolled_back {
                failure.push("dns_activation_rollback_read_back_not_converged".to_string());
            }
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
                if rolled_back {
                    "rolled_back"
                } else {
                    "unknown"
                },
            );
        }
    }
    let after = observe_dns_activation(spec, runner);
    let read_back = plan_dns_activation(spec, &after);
    if read_back.status == "converged" {
        report(
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
        )
    } else {
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
        if let Err(rollback_error) = rollback {
            residual.push(rollback_error);
        }
        if !rolled_back {
            residual.push("dns_activation_rollback_read_back_not_converged".to_string());
        }
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
            if rolled_back {
                "rolled_back"
            } else {
                "unknown"
            },
        )
    }
}

fn rollback_after_activation(
    spec: &DnsActivationSpec,
    before: &super::types::DnsActivationObservation,
    backups: &[Backup],
    created_directories: &[PathBuf],
    runner: &dyn ArgvRunner,
    executed_argv: &mut Vec<Vec<String>>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if before.service_enabled == Some(false) {
        if let Err(error) = run_rollback_command(
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "disable".to_string(),
                spec.service_unit.clone(),
            ],
        ) {
            errors.push(error);
        }
    }
    if before.service_active == Some(false) {
        if let Err(error) = run_rollback_command(
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "stop".to_string(),
                spec.service_unit.clone(),
            ],
        ) {
            errors.push(error);
        }
    }
    if let Err(error) = rollback_files_and_directories(backups, created_directories) {
        errors.push(error);
    }
    if let Err(error) = run_rollback_command(
        runner,
        executed_argv,
        vec![
            spec.service_manager_bin.clone(),
            "daemon-reload".to_string(),
        ],
    ) {
        errors.push(error);
    }
    if before.service_enabled == Some(true) {
        if let Err(error) = run_rollback_command(
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "enable".to_string(),
                spec.service_unit.clone(),
            ],
        ) {
            errors.push(error);
        }
    }
    if before.service_active == Some(true) {
        if let Err(error) = run_rollback_command(
            runner,
            executed_argv,
            vec![
                spec.service_manager_bin.clone(),
                "start".to_string(),
                spec.service_unit.clone(),
            ],
        ) {
            errors.push(error);
        }
    }
    if let Err(error) = run_rollback_command(
        runner,
        executed_argv,
        vec![
            spec.service_manager_bin.clone(),
            "restart".to_string(),
            spec.resolver_unit.clone(),
        ],
    ) {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(";"))
    }
}

fn run_rollback_command(
    runner: &dyn ArgvRunner,
    executed_argv: &mut Vec<Vec<String>>,
    argv: Vec<String>,
) -> Result<(), String> {
    let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed_argv.push(argv);
    if output.success() {
        Ok(())
    } else {
        Err(format!(
            "dns_activation_rollback_command_failed:{}",
            output.stderr.trim()
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn report(
    plan: super::types::DnsActivationPlan,
    before: super::types::DnsActivationObservation,
    after: Option<super::types::DnsActivationObservation>,
    rollback_after: Option<super::types::DnsActivationObservation>,
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

fn rollback_matches(
    before: &super::types::DnsActivationObservation,
    after: &super::types::DnsActivationObservation,
) -> bool {
    after.unknowns.is_empty()
        && before.files == after.files
        && before.service_active == after.service_active
        && before.service_enabled == after.service_enabled
        && before.probe_resolves == after.probe_resolves
}

fn capture_backups(spec: &DnsActivationSpec) -> Result<Vec<Backup>, String> {
    spec.files
        .iter()
        .map(|file| {
            let path = PathBuf::from(&file.target);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    Ok(Backup {
                        path: path.clone(),
                        bytes: Some(fs::read(&path).map_err(|error| {
                            format!("dns_activation_backup_read_failed:{error}")
                        })?),
                        mode: Some(metadata.permissions().mode() & 0o7777),
                    })
                }
                Ok(_) => Err(format!("dns_activation_target_unsafe:{}", path.display())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Backup {
                    path,
                    bytes: None,
                    mode: None,
                }),
                Err(error) => Err(format!("dns_activation_target_unobservable:{error}")),
            }
        })
        .collect()
}

fn restore_backups(backups: &[Backup]) -> Result<(), String> {
    for backup in backups.iter().rev() {
        match &backup.bytes {
            Some(bytes) => atomic_write(&backup.path, bytes, backup.mode.unwrap_or(0o644))?,
            None if backup.path.exists() => fs::remove_file(&backup.path)
                .map_err(|error| format!("dns_activation_rollback_remove_failed:{error}"))?,
            None => {}
        }
    }
    Ok(())
}

fn rollback_files_and_directories(
    backups: &[Backup],
    created_directories: &[PathBuf],
) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(error) = restore_backups(backups) {
        errors.push(error);
    }
    for directory in created_directories.iter().rev() {
        if let Err(error) = fs::remove_dir(directory) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!(
                    "dns_activation_rollback_directory_remove_failed:{}:{error}",
                    directory.display()
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(";"))
    }
}

fn missing_parent_directories(spec: &DnsActivationSpec) -> Result<Vec<PathBuf>, String> {
    let mut missing = BTreeSet::new();
    for file in &spec.files {
        let parent = Path::new(&file.target)
            .parent()
            .ok_or_else(|| "dns_activation_target_parent_missing".to_string())?;
        if !parent.is_absolute() {
            return Err("dns_activation_target_parent_not_absolute".to_string());
        }
        let mut current = PathBuf::from("/");
        for component in parent.components() {
            let name = match component {
                std::path::Component::RootDir => continue,
                std::path::Component::Normal(name) => name,
                _ => return Err("dns_activation_target_parent_unsafe".to_string()),
            };
            current.push(name);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                Ok(_) => {
                    return Err(format!(
                        "dns_activation_parent_unsafe:{}",
                        current.display()
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing.insert(current.clone());
                }
                Err(error) => {
                    return Err(format!("dns_activation_parent_unobservable:{error}"));
                }
            }
        }
    }
    let mut paths = missing.into_iter().collect::<Vec<_>>();
    paths.sort_by_key(|path| path.components().count());
    Ok(paths)
}

fn atomic_install(
    source: &Path,
    target: &Path,
    mode: u32,
    expected_sha256: &str,
) -> Result<(), String> {
    let bytes =
        fs::read(source).map_err(|error| format!("dns_activation_source_read_failed:{error}"))?;
    let observed_sha256 = format!("sha256:{}", hex::encode(sha2::Sha256::digest(&bytes)));
    if observed_sha256 != expected_sha256 {
        return Err(format!(
            "dns_activation_source_changed_after_plan:{}",
            source.display()
        ));
    }
    atomic_write(target, &bytes, mode)
}

fn atomic_write(target: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "dns_activation_target_parent_missing".to_string())?;
    ensure_directory_chain(parent)?;
    if fs::symlink_metadata(target).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(format!(
            "dns_activation_target_symlink_forbidden:{}",
            target.display()
        ));
    }
    let temporary = parent.join(format!(".cynic-dns-next-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        std::os::unix::fs::chown(&temporary, Some(0), Some(0))?;
        fs::rename(&temporary, target)?;
        fs::File::open(parent)?.sync_all()?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("dns_activation_atomic_write_failed:{error}"))
}

fn ensure_directory_chain(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("dns_activation_target_parent_not_absolute".to_string());
    }
    let mut current = PathBuf::from("/");
    for component in path.components() {
        let name = match component {
            std::path::Component::RootDir => continue,
            std::path::Component::Normal(name) => name,
            _ => return Err("dns_activation_target_parent_unsafe".to_string()),
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(format!(
                    "dns_activation_parent_unsafe:{}",
                    current.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|error| format!("dns_activation_parent_create_failed:{error}"))?;
                fs::set_permissions(&current, fs::Permissions::from_mode(0o755))
                    .map_err(|error| format!("dns_activation_parent_mode_failed:{error}"))?;
                std::os::unix::fs::chown(&current, Some(0), Some(0))
                    .map_err(|error| format!("dns_activation_parent_owner_failed:{error}"))?;
            }
            Err(error) => return Err(format!("dns_activation_parent_unobservable:{error}")),
        }
    }
    Ok(())
}

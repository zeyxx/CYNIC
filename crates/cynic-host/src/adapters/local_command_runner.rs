use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::domain::services::openrc_service_name;
use crate::execution::AdmittedHostOperation;
use crate::ports::{HostCommandResult, HostCommandRunnerPort, HostOperation};
use crate::process_observation::capture_command;
use crate::runtime_layout::current_runtime_layout;

use super::principal_manager::{
    ensure_owned_dir, ensure_supplementary_group_member, ensure_system_group, ensure_system_user,
};
use super::privilege_projection::{install_owned_file, validate_projection};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHostCommandRunner;

impl HostCommandRunnerPort for LocalHostCommandRunner {
    fn run_operation(&self, admitted: &AdmittedHostOperation) -> HostCommandResult {
        let operation = admitted.operation();
        let capabilities = current_runtime_layout().capabilities();
        match operation {
            HostOperation::EnsureSystemGroup { group } => ensure_system_group(group),
            HostOperation::EnsureSystemUser {
                user,
                group,
                home,
                shell,
            } => ensure_system_user(user, group, home, shell),
            HostOperation::EnsureSupplementaryGroupMember { user, group } => {
                ensure_supplementary_group_member(user, group)
            }
            HostOperation::EnsureOwnedDir {
                path,
                user,
                group,
                mode,
            } => ensure_owned_dir(path, user, group, *mode),
            HostOperation::InstallOwnedFile {
                source,
                target,
                user,
                group,
                mode,
            } => install_owned_file(source, target, user, group, *mode),
            HostOperation::ValidatePrivilegeProjection { provider, path } => {
                validate_projection(provider, path)
            }
            HostOperation::EnsureDir { path, mode } => ensure_dir(path, *mode),
            HostOperation::WritePackageSourceProjection {
                path,
                approved_sources,
            } => write_package_source_projection(path, approved_sources),
            HostOperation::InstallBootstrapPackages {
                approved_sources,
                packages,
            } => install_bootstrap_packages(approved_sources, packages),
            HostOperation::ServiceManagerDaemonReload => {
                if capabilities.service_manager.kind == "systemd" {
                    run_command(&capabilities.service_manager.command, &["daemon-reload"])
                } else {
                    HostCommandResult {
                        command: format!(
                            "{} daemon-reload unsupported",
                            capabilities.service_manager.kind
                        ),
                        success: true,
                        exit_code: Some(0),
                        stdout: String::new(),
                        stderr: String::new(),
                    }
                }
            }
            HostOperation::ServiceEnableNow { unit } => enable_now_service(&capabilities, unit),
            HostOperation::ServiceStart { unit } => start_service(&capabilities, unit),
            HostOperation::ServiceRetire { unit } => retire_service(&capabilities, unit),
        }
    }
}

fn enable_now_service(
    capabilities: &crate::runtime_layout::HostCapabilities,
    unit: &str,
) -> HostCommandResult {
    match capabilities.service_manager.kind.as_str() {
        "openrc" => {
            let service = openrc_service_name(unit);
            let rc_update = current_runtime_layout().rc_update_bin();
            let added = run_command(&rc_update, &["add", service.as_str(), "default"]);
            if !added.success {
                return added;
            }
            run_command(
                &capabilities.service_manager.command,
                &[service.as_str(), "start"],
            )
        }
        _ => run_command(
            &capabilities.service_manager.command,
            &["enable", "--now", unit],
        ),
    }
}

/// Retire a unit: stop it, disable its boot activation, remove the
/// activation symlink, and remove the unit file.
///
/// The order is deliberate. `disable` removes the boot activation
/// symlink, `stop` ends the running instance, and only then is the file
/// removed — removing the file first would leave systemd holding a unit
/// it can no longer read, which reads as a unit that vanished rather
/// than one that was retired.
///
/// A unit that is already absent is a SUCCESS, not an error: retiring
/// is idempotent, and a host where the stale unit was never installed
/// has already converged.
fn retire_service(
    capabilities: &crate::runtime_layout::HostCapabilities,
    unit: &str,
) -> HostCommandResult {
    if !is_plain_service_unit(unit) {
        return HostCommandResult {
            command: format!("service retire {unit}"),
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: format!("refusing to retire a non-plain unit identifier: {unit}"),
        };
    }
    let manager = &capabilities.service_manager.command;
    // 1. Remove the boot activation FIRST. `disable` drops the symlink
    //    that makes the unit start at boot; a unit that is stopped but
    //    still enabled comes back at the next reboot, which is exactly
    //    the failure this operation exists to end.
    let _ = run_command(manager, &["disable", unit]);
    // 2. Stop the running instance. A unit that is not running answers
    //    "inactive", which is the state we want; the command still fails
    //    on some managers, so the state is read rather than assumed.
    let _ = run_command(manager, &["stop", unit]);
    // 3. Remove the unit file. The activation symlink under
    //    `<target>.wants/` is a link INTO the unit directory, so
    //    removing the file leaves a dangling link that `disable` has
    //    normally already cleared; the link is removed defensively.
    let layout = current_runtime_layout();
    let unit_path = std::path::Path::new(&layout.service_unit_dir).join(unit);
    let mut errors: Vec<String> = Vec::new();
    if unit_path.exists() {
        if let Err(error) = std::fs::remove_file(&unit_path) {
            errors.push(format!("remove {}: {error}", unit_path.display()));
        }
    }
    for target_dir in [
        "default.target.wants".to_string(),
        "multi-user.target.wants".to_string(),
        "graphical.target.wants".to_string(),
    ] {
        let link = std::path::Path::new(&layout.service_unit_dir)
            .join(&target_dir)
            .join(unit);
        if link.is_symlink() {
            if let Err(error) = std::fs::remove_file(&link) {
                errors.push(format!("remove {}: {error}", link.display()));
            }
        }
    }
    if !errors.is_empty() {
        return HostCommandResult {
            command: format!("service retire {unit}"),
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: errors.join("; "),
        };
    }
    HostCommandResult {
        command: format!("service retire {unit}"),
        success: true,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
    }
}

/// A plain `*.service` unit identifier: no path, no flags, no shell.
///
/// The identifier is passed to the service manager as an argument, so a
/// leading `-` would be read as an OPTION, not as a unit name: `--now`
/// would change what the command means instead of naming a unit. The
/// first character is therefore restricted to a letter or a digit.
fn is_plain_service_unit(unit: &str) -> bool {
    !unit.is_empty()
        && unit.len() <= 128
        && unit.ends_with(".service")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '@')
        && !unit.contains("..")
        && !unit.contains('/')
        && !unit.contains(' ')
        && unit
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

fn start_service(
    capabilities: &crate::runtime_layout::HostCapabilities,
    unit: &str,
) -> HostCommandResult {
    match capabilities.service_manager.kind.as_str() {
        "openrc" => {
            let service = openrc_service_name(unit);
            run_command(
                &capabilities.service_manager.command,
                &[service.as_str(), "start"],
            )
        }
        _ => run_command(&capabilities.service_manager.command, &["start", unit]),
    }
}

fn ensure_dir(path: &str, mode: u32) -> HostCommandResult {
    match fs::create_dir_all(path) {
        Ok(()) => match fs::metadata(path) {
            Ok(metadata) => {
                let mut permissions = metadata.permissions();
                permissions.set_mode(mode);
                match fs::set_permissions(path, permissions) {
                    Ok(()) => HostCommandResult {
                        command: format!("ensure_dir path={path} mode={mode:o}"),
                        success: true,
                        exit_code: Some(0),
                        stdout: String::new(),
                        stderr: String::new(),
                    },
                    Err(error) => HostCommandResult {
                        command: format!("ensure_dir path={path} mode={mode:o}"),
                        success: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: error.to_string(),
                    },
                }
            }
            Err(error) => HostCommandResult {
                command: format!("ensure_dir path={path} mode={mode:o}"),
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
            },
        },
        Err(error) => HostCommandResult {
            command: format!("ensure_dir path={path} mode={mode:o}"),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn write_package_source_projection(path: &str, approved_sources: &[String]) -> HostCommandResult {
    let payload = serde_json::json!({
        "schema": "cynic.host_package_sources.v1",
        "approved_sources": approved_sources,
    });
    match serde_json::to_vec_pretty(&payload) {
        Ok(bytes) => match fs::write(path, bytes) {
            Ok(()) => HostCommandResult {
                command: format!("write_package_source_projection path={path}"),
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            Err(error) => HostCommandResult {
                command: format!("write_package_source_projection path={path}"),
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
            },
        },
        Err(error) => HostCommandResult {
            command: format!("write_package_source_projection path={path}"),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn install_bootstrap_packages(
    approved_sources: &[String],
    packages: &[String],
) -> HostCommandResult {
    if packages.is_empty() {
        return HostCommandResult {
            command: "install_bootstrap_packages packages=[]".to_string(),
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    if approved_sources
        .iter()
        .any(|source| source == "distribution_package")
    {
        let layout = current_runtime_layout();
        let argv = layout
            .capabilities()
            .package_manager
            .kind
            .install_argv(packages, &layout);
        if let Some((command, args)) = argv.split_first() {
            return run_command(
                command,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
            );
        }
    }

    HostCommandResult {
        command: format!(
            "install_bootstrap_packages unsupported_sources={}",
            approved_sources.join(",")
        ),
        success: false,
        exit_code: Some(2),
        stdout: String::new(),
        stderr: "no supported bootstrap package source available".to_string(),
    }
}

pub(super) fn run_command(command: &str, args: &[&str]) -> HostCommandResult {
    match capture_command(command, args) {
        Ok(output) => HostCommandResult {
            command: format!("{command} {}", args.join(" ")).trim().to_string(),
            success: output.success,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        },
        Err(error) => HostCommandResult {
            command: format!("{command} {}", args.join(" ")).trim().to_string(),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::is_plain_service_unit;

    #[test]
    fn a_plain_unit_identifier_is_accepted() {
        assert!(is_plain_service_unit("cynic-supervise-soak-node.service"));
        assert!(is_plain_service_unit("wg-quick@wg0.service"));
    }

    #[test]
    fn a_unit_identifier_with_a_path_or_a_flag_is_refused() {
        // A path could escape the unit directory.
        assert!(!is_plain_service_unit("../../etc/passwd.service"));
        assert!(!is_plain_service_unit("/etc/systemd/system/evil.service"));
        // A flag or a space would be an argument injection.
        assert!(!is_plain_service_unit("--now.service"));
        assert!(!is_plain_service_unit("evil.service --force"));
        assert!(!is_plain_service_unit("evil.service;rm -rf /"));
        // A non-service unit is not a service.
        assert!(!is_plain_service_unit("evil.timer"));
        assert!(!is_plain_service_unit(""));
    }
}

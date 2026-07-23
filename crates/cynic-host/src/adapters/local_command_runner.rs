use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::domain::services::openrc_service_name;
use crate::execution::AdmittedHostOperation;
use crate::ports::{HostCommandResult, HostCommandRunnerPort, HostOperation};
use crate::process_observation::capture_command;
use crate::runtime_layout::current_runtime_layout;

use super::principal_manager::{ensure_owned_dir, ensure_system_group, ensure_system_user};
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

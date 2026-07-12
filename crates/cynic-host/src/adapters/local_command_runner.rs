use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::ports::{HostCommandResult, HostCommandRunnerPort, HostOperation};
use crate::process_observation::capture_command;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHostCommandRunner;

impl HostCommandRunnerPort for LocalHostCommandRunner {
    fn run_operation(&self, operation: &HostOperation) -> HostCommandResult {
        let capabilities = current_runtime_layout().capabilities();
        match operation {
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
                run_command(&capabilities.service_manager.command, &["daemon-reload"])
            }
            HostOperation::ServiceEnableNow { unit } => run_command(
                &capabilities.service_manager.command,
                &["enable", "--now", unit.as_str()],
            ),
            HostOperation::ServiceStart { unit } => run_command(
                &capabilities.service_manager.command,
                &["start", unit.as_str()],
            ),
        }
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
        let package_installer = current_runtime_layout()
            .capabilities()
            .package_manager
            .installer;
        let mut args = vec!["install", "-y"];
        for package in packages {
            args.push(package.as_str());
        }
        return run_command(&package_installer, &args);
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

fn run_command(command: &str, args: &[&str]) -> HostCommandResult {
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

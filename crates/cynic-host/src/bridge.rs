use crate::bounded_helper::{BoundedHelperInstallRuntime, BoundedHelperProjection};
use crate::command_surface::{
    HostCommandSurface, argv_command, install_dir_command, install_file_command,
    validate_sudoers_command,
};
use crate::install_contract::{
    InstallMode, InstallOwnership, InstallPathPolicy, MODE_0440, MODE_0755,
};
use crate::process_observation::capture_argv;
use crate::runtime_layout::current_runtime_layout;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub mod bundle;
pub mod installer;
pub mod status;
pub mod types;

use bundle::{BridgeBundleContext, bundle_manifest_for_context, write_bridge_bundle_with_context};
use installer::bridge_install_report_from_commands;
use status::bridge_status_for_projection;
pub use types::{
    HostExecBridgeActivationPlan, HostExecBridgeBundleManifest, HostExecBridgeInstallReport,
    HostExecBridgeRootlessSmokeReport, HostExecBridgeStatus,
};

const HOST_EXEC_HELPER_FILENAME: &str = "cynic-host-exec";
const HOST_EXEC_SUDOERS_FILENAME: &str = "cynic-host-exec.sudoers";
const HOST_EXEC_SUDOERS_SUBJECT: &str = "cynic";

pub fn write_bridge_bundle(
    output_dir: impl AsRef<Path>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    let helper_source = helper_binary_source()?;
    write_bridge_bundle_from_binary(output_dir, helper_source)
}

pub fn write_bridge_bundle_from_binary(
    output_dir: impl AsRef<Path>,
    helper_source: impl AsRef<Path>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    write_bridge_bundle_with_context(
        output_dir,
        helper_source.as_ref(),
        bridge_bundle_context(helper_projection(), helper_install_runtime()),
        None,
    )
}

fn write_bridge_bundle_with_runtime(
    output_dir: impl AsRef<Path>,
    helper_source: &Path,
    projection: BoundedHelperProjection,
    helper_runtime: BoundedHelperInstallRuntime,
    prepare_directories: Option<Vec<HostCommandSurface>>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    write_bridge_bundle_with_context(
        output_dir,
        helper_source,
        bridge_bundle_context(projection, helper_runtime),
        prepare_directories,
    )
}

pub fn bridge_activation_plan(bundle_dir: impl AsRef<Path>) -> HostExecBridgeActivationPlan {
    let bundle_dir = bundle_dir.as_ref();
    let manifest = bundle_manifest("<current_executable>", &bundle_dir.to_string_lossy());
    let bundle_dir_str = bundle_dir.to_string_lossy().to_string();
    let runtime_layout = current_runtime_layout();
    let helper_runtime = helper_install_runtime();
    let sudo_bin = runtime_layout.sudo_bin();

    HostExecBridgeActivationPlan {
        schema: "cynic.host_exec_bridge.activation_plan.v1",
        mutates: false,
        bundle_dir: bundle_dir_str.clone(),
        bundle_expected_complete: bundle_complete(bundle_dir),
        bundle_files: manifest.bundle_files.clone(),
        preflight: vec![
            argv_command(
                "build",
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-q".to_string(),
                    "--manifest-path".to_string(),
                    "crates/cynic-host/Cargo.toml".to_string(),
                    "--bin".to_string(),
                    "cynic-host-exec".to_string(),
                ],
            ),
            argv_command(
                "bundle",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-q".to_string(),
                    "--manifest-path".to_string(),
                    "crates/cynic-host/Cargo.toml".to_string(),
                    "--bin".to_string(),
                    "cynic-host-exec".to_string(),
                    "--".to_string(),
                    "bridge-bundle".to_string(),
                    "--output-dir".to_string(),
                    bundle_dir_str.clone(),
                ],
            ),
            argv_command(
                "status",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-q".to_string(),
                    "--manifest-path".to_string(),
                    "crates/cynic-host/Cargo.toml".to_string(),
                    "--bin".to_string(),
                    "cynic-host-exec".to_string(),
                    "--".to_string(),
                    "bridge-status".to_string(),
                ],
            ),
        ],
        install_commands: manifest
            .prepare_directories
            .into_iter()
            .chain([
                helper_runtime.install_helper_command(&bundle_dir_str, HOST_EXEC_HELPER_FILENAME),
                helper_runtime.install_sudoers_command(&bundle_dir_str, HOST_EXEC_SUDOERS_FILENAME),
                helper_runtime.validate_sudoers_command(),
            ])
            .collect(),
        postflight: vec![
            argv_command(
                "status",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-q".to_string(),
                    "--manifest-path".to_string(),
                    "crates/cynic-host/Cargo.toml".to_string(),
                    "--bin".to_string(),
                    "cynic-host-exec".to_string(),
                    "--".to_string(),
                    "bridge-status".to_string(),
                ],
            ),
            argv_command(
                "observe_baseline",
                vec![
                    sudo_bin.clone(),
                    "-n".to_string(),
                    host_exec_helper_path(),
                    "network-baseline".to_string(),
                    "observe".to_string(),
                ],
            ),
            argv_command(
                "plan_principal_prepare",
                vec![
                    sudo_bin,
                    "-n".to_string(),
                    host_exec_helper_path(),
                    "principal-prepare".to_string(),
                ],
            ),
        ],
        denial_checks: vec![
            argv_command(
                "deny_shell",
                vec![
                    runtime_layout.sudo_bin(),
                    "-n".to_string(),
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    "true".to_string(),
                ],
            ),
            argv_command(
                "deny_service_status",
                vec![
                    runtime_layout.sudo_bin(),
                    "-n".to_string(),
                    runtime_layout.systemctl_bin(),
                    "status".to_string(),
                    "cynic-node.service".to_string(),
                ],
            ),
            argv_command(
                "deny_package_update",
                vec![
                    runtime_layout.sudo_bin(),
                    "-n".to_string(),
                    runtime_layout.package_install_bin(),
                    "update".to_string(),
                ],
            ),
        ],
        success_criteria: vec![
            "bridge-status reports status=active",
            "cynic can invoke only the host-exec helper through sudo",
            "direct shell/systemctl/apt-get sudo invocations remain denied",
        ],
    }
}

pub fn bridge_install_report(
    bundle_dir: impl AsRef<Path>,
    apply: bool,
) -> HostExecBridgeInstallReport {
    let bundle_dir = bundle_dir.as_ref();
    let plan = bridge_activation_plan(bundle_dir);
    bridge_install_report_from_commands(
        bundle_dir,
        apply,
        plan.bundle_expected_complete,
        plan.install_commands,
        true,
    )
}

pub fn bridge_rootless_smoke(
    sandbox_root: impl AsRef<Path>,
    apply: bool,
) -> Result<HostExecBridgeRootlessSmokeReport, Box<dyn std::error::Error>> {
    let sandbox_root = sandbox_root.as_ref();
    let bundle_dir = sandbox_root.join("bundle");
    let helper_runtime = BoundedHelperInstallRuntime {
        helper_target: sandbox_root
            .join("libexec")
            .join(HOST_EXEC_HELPER_FILENAME)
            .display()
            .to_string(),
        sudoers_target: sandbox_root
            .join("sudoers.d")
            .join("cynic-host-exec")
            .display()
            .to_string(),
        helper_dir: sandbox_root.join("libexec").display().to_string(),
        sudoers_dir: sandbox_root.join("sudoers.d").display().to_string(),
    };
    let projection = helper_projection_for_paths(
        helper_runtime.helper_target.clone(),
        helper_runtime.sudoers_target.clone(),
    );
    let owner_policy = current_user_owned_policy(MODE_0755);
    let sudoers_policy = current_user_owned_policy(MODE_0440);
    let prepare_directories =
        prepare_directory_commands_for_runtime(&helper_runtime, owner_policy.clone());
    let helper_source = helper_binary_source()?;
    let bundle_manifest = write_bridge_bundle_with_runtime(
        &bundle_dir,
        &helper_source,
        projection.clone(),
        helper_runtime.clone(),
        Some(prepare_directories),
    )?;
    let install_commands = install_commands_for_runtime(
        &helper_runtime,
        &bundle_dir.to_string_lossy(),
        owner_policy,
        sudoers_policy,
    );
    let install_report =
        bridge_install_report_from_commands(&bundle_dir, apply, true, install_commands, false);
    let status_report = bridge_status_for_projection(&projection);
    let status = if install_report.status == "completed" && status_report.status == "active" {
        "completed"
    } else if install_report.status == "blocked" {
        "blocked"
    } else {
        "failed"
    };

    Ok(HostExecBridgeRootlessSmokeReport {
        schema: "cynic.host_exec_bridge.rootless_smoke.v1",
        status,
        mutates: true,
        requested_apply: apply,
        sandbox_root: sandbox_root.display().to_string(),
        bundle_manifest,
        install_report,
        status_report,
    })
}

pub fn bridge_status() -> HostExecBridgeStatus {
    bridge_status_for_projection(&helper_projection())
}

fn bundle_manifest(helper_source: &str, bundle_dir: &str) -> HostExecBridgeBundleManifest {
    let helper_runtime = helper_install_runtime();
    let projection = helper_projection();
    bundle_manifest_for_context(
        helper_source,
        bundle_dir,
        &bridge_bundle_context(projection, helper_runtime.clone()),
        helper_runtime.prepare_directories(),
    )
}

fn install_commands_for_runtime(
    helper_runtime: &BoundedHelperInstallRuntime,
    bundle_dir: &str,
    directory_policy: InstallPathPolicy,
    sudoers_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    prepare_directory_commands_for_runtime(helper_runtime, directory_policy)
        .into_iter()
        .chain([
            install_file_command(
                "install_helper",
                format!("{bundle_dir}/{HOST_EXEC_HELPER_FILENAME}"),
                helper_runtime.helper_target.clone(),
                current_user_owned_policy(MODE_0755),
            ),
            install_file_command(
                "install_sudoers",
                format!("{bundle_dir}/{HOST_EXEC_SUDOERS_FILENAME}"),
                helper_runtime.sudoers_target.clone(),
                sudoers_policy,
            ),
            validate_sudoers_command("validate_sudoers", helper_runtime.sudoers_target.clone()),
        ])
        .collect()
}

fn prepare_directory_commands_for_runtime(
    helper_runtime: &BoundedHelperInstallRuntime,
    directory_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    vec![
        install_dir_command(
            "prepare_directory",
            Path::new(&helper_runtime.helper_target)
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| helper_runtime.helper_dir.clone()),
            directory_policy.clone(),
        ),
        install_dir_command(
            "prepare_directory",
            Path::new(&helper_runtime.sudoers_target)
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| helper_runtime.sudoers_dir.clone()),
            directory_policy,
        ),
    ]
}

fn helper_binary_source() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = std::env::current_exe()?;
    if !source.ends_with("cynic-host-exec") {
        let sibling = source.with_file_name(HOST_EXEC_HELPER_FILENAME);
        if sibling.exists() {
            return Ok(sibling);
        }
        if let Some(parent) = source.parent().and_then(Path::parent) {
            let target_debug = parent.join(HOST_EXEC_HELPER_FILENAME);
            if target_debug.exists() {
                return Ok(target_debug);
            }
        }
        return Err(format!(
            "unable to resolve cynic-host-exec near current executable: {}",
            source.display()
        )
        .into());
    }
    Ok(source)
}

fn bundle_complete(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let required = [
        "manifest.json",
        HOST_EXEC_HELPER_FILENAME,
        HOST_EXEC_SUDOERS_FILENAME,
    ];
    path.exists() && required.iter().all(|name| path.join(name).exists())
}

fn bridge_bundle_context(
    projection: BoundedHelperProjection,
    helper_runtime: BoundedHelperInstallRuntime,
) -> BridgeBundleContext {
    BridgeBundleContext {
        helper_filename: HOST_EXEC_HELPER_FILENAME,
        sudoers_filename: HOST_EXEC_SUDOERS_FILENAME,
        helper_runtime,
        projection,
    }
}

fn host_exec_helper_path() -> String {
    current_runtime_layout().host_exec_helper_path()
}

fn host_exec_sudoers_path() -> String {
    current_runtime_layout().host_exec_sudoers_path()
}

fn helper_projection() -> BoundedHelperProjection {
    helper_projection_for_paths(host_exec_helper_path(), host_exec_sudoers_path())
}

fn helper_projection_for_paths(
    helper_path: impl Into<String>,
    sudoers_path: impl Into<String>,
) -> BoundedHelperProjection {
    let helper_path = helper_path.into();
    BoundedHelperProjection::new(
        HOST_EXEC_SUDOERS_SUBJECT,
        helper_path.clone(),
        sudoers_path,
        "Allows the cynic service user to invoke only bounded host-exec subprofiles.",
    )
    .with_allowed_invocations(vec![
        vec![helper_path.clone(), "principal-prepare".to_string()],
        vec![
            helper_path.clone(),
            "principal-prepare".to_string(),
            "--apply".to_string(),
        ],
        vec![
            helper_path,
            "network-baseline".to_string(),
            "observe".to_string(),
        ],
    ])
}

fn helper_install_runtime() -> BoundedHelperInstallRuntime {
    BoundedHelperInstallRuntime {
        helper_target: host_exec_helper_path(),
        sudoers_target: host_exec_sudoers_path(),
        helper_dir: current_runtime_layout().helper_dir,
        sudoers_dir: current_runtime_layout().sudoers_dir,
    }
}

fn current_user_owned_policy(mode: InstallMode) -> InstallPathPolicy {
    InstallPathPolicy {
        owner: InstallOwnership {
            user: Cow::Owned(observed_user()),
            group: Cow::Owned(observed_group()),
        },
        mode,
    }
}

fn observed_user() -> String {
    capture_argv(&["id".to_string(), "-un".to_string()])
        .ok()
        .filter(|output| output.success && !output.stdout.is_empty())
        .map(|output| output.stdout)
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| HOST_EXEC_SUDOERS_SUBJECT.to_string())
}

fn observed_group() -> String {
    capture_argv(&["id".to_string(), "-gn".to_string()])
        .ok()
        .filter(|output| output.success && !output.stdout.is_empty())
        .map(|output| output.stdout)
        .unwrap_or_else(observed_user)
}

#[allow(dead_code)]
fn _sudo_available() -> bool {
    crate::process_observation::command_exit_success("sudo", &["-n", "true"]) == Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_surface::install_dir_command;
    use crate::install_contract::root_owned_path;
    use std::fs;

    #[test]
    fn bridge_install_requires_apply() {
        let bundle_dir =
            std::env::temp_dir().join(format!("cynic-bridge-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&bundle_dir);
        fs::create_dir_all(&bundle_dir).expect("create bundle dir");
        fs::write(bundle_dir.join("manifest.json"), "{}").expect("manifest");
        fs::write(bundle_dir.join(HOST_EXEC_HELPER_FILENAME), "").expect("helper");
        fs::write(bundle_dir.join(HOST_EXEC_SUDOERS_FILENAME), "").expect("sudoers");

        let report = bridge_install_report(&bundle_dir, false);

        assert_eq!(report.status, "blocked");
        assert!(report.blockers.contains(&"apply_flag_required".to_string()));
        let _ = fs::remove_dir_all(&bundle_dir);
    }

    #[test]
    fn bridge_install_dir_surface_becomes_argv_without_shell() {
        let command = install_dir_command(
            "prepare_directory",
            "/tmp/cynic-test",
            root_owned_path(MODE_0755),
        );

        let argv = crate::execution::command_to_argv(&command).expect("install_dir is executable");

        assert_eq!(
            argv,
            vec![
                "install",
                "-d",
                "-o",
                "root",
                "-g",
                "root",
                "-m",
                "0755",
                "/tmp/cynic-test"
            ]
        );
    }
}

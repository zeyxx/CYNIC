use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::bounded_helper::{BoundedHelperInstallRuntime, BoundedHelperProjection};
use crate::command_surface::{HostCommandSurface, argv_command, expectation, render_command};
use crate::install_contract::{MODE_0440, MODE_0644, MODE_0755};
use crate::runtime_layout::current_runtime_layout;

use super::types::HostExecBridgeBundleManifest;

pub(super) struct BridgeBundleContext {
    pub(super) helper_filename: &'static str,
    pub(super) sudoers_filename: &'static str,
    pub(super) helper_runtime: BoundedHelperInstallRuntime,
    pub(super) projection: BoundedHelperProjection,
}

pub(super) fn write_bridge_bundle_with_context(
    output_dir: impl AsRef<Path>,
    helper_source: &Path,
    context: BridgeBundleContext,
    prepare_directories: Option<Vec<HostCommandSurface>>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let helper_target = output_dir.join(context.helper_filename);
    let sudoers_target = output_dir.join(context.sudoers_filename);
    let manifest_target = output_dir.join("manifest.json");

    prepare_bundle_target(&helper_target)?;
    prepare_bundle_target(&sudoers_target)?;

    fs::copy(helper_source, &helper_target)?;
    fs::set_permissions(&helper_target, fs::Permissions::from_mode(MODE_0755.unix))?;

    fs::write(&sudoers_target, context.projection.sudoers_projection())?;
    fs::set_permissions(&sudoers_target, fs::Permissions::from_mode(MODE_0440.unix))?;

    let manifest = bundle_manifest_for_context(
        &context,
        prepare_directories.unwrap_or_else(|| context.helper_runtime.prepare_directories()),
    );
    fs::write(
        &manifest_target,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    fs::set_permissions(&manifest_target, fs::Permissions::from_mode(MODE_0644.unix))?;
    Ok(manifest)
}

pub(super) fn bundle_manifest_for_context(
    context: &BridgeBundleContext,
    prepare_directories: Vec<HostCommandSurface>,
) -> HostExecBridgeBundleManifest {
    let bundle_files = BTreeMap::from([
        ("helper", context.helper_filename.to_string()),
        ("sudoers", context.sudoers_filename.to_string()),
        ("manifest", "manifest.json".to_string()),
    ]);
    let helper_path = context.helper_runtime.helper_target.clone();
    let sudoers_path = context.helper_runtime.sudoers_target.clone();
    let runtime_layout = current_runtime_layout();
    let prove_denied_service_status = argv_command(
        "prove_denied_service_status",
        vec![
            runtime_layout.sudo_bin(),
            "-n".to_string(),
            runtime_layout
                .capabilities()
                .service_manager
                .command
                .clone(),
            "status".to_string(),
            "cynic-node.service".to_string(),
        ],
    );

    HostExecBridgeBundleManifest {
        schema: "cynic.host_exec_bridge.bundle_manifest.v1",
        mutates: true,
        helper_source: context.helper_filename.to_string(),
        helper_target: helper_path.clone(),
        sudoers_target: sudoers_path.clone(),
        sudoers_rule: context.projection.sudoers_rule(),
        prepare_directories,
        required_modes: context.helper_runtime.required_modes(),
        activation_checks: vec![
            expectation("prove_status", "bridge-status reports status=active"),
            expectation(
                "prove_observe_baseline",
                format!("sudo -n {helper_path} network-baseline observe succeeds"),
            ),
            expectation(
                "prove_principal_prepare_plan",
                format!("sudo -n {helper_path} principal-prepare succeeds without mutating"),
            ),
            expectation(
                "prove_denied_service_status",
                render_command(&prove_denied_service_status),
            ),
        ],
        bundle_dir: ".".to_string(),
        bundle_files,
    }
}

fn prepare_bundle_target(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

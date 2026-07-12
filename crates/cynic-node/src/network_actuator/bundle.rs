use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use cynic_host::bounded_helper::{
    BoundedHelperInstallRuntime, bundle_complete as helper_bundle_complete,
};
use cynic_host::install_contract::{
    MODE_0440, MODE_0644, MODE_0755, ROOT_OWNERSHIP, mode_map, owner_string, root_owned_path,
};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator::policy::build_policy_document;
use crate::network_actuator::types::{
    NetworkActuatorActivationPlan, NetworkActuatorBundleManifest,
};
use crate::network_actuator_contract::{
    NETWORK_ACTUATOR_HELPER_FILENAME, NETWORK_ACTUATOR_MANIFEST_FILENAME,
    NETWORK_ACTUATOR_POLICY_FILENAME, NETWORK_ACTUATOR_SUDOERS_FILENAME, bundle_required_files,
    helper_path, policy_path, sudoers_path, sudoers_projection, sudoers_rule,
};
use crate::network_command_surface::{
    NetworkCommandSurface, argv_command, expectation, from_host_command, install_dir_command,
    install_file_command, render_command,
};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};
use crate::observe::NodeObservation;

#[derive(Debug, Clone)]
struct NetworkActuatorRuntimeContext {
    helper_target: String,
    policy_target: String,
    sudoers_target: String,
    sudoers_rule: String,
    helper_dir: String,
    config_dir: String,
    sudoers_dir: String,
    sudo_bin: String,
    route_bin: String,
    wireguard_bin: String,
}

pub(crate) fn write_install_bundle(
    observation: &NodeObservation,
    output_dir: impl AsRef<Path>,
) -> Result<NetworkActuatorBundleManifest, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let helper_target = output_dir.join(NETWORK_ACTUATOR_HELPER_FILENAME);
    let policy_target = output_dir.join(NETWORK_ACTUATOR_POLICY_FILENAME);
    let sudoers_target = output_dir.join(NETWORK_ACTUATOR_SUDOERS_FILENAME);
    let manifest_target = output_dir.join(NETWORK_ACTUATOR_MANIFEST_FILENAME);

    prepare_bundle_target(&helper_target)?;
    prepare_bundle_target(&policy_target)?;
    prepare_bundle_target(&sudoers_target)?;

    let helper_source = helper_binary_source()?;
    fs::copy(&helper_source, &helper_target)?;
    fs::set_permissions(&helper_target, fs::Permissions::from_mode(MODE_0755.unix))?;

    let policy = build_policy_document(observation);
    fs::write(
        &policy_target,
        format!("{}\n", serde_json::to_string_pretty(&policy)?),
    )?;
    fs::set_permissions(&policy_target, fs::Permissions::from_mode(MODE_0644.unix))?;

    fs::write(&sudoers_target, sudoers_projection().as_bytes())?;
    fs::set_permissions(&sudoers_target, fs::Permissions::from_mode(MODE_0440.unix))?;

    let manifest = install_bundle_manifest_with_context(
        &helper_source.to_string_lossy(),
        &policy_target.to_string_lossy(),
        &output_dir.to_string_lossy(),
        &runtime_context(),
    );
    fs::write(
        &manifest_target,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    fs::set_permissions(&manifest_target, fs::Permissions::from_mode(MODE_0644.unix))?;
    Ok(manifest)
}

pub(crate) fn activation_plan(
    _observation: &NodeObservation,
    bundle_dir: impl AsRef<Path>,
) -> NetworkActuatorActivationPlan {
    let bundle_dir = bundle_dir.as_ref();
    let context = runtime_context();
    let install_plan = install_bundle_manifest_with_context(
        "<current_executable>",
        &bundle_dir
            .join("network-actuator.policy.json")
            .to_string_lossy(),
        &bundle_dir.to_string_lossy(),
        &context,
    );
    activation_plan_with_context(bundle_dir, install_plan, &context)
}

fn activation_plan_with_context(
    bundle_dir: &Path,
    install_plan: NetworkActuatorBundleManifest,
    context: &NetworkActuatorRuntimeContext,
) -> NetworkActuatorActivationPlan {
    let bundle_dir_str = bundle_dir.to_string_lossy().to_string();
    let bundle_files = install_plan.bundle_files.clone();
    let helper_runtime = helper_install_runtime(context);
    NetworkActuatorActivationPlan {
        schema: "cynic.network_actuator.activation_plan.v1",
        mutates: false,
        bundle_dir: bundle_dir_str.clone(),
        bundle_expected_complete: bundle_complete(bundle_dir),
        bundle_files,
        preflight: preflight_commands(&bundle_dir_str),
        install_commands: install_plan
            .prepare_directories
            .into_iter()
            .chain([
                install_dir_command(
                    "prepare_directory",
                    context.config_dir.clone(),
                    root_owned_path(MODE_0755),
                ),
                from_host_command(
                    helper_runtime
                        .install_helper_command(&bundle_dir_str, NETWORK_ACTUATOR_HELPER_FILENAME),
                ),
                install_file_command(
                    "install_policy",
                    format!("{bundle_dir_str}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
                    context.policy_target.clone(),
                    root_owned_path(MODE_0644),
                ),
                from_host_command(
                    helper_runtime.install_sudoers_command(
                        &bundle_dir_str,
                        NETWORK_ACTUATOR_SUDOERS_FILENAME,
                    ),
                ),
                from_host_command(helper_runtime.validate_sudoers_command()),
            ])
            .collect(),
        postflight: postflight_commands(&context.sudo_bin, &context.helper_target),
        denial_checks: denial_checks(
            &context.sudo_bin,
            &context.route_bin,
            &context.wireguard_bin,
        ),
        success_criteria: success_criteria(),
    }
}

fn install_bundle_manifest_with_context(
    helper_source: &str,
    policy_source: &str,
    bundle_dir: &str,
    context: &NetworkActuatorRuntimeContext,
) -> NetworkActuatorBundleManifest {
    let helper_runtime = helper_install_runtime(context);
    let prepare_directories = prepare_directories(context);
    let required_modes = bundle_required_modes(&helper_runtime);
    let bundle_files = bundle_files(bundle_dir);
    NetworkActuatorBundleManifest {
        schema: "cynic.network_actuator.install_plan.v1",
        mutates: false,
        helper_source: helper_source.to_string(),
        helper_target: context.helper_target.clone(),
        policy_source: policy_source.to_string(),
        policy_target: context.policy_target.clone(),
        sudoers_target: context.sudoers_target.clone(),
        sudoers_rule: context.sudoers_rule.clone(),
        prepare_directories: prepare_directories.clone(),
        required_owner: owner_string(ROOT_OWNERSHIP),
        required_modes,
        activation_checks: activation_checks(prepare_directories, context),
        bundle_dir: bundle_dir.to_string(),
        bundle_files,
    }
}

fn bundle_required_modes(
    helper_runtime: &BoundedHelperInstallRuntime,
) -> BTreeMap<&'static str, String> {
    mode_map(&[("helper", MODE_0755), ("policy", MODE_0644)])
        .into_iter()
        .chain(helper_runtime.required_modes())
        .collect()
}

fn bundle_files(bundle_dir: &str) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        (
            "helper",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_HELPER_FILENAME}"),
        ),
        (
            "policy",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
        ),
        (
            "sudoers",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_SUDOERS_FILENAME}"),
        ),
        (
            "manifest",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_MANIFEST_FILENAME}"),
        ),
    ])
}

fn activation_checks(
    prepare_directories: Vec<NetworkCommandSurface>,
    context: &NetworkActuatorRuntimeContext,
) -> Vec<NetworkCommandSurface> {
    let deny_direct_route = deny_direct_route_command(context);
    prepare_directories
        .into_iter()
        .chain([
            expectation(
                "install_helper",
                "install helper_source -> helper_target with root:root 0755",
            ),
            expectation(
                "install_policy",
                "install policy_source -> policy_target with root:root 0644",
            ),
            expectation(
                "install_sudoers",
                "install rendered_sudoers -> sudoers_target with root:root 0440",
            ),
            expectation("validate_sudoers", "validate sudoers_target with visudo -cf"),
            expectation(
                "prove_profile_apply",
                format!(
                    "sudo -n helper_target apply --profile synapse-route --registry {DEFAULT_REGISTRY_PATH} --apply"
                ),
            ),
            expectation(
                "deny_direct_route",
                format!(
                    "{} must remain unauthorized for the cynic user",
                    render_command(&deny_direct_route)
                ),
            ),
        ])
        .collect()
}

fn prepare_directories(context: &NetworkActuatorRuntimeContext) -> Vec<NetworkCommandSurface> {
    let mut commands = helper_install_runtime(context)
        .prepare_directories()
        .into_iter()
        .map(from_host_command)
        .collect::<Vec<_>>();
    commands.push(install_dir_command(
        "prepare_directory",
        Path::new(&context.policy_target)
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| context.config_dir.clone()),
        root_owned_path(MODE_0755),
    ));
    commands
}

fn runtime_context() -> NetworkActuatorRuntimeContext {
    let runtime_layout = current_runtime_layout();
    let capabilities = runtime_layout.capabilities();
    NetworkActuatorRuntimeContext {
        helper_target: helper_path(),
        policy_target: policy_path(),
        sudoers_target: sudoers_path(),
        sudoers_rule: sudoers_rule(),
        helper_dir: runtime_layout.helper_dir.clone(),
        config_dir: runtime_layout.config_dir.clone(),
        sudoers_dir: runtime_layout.sudoers_dir.clone(),
        sudo_bin: runtime_layout.sudo_bin(),
        route_bin: capabilities.route_tool.command.clone(),
        wireguard_bin: capabilities.wireguard_tool.command.clone(),
    }
}

fn helper_install_runtime(context: &NetworkActuatorRuntimeContext) -> BoundedHelperInstallRuntime {
    BoundedHelperInstallRuntime {
        helper_target: context.helper_target.clone(),
        sudoers_target: context.sudoers_target.clone(),
        helper_dir: context.helper_dir.clone(),
        sudoers_dir: context.sudoers_dir.clone(),
    }
}

fn preflight_commands(bundle_dir_str: &str) -> Vec<NetworkCommandSurface> {
    vec![
        cargo_build_helper_command(),
        cargo_run_network_command(
            "bundle",
            vec![
                "actuator-bundle".to_string(),
                "--output-dir".to_string(),
                bundle_dir_str.to_string(),
            ],
        ),
        status_command(),
        evidence_command(),
    ]
}

fn postflight_commands(sudo_bin: &str, helper_target: &str) -> Vec<NetworkCommandSurface> {
    vec![
        status_command(),
        evidence_command(),
        prove_profile_apply_command(sudo_bin, helper_target),
    ]
}

fn denial_checks(
    sudo_bin: &str,
    route_bin: &str,
    wireguard_bin: &str,
) -> Vec<NetworkCommandSurface> {
    vec![
        deny_route_command(sudo_bin, route_bin),
        deny_wireguard_command(sudo_bin, wireguard_bin),
        deny_shell_command(sudo_bin),
    ]
}

fn prove_profile_apply_command(sudo_bin: &str, helper_target: &str) -> NetworkCommandSurface {
    argv_command(
        "prove_profile_apply",
        vec![
            sudo_bin.to_string(),
            "-n".to_string(),
            helper_target.to_string(),
            "apply".to_string(),
            "--profile".to_string(),
            "synapse-route".to_string(),
            "--registry".to_string(),
            DEFAULT_REGISTRY_PATH.to_string(),
            "--apply".to_string(),
        ],
    )
}

fn deny_direct_route_command(context: &NetworkActuatorRuntimeContext) -> NetworkCommandSurface {
    argv_command(
        "deny_direct_route",
        vec![
            context.sudo_bin.clone(),
            "-n".to_string(),
            context.route_bin.clone(),
            "route".to_string(),
            "show".to_string(),
        ],
    )
}

fn success_criteria() -> Vec<NetworkDiagnostic> {
    vec![
        activation_status_active_criterion(),
        network_actuator_status_active_criterion(),
        direct_commands_denied_criterion(),
    ]
}

fn deny_route_command(sudo_bin: &str, route_bin: &str) -> NetworkCommandSurface {
    argv_command(
        "deny_route",
        vec![
            sudo_bin.to_string(),
            "-n".to_string(),
            route_bin.to_string(),
            "route".to_string(),
            "show".to_string(),
        ],
    )
}

fn deny_wireguard_command(sudo_bin: &str, wireguard_bin: &str) -> NetworkCommandSurface {
    argv_command(
        "deny_wireguard",
        vec![
            sudo_bin.to_string(),
            "-n".to_string(),
            wireguard_bin.to_string(),
            "show".to_string(),
        ],
    )
}

fn deny_shell_command(sudo_bin: &str) -> NetworkCommandSurface {
    argv_command(
        "deny_shell",
        vec![
            sudo_bin.to_string(),
            "-n".to_string(),
            "/bin/sh".to_string(),
            "-c".to_string(),
            "true".to_string(),
        ],
    )
}

fn activation_status_active_criterion() -> NetworkDiagnostic {
    diagnostic(
        "activation_status_active",
        "success_criteria",
        "activation-status reports status=active",
    )
}

fn network_actuator_status_active_criterion() -> NetworkDiagnostic {
    diagnostic(
        "network_actuator_status_active",
        "success_criteria",
        "cynic-node network actuator-status reports status=active",
    )
}

fn direct_commands_denied_criterion() -> NetworkDiagnostic {
    diagnostic(
        "direct_commands_denied",
        "success_criteria",
        "direct ip/wg/shell commands remain denied for cynic",
    )
}

fn status_command() -> NetworkCommandSurface {
    cargo_run_network_command(
        "status",
        vec!["actuator-status".to_string(), "--json".to_string()],
    )
}

fn evidence_command() -> NetworkCommandSurface {
    cargo_run_network_command(
        "evidence",
        vec!["evidence".to_string(), "--json".to_string()],
    )
}

fn cargo_build_helper_command() -> NetworkCommandSurface {
    argv_command(
        "build",
        vec![
            "cargo".to_string(),
            "build".to_string(),
            "-q".to_string(),
            "-p".to_string(),
            "cynic-node".to_string(),
            "--bin".to_string(),
            NETWORK_ACTUATOR_HELPER_FILENAME.to_string(),
        ],
    )
}

fn cargo_run_network_command(
    role: &'static str,
    network_args: Vec<String>,
) -> NetworkCommandSurface {
    let mut argv = vec![
        "cargo".to_string(),
        "run".to_string(),
        "-q".to_string(),
        "-p".to_string(),
        "cynic-node".to_string(),
        "--".to_string(),
        "network".to_string(),
    ];
    argv.extend(network_args);
    argv_command(role, argv)
}

fn prepare_bundle_target(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn helper_binary_source() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    if current_exe.file_name().and_then(|name| name.to_str()) == Some("cynic-network-actuator") {
        return Ok(current_exe);
    }

    let sibling = current_exe
        .parent()
        .map(|parent| parent.join(NETWORK_ACTUATOR_HELPER_FILENAME))
        .ok_or("missing executable parent directory")?;
    if sibling.exists() {
        return Ok(sibling);
    }

    Err(format!(
        "missing helper binary at {}; build it first with `cargo build -p cynic-node --bin {NETWORK_ACTUATOR_HELPER_FILENAME}`",
        sibling.display()
    )
    .into())
}

fn bundle_complete(bundle_dir: &Path) -> bool {
    helper_bundle_complete(bundle_dir, &bundle_required_files())
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuatorRuntimeContext, activation_plan_with_context,
        install_bundle_manifest_with_context,
    };
    use crate::network_command_surface::NetworkCommandInvocation;
    use std::path::Path;

    fn test_context() -> NetworkActuatorRuntimeContext {
        NetworkActuatorRuntimeContext {
            helper_target: "/usr/local/bin/cynic-network-actuator".to_string(),
            policy_target: "/etc/cynic/network-actuator.policy.json".to_string(),
            sudoers_target: "/etc/sudoers.d/cynic-network-actuator".to_string(),
            sudoers_rule: "cynic ALL=(root) NOPASSWD: /usr/local/bin/cynic-network-actuator"
                .to_string(),
            helper_dir: "/usr/local/bin".to_string(),
            config_dir: "/etc/cynic".to_string(),
            sudoers_dir: "/etc/sudoers.d".to_string(),
            sudo_bin: "/usr/bin/sudo".to_string(),
            route_bin: "/sbin/ip".to_string(),
            wireguard_bin: "/usr/bin/wg".to_string(),
        }
    }

    #[test]
    fn install_manifest_uses_injected_targets() {
        let manifest = install_bundle_manifest_with_context(
            "/tmp/build/cynic-network-actuator",
            "/tmp/build/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );

        assert_eq!(
            manifest.helper_target,
            "/usr/local/bin/cynic-network-actuator"
        );
        assert_eq!(
            manifest.policy_target,
            "/etc/cynic/network-actuator.policy.json"
        );
        assert_eq!(
            manifest.sudoers_target,
            "/etc/sudoers.d/cynic-network-actuator"
        );
        assert!(
            manifest
                .activation_checks
                .iter()
                .any(|item| matches!(&item.invocation, NetworkCommandInvocation::Expectation { text } if text.contains("/usr/bin/sudo -n /sbin/ip route show")))
        );
    }

    #[test]
    fn activation_plan_uses_injected_install_context() {
        let manifest = install_bundle_manifest_with_context(
            "<current_executable>",
            "/tmp/bundle/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );
        let plan =
            activation_plan_with_context(Path::new("/tmp/bundle"), manifest, &test_context());

        assert_eq!(plan.bundle_dir, "/tmp/bundle");
        let helper_install =
            plan.install_commands
                .iter()
                .find_map(|command| match &command.invocation {
                    NetworkCommandInvocation::InstallFile { target, .. }
                        if target == "/usr/local/bin/cynic-network-actuator" =>
                    {
                        Some(target)
                    }
                    _ => None,
                });
        assert_eq!(
            helper_install,
            Some(&"/usr/local/bin/cynic-network-actuator".to_string())
        );
        match &plan.denial_checks[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/usr/bin/sudo");
                assert_eq!(argv[2], "/sbin/ip");
            }
            _ => panic!("expected argv command"),
        }
    }
}

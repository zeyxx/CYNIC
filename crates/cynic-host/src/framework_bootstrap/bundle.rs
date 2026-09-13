use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::bridge;
use crate::bundle::files::{
    copy_registry, copy_runtime_artifact, copy_standalone_cli_if_available, write_executable_file,
    write_file,
};
use crate::bundle::seed::{write_seed_checksums, write_seed_manifest};
use crate::scripts::{
    BootstrapRuntimeArtifact, bootstrap_enroll_script, bootstrap_install_script,
    bootstrap_security_maintenance_script, bootstrap_verify_script,
};
use crate::types::FrameworkBootstrapBundleReport;

mod files;
mod node_control;
mod seed;

use node_control::render_node_control_config;

pub fn write_framework_bootstrap_bundle(
    output_dir: &str,
) -> Result<FrameworkBootstrapBundleReport, String> {
    write_framework_bootstrap_bundle_for_role(output_dir, "unassigned", None, None)
}

#[derive(Deserialize)]
struct RuntimeBasket {
    runtime_seed: RuntimeSeed,
    artifacts: Vec<RuntimeArtifact>,
}

#[derive(Deserialize)]
struct RuntimeSeed {
    binary: String,
    contract_revision: u64,
    signature_namespace: String,
    runtime_basket_member: bool,
}

#[derive(Clone, Deserialize)]
struct RuntimeArtifact {
    binary: String,
    install_path: String,
    roles: Vec<String>,
}

pub fn write_framework_bootstrap_bundle_for_role(
    output_dir: &str,
    role: &str,
    artifact_dir: Option<&str>,
    registry: Option<&str>,
) -> Result<FrameworkBootstrapBundleReport, String> {
    let output_dir = Path::new(output_dir);
    let basket: RuntimeBasket =
        serde_json::from_str(include_str!("../../../../infra/runtime-basket.json"))
            .map_err(|error| format!("failed to parse runtime basket: {error}"))?;
    validate_runtime_seed_boundary(&basket)?;
    let required_artifacts = if role == "unassigned" {
        Vec::new()
    } else {
        let required = basket
            .artifacts
            .iter()
            .filter(|artifact| artifact.roles.iter().any(|candidate| candidate == role))
            .cloned()
            .collect::<Vec<_>>();
        if required.is_empty() {
            return Err(format!("runtime basket does not declare role {role}"));
        }
        required
    };
    let required = required_artifacts
        .iter()
        .map(|artifact| artifact.binary.clone())
        .collect::<Vec<_>>();
    let bootstrap_runtime_artifacts = required_artifacts
        .iter()
        .map(|artifact| BootstrapRuntimeArtifact {
            binary: artifact.binary.clone(),
            install_path: artifact.install_path.clone(),
        })
        .collect::<Vec<_>>();
    let current_exe_is_cynic = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == "cynic")
        })
        .unwrap_or(false);
    let mut missing_artifacts = required
        .iter()
        .filter(|binary| {
            !(binary.as_str() == "cynic" && current_exe_is_cynic)
                && artifact_dir.map(|dir| Path::new(dir).join(binary).is_file()) != Some(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    if role != "unassigned" && registry.map(|path| Path::new(path).is_file()) != Some(true) {
        missing_artifacts.push("runtime-registry".to_string());
    }
    if !missing_artifacts.is_empty() {
        let blockers = missing_artifacts
            .iter()
            .map(|artifact| format!("required_runtime_artifact_missing:{artifact}"))
            .collect();
        return Ok(FrameworkBootstrapBundleReport {
            schema: "cynic.framework_bootstrap.bundle.v2",
            status: "blocked",
            mutates: false,
            output_dir: output_dir.display().to_string(),
            files: Vec::new(),
            required_role: role.to_string(),
            missing_artifacts,
            blockers,
            seed_id: String::new(),
            seed_manifest: "seed-manifest.json".to_string(),
            entrypoint: "enroll.sh".to_string(),
            operator_command: "provide the role-projected artifacts and runtime registry"
                .to_string(),
            maintenance_command: String::new(),
            target_runtime: "fresh_linux_node",
            security_invariants: Vec::new(),
        });
    }
    let registry_value = if let Some(registry) = registry {
        let content = fs::read(registry)
            .map_err(|error| format!("failed to read runtime registry {registry}: {error}"))?;
        Some(
            serde_json::from_slice::<serde_json::Value>(&content).map_err(|error| {
                format!("runtime registry {registry} is not valid JSON: {error}")
            })?,
        )
    } else {
        None
    };
    let host_exec_bundle_dir = output_dir.join("host-exec-bundle");
    fs::create_dir_all(&host_exec_bundle_dir)
        .map_err(|error| format!("failed to create host-exec-bundle dir: {error}"))?;
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to locate current executable: {error}"))?;
    let bridge_manifest =
        bridge::write_bridge_bundle_from_binary(&host_exec_bundle_dir, &current_exe)
            .map_err(|error| format!("failed to write host-exec bridge bundle: {error}"))?;

    let mut files = vec![
        "host-exec-bundle/cynic-host-exec".to_string(),
        "host-exec-bundle/cynic-host-exec.sudoers".to_string(),
        "host-exec-bundle/manifest.json".to_string(),
    ];

    let copied_current_cli = copy_standalone_cli_if_available(output_dir, &mut files)?;
    if let Some(dir) = artifact_dir {
        for binary in required
            .iter()
            .filter(|binary| binary.as_str() != "cynic" || !copied_current_cli)
        {
            copy_runtime_artifact(Path::new(dir), binary, output_dir, &mut files)?;
        }
    }
    if let Some(registry) = registry {
        copy_registry(Path::new(registry), output_dir, &mut files)?;
    }
    if role == "hypervisor" {
        let report = crate::lxc_exec_bridge::bundle(
            output_dir.join("lxc-exec-bridge-bundle"),
            output_dir.join("bin/cynic-lxc-exec-bridge"),
        );
        if report.status != "completed" {
            return Err(format!(
                "failed to build Rust-native LXC bridge bundle: {}",
                report.unknowns.join(",")
            ));
        }
        files.extend([
            "lxc-exec-bridge-bundle/cynic-lxc-exec-bridge".to_string(),
            "lxc-exec-bridge-bundle/cynic-lxc-exec-bridge.sudoers".to_string(),
            "lxc-exec-bridge-bundle/manifest.json".to_string(),
        ]);
    }
    if let Some(storage_config) = registry_value
        .as_ref()
        .and_then(|registry| render_storage_config(registry, role))
    {
        write_file(
            output_dir.join("etc/storage.toml"),
            storage_config,
            output_dir,
            &mut files,
        )?;
    }
    if let Some(registry) = registry_value.as_ref()
        && let Some(node_control_config) = render_node_control_config(registry, role)?
    {
        write_file(
            output_dir.join("etc/node-control.toml"),
            node_control_config,
            output_dir,
            &mut files,
        )?;
        write_file(
            output_dir.join("etc/systemd/cynic-node.service"),
            include_str!("../../../../infra/systemd/installed/cynic-node.service").to_string(),
            output_dir,
            &mut files,
        )?;
    }
    write_executable_file(
        output_dir.join("install.sh"),
        bootstrap_install_script(&bootstrap_runtime_artifacts),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("verify.sh"),
        bootstrap_verify_script(&bootstrap_runtime_artifacts),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("enroll.sh"),
        bootstrap_enroll_script(),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("security-maintenance.sh"),
        bootstrap_security_maintenance_script(),
        output_dir,
        &mut files,
    )?;
    write_file(
        output_dir.join("security-policy.json"),
        serde_json::to_string_pretty(&json!({
            "schema": "cynic.framework_bootstrap.security_policy.v1",
            "runtime_user": "cynic",
            "runtime_group": "cynic",
            "helper_target": "/usr/local/libexec/cynic-host-exec",
            "sudoers_target": "/etc/sudoers.d/cynic-host-exec",
            "allowed_invocations": [
                ["/usr/local/libexec/cynic-host-exec", "bridge-status"],
                ["/usr/local/libexec/cynic-host-exec", "principal-prepare"],
                ["/usr/local/libexec/cynic-host-exec", "hardware-baseline-enroll", "--apply"],
                ["/usr/local/libexec/cynic-host-exec", "service-reconcile", "--apply"],
                ["/usr/local/libexec/cynic-host-exec", "network-baseline", "observe"],
                ["/usr/local/libexec/cynic-host-exec", "network-actuate"]
            ],
            "denied_classes": ["shell", "raw_systemctl", "raw_package_manager", "unbounded_helper"],
            "bridge_manifest": bridge_manifest
        }))
        .map_err(|error| format!("failed to serialize security policy: {error}"))?,
        output_dir,
        &mut files,
    )?;
    write_file(
        output_dir.join("runtime-basket.json"),
        include_str!("../../../../infra/runtime-basket.json").to_string(),
        output_dir,
        &mut files,
    )?;
    let seed_manifest = write_seed_manifest(output_dir, &files, role)?;
    files.push("seed-manifest.json".to_string());
    write_seed_checksums(output_dir, &files)?;
    files.push("seed-artifacts.sha256".to_string());

    Ok(FrameworkBootstrapBundleReport {
        schema: "cynic.framework_bootstrap.bundle.v2",
        status: "written",
        mutates: false,
        output_dir: output_dir.display().to_string(),
        files,
        required_role: role.to_string(),
        missing_artifacts: Vec::new(),
        blockers: Vec::new(),
        seed_id: seed_manifest.seed_id,
        seed_manifest: "seed-manifest.json".to_string(),
        entrypoint: "enroll.sh".to_string(),
            operator_command: "use framework propagation with the registry-declared MachineEnrollment transport; subsequent mutation uses the installed machine authority and signed release surfaces".to_string(),
        maintenance_command: "sudo /tmp/cynic-framework-bootstrap/security-maintenance.sh".to_string(),
        target_runtime: "fresh_linux_node",
        security_invariants: vec![
            "runtime user is system principal cynic with nologin shell",
            "sudoers grants only exact cynic-host-exec subcommands",
            "bootstrap installs minimal OS packages through the detected package manager when missing",
            "sudoers is validated with visudo before verification",
            "direct shell/systemctl/package-manager sudo paths must remain denied",
            "standalone cynic entrypoint must not require CYNIC_PROJECT_ROOT or Python organs",
            "verification emits evidence under /var/lib/cynic/bootstrap",
            "install verifies every seed artifact checksum before host mutation",
        ],
    })
}

fn render_storage_config(registry: &serde_json::Value, target_role: &str) -> Option<String> {
    let node = registry.get("nodes")?.as_array()?.iter().find(|node| {
        node.get("role").and_then(serde_json::Value::as_str) == Some("storage_primary")
    })?;
    let service = node.get("services")?.as_array()?.iter().find(|service| {
        service.get("name").and_then(serde_json::Value::as_str) == Some("surrealdb")
    })?;
    let scheme = service.get("scheme")?.as_str()?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let port = service.get("port")?.as_u64()?;
    let host = if target_role == "storage_primary" {
        "127.0.0.1"
    } else {
        node.pointer("/transports/ssh/target")?.as_str()?
    };
    if host.is_empty()
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b':'))
    {
        return None;
    }
    let allow_loopback = target_role == "storage_primary";
    Some(format!(
        "# Generated from infra/registry.json placement; contains no secret.\n[storage]\nurl = \"{scheme}://{host}:{port}\"\nallow_loopback = {allow_loopback}\nnamespace = \"cynic\"\ndatabase = \"main\"\nusername = \"cynic-runtime\"\nsecret_file = \"/etc/cynic/secrets/storage-runtime-password\"\n"
    ))
}

fn validate_runtime_seed_boundary(basket: &RuntimeBasket) -> Result<(), String> {
    if basket.runtime_seed.binary != "cynic-host-exec"
        || basket.runtime_seed.contract_revision == 0
        || basket.runtime_seed.signature_namespace
            != cynic_protocol::RUNTIME_RELEASE_SIGNATURE_NAMESPACE
        || basket.runtime_seed.runtime_basket_member
        || basket.artifacts.iter().any(|artifact| {
            artifact.binary == basket.runtime_seed.binary
                || artifact.binary.is_empty()
                || !artifact
                    .binary
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                || !Path::new(&artifact.install_path).is_absolute()
                || Path::new(&artifact.install_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    != Some(artifact.binary.as_str())
                || artifact.install_path.contains('\n')
        })
        || basket
            .artifacts
            .iter()
            .enumerate()
            .any(|(index, artifact)| {
                basket.artifacts[index + 1..].iter().any(|candidate| {
                    candidate.binary == artifact.binary
                        || candidate.install_path == artifact.install_path
                })
            })
    {
        return Err(
            "runtime seed must be independently governed and excluded from mutable baskets"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;

<<<<<<< HEAD
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::bridge;
use crate::framework_bootstrap::bundle::files::{
    copy_registry, copy_runtime_artifact, copy_standalone_cli_if_available, write_executable_file,
    write_file,
};
use crate::framework_bootstrap::bundle::seed::{write_seed_checksums, write_seed_manifest};
use crate::framework_bootstrap::scripts::{
    BootstrapRuntimeArtifact, bootstrap_enroll_script, bootstrap_install_script,
    bootstrap_security_maintenance_script, bootstrap_verify_script,
};
use crate::framework_bootstrap::types::FrameworkBootstrapBundleReport;

mod files;
mod node_control;
mod seed;

use node_control::render_node_control_config;
=======
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::bridge;
use crate::framework_bootstrap::scripts::{
    bootstrap_install_script, bootstrap_security_maintenance_script, bootstrap_verify_script,
};
use crate::framework_bootstrap::types::{
    FrameworkBootstrapBundleReport, FrameworkSeedArtifact, FrameworkSeedEnrollment,
    FrameworkSeedManifest,
};
>>>>>>> github/main

pub fn write_framework_bootstrap_bundle(
    output_dir: &str,
) -> Result<FrameworkBootstrapBundleReport, String> {
<<<<<<< HEAD
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
=======
    let output_dir = Path::new(output_dir);
>>>>>>> github/main
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

<<<<<<< HEAD
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
=======
    copy_standalone_cli_if_available(output_dir, &mut files)?;
    write_executable_file(
        output_dir.join("install.sh"),
        bootstrap_install_script(),
>>>>>>> github/main
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("verify.sh"),
<<<<<<< HEAD
        bootstrap_verify_script(&bootstrap_runtime_artifacts),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("enroll.sh"),
        bootstrap_enroll_script(),
=======
        bootstrap_verify_script(),
>>>>>>> github/main
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
<<<<<<< HEAD
    let seed_manifest = write_seed_manifest(output_dir, &files, role)?;
=======
    let seed_manifest = write_seed_manifest(output_dir, &files)?;
>>>>>>> github/main
    files.push("seed-manifest.json".to_string());
    write_seed_checksums(output_dir, &files)?;
    files.push("seed-artifacts.sha256".to_string());

    Ok(FrameworkBootstrapBundleReport {
        schema: "cynic.framework_bootstrap.bundle.v2",
        status: "written",
        mutates: false,
        output_dir: output_dir.display().to_string(),
        files,
<<<<<<< HEAD
        required_role: role.to_string(),
        missing_artifacts: Vec::new(),
        blockers: Vec::new(),
        seed_id: seed_manifest.seed_id,
        seed_manifest: "seed-manifest.json".to_string(),
        entrypoint: "enroll.sh".to_string(),
            operator_command: "use framework propagation with the registry-declared MachineEnrollment transport; subsequent mutation uses the installed machine authority and signed release surfaces".to_string(),
=======
        seed_id: seed_manifest.seed_id,
        seed_manifest: "seed-manifest.json".to_string(),
        entrypoint: "install.sh".to_string(),
        operator_command: "explicit bootstrap transport enrollment is required; SSH may transfer the initial seed but is not a canonical runtime mutation plane".to_string(),
>>>>>>> github/main
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

<<<<<<< HEAD
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
=======
fn write_seed_checksums(output_dir: &Path, files: &[String]) -> Result<(), String> {
    let mut entries = files
        .iter()
        .map(|relative| {
            let artifact = seed_artifact(output_dir, relative)?;
            Ok(format!(
                "{}  {}",
                artifact.sha256.trim_start_matches("sha256:"),
                artifact.path
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort();
    fs::write(
        output_dir.join("seed-artifacts.sha256"),
        format!("{}\n", entries.join("\n")),
    )
    .map_err(|error| format!("failed to write seed checksum projection: {error}"))
}

fn write_seed_manifest(
    output_dir: &Path,
    files: &[String],
) -> Result<FrameworkSeedManifest, String> {
    let mut artifacts = files
        .iter()
        .map(|relative| seed_artifact(output_dir, relative))
        .collect::<Result<Vec<_>, _>>()?;
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    let identity = serde_json::to_vec(&artifacts)
        .map_err(|error| format!("failed to serialize seed artifact identity: {error}"))?;
    let seed_id = format!("sha256:{}", hex::encode(Sha256::digest(identity)));
    let manifest = FrameworkSeedManifest {
        schema: "cynic.framework_seed.v2",
        seed_id,
        status: "seed_only",
        artifacts,
        enrollment: FrameworkSeedEnrollment {
            required: true,
            target_role: "unassigned",
            identity_class: "MachineEnrollment",
            token_material: "injected_at_apply_never_embedded",
            admission_status: "external_authority_required",
        },
        proof_contract: vec![
            "host_identity_observed",
            "host_bootstrap_target_satisfied",
            "runtime_principal_active",
            "bridge_status_observed",
        ],
        rollback_intent: "reject enrollment and remove installed seed projection before activating a runtime role",
    };
    fs::write(
        output_dir.join("seed-manifest.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest)
                .map_err(|error| format!("failed to serialize seed manifest: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write seed manifest: {error}"))?;
    Ok(manifest)
}

fn seed_artifact(output_dir: &Path, relative: &str) -> Result<FrameworkSeedArtifact, String> {
    let path = output_dir.join(relative);
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("failed to stat seed artifact {}: {error}", path.display()))?;
    let content = fs::read(&path)
        .map_err(|error| format!("failed to read seed artifact {}: {error}", path.display()))?;
    Ok(FrameworkSeedArtifact {
        path: relative.to_string(),
        sha256: format!("sha256:{}", hex::encode(Sha256::digest(content))),
        mode: format!("{:04o}", metadata.permissions().mode() & 0o7777),
    })
}

fn copy_standalone_cli_if_available(
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to locate current executable: {error}"))?;
    if current_exe.file_name().and_then(|name| name.to_str()) != Some("cynic") {
        return Ok(());
    }

    let target = output_dir.join("bin/cynic");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create standalone cli dir: {error}"))?;
    }
    fs::copy(&current_exe, &target).map_err(|error| {
        format!(
            "failed to copy standalone cynic binary from {} to {}: {error}",
            current_exe.display(),
            target.display()
        )
    })?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", target.display()))?;
    files.push("bin/cynic".to_string());
    Ok(())
}

fn write_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(output_dir)
        .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
    files.push(relative.display().to_string());
    Ok(())
}

fn write_executable_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    write_file(path.clone(), content, output_dir, files)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn framework_bootstrap_bundle_writes_security_kit() {
        let out = std::env::temp_dir().join(format!(
            "cynic-framework-bootstrap-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));

        let report = super::write_framework_bootstrap_bundle(out.to_str().expect("path"))
            .expect("framework bootstrap bundle");

        assert_eq!(report.status, "written");
        assert!(out.join("host-exec-bundle/cynic-host-exec").is_file());
        assert!(out.join("install.sh").is_file());
        let install_script =
            std::fs::read_to_string(out.join("install.sh")).expect("read install.sh");
        assert!(install_script.contains("principal-prepare --apply"));
        assert!(install_script.contains("/usr/local/bin/cynic"));
        assert!(!install_script.contains("/home/$RUNTIME_USER"));
        assert!(!install_script.contains("CYNIC_PROJECT_ROOT"));
        assert!(!install_script.contains("organs/supervisor"));
        assert!(!install_script.contains("runtime_principals.py"));
        assert!(!install_script.contains("groupadd"));
        assert!(!install_script.contains("useradd"));
        assert!(out.join("verify.sh").is_file());
        assert!(out.join("security-maintenance.sh").is_file());
        assert!(out.join("security-policy.json").is_file());
        assert!(out.join("runtime-basket.json").is_file());
        assert!(out.join("seed-manifest.json").is_file());
        assert!(out.join("seed-artifacts.sha256").is_file());
        let manifest = std::fs::read_to_string(out.join("host-exec-bundle/manifest.json"))
            .expect("read bridge manifest");
        let security_policy =
            std::fs::read_to_string(out.join("security-policy.json")).expect("read policy");
        let runtime_basket =
            std::fs::read_to_string(out.join("runtime-basket.json")).expect("read runtime basket");
        assert!(!manifest.contains(out.to_string_lossy().as_ref()));
        assert!(!security_policy.contains(out.to_string_lossy().as_ref()));
        assert!(!manifest.contains("/home/"));
        assert!(!security_policy.contains("/home/"));
        let basket: serde_json::Value =
            serde_json::from_str(&runtime_basket).expect("parse runtime basket");
        assert_eq!(basket["schema"], "cynic.runtime_basket.v1");
        assert!(basket["artifacts"]
            .as_array()
            .expect("basket artifacts")
            .iter()
            .any(|artifact| artifact["binary"] == "cynic-node"));
        let seed: serde_json::Value = serde_json::from_slice(
            &std::fs::read(out.join("seed-manifest.json")).expect("read seed manifest"),
        )
        .expect("parse seed manifest");
        assert_eq!(seed["schema"], "cynic.framework_seed.v2");
        assert_eq!(seed["status"], "seed_only");
        assert_eq!(
            seed["enrollment"]["token_material"],
            "injected_at_apply_never_embedded"
        );
        assert!(
            seed["artifacts"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        );
        assert!(!seed.to_string().contains(out.to_string_lossy().as_ref()));
        let checksums = std::fs::read_to_string(out.join("seed-artifacts.sha256"))
            .expect("read seed checksums");
        assert!(checksums.contains("  install.sh"));
        assert!(checksums.contains("  seed-manifest.json"));
        assert!(!checksums.contains(out.to_string_lossy().as_ref()));
        std::fs::remove_dir_all(out).expect("cleanup");
    }
}
>>>>>>> github/main

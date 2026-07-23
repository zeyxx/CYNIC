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

pub fn write_framework_bootstrap_bundle(
    output_dir: &str,
) -> Result<FrameworkBootstrapBundleReport, String> {
    let output_dir = Path::new(output_dir);
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

    copy_standalone_cli_if_available(output_dir, &mut files)?;
    write_executable_file(
        output_dir.join("install.sh"),
        bootstrap_install_script(),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("verify.sh"),
        bootstrap_verify_script(),
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
    let seed_manifest = write_seed_manifest(output_dir, &files)?;
    files.push("seed-manifest.json".to_string());
    write_seed_checksums(output_dir, &files)?;
    files.push("seed-artifacts.sha256".to_string());

    Ok(FrameworkBootstrapBundleReport {
        schema: "cynic.framework_bootstrap.bundle.v2",
        status: "written",
        mutates: false,
        output_dir: output_dir.display().to_string(),
        files,
        seed_id: seed_manifest.seed_id,
        seed_manifest: "seed-manifest.json".to_string(),
        entrypoint: "install.sh".to_string(),
        operator_command: "explicit bootstrap transport enrollment is required; SSH may transfer the initial seed but is not a canonical runtime mutation plane".to_string(),
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

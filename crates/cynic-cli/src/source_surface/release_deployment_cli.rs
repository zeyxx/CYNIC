//! CLI executor for atomic release deployment via cynic-pkg.

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_pkg::{
    apply_release_deployment, plan_release_deployment, DeployBinarySpec,
    LocalReleaseDeployExecutor, ReleaseDeploySpec,
};

use crate::cli_json::emit_pretty_json;

pub(crate) fn execute_deploy(
    root: &Path,
    state_root: &Path,
    version_override: Option<&str>,
    apply: bool,
    json: bool,
) -> ExitCode {
    let resolved_version = if let Some(v) = version_override {
        v.to_string()
    } else {
        // Parse package.version from Cargo.toml in root
        let manifest_path = root.join("Cargo.toml");
        match fs::read_to_string(&manifest_path) {
            Ok(content) => {
                let mut ver = None;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("version") && trimmed.contains('=') {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            ver = Some(val.trim().trim_matches('"').to_string());
                            break;
                        }
                    }
                }
                match ver {
                    Some(v) => v,
                    None => {
                        eprintln!("blocked: could not parse version from {}", manifest_path.display());
                        return ExitCode::FAILURE;
                    }
                }
            }
            Err(e) => {
                eprintln!("blocked: read manifest {}: {e}", manifest_path.display());
                return ExitCode::FAILURE;
            }
        }
    };

    let target_release_dir = root.join("target").join("release");
    let binary_names = ["cynic", "cynic-node", "cynic-node-enroll"];
    let mut binaries = Vec::new();

    for name in binary_names {
        binaries.push(DeployBinarySpec {
            name: name.to_string(),
            source_path: target_release_dir.join(name),
        });
    }

    let spec = ReleaseDeploySpec {
        version: resolved_version,
        state_root: state_root.to_path_buf(),
        binaries,
    };

    let plan = match plan_release_deployment(&spec) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("blocked: release deployment planning failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !apply {
        if json {
            emit_pretty_json(&serde_json::json!({
                "status": "planned",
                "schema": "cynic.pkg.release_deploy.plan.v1",
                "plan": plan,
                "note": "pass --apply to execute blue-green atomic switch"
            }))
        } else {
            println!("Planned deployment for {}:", plan.release_tag);
            println!("  Release Dir:   {}", plan.release_dir.display());
            println!("  Current Link:  {} -> {}", plan.current_link.display(), plan.release_dir.display());
            println!("  Bin Dir:       {}", plan.bin_dir.display());
            for (name, target) in &plan.binary_targets {
                println!("    {} -> {}", name, target.display());
            }
            println!("\nPass --apply to execute atomic deployment.");
            ExitCode::SUCCESS
        }
    } else {
        let now_unix_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let executor = LocalReleaseDeployExecutor;
        match apply_release_deployment(&spec, &plan, &executor, now_unix_s) {
            Ok(report) => {
                if json {
                    emit_pretty_json(&report)
                } else {
                    println!("Successfully deployed {} (status: {})", report.release_tag, report.status);
                    println!("  Release Dir:  {}", report.release_dir.display());
                    println!("  Current Target: {}", report.active_current_target.display());
                    println!("  Binaries:     {:?}", report.deployed_binaries);
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("failed: release deployment actuation failed: {e}");
                ExitCode::FAILURE
            }
        }
    }
}

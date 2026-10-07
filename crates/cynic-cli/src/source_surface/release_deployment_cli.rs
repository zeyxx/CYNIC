//! CLI executor for atomic release deployment via cynic-pkg.

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_pkg::{
    DeployBinarySpec, LocalReleaseDeployExecutor, ReleaseDeploySpec, RestartUnitSpec,
    apply_release_deployment, plan_release_deployment,
};

use crate::cli_json::emit_pretty_json;

pub(crate) fn execute_deploy(
    root: &Path,
    state_root: &Path,
    version_override: Option<&str>,
    apply: bool,
    json: bool,
    restart_units_raw: &[String],
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
                        eprintln!(
                            "blocked: could not parse version from {}",
                            manifest_path.display()
                        );
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
        restart_units: match parse_restart_units(restart_units_raw) {
            Ok(units) => units,
            Err(e) => {
                eprintln!("blocked: {e}");
                return ExitCode::FAILURE;
            }
        },
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
            println!(
                "  Current Link:  {} -> {}",
                plan.current_link.display(),
                plan.release_dir.display()
            );
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
                    println!(
                        "Successfully deployed {} (status: {})",
                        report.release_tag, report.status
                    );
                    println!("  Release Dir:  {}", report.release_dir.display());
                    println!(
                        "  Current Target: {}",
                        report.active_current_target.display()
                    );
                    println!("  Binaries:     {:?}", report.deployed_binaries);
                    if report.activated_units.is_empty() {
                        println!(
                            "  Activation:   STAGED ONLY — no daemon restarted (pass --restart-unit to activate)"
                        );
                    } else {
                        println!("  Activation:");
                        for unit in &report.activated_units {
                            println!(
                                "    {} -> {} (health: {})",
                                unit.unit,
                                unit.active_state,
                                unit.health_status
                                    .map(|c| c.to_string())
                                    .unwrap_or_else(|| "no probe".to_string())
                            );
                        }
                    }
                    if let Some(previous) = &report.rollback_target {
                        println!("  Rollback to:  {}", previous.display());
                    }
                    if !report.pruned_dirs.is_empty() {
                        println!(
                            "  Pruned:       {} old release(s)",
                            report.pruned_dirs.len()
                        );
                    }
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

/// Parse repeatable `--restart-unit unit[:health_url]` flags.
///
/// The unit name is validated as a plain `*.service` identifier here, before
/// it ever reaches systemctl: no paths, no targets, no shell. Health URLs
/// are restricted to `http://` (loopback probes, no TLS stack in the actor).
fn parse_restart_units(raw: &[String]) -> Result<Vec<RestartUnitSpec>, String> {
    let mut units = Vec::new();
    for entry in raw {
        // `unit.service:http://127.0.0.1:port/path` — split on the `.service:`
        // boundary so the health URL keeps its own colons (host:port).
        let (unit, health_url) = match entry.split_once(".service:") {
            Some((stem, health)) if !stem.is_empty() && !health.is_empty() => {
                (format!("{stem}.service"), health.to_string())
            }
            _ => (entry.clone(), String::new()),
        };
        if !is_plain_service_unit(&unit) {
            return Err(format!(
                "refusing restart unit {unit:?}: must be a plain <name>.service identifier"
            ));
        }
        if !health_url.is_empty()
            && !health_url.starts_with("http://127.")
            && !health_url.starts_with("http://localhost")
        {
            return Err(format!(
                "refusing health URL {health_url:?}: must be a loopback http:// probe"
            ));
        }
        units.push(RestartUnitSpec {
            unit,
            health_url,
            health_timeout_secs: 30,
        });
    }
    Ok(units)
}

/// A plain `*.service` unit identifier: no path, no flags, no shell.
/// Mirrors the executor-side guard in cynic-pkg (defense in depth: the CLI
/// refuses early with a usage error, the executor refuses late with an
/// actuation error).
fn is_plain_service_unit(unit: &str) -> bool {
    !unit.is_empty()
        && unit.len() <= 128
        && unit.ends_with(".service")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@')
        && !unit.contains("..")
        && !unit.contains('/')
}

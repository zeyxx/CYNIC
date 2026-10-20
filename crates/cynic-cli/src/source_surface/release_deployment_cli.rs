//! CLI executor for atomic release deployment via cynic-pkg.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_pkg::{
    DeployBinarySpec, LocalReleaseDeployExecutor, ReleaseDeploySpec, RestartUnitSpec,
    apply_release_deployment, plan_release_deployment,
};

use crate::cli_json::emit_pretty_json;

/// Resolve the cargo target directory that holds the built release binaries.
///
/// Authority: `infra/directory-tree.json` (`build-targets/` under the state
/// root). Same rule as the Makefile: an explicit `CARGO_TARGET_DIR` wins;
/// otherwise `<state-root>/build-targets/<profile>` (`CYNIC_CARGO_TARGET_PROFILE`
/// or `default`). A bare-cargo dev tree (`<root>/target`) is the last resort
/// and only when it exists — the chosen directory is reported, never guessed
/// silently, so a deploy can never collect stale binaries from the wrong tree.
pub(crate) fn resolve_target_release_dir(root: &Path, state_root: &Path) -> PathBuf {
    if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir).join("release");
        }
    }
    let profile = env::var("CYNIC_CARGO_TARGET_PROFILE")
        .ok()
        .filter(|profile| !profile.trim().is_empty())
        .unwrap_or_else(|| "default".to_string());
    let ssot = state_root
        .join("build-targets")
        .join(&profile)
        .join("release");
    if ssot.is_dir() {
        return ssot;
    }
    let legacy = root.join("target").join("release");
    if legacy.is_dir() {
        return legacy;
    }
    ssot
}

/// Resolve the deployable binary set from the release-binaries registry.
///
/// Authority: `infra/release-binaries.json`. The deployed set used to be a
/// literal list inside this file while the Makefile built a different set —
/// the Makefile built and installed `cynic`, `cynic-node` and
/// `cynic-kernel`, and this executor installed only the first two. A
/// release that declares one set and ships another is a silent divergence.
///
/// The registry is required, not optional: a missing or malformed
/// registry refuses the deploy rather than falling back to a guess, because
/// the fallback is exactly the divergence this read removes.
pub(crate) fn resolve_deployable_binaries(root: &Path) -> Result<Vec<String>, String> {
    let registry_path = root.join("infra/release-binaries.json");
    let raw = fs::read_to_string(&registry_path)
        .map_err(|error| format!("read {}: {error}", registry_path.display()))?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("parse {}: {error}", registry_path.display()))?;
    let entries = parsed
        .get("binaries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            format!(
                "{}: no binaries array — the deployable set is undeclared",
                registry_path.display()
            )
        })?;
    let mut names = Vec::new();
    for entry in entries {
        let deployable = entry
            .get("deploy")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if !deployable {
            continue;
        }
        let name = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                format!(
                    "{}: a binary entry has no usable name",
                    registry_path.display()
                )
            })?;
        names.push(name.to_string());
    }
    if names.is_empty() {
        return Err(format!(
            "{}: the deployable set is empty",
            registry_path.display()
        ));
    }
    Ok(names)
}

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

    let target_release_dir = resolve_target_release_dir(root, state_root);
    eprintln!(
        "collecting release binaries from {}",
        target_release_dir.display()
    );
    let binary_names = match resolve_deployable_binaries(root) {
        Ok(names) => names,
        Err(error) => {
            eprintln!("blocked: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut binaries = Vec::new();

    for name in &binary_names {
        binaries.push(DeployBinarySpec {
            name: name.clone(),
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

#[cfg(test)]
mod tests {
    use super::resolve_deployable_binaries;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cynic-release-bins-{}-{label}",
            std::process::id()
        ));
        fs::create_dir_all(dir.join("infra")).expect("temp infra dir");
        dir
    }

    /// The deployed set and the built set must be the same set. The
    /// Makefile builds and installs `cynic`, `cynic-node` and
    /// `cynic-kernel`; the deploy executor used to install only the first
    /// two, so a release declared three binaries and shipped two.
    #[test]
    fn the_registry_declares_the_built_binary_set() {
        let root = temp_root("built-set");
        fs::write(
            root.join("infra/release-binaries.json"),
            r#"{
              "binaries": [
                {"name": "cynic", "deploy": true},
                {"name": "cynic-node", "deploy": true},
                {"name": "cynic-kernel", "deploy": true}
              ]
            }"#,
        )
        .expect("write registry");

        let names = resolve_deployable_binaries(&root).expect("resolve binaries");
        assert_eq!(names, vec!["cynic", "cynic-node", "cynic-kernel"]);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_non_deployable_entry_is_not_installed() {
        let root = temp_root("not-deployable");
        fs::write(
            root.join("infra/release-binaries.json"),
            r#"{
              "binaries": [
                {"name": "cynic", "deploy": true},
                {"name": "cynic-docs", "deploy": false}
              ]
            }"#,
        )
        .expect("write registry");

        let names = resolve_deployable_binaries(&root).expect("resolve binaries");
        assert_eq!(names, vec!["cynic"]);
        fs::remove_dir_all(&root).ok();
    }

    /// A missing registry refuses the deploy. The fallback to a literal
    /// list is the divergence this read removes.
    #[test]
    fn a_missing_registry_refuses_rather_than_guesses() {
        let root = temp_root("missing");
        let error = resolve_deployable_binaries(&root)
            .expect_err("a missing registry must refuse");
        assert!(
            error.contains("release-binaries.json"),
            "the refusal must name the missing registry: {error}"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_malformed_registry_refuses_rather_than_guesses() {
        let root = temp_root("malformed");
        fs::write(root.join("infra/release-binaries.json"), "{ not json").expect("write registry");
        assert!(resolve_deployable_binaries(&root).is_err());

        fs::write(root.join("infra/release-binaries.json"), r#"{"binaries": []}"#)
            .expect("write registry");
        assert!(resolve_deployable_binaries(&root).is_err());

        fs::write(
            root.join("infra/release-binaries.json"),
            r#"{"binaries": [{"deploy": true}]}"#,
        )
        .expect("write registry");
        assert!(resolve_deployable_binaries(&root).is_err());

        fs::remove_dir_all(&root).ok();
    }

    /// The registry the repository ships is the one the Makefile builds.
    #[test]
    fn the_shipped_registry_matches_the_makefile_binary_set() {
        let root = crate::rust_source::workspace_root();
        let names = resolve_deployable_binaries(&root).expect("shipped registry resolves");
        assert!(
            names.contains(&"cynic-kernel".to_string()),
            "cynic-kernel is built by the Makefile and must be deployed: {names:?}"
        );
        assert!(names.contains(&"cynic".to_string()));
        assert!(names.contains(&"cynic-node".to_string()));
    }
}

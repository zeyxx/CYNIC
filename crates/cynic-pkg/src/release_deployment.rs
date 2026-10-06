//! Sovereign release deployment and blue-green activation.
//!
//! Provides pure planning, atomic symlink-based blue-green activation,
//! and read-back proof generation for CYNIC binary releases.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub const RELEASE_DEPLOY_REPORT_SCHEMA: &str = "cynic.pkg.release_deploy.report.v1";

/// Systemd unit to restart after the symlink switch, with its health probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartUnitSpec {
    /// systemd unit name, e.g. `cynic-kernel.service`. Must be declared here:
    /// undeclared units are never restarted.
    pub unit: String,
    /// Health probe answered after restart, e.g. `http://127.0.0.1:3030/health`.
    /// Empty means "active-state read-back only".
    pub health_url: String,
    /// Seconds to wait for the health probe before declaring failure.
    pub health_timeout_secs: u64,
}

/// Specification of a binary to be deployed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployBinarySpec {
    pub name: String,
    pub source_path: PathBuf,
}

/// Specification for a release deployment.
///
/// `restart_units` implements `contracts/runtime-update.json` steps 5-7:
/// without declared units the release stays staged (files placed, symlink
/// switched) but no daemon is touched — placement without activation is not
/// a deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeploySpec {
    pub version: String,
    pub state_root: PathBuf,
    pub binaries: Vec<DeployBinarySpec>,
    #[serde(default)]
    pub restart_units: Vec<RestartUnitSpec>,
}

/// Planned actions for the release deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeployPlan {
    pub release_tag: String,
    pub release_dir: PathBuf,
    pub current_link: PathBuf,
    pub bin_dir: PathBuf,
    pub binary_targets: Vec<(String, PathBuf)>,
}

/// Read-back proof of an executed release deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeployReport {
    pub schema: String,
    pub status: String,
    pub release_tag: String,
    pub release_dir: PathBuf,
    pub active_current_target: PathBuf,
    pub deployed_binaries: Vec<String>,
    pub deployed_at_unix_s: u64,
    /// Per-unit activation outcome. Empty when `spec.restart_units` is empty
    /// (staged but not activated — placement without activation).
    #[serde(default)]
    pub activated_units: Vec<ActivatedUnitReport>,
    /// Previous release the rollback would return to (`None` on first deploy).
    #[serde(default)]
    pub rollback_target: Option<PathBuf>,
    /// Release dirs pruned by GC after convergence (never the rollback target).
    #[serde(default)]
    pub pruned_dirs: Vec<PathBuf>,
}

/// Activation outcome for one declared unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivatedUnitReport {
    pub unit: String,
    pub active_state: String,
    /// HTTP status observed on the health probe, if one was declared.
    pub health_status: Option<u16>,
}

/// Purely plan a release deployment layout without filesystem mutations.
pub fn plan_release_deployment(spec: &ReleaseDeploySpec) -> Result<ReleaseDeployPlan, String> {
    if spec.version.trim().is_empty() {
        return Err("release deployment: version cannot be empty".to_string());
    }

    let release_tag = if spec.version.starts_with('v') {
        spec.version.clone()
    } else {
        format!("v{}", spec.version)
    };

    let release_dir = spec.state_root.join("releases").join(&release_tag);
    let current_link = spec.state_root.join("current");
    let bin_dir = spec.state_root.join("bin");

    let mut binary_targets = Vec::new();
    for bin in &spec.binaries {
        if bin.name.trim().is_empty() {
            return Err("release deployment: binary name cannot be empty".to_string());
        }
        let target = release_dir.join(&bin.name);
        binary_targets.push((bin.name.clone(), target));
    }

    Ok(ReleaseDeployPlan {
        release_tag,
        release_dir,
        current_link,
        bin_dir,
        binary_targets,
    })
}

/// Port for release deployment actuation: filesystem placement plus bounded
/// daemon activation.
///
/// The filesystem methods are pure placement. The service methods are the
/// activation half of `contracts/runtime-update.json`: restart a DECLARED
/// unit, read back its active state, and probe health. Implementations must
/// refuse any unit outside the plan's declared list.
pub trait ReleaseDeployExecutorPort {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn copy_file(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
    fn atomic_symlink(&self, target: &Path, link_path: &Path) -> std::io::Result<()>;
    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf>;
    /// Restart a declared unit. Fails when `unit` is not in the plan.
    /// Returns the unit's active state after restart (e.g. `active`).
    fn restart_unit(&self, unit: &str) -> Result<String, String>;
    /// Probe an HTTP health endpoint until it answers 2xx or `timeout_secs`
    /// elapse. Returns the HTTP status code observed.
    fn probe_health(&self, url: &str, timeout_secs: u64) -> Result<u16, String>;
    /// Remove a previous release directory (GC). Never removes the directory
    /// currently pointed at by `current_link`.
    fn remove_release_dir(&self, path: &Path) -> std::io::Result<()>;
    /// List release directories under `<state_root>/releases`, oldest first.
    fn list_release_dirs(&self, state_root: &Path) -> std::io::Result<Vec<PathBuf>>;
}

/// Default local executor implementing blue-green deployment via POSIX rename.
pub struct LocalReleaseDeployExecutor;

impl ReleaseDeployExecutorPort for LocalReleaseDeployExecutor {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn copy_file(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        let parent = to.parent().unwrap_or_else(|| Path::new("."));
        let temp_target = parent.join(format!(
            ".tmp-bin-{}-{}",
            std::process::id(),
            to.file_name().and_then(|n| n.to_str()).unwrap_or("bin")
        ));
        let _ = fs::remove_file(&temp_target);
        let bytes = fs::copy(from, &temp_target)?;
        fs::rename(&temp_target, to)?;
        Ok(bytes)
    }

    fn atomic_symlink(&self, target: &Path, link_path: &Path) -> std::io::Result<()> {
        let parent = link_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_link = parent.join(format!(
            ".tmp-symlink-{}-{}",
            std::process::id(),
            link_path.file_name().and_then(|n| n.to_str()).unwrap_or("link")
        ));

        let _ = fs::remove_file(&temp_link);
        symlink(target, &temp_link)?;
        fs::rename(&temp_link, link_path)?;
        Ok(())
    }

    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf> {
        fs::read_link(path)
    }

    fn restart_unit(&self, unit: &str) -> Result<String, String> {
        // Bounded actuation: exactly one systemctl verb, one declared unit.
        // No shell, no flags, no extra operands — the unit name is validated
        // as a plain `*.service` identifier before it reaches the kernel.
        if !is_plain_service_unit(unit) {
            return Err(format!("refusing to restart non-service unit: {unit}"));
        }
        let status = std::process::Command::new("systemctl")
            .arg("restart")
            .arg(unit)
            .status()
            .map_err(|e| format!("systemctl restart {unit}: {e}"))?;
        if !status.success() {
            return Err(format!("systemctl restart {unit} exited {status}"));
        }
        // Read-back: the unit must report active after restart.
        let output = std::process::Command::new("systemctl")
            .arg("is-active")
            .arg(unit)
            .output()
            .map_err(|e| format!("systemctl is-active {unit}: {e}"))?;
        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if state != "active" {
            return Err(format!("unit {unit} state after restart: '{state}' (expected active)"));
        }
        Ok(state)
    }

    fn probe_health(&self, url: &str, timeout_secs: u64) -> Result<u16, String> {
        if url.trim().is_empty() {
            return Ok(200);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs.max(1));
        loop {
            match http_status_code(url) {
                Ok(code) if (200..300).contains(&code) => return Ok(code),
                Ok(_) => {}
                Err(_) => {}
            }
            if std::time::Instant::now() >= deadline {
                return Err(format!("health probe timed out after {timeout_secs}s: {url}"));
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    fn remove_release_dir(&self, path: &Path) -> std::io::Result<()> {
        if !path.is_dir() {
            return Ok(());
        }
        // Never remove a directory outside a releases/ tree.
        if !path.components().any(|c| c.as_os_str() == "releases") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("refusing to remove non-release directory: {}", path.display()),
            ));
        }
        fs::remove_dir_all(path)
    }

    fn list_release_dirs(&self, state_root: &Path) -> std::io::Result<Vec<PathBuf>> {
        let releases = state_root.join("releases");
        let mut dirs = Vec::new();
        if let Ok(reader) = fs::read_dir(&releases) {
            for entry in reader.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                }
            }
        }
        dirs.sort();
        Ok(dirs)
    }
}

/// A plain `*.service` unit identifier: no path, no flags, no shell.
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

/// Minimal blocking HTTP status fetch without external crates.
fn http_status_code(url: &str) -> Result<u16, String> {
    let (host_port, path) = url
        .strip_prefix("http://")
        .map(|rest| {
            let mut parts = rest.splitn(2, '/');
            (parts.next().unwrap_or(""), format!("/{}", parts.next().unwrap_or("")))
        })
        .ok_or_else(|| format!("health probe supports http:// URLs only: {url}"))?;
    let mut host_parts = host_port.splitn(2, ':');
    let host = host_parts.next().unwrap_or("");
    let port: u16 = host_parts.next().unwrap_or("80").parse().map_err(|_| format!("bad port in {url}"))?;
    let addr = format!("{host}:{port}");
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("health connect {addr}: {e}"))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n")
        .map_err(|e| e.to_string())?;
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(&stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).map_err(|e| e.to_string())?;
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| format!("unparseable health response from {url}"))
}

/// Apply a planned release deployment using the injected executor port.
pub fn apply_release_deployment<E: ReleaseDeployExecutorPort>(
    spec: &ReleaseDeploySpec,
    plan: &ReleaseDeployPlan,
    executor: &E,
    now_unix_s: u64,
) -> Result<ReleaseDeployReport, String> {
    // 1. Create target release staging directory
    executor
        .create_dir_all(&plan.release_dir)
        .map_err(|e| format!("create release dir {}: {e}", plan.release_dir.display()))?;

    // 2. Stage binaries into the release directory
    let mut deployed_binaries = Vec::new();
    for (bin_spec, (_, target_path)) in spec.binaries.iter().zip(plan.binary_targets.iter()) {
        if !bin_spec.source_path.exists() {
            return Err(format!(
                "source binary not found: {}",
                bin_spec.source_path.display()
            ));
        }
        executor
            .copy_file(&bin_spec.source_path, target_path)
            .map_err(|e| {
                format!(
                    "copy binary from {} to {}: {e}",
                    bin_spec.source_path.display(),
                    target_path.display()
                )
            })?;
        deployed_binaries.push(bin_spec.name.clone());
    }

    // Remember the rollback target BEFORE the switch: the release `current`
    // points at right now (None on first deploy). Reading it after the
    // switch would always return the new release itself.
    let previous_target =
        read_previous_target(executor, &plan.current_link, &plan.release_dir);

    // 3. Atomically switch /current symlink to the new release
    executor
        .atomic_symlink(&plan.release_dir, &plan.current_link)
        .map_err(|e| {
            format!(
                "atomic symlink {} -> {}: {e}",
                plan.current_link.display(),
                plan.release_dir.display()
            )
        })?;

    // 4. Update /bin symlinks pointing through /current/<binary>
    executor
        .create_dir_all(&plan.bin_dir)
        .map_err(|e| format!("create bin dir {}: {e}", plan.bin_dir.display()))?;

    for bin_name in &deployed_binaries {
        let current_bin_target = plan.current_link.join(bin_name);
        let bin_link = plan.bin_dir.join(bin_name);
        executor
            .atomic_symlink(&current_bin_target, &bin_link)
            .map_err(|e| {
                format!(
                    "atomic symlink {} -> {}: {e}",
                    bin_link.display(),
                    current_bin_target.display()
                )
            })?;
    }

    // 5. Read back proof verification
    let active_current_target = executor
        .read_link(&plan.current_link)
        .map_err(|e| format!("read back symlink {}: {e}", plan.current_link.display()))?;

    if active_current_target != plan.release_dir {
        return Err(format!(
            "read back mismatch: expected current -> {} but got {}",
            plan.release_dir.display(),
            active_current_target.display()
        ));
    }

    // 6. Activation: restart declared units, probe health. Health failure
    // triggers rollback to the previous release (switch back + restart +
    // re-probe). A release that cannot prove health stays staged, never active.
    let mut activated_units = Vec::new();
    if !spec.restart_units.is_empty() {
        for unit_spec in &spec.restart_units {
            let active_state = executor.restart_unit(&unit_spec.unit).map_err(|e| {
                rollback(
                    executor,
                    &plan,
                    &previous_target,
                    &activated_units,
                    &format!("activation failed restarting {}: {e}", unit_spec.unit),
                )
            })?;
            let health_status = if unit_spec.health_url.trim().is_empty() {
                None
            } else {
                match executor.probe_health(&unit_spec.health_url, unit_spec.health_timeout_secs) {
                    Ok(code) => Some(code),
                    Err(e) => {
                        return Err(rollback(
                            executor,
                            &plan,
                            &previous_target,
                            &activated_units,
                            &format!("health probe failed for {}: {e}", unit_spec.unit),
                        ));
                    }
                }
            };
            activated_units.push(ActivatedUnitReport {
                unit: unit_spec.unit.clone(),
                active_state,
                health_status,
            });
        }
    }

    // 7. GC: keep current + two previous release dirs; prune older ones.
    // Never prune the rollback target.
    let mut pruned_dirs = Vec::new();
    if let Ok(mut dirs) = executor.list_release_dirs(&spec.state_root) {
        dirs.retain(|d| *d != plan.release_dir);
        if let Some(previous) = &previous_target {
            dirs.retain(|d| d != previous);
        }
        dirs.sort();
        while dirs.len() > 2 {
            let oldest = dirs.remove(0);
            executor.remove_release_dir(&oldest).map_err(|e| {
                format!("gc failed removing {}: {e}", oldest.display())
            })?;
            pruned_dirs.push(oldest);
        }
    }

    Ok(ReleaseDeployReport {
        schema: RELEASE_DEPLOY_REPORT_SCHEMA.to_string(),
        status: if spec.restart_units.is_empty() {
            "staged".to_string()
        } else {
            "converged".to_string()
        },
        release_tag: plan.release_tag.clone(),
        release_dir: plan.release_dir.clone(),
        active_current_target,
        deployed_binaries,
        deployed_at_unix_s: now_unix_s,
        activated_units,
        rollback_target: previous_target,
        pruned_dirs,
    })
}

/// Read the release dir `current` pointed at before this deploy, if any.
/// Returns `None` on first deploy or when the link did not resolve to a
/// sibling release directory.
fn read_previous_target<E: ReleaseDeployExecutorPort>(
    executor: &E,
    current_link: &Path,
    new_release_dir: &Path,
) -> Option<PathBuf> {
    let previous = executor.read_link(current_link).ok()?;
    if previous == *new_release_dir || previous.as_os_str().is_empty() {
        return None;
    }
    Some(previous)
}

/// Roll back to the previous release: switch the symlink back, restart the
/// units that were already activated, re-probe their health. Returns the
/// combined error message (original failure + rollback outcome) so the caller
/// always knows both halves.
fn rollback<E: ReleaseDeployExecutorPort>(
    executor: &E,
    plan: &ReleaseDeployPlan,
    previous_target: &Option<PathBuf>,
    activated_units: &[ActivatedUnitReport],
    failure: &str,
) -> String {
    let Some(previous) = previous_target else {
        return format!("{failure}; no previous release to roll back to (first deploy)");
    };
    if let Err(e) = executor.atomic_symlink(previous, &plan.current_link) {
        return format!("{failure}; ROLLBACK FAILED switching current back: {e}");
    }
    for activated in activated_units {
        if let Err(e) = executor.restart_unit(&activated.unit) {
            return format!("{failure}; ROLLBACK FAILED restarting {}: {e}", activated.unit);
        }
    }
    match executor.read_link(&plan.current_link) {
        Ok(target) if target == *previous => {
            format!("{failure}; rolled back to {}", previous.display())
        }
        Ok(target) => format!(
            "{failure}; ROLLBACK UNVERIFIED: current -> {} (expected {})",
            target.display(),
            previous.display()
        ),
        Err(e) => format!("{failure}; ROLLBACK UNVERIFIED: cannot read current: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn plans_release_deployment_correctly() {
        let spec = ReleaseDeploySpec {
            version: "0.8.0".to_string(),
            state_root: PathBuf::from("/var/lib/cynic"),
            binaries: vec![
                DeployBinarySpec {
                    name: "cynic".to_string(),
                    source_path: PathBuf::from("target/release/cynic"),
                },
                DeployBinarySpec {
                    name: "cynic-node".to_string(),
                    source_path: PathBuf::from("target/release/cynic-node"),
                },
            ],
            restart_units: vec![],
        };

        let plan = plan_release_deployment(&spec).expect("plan");
        assert_eq!(plan.release_tag, "v0.8.0");
        assert_eq!(
            plan.release_dir,
            PathBuf::from("/var/lib/cynic/releases/v0.8.0")
        );
        assert_eq!(plan.current_link, PathBuf::from("/var/lib/cynic/current"));
        assert_eq!(plan.bin_dir, PathBuf::from("/var/lib/cynic/bin"));
        assert_eq!(plan.binary_targets.len(), 2);
    }

    #[test]
    fn applies_deployment_with_atomic_symlinks_and_read_back() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        let source_bin = root.join("src-cynic");
        fs::write(&source_bin, b"binary-content-v0.8.0").expect("write fake bin");

        let spec = ReleaseDeploySpec {
            version: "0.8.0".to_string(),
            state_root: root.clone(),
            binaries: vec![DeployBinarySpec {
                name: "cynic".to_string(),
                source_path: source_bin,
            }],
            restart_units: vec![],
        };

        let plan = plan_release_deployment(&spec).expect("plan");
        let executor = LocalReleaseDeployExecutor;
        let report = apply_release_deployment(&spec, &plan, &executor, 1790000000).expect("apply");

        // No declared units: staged, not activated. Placement without
        // activation is not a deployment (contracts/runtime-update.json).
        assert_eq!(report.status, "staged");
        assert!(report.activated_units.is_empty());
        assert_eq!(report.release_tag, "v0.8.0");
        assert_eq!(report.active_current_target, plan.release_dir);

        let bin_on_disk = plan.release_dir.join("cynic");
        assert!(bin_on_disk.exists());
        assert_eq!(
            fs::read(&bin_on_disk).unwrap(),
            b"binary-content-v0.8.0"
        );

        let bin_link = plan.bin_dir.join("cynic");
        assert!(bin_link.exists());
    }

    /// Scripted executor: real filesystem for placement, scripted answers for
    /// activation. Never touches systemctl or the network.
    struct ScriptedExecutor {
        local: LocalReleaseDeployExecutor,
        fail_restart: Vec<String>,
        fail_health: Vec<String>,
        restarted: std::cell::RefCell<Vec<String>>,
    }

    impl ReleaseDeployExecutorPort for ScriptedExecutor {
        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.local.create_dir_all(path)
        }
        fn copy_file(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
            self.local.copy_file(from, to)
        }
        fn atomic_symlink(&self, target: &Path, link_path: &Path) -> std::io::Result<()> {
            self.local.atomic_symlink(target, link_path)
        }
        fn read_link(&self, path: &Path) -> std::io::Result<PathBuf> {
            self.local.read_link(path)
        }
        fn restart_unit(&self, unit: &str) -> Result<String, String> {
            self.restarted.borrow_mut().push(unit.to_string());
            if self.fail_restart.iter().any(|u| u == unit) {
                return Err(format!("scripted restart failure: {unit}"));
            }
            Ok("active".to_string())
        }
        fn probe_health(&self, url: &str, _timeout_secs: u64) -> Result<u16, String> {
            if self.fail_health.iter().any(|u| u == url) {
                return Err(format!("scripted health failure: {url}"));
            }
            Ok(200)
        }
        fn remove_release_dir(&self, path: &Path) -> std::io::Result<()> {
            self.local.remove_release_dir(path)
        }
        fn list_release_dirs(&self, state_root: &Path) -> std::io::Result<Vec<PathBuf>> {
            self.local.list_release_dirs(state_root)
        }
    }

    fn staged_spec(root: &Path, version: &str, units: Vec<RestartUnitSpec>) -> (ReleaseDeploySpec, ReleaseDeployPlan) {
        let source_bin = root.join(format!("src-cynic-{version}"));
        fs::write(&source_bin, format!("binary-content-{version}")).expect("write fake bin");
        let spec = ReleaseDeploySpec {
            version: version.to_string(),
            state_root: root.to_path_buf(),
            binaries: vec![DeployBinarySpec {
                name: "cynic".to_string(),
                source_path: source_bin,
            }],
            restart_units: units,
        };
        let plan = plan_release_deployment(&spec).expect("plan");
        (spec, plan)
    }

    #[test]
    fn activation_converges_with_declared_units_and_health() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        let (spec, plan) = staged_spec(
            &root,
            "0.9.0",
            vec![RestartUnitSpec {
                unit: "cynic-kernel.service".to_string(),
                health_url: "http://127.0.0.1:3030/health".to_string(),
                health_timeout_secs: 5,
            }],
        );
        let executor = ScriptedExecutor {
            local: LocalReleaseDeployExecutor,
            fail_restart: vec![],
            fail_health: vec![],
            restarted: std::cell::RefCell::new(vec![]),
        };
        let report = apply_release_deployment(&spec, &plan, &executor, 1790000001).expect("apply");

        assert_eq!(report.status, "converged");
        assert_eq!(report.activated_units.len(), 1);
        assert_eq!(report.activated_units[0].unit, "cynic-kernel.service");
        assert_eq!(report.activated_units[0].active_state, "active");
        assert_eq!(report.activated_units[0].health_status, Some(200));
        assert!(report.rollback_target.is_none());
        assert_eq!(*executor.restarted.borrow(), vec!["cynic-kernel.service".to_string()]);
    }

    #[test]
    fn health_failure_rolls_back_to_previous_release() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        // First deploy converges (no units): establishes the rollback target.
        let (spec_v1, plan_v1) = staged_spec(&root, "0.9.0", vec![]);
        let local = LocalReleaseDeployExecutor;
        apply_release_deployment(&spec_v1, &plan_v1, &local, 1790000001).expect("v1");

        // Second deploy activates but its health probe fails.
        let (spec_v2, plan_v2) = staged_spec(
            &root,
            "0.9.1",
            vec![RestartUnitSpec {
                unit: "cynic-kernel.service".to_string(),
                health_url: "http://127.0.0.1:3030/health".to_string(),
                health_timeout_secs: 5,
            }],
        );
        let executor = ScriptedExecutor {
            local: LocalReleaseDeployExecutor,
            fail_restart: vec![],
            fail_health: vec!["http://127.0.0.1:3030/health".to_string()],
            restarted: std::cell::RefCell::new(vec![]),
        };
        let err = apply_release_deployment(&spec_v2, &plan_v2, &executor, 1790000002)
            .expect_err("health failure must fail the deploy");
        assert!(err.contains("health probe failed"), "unexpected error: {err}");
        assert!(err.contains("rolled back to"), "no rollback proof: {err}");

        // Read-back: current points at v0.9.0 again.
        let current = fs::read_link(root.join("current")).expect("read current");
        assert_eq!(current, plan_v1.release_dir);
    }

    #[test]
    fn gc_keeps_current_plus_two_and_never_the_rollback_target() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        let local = LocalReleaseDeployExecutor;

        let mut plans = Vec::new();
        for (i, version) in ["0.9.0", "0.9.1", "0.9.2", "0.9.3", "0.9.4"].iter().enumerate() {
            let (spec, plan) = staged_spec(&root, version, vec![]);
            apply_release_deployment(&spec, &plan, &local, 1790000001 + i as u64).expect("apply");
            plans.push(plan);
        }

        let dirs = local.list_release_dirs(&root).expect("list");
        assert_eq!(dirs.len(), 3, "expected current + 2 kept, got {dirs:?}");
        assert!(dirs.contains(&plans[4].release_dir), "current must be kept");
        assert!(dirs.contains(&plans[3].release_dir), "rollback target must be kept");
    }

    #[test]
    fn refuses_to_restart_non_service_unit_names() {
        // Pure validation: refused before any systemctl invocation.
        assert!(is_plain_service_unit("cynic-kernel.service"));
        assert!(is_plain_service_unit("cynic-dns-synapse.service"));
        for bad in ["reboot.target", "../evil.service", "/bin/sh", "a;rm -rf", ""] {
            assert!(!is_plain_service_unit(bad), "must refuse unit name: {bad:?}");
        }
    }
}

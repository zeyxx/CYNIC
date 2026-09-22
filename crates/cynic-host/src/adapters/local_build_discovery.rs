//! Local build discovery — binds `BuildDiscoveryObservationPort` to the
//! host's own `cargo metadata` + `rustc` probes. No SSH. Uses
//! `capture_command_in_dir` from the workspace root (parent of binary) and
//! reuses pure `parse_cargo_metadata`.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::build_discovery::port::BuildDiscoveryObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use crate::process_observation::capture_command_in_dir;
use cynic_build::build_discovery::BuildDiscoveryReport;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBuildDiscoveryAdapter;

fn workspace_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

fn run(
    dir: &std::path::Path,
    cmd: &str,
    args: &[&str],
) -> Result<crate::process_observation::CommandCapture, String> {
    capture_command_in_dir(cmd, args, Some(dir)).map_err(|e| format!("{cmd}_spawn: {e}"))
}

impl BuildDiscoveryObservationPort for LocalBuildDiscoveryAdapter {
    fn observe(&self) -> Result<BuildDiscoveryReport, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let root = workspace_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let meta = run(
            &root,
            "cargo",
            &["metadata", "--format-version", "1", "--no-deps", "--frozen"],
        )?;
        if !meta.success {
            return Err(format!(
                "cargo_metadata_failed: exit={:?} stderr={}",
                meta.exit_code, meta.stderr
            ));
        }

        let toolchain = run(&root, "rustc", &["--version", "--verbose"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout)
            .unwrap_or_else(|| "unknown".to_string());

        let host_triple = run(&root, "rustc", &["--print", "host-tuple"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        parse_cargo_metadata(&meta.stdout, now, toolchain, host_triple)
    }
}

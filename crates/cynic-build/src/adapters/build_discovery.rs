//! Build discovery adapters for cynic-build.
//!
//! LocalBuildDiscoveryAdapter: runs cargo metadata + rustc probes locally.
//! RemoteBuildDiscoveryAdapter: runs the same probes over SSH.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::build_discovery::port::BuildDiscoveryObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use cynic_process::ArgvRunner;
use cynic_protocol::build_discovery::BuildDiscoveryReport;

// ── Local adapter ───────────────────────────────────────────────────────────

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

impl BuildDiscoveryObservationPort for LocalBuildDiscoveryAdapter {
    fn observe(&self) -> Result<BuildDiscoveryReport, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let root = workspace_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let runner = cynic_process::LocalArgvRunner;

        let meta_args: Vec<&str> = std::iter::once("cargo")
            .chain(cynic_protocol::cargo_metadata::CARGO_METADATA_ARGS.iter().copied())
            .collect();
        let meta = runner.run(&meta_args);

        // LocalArgvRunner.run returns ArgvOutput; check exit_code
        if meta.exit_code != Some(0) {
            return Err(format!(
                "cargo_metadata_failed: exit={:?} stderr={}",
                meta.exit_code, meta.stderr
            ));
        }

        let toolchain = {
            let args = ["rustc", "--version", "--verbose"];
            let c = runner.run(&args);
            if c.exit_code == Some(0) { c.stdout } else { "unknown".to_string() }
        };

        let host_triple = {
            let args = ["rustc", "--print", "host-tuple"];
            let c = runner.run(&args);
            if c.exit_code == Some(0) {
                let s = c.stdout.trim().to_string();
                if s.is_empty() { "unknown".to_string() } else { s }
            } else {
                "unknown".to_string()
            }
        };

        parse_cargo_metadata(&meta.stdout, now, toolchain, host_triple)
    }
}

fn make_argv(cmd: &str, args: &[&str]) -> Vec<String> {
    let mut argv = vec![cmd.to_string()];
    argv.extend(args.iter().map(|a| a.to_string()));
    argv
}

// ── Remote adapter ──────────────────────────────────────────────────────────

pub struct RemoteBuildDiscoveryAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteBuildDiscoveryAdapter<R> {
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn ssh_run(&self, remote_argv: &[&str]) -> Result<String, String> {
        let argv = cynic_ssh::argv::ssh_argv(&self.ssh_target, remote_argv)
            .map_err(|e| format!("ssh_argv_build_failed: {e}"))?;
        let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let out = self.runner.run(&argv_refs);
        if out.exit_code == Some(0) {
            Ok(out.stdout)
        } else {
            Err(format!(
                "remote_build_discovery_failed: {}",
                out.spawn_error.clone().unwrap_or(out.stderr)
            ))
        }
    }
}

impl<R: ArgvRunner> BuildDiscoveryObservationPort for RemoteBuildDiscoveryAdapter<R> {
    fn observe(&self) -> Result<BuildDiscoveryReport, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let meta_stdout = self.ssh_run(&cynic_protocol::cargo_metadata::cargo_metadata_argv())?;

        let toolchain = self
            .ssh_run(&["rustc", "--version", "--verbose"])
            .unwrap_or_else(|_| "unknown".to_string());

        let host_triple = self
            .ssh_run(&["rustc", "--print", "host-tuple"])
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        parse_cargo_metadata(&meta_stdout, now, toolchain, host_triple)
    }
}

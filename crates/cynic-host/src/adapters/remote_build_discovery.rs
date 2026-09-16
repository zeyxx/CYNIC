//! Remote build discovery over SSH — injectable `ArgvRunner` so tests
//! inject `MapArgvRunner` with canned `cargo metadata` / `rustc` stdout.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::build_discovery::port::BuildDiscoveryObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use cynic_lifecycle::ArgvRunner;
use cynic_build::build_discovery::BuildDiscoveryReport;

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
        let mut argv: Vec<&str> = vec![
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=yes",
            &self.ssh_target,
        ];
        argv.extend_from_slice(remote_argv);
        let out = self.runner.run(&argv);
        if out.success() {
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

        let meta_stdout = self.ssh_run(&["cargo", "metadata", "--format-version", "1", "--no-deps"])?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    fn ok_out(stdout: &str) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    #[test]
    fn rejects_invalid_target() {
        let r = Arc::new(MapArgvRunner { responses: vec![] });
        assert!(RemoteBuildDiscoveryAdapter::new(r, "").is_err());
        let r2 = Arc::new(MapArgvRunner { responses: vec![] });
        assert!(RemoteBuildDiscoveryAdapter::new(r2, "--evil").is_err());
    }

    #[test]
    fn observes_via_ssh_runner() {
        let meta_json = r#"{"packages":[{"name":"cynic-host","edition":"2024","targets":[{"name":"cynic-host","kind":["lib"]}]}],"workspace_members":["cynic-host 0.1.0 (path+file:///tmp)"],"target_directory":"/tmp/target","version":1}"#;
        let runner = MapArgvRunner {
            responses: vec![
                (
                    vec![
                        "ssh".to_string(),
                        "-o".to_string(),
                        "BatchMode=yes".to_string(),
                        "-o".to_string(),
                        "ConnectTimeout=5".to_string(),
                        "-o".to_string(),
                        "StrictHostKeyChecking=yes".to_string(),
                        "cynic@10.0.0.1".to_string(),
                        "cargo".to_string(),
                        "metadata".to_string(),
                        "--format-version".to_string(),
                        "1".to_string(),
                        "--no-deps".to_string(),
                    ],
                    ok_out(meta_json),
                ),
                (
                    vec![
                        "ssh".to_string(),
                        "-o".to_string(),
                        "BatchMode=yes".to_string(),
                        "-o".to_string(),
                        "ConnectTimeout=5".to_string(),
                        "-o".to_string(),
                        "StrictHostKeyChecking=yes".to_string(),
                        "cynic@10.0.0.1".to_string(),
                        "rustc".to_string(),
                        "--version".to_string(),
                        "--verbose".to_string(),
                    ],
                    ok_out("rustc 1.82.0 (verbose)"),
                ),
                (
                    vec![
                        "ssh".to_string(),
                        "-o".to_string(),
                        "BatchMode=yes".to_string(),
                        "-o".to_string(),
                        "ConnectTimeout=5".to_string(),
                        "-o".to_string(),
                        "StrictHostKeyChecking=yes".to_string(),
                        "cynic@10.0.0.1".to_string(),
                        "rustc".to_string(),
                        "--print".to_string(),
                        "host-tuple".to_string(),
                    ],
                    ok_out("x86_64-unknown-linux-gnu"),
                ),
            ],
        };
        let adapter = RemoteBuildDiscoveryAdapter::new(Arc::new(runner), "cynic@10.0.0.1").unwrap();
        let report = adapter.observe().unwrap();
        assert_eq!(report.crates.len(), 1);
        assert_eq!(report.crates[0].name, "cynic-host");
        assert_eq!(report.host_triple.0, "x86_64-unknown-linux-gnu");
    }
}

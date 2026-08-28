use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::time::now_unix_ms;
use crate::transport::{
    ExecutableObservation, OperatorTransportObservationPort, SshHostKeyObservation,
    SshTargetInspectionPort, SshTargetInspectionReport, TmuxHarnessObservation,
    TmuxSessionObservation,
};

/// Local process adapter for read-only SSH/tmux operator evidence.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalOperatorTransport;

impl OperatorTransportObservationPort for LocalOperatorTransport {
    fn observe_ssh_target(
        &self,
        target: &str,
        port: Option<u16>,
    ) -> SshTargetInspectionReport {
        let ssh = self.inspect_ssh_config(target);
        let ssh_keyscan = self.scan_host_key(target, port);
        let host_keys = self.read_known_hosts(target);
        let trust_state = self.assess_trust(&ssh_keyscan, &host_keys);

        SshTargetInspectionReport {
            schema: "cynic.ssh_target_inspection.v1".to_string(),
            target: target.to_string(),
            status: "observed".to_string(),
            observed_at_unix_ms: now_unix_ms(),
            ssh,
            ssh_keyscan,
            host_keys,
            trust_state,
            mutation: false,
        }
    }

    fn observe_executable(&self, path: &Path) -> Result<ExecutableObservation, String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|e| format!("stat {}: {}", path.display(), e))?;
        let executable = metadata.is_file() && metadata.mode() & 0o111 != 0;
        Ok(ExecutableObservation {
            schema: "cynic.executable_observation.v1".to_string(),
            path: path.to_string_lossy().to_string(),
            exists: true,
            executable,
            observed_at_unix_ms: now_unix_ms(),
        })
    }

    fn observe_tmux_harness(&self) -> TmuxHarnessObservation {
        let version = self.run_tmux_version();
        let sessions = self.list_tmux_sessions();
        TmuxHarnessObservation {
            schema: "cynic.tmux_harness_observation.v1".to_string(),
            version,
            sessions,
            observed_at_unix_ms: now_unix_ms(),
        }
    }
}

impl LocalOperatorTransport {
    fn inspect_ssh_config(&self, target: &str) -> Option<String> {
        let output = Command::new("ssh")
            .args(["-G", target])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn scan_host_key(&self, target: &str, port: Option<u16>) -> Option<SshHostKeyObservation> {
        let mut cmd = Command::new("ssh-keyscan");
        if let Some(p) = port {
            cmd.args(["-p", &p.to_string()]);
        }
        cmd.arg(target);
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let keys = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();
        Some(SshHostKeyObservation {
            target: target.to_string(),
            keys,
            observed_at_unix_ms: now_unix_ms(),
        })
    }

    fn read_known_hosts(&self, target: &str) -> Vec<SshHostKeyObservation> {
        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
        let known_hosts = PathBuf::from(home).join(".ssh/known_hosts");
        let content = fs::read_to_string(&known_hosts).unwrap_or_default();
        content
            .lines()
            .filter(|l| l.contains(target))
            .map(|l| SshHostKeyObservation {
                target: target.to_string(),
                keys: vec![l.to_string()],
                observed_at_unix_ms: now_unix_ms(),
            })
            .collect()
    }

    fn assess_trust(&self, scan: &Option<SshHostKeyObservation>, known: &[SshHostKeyObservation]) -> String {
        let Some(scan) = scan else {
            return "not_observed".to_string();
        };
        if known.is_empty() {
            return "unknown".to_string();
        }
        for k in known {
            if k.keys.iter().any(|key| scan.keys.contains(key)) {
                return "trusted".to_string();
            }
        }
        return "changed".to_string();
    }

    fn run_tmux_version(&self) -> Option<String> {
        let output = Command::new("tmux").args(["-V"]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn list_tmux_sessions(&self) -> Vec<TmuxSessionObservation> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}:#{session_created}"])
            .output();
        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                let mut parts = l.splitn(2, ':');
                let name = parts.next()?.to_string();
                let created = parts.next().and_then(|s| s.parse::<i64>().ok())?;
                Some(TmuxSessionObservation {
                    name,
                    created_unix: created,
                    observed_at_unix_ms: now_unix_ms(),
                })
            })
            .collect()
    }
}

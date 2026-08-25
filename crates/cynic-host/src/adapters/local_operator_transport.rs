use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::transport::{
    ExecutableObservation, OperatorTransportObservationPort, SshHostKeyObservation,
    SshTargetInspectionPort, SshTargetInspectionReport, TmuxHarnessObservation,
    TmuxSessionObservation,
};

/// Local process adapter for read-only SSH/tmux operator evidence.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalOperatorTransportObserver;

impl OperatorTransportObservationPort for LocalOperatorTransportObserver {
    fn observe_ssh(&self) -> ExecutableObservation {
        observe_executable("ssh")
    }

    fn observe_tmux(&self) -> TmuxHarnessObservation {
        let executable = observe_executable("tmux");
        if executable.status != "available" {
            return TmuxHarnessObservation {
                executable,
                status: "unavailable".to_string(),
                sessions: Vec::new(),
                detail: Some("tmux binary is not available on this operator host".to_string()),
            };
        }

        match Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}\t#{session_windows}\t#{session_attached}",
            ])
            .output()
        {
            Ok(output) if output.status.success() => TmuxHarnessObservation {
                executable,
                status: "active".to_string(),
                sessions: parse_tmux_sessions(&output.stdout),
                detail: None,
            },
            Ok(output) => {
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
                TmuxHarnessObservation {
                    executable,
                    status: "inactive".to_string(),
                    sessions: Vec::new(),
                    detail: (!detail.is_empty()).then_some(detail),
                }
            }
            Err(error) => TmuxHarnessObservation {
                executable,
                status: "unknown".to_string(),
                sessions: Vec::new(),
                detail: Some(format!("failed to observe tmux sessions: {error}")),
            },
        }
    }
}

impl SshTargetInspectionPort for LocalOperatorTransportObserver {
    fn inspect_ssh_target(&self, target: &str) -> SshTargetInspectionReport {
        let ssh = observe_executable("ssh");
        let ssh_keyscan = observe_executable("ssh-keyscan");
        if ssh.status != "available" || ssh_keyscan.status != "available" {
            return SshTargetInspectionReport {
                schema: "cynic.ssh_target_inspection.v1",
                target: target.to_string(),
                status: "prerequisite_missing".to_string(),
                observed_at_unix_ms: now_unix_ms(),
                ssh,
                ssh_keyscan,
                host_keys: Vec::new(),
                trust_state: "not_observed".to_string(),
                mutation: false,
                next_action: "install the OpenSSH client before inspecting bootstrap targets"
                    .to_string(),
                detail: None,
            };
        }

        let scan = Command::new("ssh-keyscan")
            .args(["-T", "5", "-t", "ed25519", target])
            .output();
        match scan {
            Ok(output) if output.status.success() && !output.stdout.is_empty() => {
                let host_keys = fingerprints(&output.stdout);
                SshTargetInspectionReport {
                    schema: "cynic.ssh_target_inspection.v1",
                    target: target.to_string(),
                    status: if host_keys.is_empty() {
                        "unparseable".to_string()
                    } else {
                        "observed".to_string()
                    },
                    observed_at_unix_ms: now_unix_ms(),
                    ssh,
                    ssh_keyscan,
                    host_keys,
                    trust_state: "untrusted_observed".to_string(),
                    mutation: false,
                    next_action: "compare host key fingerprints with an independent Proxmox/operator channel before trust acceptance".to_string(),
                    detail: None,
                }
            }
            Ok(output) => SshTargetInspectionReport {
                schema: "cynic.ssh_target_inspection.v1",
                target: target.to_string(),
                status: "unreachable".to_string(),
                observed_at_unix_ms: now_unix_ms(),
                ssh,
                ssh_keyscan,
                host_keys: Vec::new(),
                trust_state: "not_observed".to_string(),
                mutation: false,
                next_action: "verify network reachability and target SSH exposure".to_string(),
                detail: stderr_detail(&output.stderr),
            },
            Err(error) => SshTargetInspectionReport {
                schema: "cynic.ssh_target_inspection.v1",
                target: target.to_string(),
                status: "observation_failed".to_string(),
                observed_at_unix_ms: now_unix_ms(),
                ssh,
                ssh_keyscan,
                host_keys: Vec::new(),
                trust_state: "not_observed".to_string(),
                mutation: false,
                next_action: "repair local SSH observation adapter".to_string(),
                detail: Some(error.to_string()),
            },
        }
    }
}

fn observe_executable(binary: &str) -> ExecutableObservation {
    let path = find_in_path(binary).map(|path| path.display().to_string());
    ExecutableObservation {
        binary: binary.to_string(),
        status: if path.is_some() {
            "available"
        } else {
            "unavailable"
        }
        .to_string(),
        path,
    }
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(binary))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn parse_tmux_sessions(bytes: &[u8]) -> Vec<TmuxSessionObservation> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim();
            let windows = fields.next()?.trim().parse().ok()?;
            let attached_clients = fields.next()?.trim().parse().ok()?;
            (!name.is_empty()).then(|| TmuxSessionObservation {
                name: name.to_string(),
                windows,
                attached_clients,
            })
        })
        .collect()
}

fn fingerprints(host_keys: &[u8]) -> Vec<SshHostKeyObservation> {
    let Ok(mut child) = Command::new("ssh-keygen")
        .args(["-lf", "-", "-E", "sha256"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    let Some(stdin) = child.stdin.as_mut() else {
        return Vec::new();
    };
    if stdin.write_all(host_keys).is_err() {
        return Vec::new();
    }
    let Ok(output) = child.wait_with_output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _bits = fields.next()?;
            let fingerprint = fields.next()?;
            let _host = fields.next()?;
            let algorithm = fields.next()?.trim_matches(['(', ')']);
            Some(SshHostKeyObservation {
                algorithm: algorithm.to_string(),
                fingerprint: fingerprint.to_string(),
            })
        })
        .collect()
}

fn stderr_detail(stderr: &[u8]) -> Option<String> {
    let value = String::from_utf8_lossy(stderr).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{fingerprints, parse_tmux_sessions};

    #[test]
    fn parses_tmux_session_projection() {
        let sessions = parse_tmux_sessions(b"cynic\t2\t1\noperator\t1\t0\n");

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "cynic");
        assert_eq!(sessions[0].windows, 2);
        assert_eq!(sessions[1].attached_clients, 0);
    }

    #[test]
    fn fingerprints_public_host_keys_without_persisting_them() {
        let key = b"example ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG4ZqOi0AMk9pLwD8PLfeOQ9PD6czSw0u5KqY7M7LGm2\n";
        let observations = fingerprints(key);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].algorithm, "ED25519");
        assert!(observations[0].fingerprint.starts_with("SHA256:"));
    }
}

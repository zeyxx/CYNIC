//! Adapter: `BuildRunnerPort` over the network build runner via SSH.
//!
//! The adapter shells out to the runner's `build-runner-bootstrap` script on
//! 10.10.10.119 (exactly as the distributed pipeline expects), passing the
//! exact source commit + target phase. It is disposable; the port contract in
//! `build_runner.rs` is durable. Tests inject a fake `CommandRunner`.

use crate::ports::build_runner::{
    BuildError, BuildProofBundle, BuildRunnerPort, BuildTrigger, LayerProof, LayerStatus,
    PipelineStatus, RunnerHealth,
};

/// Minimal argv command runner (no shell composition).
///
/// Mirrors the `ArgvRunner` pattern from `cynic-host::update_lifecycle::port`,
/// kept local here so `cynic-node` does not depend on `cynic-host`.
pub trait CommandRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> CommandOutput;
}

/// Output of a command run through [`CommandRunner`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }
}

/// Production runner: spawns `argv[0]` with the remaining args.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalCommandRunner;

impl CommandRunner for LocalCommandRunner {
    fn run(&self, argv: &[&str]) -> CommandOutput {
        if argv.is_empty() {
            return CommandOutput {
                spawn_error: Some("empty argv".into()),
                ..Default::default()
            };
        }
        match std::process::Command::new(argv[0]).args(&argv[1..]).output() {
            Ok(o) => CommandOutput {
                exit_code: o.status.code(),
                stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
                spawn_error: None,
            },
            Err(e) => CommandOutput {
                spawn_error: Some(e.to_string()),
                ..Default::default()
            },
        }
    }
}

/// `BuildRunnerPort` implemented over SSH to the network runner.
///
/// The SSH host runs `build-runner-bootstrap` (already present on the runner
/// node) which materializes a clean detached workspace from the exact commit
/// and emits a `cynic.pipeline_build_proof.v1` JSON on stdout.
pub struct SshBuildRunner {
    pub host: String,
    pub user: String,
    pub runner: Box<dyn CommandRunner>,
}

impl SshBuildRunner {
    /// Build the argv passed to the remote runner.
    fn build_argv(&self, trigger: &BuildTrigger) -> Vec<String> {
        vec![
            "ssh".to_string(),
            format!("{}@{}", self.user, self.host),
            "build-runner-bootstrap".to_string(),
            "--source-commit".to_string(),
            trigger.source_commit.clone(),
            "--target-phase".to_string(),
            trigger.phase.to_string(),
            "--network-mode".to_string(),
            trigger.network_mode.as_arg().to_string(),
            "--yes".to_string(),
        ]
    }
}

impl BuildRunnerPort for SshBuildRunner {
    fn trigger(&self, trigger: &BuildTrigger) -> Result<BuildProofBundle, BuildError> {
        trigger.validate()?;
        let argv: Vec<&str> = self
            .build_argv(trigger)
            .iter()
            .map(|s| s.as_str())
            .collect();
        let out = self.runner.run(&argv);
        if let Some(e) = &out.spawn_error {
            return Err(BuildError::RunnerUnreachable(e.clone()));
        }
        if !out.success() {
            return Err(BuildError::AdmissionBlocked(out.stderr.trim().to_string()));
        }
        parse_proof(&out.stdout)
    }

    fn observe_runner(&self) -> RunnerHealth {
        let argv = vec![
            "ssh",
            &format!("{}@{}", self.user, self.host),
            "echo",
            "cynic-build-runner",
        ];
        let out = self.runner.run(&argv);
        let reachable = out.success();
        RunnerHealth {
            node_id: self.host.clone(),
            reachable,
            cache_warm: reachable, // conservative: assume warm if reachable
            toolchain: "rustup (per-crate dispatch)".to_string(),
        }
    }
}

/// Parse a `cynic.pipeline_build_proof.v1` JSON into a [`BuildProofBundle`].
///
/// Hand-rolled minimal parse (no serde rename juggling); the proof is small
/// and stable. We extract the fields we need and leave the rest to the raw
/// JSON which callers may retain.
fn parse_proof(json: &str) -> Result<BuildProofBundle, BuildError> {
    let get = |key: &str| {
        // crude "key":"value" scanner good enough for the proof shape
        let pat = format!("\"{key}\":");
        json.find(&pat).and_then(|i| {
            let rest = &json[i + pat.len()..];
            let s = rest.trim_start().strip_prefix('"')?;
            let end = s.find('"')?;
            Some(s[..end].to_string())
        })
    };
    let schema = get("schema").unwrap_or_else(|| "unknown".to_string());
    let node = get("node").unwrap_or_default();
    let source_commit = get("source_commit").unwrap_or_default();
    let action = get("action").unwrap_or_default();
    let status_raw = get("status").unwrap_or_else(|| "blocked".to_string());
    let status = if status_raw == "converged" {
        PipelineStatus::Converged
    } else {
        PipelineStatus::Blocked
    };
    let broken_layer = get("broken_layer").filter(|s| !s.is_empty());
    // layers: count by scanning "layer":"<crate>" occurrences
    let mut layers: Vec<LayerProof> = Vec::new();
    let mut start = 0;
    while let Some(i) = json[start..].find("\"layer\":") {
        let abs = start + i;
        let rest = &json[abs + "\"layer\":".len()..];
        if let Some(s) = rest.trim_start().strip_prefix('"') {
            if let Some(end) = s.find('"') {
                let layer = s[..end].to_string();
                // status for this layer
                let status = if rest.contains("\"status\":\"converged\"") {
                    LayerStatus::Converged
                } else if rest.contains("\"status\":\"failed\"") {
                    LayerStatus::Failed
                } else {
                    LayerStatus::BlockedUpstream
                };
                layers.push(LayerProof {
                    layer,
                    status,
                    blocked_by: None,
                });
            }
        }
        start = abs + 1;
    }
    Ok(BuildProofBundle {
        schema,
        node,
        source_commit,
        action,
        status,
        broken_layer,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeCommandRunner {
        proof: String,
        reachable: bool,
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(&self, argv: &[&str]) -> CommandOutput {
            // observe_runner uses `echo cynic-build-runner`
            if argv.len() >= 2 && argv[1].ends_with("echo") {
                return CommandOutput {
                    exit_code: if self.reachable { Some(0) } else { Some(1) },
                    stdout: "cynic-build-runner\n".into(),
                    ..Default::default()
                };
            }
            CommandOutput {
                exit_code: Some(0),
                stdout: self.proof.clone(),
                ..Default::default()
            }
        }
    }

    fn converged_proof(commit: &str) -> String {
        format!(
            "{{\"schema\":\"cynic.pipeline_build_proof.v1\",\"node\":\"runner-1\",\"source_commit\":\"{commit}\",\"action\":\"check\",\"status\":\"converged\",\"broken_layer\":null,\"layers\":[{{\"layer\":\"cynic-protocol\",\"status\":\"converged\"}},{{\"layer\":\"cynic-node\",\"status\":\"converged\"}}]}}"
        )
    }

    #[test]
    fn trigger_returns_converged_proof() {
        let commit = "a".repeat(40);
        let runner = SshBuildRunner {
            host: "10.10.10.119".into(),
            user: "cynic".into(),
            runner: Box::new(FakeCommandRunner {
                proof: converged_proof(&commit),
                reachable: true,
            }),
        };
        let trigger = BuildTrigger {
            source_commit: commit.clone(),
            phase: crate::ports::build_runner::BuildPhase::Check,
            network_mode: crate::ports::build_runner::NetworkMode::Offline,
        };
        let proof = runner.trigger(&trigger).expect("trigger should succeed");
        assert_eq!(proof.source_commit, commit);
        assert_eq!(proof.status, PipelineStatus::Converged);
        assert_eq!(proof.layers.len(), 2);
        assert_eq!(proof.layers[0].layer, "cynic-protocol");
    }

    #[test]
    fn trigger_rejects_bad_commit() {
        let runner = SshBuildRunner {
            host: "10.10.10.119".into(),
            user: "cynic".into(),
            runner: Box::new(FakeCommandRunner {
                proof: String::new(),
                reachable: true,
            }),
        };
        let trigger = BuildTrigger {
            source_commit: "short".into(),
            phase: crate::ports::build_runner::BuildPhase::Check,
            network_mode: crate::ports::build_runner::NetworkMode::Offline,
        };
        assert!(runner.trigger(&trigger).is_err());
    }

    #[test]
    fn observe_runner_reports_health() {
        let runner = SshBuildRunner {
            host: "10.10.10.119".into(),
            user: "cynic".into(),
            runner: Box::new(FakeCommandRunner {
                proof: String::new(),
                reachable: true,
            }),
        };
        let h = runner.observe_runner();
        assert!(h.reachable);
        assert_eq!(h.node_id, "10.10.10.119");
    }
}

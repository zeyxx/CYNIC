//! Adapter: `BuildRunnerPort` over the network build runner via SSH.
//!
//! No script runs on the runner. Every remote step is an explicit argv:
//! `git` pins a per-SHA checkout, then `cargo` (translated from the
//! canonical protocol action) builds it. Disposable; the port contract in
//! `build_runner.rs` is durable. Tests inject a fake `CommandRunner`.

use crate::ports::build_runner::{
    BuildError, BuildPhase, BuildProofBundle, BuildRunnerPort, BuildTrigger, LayerProof,
    LayerStatus, NetworkMode, PipelineStatus, RunnerHealth,
};
use cynic_protocol::build::canonical_argv_from_build_action;

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
        observe_argv(argv);
        if argv.is_empty() {
            return CommandOutput {
                spawn_error: Some("empty argv".into()),
                ..Default::default()
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
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

/// Observe mode (tranche 2): same contract as
/// `cynic-host::update_lifecycle::port::observe_argv` — pure verdict from
/// `cynic-protocol::argv_allow`, best-effort JSONL append, never blocks
/// execution. Duplicated here (not shared) because `cynic-node` must not
/// depend on `cynic-host`; the verdict function is the single source.
fn observe_argv(argv: &[&str]) {
    use cynic_protocol::argv_allow::{ArgvObservation, observe_argv_verdict};
    use std::io::Write as _;

    let verdict = observe_argv_verdict(argv, &[]);
    let obs = ArgvObservation::new(argv, &verdict);
    let line = match serde_json::to_string(&obs) {
        Ok(line) => line,
        Err(_) => return,
    };
    let path = match observe_log_path() {
        Some(path) => path,
        None => return,
    };
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{line}");
        }
        Err(e) => eprintln!("argv-observe: log append failed ({path}): {e}"),
    }
}

fn observe_log_path() -> Option<String> {
    // Explicit path wins: the operator or principal decides, we never guess.
    // Follow-up: converge on the canonical principal-paths authority
    // (today in cynic-inference, unreachable from low layers) instead of
    // the degraded process-HOME fallback below.
    if let Ok(explicit) = std::env::var("CYNIC_ARGV_OBSERVE_LOG") {
        if !explicit.trim().is_empty() {
            return Some(explicit);
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|home| format!("{home}/.local/share/cynic/argv-observe.jsonl"))
}

/// `BuildRunnerPort` implemented over SSH to the network runner.
///
/// No script runs on the runner. Every remote step is an explicit argv:
/// `git` pins a per-SHA checkout, then `cargo` (translated from the
/// canonical protocol action) builds it. Proof is constructed from the
/// remote exit status plus a verified `rev-parse HEAD == source_commit`
/// read-back — never from a script's stdout.
pub struct SshBuildRunner {
    pub host: String,
    pub user: String,
    /// Base dir for per-SHA checkouts, e.g. `/var/lib/cynic/checkouts`.
    pub checkout_base: String,
    /// Source of truth cloned when no checkout exists for the SHA.
    pub ssot_url: String,
    /// Optional single package scope (`cargo --package`). None = workspace.
    pub package: Option<String>,
    pub runner: Box<dyn CommandRunner>,
}

/// Default checkout base on runner nodes.
pub const DEFAULT_CHECKOUT_BASE: &str = "/var/lib/cynic/checkouts";
/// Shared compilation cache across per-SHA checkouts. Third-party
/// dependencies (same lockfile, same toolchain) compile once; only the
/// workspace crates rebuild per checkout. Cargo fingerprints keep SHAs
/// isolated; concurrent builds serialize on the target-dir lock.
pub const DEFAULT_SHARED_TARGET_DIR: &str = "/var/lib/cynic/shared-target";
/// Default source of truth for fresh checkouts.
pub const DEFAULT_SSOT_URL: &str = "ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git";

impl SshBuildRunner {
    fn ssh_target(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    fn checkout_dir(&self, sha: &str) -> String {
        format!("{}/cynic-{}", self.checkout_base, &sha[..12.min(sha.len())])
    }

    fn action_for_phase(phase: BuildPhase) -> Result<&'static str, BuildError> {
        match phase {
            BuildPhase::Check => Ok("cargo:check"),
            BuildPhase::Build => Ok("cargo:build"),
            BuildPhase::Activate => Err(BuildError::AdmissionBlocked(
                "remote activate not supported by trigger phase 1".to_string(),
            )),
        }
    }
}

impl BuildRunnerPort for SshBuildRunner {
    fn trigger(&self, trigger: &BuildTrigger) -> Result<BuildProofBundle, BuildError> {
        trigger.validate()?;
        let action = Self::action_for_phase(trigger.phase)?;
        let sha = trigger.source_commit.clone();
        let dir = self.checkout_dir(&sha);
        let target = self.ssh_target();

        // Step 1: ensure a checkout exists for the SHA.
        let probe = self.runner.run(&[
            "ssh",
            target.as_str(),
            "test",
            "-d",
            format!("{dir}/.git").as_str(),
        ]);
        if let Some(e) = &probe.spawn_error {
            return Err(BuildError::RunnerUnreachable(e.clone()));
        }
        if !probe.success() {
            let clone = self.runner.run(&[
                "ssh",
                target.as_str(),
                "git",
                "clone",
                self.ssot_url.as_str(),
                dir.as_str(),
            ]);
            if let Some(e) = &clone.spawn_error {
                return Err(BuildError::RunnerUnreachable(e.clone()));
            }
            if !clone.success() {
                return Err(BuildError::AdmissionBlocked(format!(
                    "checkout clone failed: {}",
                    tail(&clone.stderr)
                )));
            }
        }

        // Step 2: pin the checkout to the exact SHA and read it back.
        let checkout = self.runner.run(&[
            "ssh",
            target.as_str(),
            "git",
            "-C",
            dir.as_str(),
            "checkout",
            sha.as_str(),
        ]);
        if let Some(e) = &checkout.spawn_error {
            return Err(BuildError::RunnerUnreachable(e.clone()));
        }
        // Git may return a non-zero transport status while still switching the
        // checkout (for example, an SSH wrapper propagates advisory detached-HEAD
        // output). The authoritative result is the independent read-back below:
        // accept only `rev-parse HEAD == requested SHA`, never stderr text.
        let checkout_failed = !checkout.success();
        let checkout_stderr = tail(&checkout.stderr);
        let head = self.runner.run(&[
            "ssh",
            target.as_str(),
            "git",
            "-C",
            dir.as_str(),
            "rev-parse",
            "HEAD",
        ]);
        if let Some(e) = &head.spawn_error {
            return Err(BuildError::RunnerUnreachable(e.clone()));
        }
        if !head.success() || head.stdout.trim() != sha {
            let detail = if checkout_failed {
                format!("checkout exit non-zero ({checkout_stderr}); ")
            } else {
                String::new()
            };
            return Err(BuildError::ProofUnparseable(format!(
                "{detail}checkout HEAD does not match source commit"
            )));
        }

        // Step 3: build with the canonical argv for the action.
        let mut cargo = canonical_argv_from_build_action(action).ok_or_else(|| {
            BuildError::AdmissionBlocked(format!("no canonical argv for action: {action}"))
        })?;
        cargo.push("--manifest-path".to_string());
        cargo.push(format!("{dir}/Cargo.toml"));
        if let Some(package) = self.package.as_deref() {
            cargo.push("--package".to_string());
            cargo.push(package.to_string());
            cargo.push("--lib".to_string());
        }
        cargo.push("--locked".to_string());
        if trigger.network_mode == NetworkMode::Offline {
            cargo.push("--offline".to_string());
        }
        let mut argv: Vec<String> = vec![
            "ssh".to_string(),
            target,
            "env".to_string(),
            "RUST_MIN_STACK=8589934592".to_string(),
            "CARGO_BUILD_JOBS=1".to_string(),
            "CARGO_INCREMENTAL=0".to_string(),
            format!("CARGO_TARGET_DIR={DEFAULT_SHARED_TARGET_DIR}"),
        ];
        argv.extend(cargo);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let started = std::time::Instant::now();
        let out = self.runner.run(&refs);
        let duration_ms = started.elapsed().as_millis() as u64;
        if let Some(e) = &out.spawn_error {
            return Err(BuildError::RunnerUnreachable(e.clone()));
        }
        let scope = self
            .package
            .clone()
            .unwrap_or_else(|| "workspace".to_string());
        if !out.success() {
            return Err(BuildError::AdmissionBlocked(format!(
                "remote build failed: {}",
                tail(&out.stderr)
            )));
        }
        Ok(BuildProofBundle {
            schema: "cynic.pipeline_build_proof.v1".to_string(),
            node: self.host.clone(),
            source_commit: sha,
            action: action.to_string(),
            status: PipelineStatus::Converged,
            broken_layer: None,
            layers: vec![LayerProof {
                layer: scope,
                status: LayerStatus::Converged,
                blocked_by: None,
            }],
            duration_ms,
        })
    }

    fn observe_runner(&self) -> RunnerHealth {
        let target = self.ssh_target();
        let cargo = self
            .runner
            .run(&["ssh", target.as_str(), "cargo", "--version"]);
        let rustc = self
            .runner
            .run(&["ssh", target.as_str(), "rustc", "--version"]);
        let reachable = cargo.success() && rustc.success();
        let toolchain = if reachable {
            rustc.stdout.trim().to_string()
        } else {
            "unknown".to_string()
        };
        let warm = self
            .runner
            .run(&[
                "ssh",
                target.as_str(),
                "test",
                "-d",
                self.checkout_base.as_str(),
            ])
            .success();
        let version = self
            .runner
            .run(&["ssh", target.as_str(), "cynic", "version", "--json"]);
        let binary_sha = if version.success() {
            parse_binary_sha(&version.stdout)
        } else {
            None
        };
        RunnerHealth {
            node_id: self.host.clone(),
            reachable,
            cache_warm: reachable && warm,
            toolchain,
            binary_sha,
        }
    }
}

/// Source SHA a runner binary reports, if it knows the version surface.
fn parse_binary_sha(stdout: &str) -> Option<String> {
    let report: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let sha = report.get("git_sha")?.as_str()?;
    if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(sha.to_string())
    } else {
        None
    }
}

/// Last 2 KiB of stderr for error reports — full logs stay on the runner.
fn tail(stderr: &str) -> String {
    const LIMIT: usize = 2048;
    if stderr.len() <= LIMIT {
        return stderr.trim().to_string();
    }
    stderr[stderr.len() - LIMIT..].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Script {
        responses: VecDeque<CommandOutput>,
        log: Vec<Vec<String>>,
    }

    #[derive(Clone, Default)]
    struct FakeRunner {
        script: Arc<Mutex<Script>>,
    }

    impl FakeRunner {
        fn with(responses: Vec<CommandOutput>) -> Self {
            Self {
                script: Arc::new(Mutex::new(Script {
                    responses: responses.into(),
                    log: Vec::new(),
                })),
            }
        }

        fn argv_log(&self) -> Vec<Vec<String>> {
            self.script.lock().expect("read argv log").log.clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, argv: &[&str]) -> CommandOutput {
            let mut script = self.script.lock().expect("pop scripted response");
            script
                .log
                .push(argv.iter().map(|arg| (*arg).to_string()).collect());
            script.responses.pop_front().unwrap_or_default()
        }
    }

    fn ok(stdout: &str) -> CommandOutput {
        CommandOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            ..Default::default()
        }
    }

    fn fail(stderr: &str) -> CommandOutput {
        CommandOutput {
            exit_code: Some(1),
            stderr: stderr.to_string(),
            ..Default::default()
        }
    }

    fn test_runner(fake: FakeRunner) -> (SshBuildRunner, FakeRunner) {
        let handle = fake.clone();
        let runner = SshBuildRunner {
            host: "runner-1".into(),
            user: "cynic".into(),
            checkout_base: "/var/lib/cynic/checkouts".into(),
            ssot_url: "ssh://example/CYNIC.git".into(),
            package: Some("cynic-host".into()),
            runner: Box::new(fake),
        };
        (runner, handle)
    }

    fn check_trigger(commit: &str) -> BuildTrigger {
        BuildTrigger {
            source_commit: commit.into(),
            phase: BuildPhase::Check,
            network_mode: NetworkMode::Offline,
        }
    }

    #[test]
    fn trigger_converges_with_existing_checkout() {
        let commit = "a".repeat(40);
        let fake = FakeRunner::with(vec![ok(""), ok(""), ok(&commit), ok("Checking cynic-host")]);
        let (runner, handle) = test_runner(fake);
        let proof = runner
            .trigger(&check_trigger(&commit))
            .expect("trigger succeeds");
        assert_eq!(proof.source_commit, commit);
        assert_eq!(proof.status, PipelineStatus::Converged);
        assert_eq!(proof.layers.len(), 1);
        assert_eq!(proof.layers[0].layer, "cynic-host");
        assert!(proof.to_json().contains("\"duration_ms\":"));
        let log = handle.argv_log();
        assert_eq!(log.len(), 4);
        assert!(log.iter().all(|argv| argv[0] == "ssh"));
        assert!(
            log.iter()
                .all(|argv| !argv.iter().any(|arg| arg.contains("bootstrap")))
        );
        let cargo = log.last().unwrap().join(" ");
        assert!(cargo.contains("cargo check"));
        assert!(cargo.contains("CARGO_TARGET_DIR=/var/lib/cynic/shared-target"));
        assert!(cargo.contains("--manifest-path"));
        assert!(cargo.contains("--package cynic-host"));
        assert!(cargo.contains("--offline"));
    }

    #[test]
    fn trigger_clones_when_checkout_absent() {
        let commit = "b".repeat(40);
        let fake = FakeRunner::with(vec![
            fail(""),
            ok("Cloning"),
            ok(""),
            ok(&commit),
            ok("Checking cynic-host"),
        ]);
        let (runner, handle) = test_runner(fake);
        let proof = runner
            .trigger(&check_trigger(&commit))
            .expect("trigger succeeds");
        assert_eq!(proof.status, PipelineStatus::Converged);
        let log = handle.argv_log();
        assert_eq!(log.len(), 5);
        assert!(log[1].contains(&"git".to_string()));
        assert!(log[1].contains(&"clone".to_string()));
        assert!(log[1].contains(&"ssh://example/CYNIC.git".to_string()));
    }

    #[test]
    fn trigger_accepts_nonzero_checkout_when_readback_matches() {
        let commit = "f".repeat(40);
        let fake = FakeRunner::with(vec![
            ok(""),
            fail("advisory detached HEAD message"),
            ok(&commit),
            ok("Checking cynic-host"),
        ]);
        let (runner, handle) = test_runner(fake);
        let proof = runner
            .trigger(&check_trigger(&commit))
            .expect("matching read-back is authoritative");
        assert_eq!(proof.status, PipelineStatus::Converged);
        assert_eq!(handle.argv_log().len(), 4);
    }

    #[test]
    fn trigger_blocks_on_head_mismatch() {
        let commit = "c".repeat(40);
        let fake = FakeRunner::with(vec![ok(""), ok(""), ok(&"d".repeat(40))]);
        let (runner, _) = test_runner(fake);
        assert!(matches!(
            runner.trigger(&check_trigger(&commit)),
            Err(BuildError::ProofUnparseable(message))
                if message == "checkout HEAD does not match source commit"
        ));
    }

    #[test]
    fn trigger_blocks_on_build_failure() {
        let commit = "e".repeat(40);
        let fake = FakeRunner::with(vec![ok(""), ok(""), ok(&commit), fail("error[E0432]")]);
        let (runner, _) = test_runner(fake);
        assert!(matches!(
            runner.trigger(&check_trigger(&commit)),
            Err(BuildError::AdmissionBlocked(message))
                if message.contains("remote build failed")
        ));
    }

    #[test]
    fn trigger_rejects_bad_commit() {
        let (runner, _) = test_runner(FakeRunner::with(vec![]));
        let trigger = BuildTrigger {
            source_commit: "short".into(),
            phase: BuildPhase::Check,
            network_mode: NetworkMode::Offline,
        };
        assert!(runner.trigger(&trigger).is_err());
    }

    #[test]
    fn trigger_blocks_activate_phase() {
        let (runner, handle) = test_runner(FakeRunner::with(vec![]));
        let trigger = BuildTrigger {
            source_commit: "f".repeat(40),
            phase: BuildPhase::Activate,
            network_mode: NetworkMode::Offline,
        };
        assert!(matches!(
            runner.trigger(&trigger),
            Err(BuildError::AdmissionBlocked(_))
        ));
        assert!(handle.argv_log().is_empty());
    }

    #[test]
    fn observe_runner_reports_health() {
        let fake = FakeRunner::with(vec![ok("cargo 1.95.0"), ok("rustc 1.95.0"), ok("")]);
        let (runner, _) = test_runner(fake);
        let health = runner.observe_runner();
        assert!(health.reachable);
        assert!(health.cache_warm);
        assert_eq!(health.toolchain, "rustc 1.95.0");
        assert_eq!(health.node_id, "runner-1");
    }
}

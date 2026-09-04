//! Execution port for lifecycle gates/repairs.
//!
//! Domain must not hard-wire `Command::new("cynic")`. CLI injects
//! [`LocalArgvRunner`]; tests inject fakes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgvOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl ArgvOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }

    pub fn summary(&self) -> String {
        if let Some(err) = &self.spawn_error {
            return format!("spawn error: {err}");
        }
        let code = self
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into());
        let msg = if !self.stdout.trim().is_empty() {
            self.stdout.trim()
        } else if !self.stderr.trim().is_empty() {
            self.stderr.trim()
        } else {
            "empty output"
        };
        format!("exit {code}: {msg}")
    }
}

/// Argv-only command runner (no shell composition).
pub trait ArgvRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> ArgvOutput;
}

/// Production runner: spawns argv[0] with remaining args.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalArgvRunner;

impl ArgvRunner for LocalArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        observe_argv(argv);
        if argv.is_empty() {
            return ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("empty argv".into()),
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(o) => ArgvOutput {
                exit_code: o.status.code(),
                stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
                spawn_error: None,
            },
            Err(e) => ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some(format!("spawn {}: {e}", argv[0])),
            },
        }
    }
}

/// Observe mode (tranche 2): compute the pure verdict and append one JSONL
/// line. Best-effort by contract: a logging failure warns on stderr and
/// never blocks or alters execution. Enforcement (deny-by-default) is a
/// later slice; this one only makes every argv visible.
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

/// Test double: returns fixed JSON for known commands.
#[derive(Debug, Default, Clone)]
pub struct MapArgvRunner {
    pub responses: Vec<(Vec<String>, ArgvOutput)>,
}

impl MapArgvRunner {
    /// Build a test runner from shell-like command keys.
    pub fn new(responses: HashMap<String, ArgvOutput>) -> Self {
        Self {
            responses: responses
                .into_iter()
                .map(|(command, output)| {
                    (
                        command.split_whitespace().map(str::to_string).collect(),
                        output,
                    )
                })
                .collect(),
        }
    }
}

impl ArgvRunner for MapArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let key: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
        for (k, v) in &self.responses {
            if k == &key {
                return v.clone();
            }
        }
        // prefix match: first N tokens
        for (k, v) in &self.responses {
            if key.len() >= k.len() && key[..k.len()] == k[..] {
                return v.clone();
            }
        }
        ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("MapArgvRunner: no stub for {argv:?}"),
            spawn_error: Some(format!("MapArgvRunner: no stub for {argv:?}")),
        }
    }
}

pub fn split_command(command: &str) -> Vec<&str> {
    command.split_whitespace().collect()
}

pub fn json_status(stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    v.get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

pub fn parse_json(stdout: &str) -> Option<serde_json::Value> {
    serde_json::from_str(stdout.trim()).ok()
}

/// Blanket impl: a shared runner behind an `Arc` is still an argv runner.
/// Lets fleet observers hold `Arc<R>` and hand `Arc<R>` clones to adapters.
impl<R: ArgvRunner> ArgvRunner for std::sync::Arc<R> {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        self.as_ref().run(argv)
    }
}

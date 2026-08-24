//! Execution port for lifecycle gates/repairs.
//!
//! Domain must not hard-wire `Command::new("cynic")`. CLI injects
//! [`LocalArgvRunner`]; tests inject fakes.

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

/// Test double: returns fixed JSON for known commands.
#[derive(Debug, Default, Clone)]
pub struct MapArgvRunner {
    pub responses: Vec<(Vec<String>, ArgvOutput)>,
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

use std::io::Write as _;

use cynic_protocol::argv::{ArgvOutput, ArgvRunner};
use cynic_protocol::argv_allow::{ArgvObservation, observe_argv_verdict};

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
/// never blocks or alters execution.
fn observe_argv(argv: &[&str]) {
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
    if let Ok(explicit) = std::env::var("CYNIC_ARGV_OBSERVE_LOG") {
        if !explicit.trim().is_empty() {
            return Some(explicit);
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|home| format!("{home}/.local/share/cynic/argv-observe.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::argv::ArgvRunner;

    #[test]
    fn echo_runs() {
        let r = LocalArgvRunner;
        let out = r.run(&["echo", "hello"]);
        assert!(out.success());
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn false_exits_nonzero() {
        let r = LocalArgvRunner;
        let out = r.run(&["false"]);
        assert!(!out.success());
        assert_eq!(out.exit_code, Some(1));
    }
}

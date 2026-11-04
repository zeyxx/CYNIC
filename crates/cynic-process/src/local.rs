use std::io::{Read, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cynic_protocol::argv::{ArgvOutput, ArgvRunner};
use cynic_protocol::argv_allow::{ArgvObservation, observe_argv_verdict};

fn default_command_timeout() -> Duration {
    std::env::var("CYNIC_HOST_PROBE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(30))
}

/// Production runner: spawns argv[0] with remaining args.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalArgvRunner;

impl LocalArgvRunner {
    /// Spawns argv[0] with remaining args, optionally changing current directory, with a timeout.
    pub fn run_in_dir_with_timeout(
        &self,
        argv: &[&str],
        cwd: Option<&Path>,
        timeout: Duration,
    ) -> ArgvOutput {
        observe_argv(argv);
        if argv.is_empty() {
            return ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("empty argv".into()),
            };
        }
        let mut cmd = Command::new(argv[0]);
        cmd.args(&argv[1..]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(directory) = cwd.filter(|path| path.is_dir()) {
            cmd.current_dir(directory);
        }

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: Some(format!("spawn {}: {e}", argv[0])),
                };
            }
        };

        let stdout = match child.stdout.take() {
            Some(out) => out,
            None => {
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: Some("stdout pipe missing".into()),
                };
            }
        };
        let stderr = match child.stderr.take() {
            Some(err) => err,
            None => {
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: Some("stderr pipe missing".into()),
                };
            }
        };

        let stdout_reader = std::thread::spawn(move || {
            let mut r = stdout;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut r = stderr;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        });

        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout_bytes = stdout_reader.join().unwrap_or_default();
                    let stderr_bytes = stderr_reader.join().unwrap_or_default();
                    return ArgvOutput {
                        exit_code: status.code(),
                        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                        spawn_error: None,
                    };
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = child.kill();
                    let stdout_bytes = stdout_reader.join().unwrap_or_default();
                    let stderr_bytes = stderr_reader.join().unwrap_or_default();
                    return ArgvOutput {
                        exit_code: None,
                        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                        spawn_error: Some(format!("wait error: {e}")),
                    };
                }
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let stdout_bytes = stdout_reader.join().unwrap_or_default();
                let stderr_bytes = stderr_reader.join().unwrap_or_default();
                let _ = child.wait();
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                    stderr: format!(
                        "command timed out after {}ms: {}",
                        timeout.as_millis(),
                        String::from_utf8_lossy(&stderr_bytes).trim()
                    ),
                    spawn_error: Some(format!("command timed out after {}ms", timeout.as_millis())),
                };
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Spawns argv[0] with remaining args, optionally changing current directory.
    pub fn run_in_dir(&self, argv: &[&str], cwd: Option<&Path>) -> ArgvOutput {
        self.run_in_dir_with_timeout(argv, cwd, default_command_timeout())
    }
}

impl LocalArgvRunner {
    /// Spawns argv[0] with remaining args, feeding `stdin` to its standard input.
    pub fn run_with_stdin_bytes(
        &self,
        argv: &[&str],
        stdin: &[u8],
    ) -> ArgvOutput {
        self.run_with_stdin_bytes_and_timeout(argv, stdin, default_command_timeout())
    }

    pub fn run_with_stdin_bytes_and_timeout(
        &self,
        argv: &[&str],
        stdin: &[u8],
        timeout: Duration,
    ) -> ArgvOutput {
        use std::io::Write as _;
        if argv.is_empty() {
            return ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("empty argv".into()),
            };
        }
        let mut cmd = Command::new(argv[0]);
        cmd.args(&argv[1..]);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: Some(format!("spawn {}: {e}", argv[0])),
                };
            }
        };
        if let Some(mut stdin_pipe) = child.stdin.take() {
            let _ = stdin_pipe.write_all(stdin);
            let _ = stdin_pipe.flush();
            drop(stdin_pipe);
        }
        // Reap using try_wait loop with timeout, reading stdout/stderr.
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_reader = stdout.map(|out| {
            std::thread::spawn(move || {
                let mut r = out;
                let mut buf = Vec::new();
                let _ = r.read_to_end(&mut buf);
                buf
            })
        });
        let stderr_reader = stderr.map(|err| {
            std::thread::spawn(move || {
                let mut r = err;
                let mut buf = Vec::new();
                let _ = r.read_to_end(&mut buf);
                buf
            })
        });
        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout_bytes = stdout_reader
                        .map(|h| h.join().unwrap_or_default())
                        .unwrap_or_default();
                    let stderr_bytes = stderr_reader
                        .map(|h| h.join().unwrap_or_default())
                        .unwrap_or_default();
                    return ArgvOutput {
                        exit_code: status.code(),
                        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                        spawn_error: None,
                    };
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = child.kill();
                    return ArgvOutput {
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        spawn_error: Some(format!("wait error: {e}")),
                    };
                }
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return ArgvOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "command timed out".into(),
                    spawn_error: Some(format!("command timed out after {}ms", timeout.as_millis())),
                };
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl ArgvRunner for LocalArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        self.run_in_dir(argv, None)
    }

    fn run_with_stdin(&self, argv: &[&str], stdin: &[u8]) -> ArgvOutput {
        self.run_with_stdin_bytes(argv, stdin)
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

use crate::runtime_layout::current_runtime_layout;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandObservation {
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TcpObservation {
    pub host: String,
    pub port: u16,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCapture {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

fn command_observation(
    command: &str,
    args: &[&str],
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
) -> CommandObservation {
    CommandObservation {
        command: command.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        exit_code,
        stdout,
        stderr,
    }
}

pub fn capture_command(command: &str, args: &[&str]) -> Result<CommandCapture, String> {
    capture_command_in_dir(command, args, Option::<&Path>::None)
}

pub fn capture_command_in_dir(
    command: &str,
    args: &[&str],
    current_dir: Option<&Path>,
) -> Result<CommandCapture, String> {
    let mut process = Command::new(command);
    process.args(args);
    process.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = current_dir {
        process.current_dir(dir);
    }

    let mut child = process.spawn().map_err(|error| error.to_string())?;
    let deadline = Instant::now() + command_timeout();
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            return Ok(CommandCapture {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            return Ok(CommandCapture {
                success: false,
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: format!(
                    "command timed out after {}ms{}",
                    command_timeout().as_millis(),
                    timeout_stderr_suffix(&output.stderr)
                ),
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn command_timeout() -> Duration {
    std::env::var("CYNIC_HOST_PROBE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(3))
}

fn timeout_stderr_suffix(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        String::new()
    } else {
        format!("; stderr={stderr}")
    }
}

#[cfg(unix)]
pub fn reexec_current_process_with_env(extra_env: &BTreeMap<String, String>) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut cmd = Command::new(&exe);
    cmd.args(&args);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let error = cmd.exec();
    Err(error.to_string())
}

#[cfg(not(unix))]
pub fn reexec_current_process_with_env(
    _extra_env: &BTreeMap<String, String>,
) -> Result<(), String> {
    Err("reexec_current_process_with_env is only supported on unix".to_string())
}

pub fn capture_argv(argv: &[String]) -> Result<CommandCapture, String> {
    let (command, args) = argv
        .split_first()
        .ok_or_else(|| "empty argv cannot be executed".to_string())?;
    let args_ref = args.iter().map(String::as_str).collect::<Vec<_>>();
    capture_command(command, &args_ref)
}

pub async fn capture_command_with_stdin(
    command: &str,
    args: &[&str],
    stdin_payload: &str,
) -> Result<CommandCapture, String> {
    let mut child = TokioCommand::new(command)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_payload.as_bytes())
            .await
            .map_err(|error| error.to_string())?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| error.to_string())?;

    Ok(CommandCapture {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

pub fn command_json(command: &str, args: &[&str]) -> CommandObservation {
    match capture_command(command, args) {
        Ok(output) => command_observation(
            command,
            args,
            output.exit_code,
            output.stdout,
            output.stderr,
        ),
        Err(error) => command_observation(command, args, None, String::new(), error.to_string()),
    }
}

pub fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    capture_command(command, args).ok().and_then(|output| {
        if output.success && !output.stdout.is_empty() {
            Some(output.stdout)
        } else {
            None
        }
    })
}

pub fn command_exit_success(command: &str, args: &[&str]) -> Option<bool> {
    capture_command(command, args)
        .ok()
        .map(|output| output.success)
}

pub fn successful_stdout(output: std::io::Result<Output>) -> Option<String> {
    output.ok().and_then(|output| {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() { None } else { Some(value) }
        } else {
            None
        }
    })
}

pub fn stdout_non_empty(value: &CommandObservation) -> bool {
    !value.stdout.trim().is_empty()
}

pub fn command_succeeded_with_stdout(value: &CommandObservation) -> bool {
    value.exit_code == Some(0) && stdout_non_empty(value)
}

pub fn command_reports_active(value: &CommandObservation) -> bool {
    value.exit_code == Some(0) && value.stdout.trim() == "active"
}

pub fn resolved_ip_from_command(value: &CommandObservation) -> Option<String> {
    if value.exit_code != Some(0) {
        return None;
    }
    value
        .stdout
        .split_whitespace()
        .next()
        .map(|item| item.to_string())
}

pub fn resolve_host(name: &str) -> CommandObservation {
    let resolver_command = current_runtime_layout()
        .capabilities()
        .name_service
        .resolver_command;
    command_json(&resolver_command, &["hosts", name])
}

#[cfg(test)]
fn tcp_status_from_connection_result(connection_result: std::io::Result<()>) -> &'static str {
    match connection_result {
        Ok(()) => "open",
        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => "timeout",
        Err(_) => "closed",
    }
}

fn tcp_status_from_resolution<I>(resolved_addrs: Option<I>) -> &'static str
where
    I: IntoIterator<Item = std::net::SocketAddr>,
{
    if resolved_addrs
        .into_iter()
        .flat_map(|addrs| addrs.into_iter())
        .next()
        .is_some()
    {
        "resolved"
    } else {
        "unresolved"
    }
}

pub fn tcp_probe(host: &str, port: u16) -> TcpObservation {
    tcp_probe_with_timeout(host, port, Duration::from_secs(2))
}

pub fn tcp_probe_with_timeout(host: &str, port: u16, timeout: Duration) -> TcpObservation {
    let addresses = resolve_socket_addresses(host, port);
    let status = if addresses.is_empty() {
        tcp_status_from_resolution::<Vec<std::net::SocketAddr>>(None).to_string()
    } else {
        let mut saw_timeout = false;
        let mut open = false;
        for address in addresses {
            match std::net::TcpStream::connect_timeout(&address, timeout) {
                Ok(_) => {
                    open = true;
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    saw_timeout = true;
                }
                Err(_) => {}
            }
        }
        if open {
            "open"
        } else if saw_timeout {
            "timeout"
        } else {
            "closed"
        }
        .to_string()
    };
    TcpObservation {
        host: host.to_string(),
        port,
        status,
    }
}

pub fn resolve_socket_addresses(host: &str, port: u16) -> Vec<std::net::SocketAddr> {
    std::net::ToSocketAddrs::to_socket_addrs(&(host, port))
        .map(|addresses| addresses.collect())
        .unwrap_or_default()
}

pub fn tcp_open(value: &TcpObservation) -> bool {
    value.status == "open"
}

#[cfg(test)]
mod tests {
    use super::{
        command_observation, resolved_ip_from_command, tcp_status_from_connection_result,
        tcp_status_from_resolution,
    };
    use std::io;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn resolved_ip_parses_first_token_only_on_success() {
        let ok = command_observation(
            "getent",
            &["hosts", "forge.synapse.cynic"],
            Some(0),
            "10.20.0.10 forge.synapse.cynic alias".to_string(),
            String::new(),
        );
        let failed = command_observation(
            "getent",
            &["hosts", "forge.synapse.cynic"],
            Some(2),
            String::new(),
            "not found".to_string(),
        );

        assert_eq!(resolved_ip_from_command(&ok).as_deref(), Some("10.20.0.10"));
        assert_eq!(resolved_ip_from_command(&failed), None);
    }

    #[test]
    fn tcp_status_helpers_are_pure_and_deterministic() {
        let timed_out = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let refused = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 22);

        assert_eq!(tcp_status_from_connection_result(Ok(())), "open");
        assert_eq!(tcp_status_from_connection_result(Err(timed_out)), "timeout");
        assert_eq!(tcp_status_from_connection_result(Err(refused)), "closed");
        assert_eq!(tcp_status_from_resolution(Some(vec![addr])), "resolved");
        assert_eq!(
            tcp_status_from_resolution::<Vec<SocketAddr>>(None),
            "unresolved"
        );
    }
}

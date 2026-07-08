use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct CommandObservation {
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TcpObservation {
    pub host: String,
    pub port: u16,
    pub status: String,
}

pub fn command_json(command: &str, args: &[&str]) -> CommandObservation {
    match Command::new(command).args(args).output() {
        Ok(output) => CommandObservation {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(error) => CommandObservation {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
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
    value.stdout
        .split_whitespace()
        .next()
        .map(|item| item.to_string())
}

pub fn resolve_host(name: &str) -> CommandObservation {
    command_json("getent", &["hosts", name])
}

pub fn tcp_probe(host: &str, port: u16) -> TcpObservation {
    let address = format!("{host}:{port}");
    let status = std::net::ToSocketAddrs::to_socket_addrs(&address)
        .ok()
        .and_then(|mut addrs| addrs.next())
        .map(|addr| {
            std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2))
                .map(|_| "open".to_string())
                .unwrap_or_else(|error| {
                    if error.kind() == std::io::ErrorKind::TimedOut {
                        "timeout".to_string()
                    } else {
                        "closed".to_string()
                    }
                })
        })
        .unwrap_or_else(|| "unresolved".to_string());
    TcpObservation {
        host: host.to_string(),
        port,
        status,
    }
}

pub fn tcp_open(value: &TcpObservation) -> bool {
    value.status == "open"
}

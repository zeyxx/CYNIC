#![allow(dead_code)]
use cynic_host::runtime_layout::{observed_hostname, observed_local_ips, observed_service_status};

pub(super) trait WatchdogHostRuntimePort {
    fn hostname(&self) -> String;
    fn local_ips(&self) -> Vec<String>;
    fn service_status(&self, service: &str) -> String;
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct LocalWatchdogHostRuntime;

#[cfg(test)]
fn hostname_from_env_or_command(
    env_hostname: Option<String>,
    command_stdout: Option<String>,
) -> String {
    env_hostname
        .filter(|value| !value.is_empty())
        .or(command_stdout)
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
fn local_ips_from_stdout(command_stdout: Option<String>) -> Vec<String> {
    command_stdout
        .map(|stdout| stdout.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

#[cfg(test)]
fn service_status_from_stdout(command_stdout: Option<String>) -> String {
    command_stdout.unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn hostname() -> String {
    observed_hostname()
}

pub(super) fn local_ips() -> Vec<String> {
    observed_local_ips()
}

pub(super) fn systemd_service_status(service: &str) -> String {
    observed_service_status(service)
}

impl WatchdogHostRuntimePort for LocalWatchdogHostRuntime {
    fn hostname(&self) -> String {
        hostname()
    }

    fn local_ips(&self) -> Vec<String> {
        local_ips()
    }

    fn service_status(&self, service: &str) -> String {
        systemd_service_status(service)
    }
}

#[cfg(test)]
mod tests {
    use super::{hostname_from_env_or_command, local_ips_from_stdout, service_status_from_stdout};

    #[test]
    fn hostname_prefers_env_then_command_then_unknown() {
        assert_eq!(
            hostname_from_env_or_command(Some("node-a".to_string()), Some("node-b".to_string())),
            "node-a"
        );
        assert_eq!(
            hostname_from_env_or_command(None, Some("node-b".to_string())),
            "node-b"
        );
        assert_eq!(hostname_from_env_or_command(None, None), "unknown");
    }

    #[test]
    fn local_ips_parser_splits_whitespace() {
        assert_eq!(
            local_ips_from_stdout(Some("10.20.0.2 192.168.0.10".to_string())),
            vec!["10.20.0.2".to_string(), "192.168.0.10".to_string()]
        );
        assert!(local_ips_from_stdout(None).is_empty());
    }

    #[test]
    fn service_status_defaults_to_unknown() {
        assert_eq!(
            service_status_from_stdout(Some("active".to_string())),
            "active"
        );
        assert_eq!(service_status_from_stdout(None), "unknown");
    }
}

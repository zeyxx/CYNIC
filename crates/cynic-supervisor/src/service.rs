use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_protocol::{
    SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID, SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
    ServiceSupervisorObservation, ServiceSupervisorProviderAttempt,
    ServiceSupervisorUnitObservation, service_supervisor_status,
};

use cynic_protocol::{ServicePolicy, ServiceUnitPolicy};
use cynic_process::LocalArgvRunner;
use cynic_protocol::argv::ArgvRunner;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorProvider {
    Auto,
    SystemdSystem,
    SystemdUser,
    OpenRc,
    ProcessPort,
}

impl SupervisorProvider {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "systemd-system" | "system" => Ok(Self::SystemdSystem),
            "systemd-user" | "user" => Ok(Self::SystemdUser),
            "openrc" => Ok(Self::OpenRc),
            "process-port" | "port" => Ok(Self::ProcessPort),
            other => Err(format!("unknown supervisor provider: {other}")),
        }
    }
}

pub fn observe_service_supervisor(
    subject: String,
    policy: &ServicePolicy,
) -> ServiceSupervisorObservation {
    observe_service_supervisor_with_provider(subject, policy, SupervisorProvider::Auto)
}

pub fn observe_service_supervisor_with_provider(
    subject: String,
    policy: &ServicePolicy,
    provider: SupervisorProvider,
) -> ServiceSupervisorObservation {
    let services = policy
        .units
        .iter()
        .map(|unit| observe_unit(unit, provider))
        .collect::<Vec<_>>();
    let (status, active_count, degraded_count, unknown_count) =
        service_supervisor_status(&services);

    ServiceSupervisorObservation {
        schema: SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
        evidence_id: SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID,
        status,
        subject,
        supervisor_kind: provider_name(provider).to_string(),
        supervisor_scope: provider_scope(provider).to_string(),
        target_source: policy_source(policy),
        observed_at_unix_ms: observed_at_unix_ms(),
        service_count: services.len(),
        active_count,
        degraded_count,
        unknown_count,
        services,
    }
}

fn observe_unit(
    unit: &ServiceUnitPolicy,
    provider: SupervisorProvider,
) -> ServiceSupervisorUnitObservation {
    let effective_provider = effective_provider(unit, provider);
    let attempts = match effective_provider {
        SupervisorProvider::Auto => vec![
            probe_systemd(unit, SupervisorProvider::SystemdSystem),
            probe_systemd(unit, SupervisorProvider::SystemdUser),
            probe_openrc(unit),
            probe_process_port(unit),
        ],
        SupervisorProvider::SystemdSystem => {
            vec![probe_systemd(unit, SupervisorProvider::SystemdSystem)]
        }
        SupervisorProvider::SystemdUser => {
            vec![probe_systemd(unit, SupervisorProvider::SystemdUser)]
        }
        SupervisorProvider::OpenRc => vec![probe_openrc(unit)],
        SupervisorProvider::ProcessPort => vec![probe_process_port(unit)],
    };
    let selected = select_attempt(&attempts);
    let fields = selected.fields.clone().unwrap_or_default();
    let load_state = field_or_unknown(&fields, "LoadState");
    let unit_file_state = field_or_unknown(&fields, "UnitFileState");
    let active_state = selected
        .active_state
        .clone()
        .unwrap_or_else(|| field_or_unknown(&fields, "ActiveState"));
    let sub_state = field_or_unknown(&fields, "SubState");
    let restart_count = fields
        .get("NRestarts")
        .and_then(|value| value.parse::<u64>().ok());
    let active_enter_timestamp_monotonic = fields
        .get("ActiveEnterTimestampMonotonic")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);

    ServiceSupervisorUnitObservation {
        service_id: policy_service_id(unit),
        unit_name: unit.unit_name.clone(),
        health_port: unit.health_port,
        health_kind: unit.health_kind.clone(),
        desired_provider: unit.supervisor_provider.clone(),
        selected_provider: selected.provider.clone(),
        selected_scope: selected.scope.clone(),
        target_source: policy_unit_source(unit),
        desired_state: if unit.enable_on_boot {
            "enabled_and_active"
        } else {
            "active"
        }
        .to_string(),
        restart_policy: unit.restart_mode.clone(),
        probe_status: selected.probe_status,
        probe_detail: selected.probe_detail.clone(),
        provider_attempts: attempts
            .iter()
            .map(|attempt| ServiceSupervisorProviderAttempt {
                provider: attempt.provider.clone(),
                scope: attempt.scope.clone(),
                probe_status: attempt.probe_status,
                probe_detail: attempt.probe_detail.clone(),
            })
            .collect(),
        load_state,
        unit_file_state,
        active_state,
        sub_state,
        restart_count,
        active_enter_timestamp_monotonic,
        status: selected.status,
    }
}

fn effective_provider(
    unit: &ServiceUnitPolicy,
    requested: SupervisorProvider,
) -> SupervisorProvider {
    if requested != SupervisorProvider::Auto {
        return requested;
    }
    unit.supervisor_provider
        .as_deref()
        .and_then(|value| SupervisorProvider::parse(value).ok())
        .unwrap_or(SupervisorProvider::Auto)
}

#[derive(Debug, Clone)]
struct ProbeAttempt {
    provider: String,
    scope: String,
    probe_status: &'static str,
    probe_detail: String,
    status: &'static str,
    active_state: Option<String>,
    fields: Option<BTreeMap<String, String>>,
}

fn select_attempt(attempts: &[ProbeAttempt]) -> &ProbeAttempt {
    attempts
        .iter()
        .find(|attempt| attempt.status == "active")
        .or_else(|| attempts.iter().find(|attempt| attempt.probe_status == "ok"))
        .or_else(|| {
            attempts
                .iter()
                .find(|attempt| attempt.probe_status == "blocked")
        })
        .or_else(|| attempts.first())
        .expect("at least one probe attempt")
}

fn probe_systemd(unit: &ServiceUnitPolicy, provider: SupervisorProvider) -> ProbeAttempt {
    let runner = LocalArgvRunner;
    let mut argv: Vec<String> = Vec::new();
    argv.push("systemctl".to_string());
    if provider == SupervisorProvider::SystemdUser {
        argv.push("--user".to_string());
    }
    argv.extend([
        "show".to_string(),
        unit.unit_name.clone(),
        "--no-page".to_string(),
        "-p".to_string(),
        "LoadState".to_string(),
        "-p".to_string(),
        "UnitFileState".to_string(),
        "-p".to_string(),
        "ActiveState".to_string(),
        "-p".to_string(),
        "SubState".to_string(),
        "-p".to_string(),
        "NRestarts".to_string(),
        "-p".to_string(),
        "ActiveEnterTimestampMonotonic".to_string(),
    ]);
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let output = runner.run(&argv_refs);
    if let Some(err) = output.spawn_error {
        return ProbeAttempt {
            provider: "systemd".to_string(),
            scope: provider_scope(provider).to_string(),
            probe_status: "error",
            probe_detail: err,
            status: "unknown",
            active_state: None,
            fields: None,
        };
    }
    if output.exit_code != Some(0) {
        return ProbeAttempt {
            provider: "systemd".to_string(),
            scope: provider_scope(provider).to_string(),
            probe_status: "blocked",
            probe_detail: if output.stderr.is_empty() {
                format!("systemctl show failed exit_code={:?}", output.exit_code)
            } else {
                format!(
                    "systemctl show failed exit_code={:?} stderr={}",
                    output.exit_code, output.stderr
                )
            },
            status: "unknown",
            active_state: None,
            fields: None,
        };
    }
    let fields = parse_systemctl_show(&output.stdout);
    let load_state = field_or_unknown(&fields, "LoadState");
    let active_state = field_or_unknown(&fields, "ActiveState");
    ProbeAttempt {
        provider: "systemd".to_string(),
        scope: provider_scope(provider).to_string(),
        probe_status: "ok",
        probe_detail: format!("systemctl show succeeded exit_code={:?}", output.exit_code),
        status: unit_status(&load_state, &active_state),
        active_state: Some(active_state),
        fields: Some(fields),
    }
}

fn probe_openrc(unit: &ServiceUnitPolicy) -> ProbeAttempt {
    let service = openrc_service_name(unit);
    let runner = LocalArgvRunner;
    let output = runner.run(&["rc-service", service.as_str(), "status"]);
    if let Some(err) = output.spawn_error {
        return ProbeAttempt {
            provider: "openrc".to_string(),
            scope: "system".to_string(),
            probe_status: "error",
            probe_detail: err,
            status: "unknown",
            active_state: None,
            fields: None,
        };
    }
    if !output.success() {
        return ProbeAttempt {
            provider: "openrc".to_string(),
            scope: "system".to_string(),
            probe_status: "blocked",
            probe_detail: if output.stderr.is_empty() {
                format!("rc-service status failed exit_code={:?}", output.exit_code)
            } else {
                format!(
                    "rc-service status failed exit_code={:?} stderr={}",
                    output.exit_code, output.stderr
                )
            },
            status: "unknown",
            active_state: None,
            fields: None,
        };
    }
    ProbeAttempt {
        provider: "openrc".to_string(),
        scope: "system".to_string(),
        probe_status: "ok",
        probe_detail: format!("rc-service status succeeded stdout={}", output.stdout),
        status: if output.stdout.contains("started") || output.stdout.contains("running") {
            "active"
        } else {
            "degraded"
        },
        active_state: Some(output.stdout),
        fields: None,
    }
}

fn probe_process_port(unit: &ServiceUnitPolicy) -> ProbeAttempt {
    if let Some(port) = unit.health_port {
        use std::net::{SocketAddr, TcpStream};
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let observation_status = match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
            Ok(_) => "open",
            Err(_) => "closed",
        };
        return match observation_status {
            "open" => ProbeAttempt {
                provider: "process_port".to_string(),
                scope: "local_tcp".to_string(),
                probe_status: "ok",
                probe_detail: format!(
                    "tcp probe host=127.0.0.1 port={} status=open",
                    port
                ),
                status: "active",
                active_state: Some("tcp_open".to_string()),
                fields: None,
            },
            _ => ProbeAttempt {
                provider: "process_port".to_string(),
                scope: "local_tcp".to_string(),
                probe_status: "blocked",
                probe_detail: format!(
                    "tcp probe host=127.0.0.1 port={} status=closed",
                    port
                ),
                status: "unknown",
                active_state: Some("tcp_closed".to_string()),
                fields: None,
            },
        };
    }

    if let Some(process_name) = unit.process_name.as_deref() {
        let runner = LocalArgvRunner;
        let out = runner.run(&["pgrep", "-x", process_name]);
        let ok = out.success();
        return ProbeAttempt {
            provider: "process".to_string(),
            scope: "local_process".to_string(),
            probe_status: if ok { "ok" } else { "blocked" },
            probe_detail: format!("pgrep -x {process_name} returned {ok}"),
            status: if ok { "active" } else { "unknown" },
            active_state: Some(
                if ok {
                    "process_running"
                } else {
                    "process_absent"
                }
                .to_string(),
            ),
            fields: None,
        };
    }

    ProbeAttempt {
        provider: "process_port".to_string(),
        scope: "local".to_string(),
        probe_status: "unsupported",
        probe_detail: "no health_port or process_name declared".to_string(),
        status: "unknown",
        active_state: None,
        fields: None,
    }
}

fn provider_name(provider: SupervisorProvider) -> &'static str {
    match provider {
        SupervisorProvider::Auto => "auto",
        SupervisorProvider::SystemdSystem | SupervisorProvider::SystemdUser => "systemd",
        SupervisorProvider::OpenRc => "openrc",
        SupervisorProvider::ProcessPort => "process_port",
    }
}

fn provider_scope(provider: SupervisorProvider) -> &'static str {
    match provider {
        SupervisorProvider::Auto => "auto",
        SupervisorProvider::SystemdSystem => "system",
        SupervisorProvider::SystemdUser => "user",
        SupervisorProvider::OpenRc => "system",
        SupervisorProvider::ProcessPort => "local",
    }
}

fn openrc_service_name(unit: &ServiceUnitPolicy) -> String {
    openrc_unit_name(&unit.unit_name)
}

fn policy_source(policy: &ServicePolicy) -> String {
    let mut sources = policy
        .units
        .iter()
        .map(policy_unit_source)
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    if sources.is_empty() {
        "unknown".to_string()
    } else {
        sources.join(",")
    }
}

fn policy_unit_source(unit: &ServiceUnitPolicy) -> String {
    if unit.source.is_empty() {
        "unknown".to_string()
    } else {
        unit.source.clone()
    }
}

fn policy_service_id(unit: &ServiceUnitPolicy) -> String {
    if unit.service_id.is_empty() {
        unit.unit_name.clone()
    } else {
        unit.service_id.clone()
    }
}

fn parse_systemctl_show(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn field_or_unknown(fields: &BTreeMap<String, String>, key: &str) -> String {
    fields
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

fn unit_status(load_state: &str, active_state: &str) -> &'static str {
    if load_state == "unknown" || active_state == "unknown" {
        "unknown"
    } else if load_state == "loaded" && active_state == "active" {
        "active"
    } else {
        "degraded"
    }
}


fn openrc_service_name(unit: &ServiceUnitPolicy) -> String {
    unit.unit_name
        .strip_suffix(".service")
        .unwrap_or(&unit.unit_name)
        .to_string()
}

fn observed_at_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_systemctl_show_key_value_lines() {
        let fields = parse_systemctl_show(
            "LoadState=loaded\nUnitFileState=enabled\nActiveState=active\nNRestarts=2\n",
        );
        assert_eq!(fields.get("LoadState").map(String::as_str), Some("loaded"));
        assert_eq!(fields.get("NRestarts").map(String::as_str), Some("2"));
    }

    #[test]
    fn unit_status_marks_inactive_loaded_unit_as_degraded() {
        assert_eq!(unit_status("loaded", "inactive"), "degraded");
    }

    #[test]
    fn openrc_service_name_strips_systemd_suffix() {
        let unit = ServiceUnitPolicy {
            service_id: "node".to_string(),
            unit_name: "cynic-node.service".to_string(),
            enable_on_boot: true,
            restart_mode: "on-failure".to_string(),
            source: "test".to_string(),
            health_port: None,
            process_name: None,
            supervisor_provider: None,
            health_kind: None,
        };
        assert_eq!(openrc_service_name(&unit), "cynic-node");
    }
}

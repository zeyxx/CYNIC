use cynic_host::runtime_layout::observed_hostname;
use serde::Serialize;

use crate::adapters::host_observation::{
    CommandObservation, command_reports_active, resolved_ip_from_command,
};
use crate::domain::network_projection::{DnsProjectionObservation, dns_projection_observation};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsResolutionObservation {
    pub(crate) host: String,
    pub(crate) expected_synapse_ip: Option<String>,
    pub(crate) resolved_ip: Option<String>,
    pub(crate) resolution_status: String,
    pub(crate) command: CommandObservation,
}

pub(crate) fn dns_resolution_observation(
    host: &str,
    expected_synapse_ip: Option<String>,
    command: CommandObservation,
) -> DnsResolutionObservation {
    let resolved_ip = resolved_ip_from_command(&command);
    let resolution_status = match (&expected_synapse_ip, &resolved_ip) {
        (_, None) => "unresolved".to_string(),
        (Some(expected), Some(resolved)) if expected == resolved => {
            "resolved_expected_synapse_ip".to_string()
        }
        (Some(_), Some(_)) => "resolved_noncanonical_ip".to_string(),
        (None, Some(_)) => "resolved_without_expected_synapse_ip".to_string(),
    };
    DnsResolutionObservation {
        host: host.to_string(),
        expected_synapse_ip,
        resolved_ip,
        resolution_status,
        command,
    }
}

pub(crate) fn dns_projection_surface(
    dns_checks: &[DnsResolutionObservation],
    provider_fallback_active: bool,
    systemd_resolved: &CommandObservation,
) -> DnsProjectionObservation {
    let dns_ok = dns_checks
        .iter()
        .all(|item| item.command.exit_code == Some(0));
    let resolved_matches_expected_synapse = dns_checks.iter().all(|item| {
        matches!(
            item.resolution_status.as_str(),
            "resolved_expected_synapse_ip" | "resolved_without_expected_synapse_ip"
        )
    });
    dns_projection_observation(
        dns_ok,
        resolved_matches_expected_synapse,
        provider_fallback_active,
        command_reports_active(systemd_resolved),
    )
}

pub(crate) fn hostname() -> String {
    observed_hostname()
}

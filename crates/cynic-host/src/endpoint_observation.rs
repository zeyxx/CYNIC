use serde::Serialize;

use crate::process_observation::{resolve_socket_addresses, tcp_probe};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndpointObservation {
    pub endpoint: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub resolved_addresses: Vec<String>,
    pub resolution_status: String,
    pub tcp_reachability_status: String,
}

pub fn observe_tcp_endpoint(endpoint: &str, declared_port: Option<u16>) -> EndpointObservation {
    let host = endpoint_host(endpoint);
    let port = endpoint_port(endpoint, declared_port);
    let addresses = match (host.as_deref(), port) {
        (Some(host), Some(port)) => resolve_socket_addresses(host, port),
        _ => Vec::new(),
    };
    let resolution_status = if addresses.is_empty() {
        "unresolved"
    } else {
        "resolved"
    };
    let tcp_reachability_status = match (host.as_deref(), port) {
        (Some(host), Some(port)) => match tcp_probe(host, port).status.as_str() {
            "open" => "open",
            "unresolved" => "unresolved",
            _ => "closed_or_filtered",
        },
        _ => "unresolved",
    };
    EndpointObservation {
        endpoint: endpoint.to_string(),
        host,
        port,
        resolved_addresses: addresses
            .into_iter()
            .map(|address| address.to_string())
            .collect(),
        resolution_status: resolution_status.to_string(),
        tcp_reachability_status: tcp_reachability_status.to_string(),
    }
}

fn endpoint_host(endpoint: &str) -> Option<String> {
    let without_scheme = endpoint
        .split_once("://")
        .map(|(_, authority)| authority)
        .unwrap_or(endpoint);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host = authority
        .rsplit_once('@')
        .map(|(_, right)| right)
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or(authority)
        .trim();
    (!host.is_empty()).then(|| host.to_string())
}

fn endpoint_port(endpoint: &str, declared_port: Option<u16>) -> Option<u16> {
    let without_scheme = endpoint
        .split_once("://")
        .map(|(_, authority)| authority)
        .unwrap_or(endpoint);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    if let Some((_, value)) = authority.rsplit_once(':')
        && let Ok(port) = value.parse::<u16>()
    {
        return Some(port);
    }
    declared_port.or_else(
        || match endpoint.split_once("://").map(|(scheme, _)| scheme) {
            Some("https") => Some(443),
            Some("http") => Some(80),
            Some("ssh") => Some(22),
            _ => None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observes_ssh_identity_without_http_assumptions() {
        let observation = observe_tcp_endpoint("ssh://git@forgejo.invalid/repo.git", None);
        assert_eq!(observation.host.as_deref(), Some("forgejo.invalid"));
        assert_eq!(observation.port, Some(22));
        assert_eq!(observation.resolution_status, "unresolved");
    }
}

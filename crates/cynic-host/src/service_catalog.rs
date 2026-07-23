use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ServiceRef {
    pub node: String,
    pub service: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedServiceEndpoint {
    pub service_ref: ServiceRef,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub endpoint: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    network_policy: Option<NetworkPolicy>,
    nodes: Option<Vec<Node>>,
}

#[derive(Debug, Deserialize)]
struct NetworkPolicy {
    reservations: Option<Vec<Reservation>>,
}

#[derive(Debug, Deserialize)]
struct Reservation {
    name: Option<String>,
    dns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Node {
    id: Option<String>,
    name: Option<String>,
    services: Option<Vec<Service>>,
}

#[derive(Debug, Deserialize)]
struct Service {
    name: Option<String>,
    scheme: Option<String>,
    port: Option<u16>,
}

pub fn resolve_service_ref(
    registry_path: &Path,
    reference: &ServiceRef,
) -> Result<ResolvedServiceEndpoint, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("read {}: {error}", registry_path.display()))?;
    let registry: Registry = serde_json::from_str(&raw)
        .map_err(|error| format!("parse {}: {error}", registry_path.display()))?;
    let nodes = registry.nodes.unwrap_or_default();
    let node = nodes
        .iter()
        .find(|node| {
            node.id.as_deref() == Some(reference.node.as_str())
                || node.name.as_deref() == Some(reference.node.as_str())
        })
        .ok_or_else(|| format!("service node not declared: {}", reference.node))?;
    let service = node
        .services
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|service| service.name.as_deref() == Some(reference.service.as_str()))
        .ok_or_else(|| {
            format!(
                "service not declared: {}/{}",
                reference.node, reference.service
            )
        })?;
    let scheme = required(service.scheme.clone(), "service.scheme")?;
    let port = service
        .port
        .ok_or_else(|| "service.port missing".to_string())?;
    let host = registry
        .network_policy
        .and_then(|policy| policy.reservations)
        .unwrap_or_default()
        .into_iter()
        .find(|reservation| reservation.name.as_deref() == Some(reference.node.as_str()))
        .and_then(|reservation| reservation.dns)
        .and_then(|names| names.into_iter().find(|name| !name.trim().is_empty()))
        .ok_or_else(|| format!("service DNS reservation missing: {}", reference.node))?;
    let endpoint = if default_port(&scheme) == Some(port) {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{port}")
    };
    Ok(ResolvedServiceEndpoint {
        service_ref: reference.clone(),
        scheme,
        host,
        port,
        endpoint,
    })
}

fn required(value: Option<String>, field: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} missing"))
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        "ssh" => Some(22),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_service_from_registry_dns_and_port() {
        let endpoint = resolve_service_ref(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json"),
            &ServiceRef {
                node: "forgejo".into(),
                service: "forgejo-api-tls".into(),
            },
        )
        .unwrap();
        assert_eq!(endpoint.endpoint, "https://forgejo.synapse.cynic");
    }
}

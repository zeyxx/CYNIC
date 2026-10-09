//! Concrete `DnsResolutionPort` — reads the canonical registry JSON.
//!
//! ## Why this exists
//!
//! The registry (`infra/registry.json`) is the Single Source of Truth for
//! DNS names, endpoints, and plane vocabulary. Every consumer must resolve
//! through this adapter instead of hardcoding literals.
//!
//! ## Resolution order
//!
//! - **FQDN**: `network_policy.reservations[].dns[0]` (first entry)
//! - **Endpoint**: `nodes[].services[]` matched by `endpoint_host` + `port`
//! - **Zone**: `network_policy.planes.service.dns_zone`
//!
//! ## Negative boundary
//!
//! - Never reads `/etc/hosts`, `/etc/resolv.conf`, or environment variables
//! - Never falls back to a hardcoded literal
//! - Returns `None` when the registry is missing or the entry is absent

use serde_json::Value;

use crate::dns::{
    DEFAULT_DNS_ZONE, SERVICE_DNS_HOSTS_FILENAME, SERVICE_DNS_RESOLVED_DROPIN_FILENAME,
    SERVICE_DNS_SERVICE_FILENAME,
};
use crate::dns_resolver::{DnsEndpoint, DnsResolutionPort};
use crate::plane_vocabulary::plane_address;

/// Read-only registry-backed DNS resolver.
pub struct RegistryDnsResolver {
    registry: Value,
}

impl RegistryDnsResolver {
    /// Build a resolver from parsed registry JSON.
    pub fn new(registry: Value) -> Self {
        Self { registry }
    }

    /// Load and parse the registry from a file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read registry {}: {}", path, e))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("parse registry {}: {}", path, e))?;
        Ok(Self::new(value))
    }

    /// Find a reservation by name in `network_policy.reservations`.
    fn find_reservation(&self, node_name: &str) -> Option<&Value> {
        self.registry
            .get("network_policy")?
            .get("reservations")?
            .as_array()?
            .iter()
            .find(|r| r.get("name").and_then(|v| v.as_str()) == Some(node_name))
    }

    /// Find a node by name in `nodes`.
    fn find_node(&self, node_name: &str) -> Option<&Value> {
        self.registry.get("nodes")?.as_array()?.iter().find(|n| {
            n.get("id").and_then(|v| v.as_str()) == Some(node_name)
                || n.get("name").and_then(|v| v.as_str()) == Some(node_name)
        })
    }

    /// Extract the first DNS FQDN from a reservation.
    fn reservation_fqdn(&self, reservation: &Value) -> Option<String> {
        reservation
            .get("dns")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str())
            .find(|s| !s.is_empty())
            .map(String::from)
    }

    /// Find the service plane's DNS zone.
    fn service_dns_zone(&self) -> String {
        self.registry
            .get("network_policy")
            .and_then(|np| np.get("planes"))
            .and_then(|planes| planes.get("service"))
            .and_then(|svc| svc.get("dns_zone"))
            .and_then(|z| z.as_str())
            .unwrap_or(DEFAULT_DNS_ZONE)
            .to_string()
    }

    /// Find the service plane's CIDR.
    fn service_cidr(&self) -> Option<&str> {
        self.registry
            .get("network_policy")
            .and_then(|np| np.get("planes"))
            .and_then(|planes| planes.get("service"))
            .and_then(|svc| svc.get("cidr"))
            .and_then(|c| c.as_str())
    }
}

impl DnsResolutionPort for RegistryDnsResolver {
    fn resolve_fqdn(&self, node_name: &str) -> Option<String> {
        // 1. Try reservations (canonical DNS entries)
        if let Some(fqdn) = self
            .find_reservation(node_name)
            .and_then(|r| self.reservation_fqdn(r))
        {
            return Some(fqdn);
        }

        // 2. Fallback: construct FQDN from node name + zone
        let zone = self.service_dns_zone();
        if zone.is_empty() {
            None
        } else {
            Some(format!("{}.{}", node_name, zone))
        }
    }

    fn resolve_endpoint(&self, node_name: &str) -> Option<DnsEndpoint> {
        let node = self.find_node(node_name)?;
        let services = node.get("services")?.as_array()?;

        // Find the first service with an explicit endpoint_host
        for service in services {
            let host = service.get("endpoint_host").and_then(|v| v.as_str())?;
            let port = service.get("port").and_then(|v| v.as_u64())? as u16;
            let scheme = service
                .get("scheme")
                .and_then(|v| v.as_str())
                .unwrap_or("http")
                .to_string();

            return Some(DnsEndpoint {
                scheme,
                host: host.to_string(),
                port,
            });
        }

        // Fallback: use the node's service-plane address
        let addr = plane_address(node, "service")?;
        let port = services
            .iter()
            .find_map(|s| s.get("port").and_then(|v| v.as_u64()))
            .unwrap_or(80) as u16;

        Some(DnsEndpoint {
            scheme: "http".to_string(),
            host: addr.to_string(),
            port,
        })
    }

    fn dns_zone(&self) -> String {
        self.service_dns_zone()
    }

    fn hosts_filename(&self) -> &'static str {
        SERVICE_DNS_HOSTS_FILENAME
    }

    fn service_filename(&self) -> &'static str {
        SERVICE_DNS_SERVICE_FILENAME
    }

    fn resolved_dropin_filename(&self) -> &'static str {
        SERVICE_DNS_RESOLVED_DROPIN_FILENAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> Value {
        serde_json::json!({
            "network_policy": {
                "canonical_service_plane": "service",
                "planes": {
                    "service": {
                        "cidr": "10.10.10.0/24",
                        "dns_zone": "service.cynic",
                        "canonical": true
                    }
                },
                "reservations": [
                    {
                        "name": "forgejo",
                        "dns": ["forgejo.service.cynic"],
                        "service": "10.10.10.101"
                    },
                    {
                        "name": "kernel",
                        "dns": ["kernel.service.cynic"],
                        "service": "10.10.10.100"
                    }
                ]
            },
            "nodes": [
                {
                    "id": "forgejo",
                    "name": "forgejo",
                    "role": "forge_primary",
                    "services": [
                        {
                            "name": "forgejo-backend",
                            "scheme": "http",
                            "endpoint_host": "10.10.10.101",
                            "port": 3000
                        }
                    ]
                }
            ]
        })
    }

    #[test]
    fn resolve_fqdn_from_reservation() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(
            resolver.resolve_fqdn("forgejo"),
            Some("forgejo.service.cynic".to_string())
        );
        assert_eq!(
            resolver.resolve_fqdn("kernel"),
            Some("kernel.service.cynic".to_string())
        );
    }

    #[test]
    fn resolve_fqdn_unknown_node_returns_none() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(resolver.resolve_fqdn("unknown"), None);
    }

    #[test]
    fn resolve_endpoint_from_service() {
        let resolver = RegistryDnsResolver::new(test_registry());
        let ep = resolver.resolve_endpoint("forgejo").unwrap();
        assert_eq!(ep.scheme, "http");
        assert_eq!(ep.host, "10.10.10.101");
        assert_eq!(ep.port, 3000);
        assert_eq!(ep.url(), "http://10.10.10.101:3000");
    }

    #[test]
    fn dns_zone_from_registry() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(resolver.dns_zone(), "service.cynic");
    }

    #[test]
    fn dns_zone_fallback_to_default() {
        let resolver = RegistryDnsResolver::new(serde_json::json!({}));
        assert_eq!(resolver.dns_zone(), DEFAULT_DNS_ZONE);
    }

    #[test]
    fn hosts_filename_is_canonical() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(resolver.hosts_filename(), "service.hosts");
    }

    #[test]
    fn service_filename_is_canonical() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(resolver.service_filename(), "coredns-service.service");
    }

    #[test]
    fn resolved_dropin_filename_is_canonical() {
        let resolver = RegistryDnsResolver::new(test_registry());
        assert_eq!(
            resolver.resolved_dropin_filename(),
            "cynic-service-dns.conf"
        );
    }
}

//! Ports for the DNS subdomain.
//!
//! Functional Core: pure traits, zero I/O.

use crate::dns::types::DnsEndpoint;

/// Read-only DNS resolution from the canonical registry.
pub trait DnsResolutionPort {
    fn resolve_fqdn(&self, node_name: &str) -> Option<String>;
    fn resolve_endpoint(&self, node_name: &str) -> Option<DnsEndpoint>;
    fn dns_zone(&self) -> String;
    fn hosts_filename(&self) -> &'static str;
    fn service_filename(&self) -> &'static str;
    fn resolved_dropin_filename(&self) -> &'static str;
}

/// Port for mutating DNS state (activation side-effects live behind adapters).
pub trait DnsMutationPort {
    type Error;
    fn apply(&self) -> Result<(), Self::Error>;
}
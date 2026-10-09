//! DNS resolution port — the single authority for FQDN and endpoint resolution.
//!
//! ## Why this exists
//!
//! Before this port, every consumer hardcoded DNS literals (`"synapse.cynic"`,
//! `"forgejo.synapse.cynic"`, `"/etc/cynic/dns/synapse.hosts"`) in ~40 files.
//! When the canonical zone migrated from `synapse.cynic` to `service.cynic`,
//! all 40 files broke silently — no compiler error, just wrong resolution.
//!
//! This port is the ONLY place that knows how to resolve a node name to its
//! FQDN or endpoint. Consumers import the trait and call it; they never
//! spell a literal.
//!
//! ## FCIS
//!
//! - **Functional Core**: this module (pure trait + types, zero I/O)
//! - **Imperative Shell**: `adapters/dns_resolver.rs` (reads registry JSON)

use serde::{Deserialize, Serialize};

/// A resolved network endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsEndpoint {
    /// URL scheme (`http`, `https`, `ssh`).
    pub scheme: String,
    /// Host address (IP or FQDN).
    pub host: String,
    /// Port number.
    pub port: u16,
}

impl DnsEndpoint {
    /// Build a `scheme://host:port` URL.
    pub fn url(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.port)
    }
}

/// Read-only DNS resolution from the canonical registry.
///
/// Implementors MUST resolve from `infra/registry.json` — never from
/// hardcoded literals, `/etc/hosts`, or environment variables.
pub trait DnsResolutionPort {
    /// Resolve a node name to its canonical FQDN.
    ///
    /// Returns `None` when the node is not in the registry or has no
    /// DNS reservation.
    fn resolve_fqdn(&self, node_name: &str) -> Option<String>;

    /// Resolve a node name to its primary service endpoint.
    ///
    /// Returns `None` when the node has no declared service.
    fn resolve_endpoint(&self, node_name: &str) -> Option<DnsEndpoint>;

    /// The canonical DNS zone for the service plane.
    fn dns_zone(&self) -> String;

    /// The canonical hosts filename (without directory).
    fn hosts_filename(&self) -> &'static str;

    /// The canonical CoreDNS service unit filename.
    fn service_filename(&self) -> &'static str;

    /// The canonical systemd-resolved drop-in filename.
    fn resolved_dropin_filename(&self) -> &'static str;
}

//! Pure domain models for the DNS subdomain of cynic-network.
//!
//! Functional Core: zero I/O, zero side effects.

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

/// A DNS host record rendered into hosts/CoreDNS files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    pub ip: String,
    pub fqdn: String,
    pub aliases: Vec<String>,
}

/// Backward-compatible alias for the historical name.
pub type DnsHostRecord = DnsRecord;

/// A DNS zone (canonical or compat).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsZone {
    pub name: String,
}

impl DnsZone {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Errors raised by DNS resolution ports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsResolutionError {
    NotFound,
    InvalidInput(String),
    Unavailable(String),
}
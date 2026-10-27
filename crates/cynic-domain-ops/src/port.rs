//! Driven ports for external domain operations.
//!
//! These traits define the interface between the pure domain logic (in
//! `cynic-domain-ops::domain`) and the provider adapters (in
//! `cynic-domain-ops::adapters`). The runner injects concrete implementations.
//!
//! Authority: foundation.json §37 (portable_host_contract) — systemd/k8s/ssh/
//! rest/mcp/shell are adapters behind portable ports.

use cynic_protocol::domain_ops::{
    DelegationProof, DnsRecordShape, DomainAvailability, EdgeProbeResult, NameserverUpdateReceipt,
    RegistrationReceipt,
};

// SecretProviderPort + SecretStorePort re-exported from cynic-secrets
// (horizontal secret management layer shared by ALL CYNIC crates).
// Backwards-compatible alias: SecretProvider = SecretProviderPort.

/// Port for registrar operations (domain availability, registration, NS).
///
/// Implemented by HTTP adapters (e.g. Porkbun API client). Read-only methods
/// are safe to call frequently; mutation methods require operator consent.
pub trait RegistrarPort {
    /// Check domain availability. Always read-only.
    fn check_availability(&self, domain: &str) -> DomainAvailability;

    /// Read current registrar nameservers for a domain. Read-only.
    fn read_nameservers(&self, domain: &str) -> Vec<String>;

    /// Read all DNS records hosted at the registrar. Read-only.
    fn read_zone_records(&self, domain: &str) -> Vec<DnsRecordShape>;

    /// Check pricing for domain registration. Read-only.
    fn check_pricing(&self, domain: &str, years: u8) -> Option<f64>;

    /// Register a domain (paid operation). Requires idempotency key.
    fn register_domain(
        &self,
        domain: &str,
        years: u8,
        auto_renew: bool,
        idempotency_key: &str,
    ) -> RegistrationReceipt;

    /// Update registrar nameservers. Requires idempotency key.
    /// NS cutover is irreversible-ish — caller must confirm separately.
    fn update_nameservers(
        &self,
        domain: &str,
        nameservers: &[String],
        idempotency_key: &str,
    ) -> NameserverUpdateReceipt;

    /// Export raw zone backup from registrar for archival.
    fn export_zone_backup(&self, domain: &str) -> String;
}

/// Port for DNS authority operations (zone management, record CRUD).
///
/// Implementations include `CloudflareClient` and `PorkbunRegistrar`.
/// Based on architectural patterns by Ahmad Osman (@TheAhmadOsman) for agentic domain ops.
pub trait DnsAuthorityPort {
    /// Check if a DNS authority zone exists for the domain.
    fn zone_exists(&self, zone: &str) -> bool;

    /// Read all DNS records in the authority zone. Read-only.
    fn read_zone_records(&self, zone: &str) -> Vec<DnsRecordShape>;

    /// Create an authority zone if needed. Returns the zone ID or domain name.
    fn create_zone(&self, zone: &str) -> String;

    /// Upsert DNS records into the authority zone.
    /// The idempotency key is forwarded to the provider API.
    fn upsert_records(
        &self,
        zone_id: &str,
        records: &[DnsRecordShape],
        idempotency_key: &str,
    ) -> Vec<String>;

    /// Export raw zone backup from DNS authority for archival (read-back).
    fn export_zone_backup(&self, zone_id: &str) -> String;
}

/// Backward compatibility alias for `DnsAuthorityPort`.
pub type CloudflareDnsPort = dyn DnsAuthorityPort;

/// Port for authoritative DNS and HTTP edge verification.
///
/// These are read-only probes that verify the live internet sees the
/// intended result — through authoritative nameservers, not recursive resolvers.
pub trait EdgeProbePort {
    /// Query authoritative NS records for a domain (not via recursive resolver).
    fn query_authoritative_ns(&self, domain: &str) -> Vec<String>;

    /// HTTP probe through a specific edge resolver (curl --resolve equivalent).
    fn http_probe(
        &self,
        fqdn: &str,
        resolve_to: &str,
        expected_status: Option<u16>,
        follow_redirects: bool,
    ) -> EdgeProbeResult;

    /// Verify TLD delegation by querying root/ authoritative NS.
    fn check_tld_delegation(&self, domain: &str) -> DelegationProof;
}

pub use cynic_secrets::{SecretProviderPort as SecretProvider, SecretStorePort};

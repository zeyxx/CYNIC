//! Driven ports for external domain operations.
//!
//! These traits define the interface between the pure domain logic (in
//! `cynic-domain-ops::domain`) and the provider adapters (in
//! `cynic-domain-ops::adapters`). The runner injects concrete implementations.
//!
//! Authority: foundation.json §37 (portable_host_contract) — systemd/k8s/ssh/
//! rest/mcp/shell are adapters behind portable ports.

use cynic_protocol::secret_reference::SecretReference;

use cynic_protocol::domain_ops::{
    DelegationProof, DnsRecordShape, DomainAvailability, EdgeProbeResult,
    NameserverUpdateReceipt, RegistrationReceipt,
};

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

/// Port for Cloudflare DNS operations (zone management, record CRUD).
pub trait CloudflareDnsPort {
    /// Check if a Cloudflare zone exists for the domain.
    fn zone_exists(&self, zone: &str) -> bool;

    /// Read all DNS records in the Cloudflare zone. Read-only.
    fn read_zone_records(&self, zone: &str) -> Vec<DnsRecordShape>;

    /// Create a Cloudflare zone. Returns the zone ID.
    fn create_zone(&self, zone: &str) -> String;

    /// Upsert DNS records into the Cloudflare zone.
    /// The idempotency key is forwarded to the provider API.
    fn upsert_records(
        &self,
        zone_id: &str,
        records: &[DnsRecordShape],
        idempotency_key: &str,
    ) -> Vec<String>;

    /// Export raw zone backup from Cloudflare for archival (read-back).
    fn export_zone_backup(&self, zone_id: &str) -> String;
}

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

/// Port for secret resolution — resolves `SecretReference` to actual secret value.
///
/// Implementations read from: env vars, principal-owned files, OS keyring.
/// Never stores or logs the secret value.
pub trait SecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String>;
}

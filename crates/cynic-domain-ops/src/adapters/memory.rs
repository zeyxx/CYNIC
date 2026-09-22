//! In-memory implementations of all four port traits for runner tests.
//!
//! Each implementation uses `HashMap` for state and returns canned responses
//! that test setups can configure via builder methods (`with_*`).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::port::{CloudflareDnsPort, EdgeProbePort, RegistrarPort, SecretProvider};
use cynic_protocol::domain_ops::*;
use cynic_protocol::secret_reference::SecretReference;

// ── MemoryRegistrar ────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MemoryRegistrar {
    availability: RefCell<HashMap<String, bool>>,
    nameservers: RefCell<HashMap<String, Vec<String>>>,
    zone_records: RefCell<HashMap<String, Vec<DnsRecordShape>>>,
    pricing: RefCell<HashMap<String, f64>>,
}

impl MemoryRegistrar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_availability(mut self, domain: &str, available: bool) -> Self {
        self.availability
            .borrow_mut()
            .insert(domain.to_string(), available);
        self
    }

    pub fn with_nameservers(mut self, domain: &str, ns: Vec<String>) -> Self {
        self.nameservers.borrow_mut().insert(domain.to_string(), ns);
        self
    }

    pub fn with_zone_records(mut self, domain: &str, records: Vec<DnsRecordShape>) -> Self {
        self.zone_records
            .borrow_mut()
            .insert(domain.to_string(), records);
        self
    }

    pub fn with_pricing(mut self, domain: &str, price: f64) -> Self {
        self.pricing.borrow_mut().insert(domain.to_string(), price);
        self
    }
}

impl RegistrarPort for MemoryRegistrar {
    fn check_availability(&self, domain: &str) -> DomainAvailability {
        let available = self
            .availability
            .borrow()
            .get(domain)
            .copied()
            .unwrap_or(false);
        DomainAvailability {
            schema: DOMAIN_OPS_AVAILABILITY_SCHEMA.to_string(),
            domain: domain.to_string(),
            registrar: "memory".to_string(),
            available,
            premium: false,
            price: self.pricing.borrow().get(domain).copied(),
            currency: None,
        }
    }

    fn read_nameservers(&self, domain: &str) -> Vec<String> {
        self.nameservers
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default()
    }

    fn read_zone_records(&self, domain: &str) -> Vec<DnsRecordShape> {
        self.zone_records
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default()
    }

    fn check_pricing(&self, domain: &str, _years: u8) -> Option<f64> {
        self.pricing.borrow().get(domain).copied()
    }

    fn register_domain(
        &self,
        domain: &str,
        _years: u8,
        _auto_renew: bool,
        idempotency_key: &str,
    ) -> RegistrationReceipt {
        RegistrationReceipt {
            schema: DOMAIN_OPS_REGISTRATION_SCHEMA.to_string(),
            domain: domain.to_string(),
            registrar: "memory".to_string(),
            status: "registered".to_string(),
            expiry_date: None,
            eap_code: None,
            idempotency_key: idempotency_key.to_string(),
        }
    }

    fn update_nameservers(
        &self,
        domain: &str,
        nameservers: &[String],
        idempotency_key: &str,
    ) -> NameserverUpdateReceipt {
        let previous = self
            .nameservers
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default();
        self.nameservers
            .borrow_mut()
            .insert(domain.to_string(), nameservers.to_vec());
        NameserverUpdateReceipt {
            schema: DOMAIN_OPS_NS_UPDATE_SCHEMA.to_string(),
            domain: domain.to_string(),
            registrar: "memory".to_string(),
            previous_nameservers: previous,
            current_nameservers: nameservers.to_vec(),
            idempotency_key: idempotency_key.to_string(),
        }
    }

    fn export_zone_backup(&self, domain: &str) -> String {
        let records = self
            .zone_records
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default();
        serde_json::to_string(&records).unwrap_or_default()
    }
}

// ── MemoryCloudflare ───────────────────────────────────────────────────────

#[derive(Default)]
pub struct MemoryCloudflare {
    zones: RefCell<HashMap<String, String>>, // domain → zone_id
    zone_records: RefCell<HashMap<String, Vec<DnsRecordShape>>>, // zone_id → records
}

impl MemoryCloudflare {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_zone(mut self, domain: &str, zone_id: &str) -> Self {
        self.zones
            .borrow_mut()
            .insert(domain.to_string(), zone_id.to_string());
        self
    }

    pub fn with_zone_records(mut self, zone_id: &str, records: Vec<DnsRecordShape>) -> Self {
        self.zone_records
            .borrow_mut()
            .insert(zone_id.to_string(), records);
        self
    }
}

impl CloudflareDnsPort for MemoryCloudflare {
    fn zone_exists(&self, zone: &str) -> bool {
        self.zones.borrow().contains_key(zone)
    }

    fn read_zone_records(&self, zone: &str) -> Vec<DnsRecordShape> {
        let zone_id = self.zones.borrow().get(zone).cloned().unwrap_or_default();
        self.zone_records
            .borrow()
            .get(&zone_id)
            .cloned()
            .unwrap_or_default()
    }

    fn create_zone(&self, zone: &str) -> String {
        let zones = self.zones.borrow();
        if let Some(id) = zones.get(zone) {
            id.clone()
        } else {
            drop(zones);
            let zone_id = format!("zone-{}", zone);
            self.zones
                .borrow_mut()
                .insert(zone.to_string(), zone_id.clone());
            zone_id
        }
    }

    fn upsert_records(
        &self,
        zone_id: &str,
        records: &[DnsRecordShape],
        _idempotency_key: &str,
    ) -> Vec<String> {
        let mut stored = records.to_vec();
        let mut zone_records = self.zone_records.borrow_mut();
        let existing = zone_records.entry(zone_id.to_string()).or_default();
        existing.clear();
        existing.append(&mut stored);
        (0..records.len())
            .map(|i| format!("rec-{}-{}", zone_id, i))
            .collect()
    }

    fn export_zone_backup(&self, zone_id: &str) -> String {
        let records = self
            .zone_records
            .borrow()
            .get(zone_id)
            .cloned()
            .unwrap_or_default();
        serde_json::to_string(&records).unwrap_or_default()
    }
}

// ── MemoryEdgeProbe ─────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MemoryEdgeProbe {
    ns_delegations: RefCell<HashMap<String, Vec<String>>>,
    http_probes: RefCell<HashMap<String, EdgeProbeResult>>,
    tld_delegations: RefCell<HashMap<String, DelegationProof>>,
}

impl MemoryEdgeProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ns_delegation(mut self, domain: &str, ns: Vec<String>) -> Self {
        self.ns_delegations
            .borrow_mut()
            .insert(domain.to_string(), ns);
        self
    }

    pub fn with_http_probe(mut self, fqdn: &str, status: u16) -> Self {
        self.http_probes.borrow_mut().insert(
            fqdn.to_string(),
            EdgeProbeResult {
                schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
                fqdn: fqdn.to_string(),
                expected_status: None,
                observed_status: Some(status),
                observed_redirect: None,
                passed: status == 200,
            },
        );
        self
    }

    pub fn with_tld_delegation(mut self, domain: &str, delegated: bool, ns: Vec<String>) -> Self {
        self.tld_delegations.borrow_mut().insert(
            domain.to_string(),
            DelegationProof {
                schema: DOMAIN_OPS_DELEGATION_SCHEMA.to_string(),
                domain: domain.to_string(),
                delegated,
                observed_nameservers: ns.clone(),
                expected_nameservers: ns,
                ns_set_matches: delegated,
            },
        );
        self
    }
}

impl EdgeProbePort for MemoryEdgeProbe {
    fn query_authoritative_ns(&self, domain: &str) -> Vec<String> {
        self.ns_delegations
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default()
    }

    fn http_probe(
        &self,
        fqdn: &str,
        _resolve_to: &str,
        expected_status: Option<u16>,
        _follow_redirects: bool,
    ) -> EdgeProbeResult {
        let result = self
            .http_probes
            .borrow()
            .get(fqdn)
            .cloned()
            .unwrap_or(EdgeProbeResult {
                schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
                fqdn: fqdn.to_string(),
                expected_status,
                observed_status: None,
                observed_redirect: None,
                passed: false,
            });
        result
    }

    fn check_tld_delegation(&self, domain: &str) -> DelegationProof {
        // If the test configured a delegation proof, return it
        if let Some(proof) = self.tld_delegations.borrow().get(domain) {
            return proof.clone();
        }
        // Fallback: derive from ns_delegations
        let observed = self
            .ns_delegations
            .borrow()
            .get(domain)
            .cloned()
            .unwrap_or_default();
        let delegated = !observed.is_empty();
        DelegationProof {
            schema: DOMAIN_OPS_DELEGATION_SCHEMA.to_string(),
            domain: domain.to_string(),
            delegated,
            observed_nameservers: observed,
            expected_nameservers: vec![],
            ns_set_matches: false,
        }
    }
}

// ── MemorySecretProvider ─────────────────────────────────────────────────────

#[derive(Default)]
pub struct MemorySecretProvider {
    secrets: RefCell<HashMap<String, String>>,
}

impl MemorySecretProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(mut self, key: &str, value: &str) -> Self {
        self.secrets
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
        self
    }
}

impl SecretProvider for MemorySecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        if reference.provider
            == cynic_protocol::secret_reference::SecretProvider::ProcessEnvironment
        {
            self.secrets.borrow().get(&reference.locator).cloned()
        } else {
            None
        }
    }
}

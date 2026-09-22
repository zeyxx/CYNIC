//! Cloudflare API + DNS-over-HTTPS adapter.
//!
//! Implements `CloudflareDnsPort` (zone management, DNS record CRUD via
//! Cloudflare API v4) and `EdgeProbePort` (NS queries via Cloudflare
//! DNS-over-HTTPS, HTTP edge verification with IP pinning).

use cynic_domain_ops::{CloudflareDnsPort, EdgeProbePort, SecretProvider as SecretProviderPort};
use cynic_protocol::domain_ops::*;
use cynic_protocol::secret_reference::SecretReference;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::net::SocketAddr;

use super::error::CloudflareError;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const DOH_ENDPOINT: &str = "https://1.1.1.1/dns-query";

// ── Cloudflare API response envelope ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CfResponse<T> {
    success: bool,
    result: Option<T>,
}

// ── DNS-over-HTTPS response types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DohResponse {
    #[serde(rename = "Status")]
    status: u16,
    #[serde(rename = "Answer", default)]
    answer: Option<Vec<DohAnswer>>,
}

#[derive(Debug, Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    r#type: u16, // 2 = NS, 1 = A, 5 = CNAME, 15 = MX, 16 = TXT
    #[serde(rename = "NS")]
    ns: Option<String>,
}

// ── Cloudflare DNS record types ────────────────────────────────────────────

/// Cloudflare DNS record as returned by the API.
#[derive(Debug, Deserialize)]
struct CfDnsRecord {
    id: String,
    name: String, // FQDN, e.g. "www.example.com"
    #[serde(rename = "type")]
    r#type: String,
    content: String,
    ttl: u64, // 1 = automatic (proxied), >1 = explicit TTL
    priority: Option<u16>,
    proxied: bool,
}

/// Cloudflare zone ID wrapper.
#[derive(Debug, Deserialize)]
struct CfZone {
    id: String,
}

/// Cloudflare record creation/update payload.
#[derive(Debug, serde::Serialize)]
struct CfRecordPayload {
    #[serde(rename = "type")]
    r#type: String,
    name: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxied: Option<bool>,
}

/// Cloudflare zone creation payload.
#[derive(Debug, serde::Serialize)]
struct CfZonePayload {
    name: String,
    #[serde(rename = "account_id")]
    account_id: String,
    #[serde(rename = "jump_start")]
    jump_start: bool,
}

// ── CloudflareClient ──────────────────────────────────────────────────────

/// CloudflareClient — single struct implementing both CloudflareDnsPort
/// and EdgeProbePort. Configured at construction with API token and
/// expected NS set (for delegation verification).
#[derive(Debug)]
pub struct CloudflareClient {
    http: Client,
    account_id: String,
    expected_nameservers: Vec<String>,
    dns_resolver: Client, // uses 1.1.1.1 DoH
}

impl CloudflareClient {
    /// Build a Cloudflare client from resolved credentials.
    ///
    /// `api_token` — Cloudflare API token (zone:DNS, zone:zone read, zone:zone edit).
    /// `account_id` — Cloudflare account ID for zone creation.
    /// `expected_nameservers` — NS records expected after migration (e.g. Cloudflare's NS).
    pub fn new(api_token: String, account_id: String, expected_nameservers: Vec<String>) -> Self {
        Self {
            http: Client::builder()
                .user_agent("cynic-domain-cloudflare/0.1.0")
                .timeout(std::time::Duration::from_secs(30))
                .default_headers({
                    let mut h = reqwest::header::HeaderMap::new();
                    h.insert(
                        "Authorization",
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {api_token}"))
                            .unwrap_or(reqwest::header::HeaderValue::from_static("Bearer")),
                    );
                    h.insert(
                        "Content-Type",
                        reqwest::header::HeaderValue::from_static("application/json"),
                    );
                    h
                })
                .build()
                .unwrap_or_else(|e| panic!("reqwest client init failed: {e}")),
            account_id,
            expected_nameservers,
            dns_resolver: Client::builder()
                .user_agent("cynic-domain-cloudflare/0.1.0")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|e| panic!("reqwest dns client init failed: {e}")),
        }
    }

    /// Build from a `SecretReference` pair — resolved by an injected `SecretProvider`.
    pub fn from_secret_refs(
        token_ref: &SecretReference,
        account_ref: &SecretReference,
        provider: &impl SecretProviderPort,
        expected_nameservers: Vec<String>,
    ) -> Result<Self, CloudflareError> {
        let api_token = provider
            .resolve(token_ref)
            .ok_or_else(|| CloudflareError::CredentialNotFound(token_ref.id.clone()))?;
        let account_id = provider
            .resolve(account_ref)
            .ok_or_else(|| CloudflareError::CredentialNotFound(account_ref.id.clone()))?;
        Ok(Self::new(api_token, account_id, expected_nameservers))
    }

    /// Find the Cloudflare zone ID for a domain.
    fn find_zone_id(&self, zone: &str) -> Option<String> {
        let url = format!("{CF_API_BASE}/zones?name={zone}&status=active");
        let resp: CfResponse<Vec<CfZone>> = self
            .http
            .get(&url)
            .send()
            .ok()
            .and_then(|r| r.json().ok())?;
        resp.result
            .and_then(|zones| zones.into_iter().next())
            .map(|z| z.id)
    }

    /// Convert a `DnsRecordShape` to a Cloudflare record payload.
    fn record_to_payload(rec: &DnsRecordShape) -> CfRecordPayload {
        let r#type = match rec.r#type {
            DnsRecordType::A => "A",
            DnsRecordType::AAAA => "AAAA",
            DnsRecordType::CNAME => "CNAME",
            DnsRecordType::TXT => "TXT",
            DnsRecordType::MX => "MX",
            DnsRecordType::NS => "NS",
            DnsRecordType::CAA => "CAA",
            DnsRecordType::SRV => "_SRV",
            DnsRecordType::DS => "DS",
        };

        let (ttl, proxied) = if rec.proxied {
            (Some(1), Some(true))
        } else {
            (Some(rec.ttl as u64), Some(false))
        };

        CfRecordPayload {
            r#type: r#type.to_string(),
            name: rec.name.clone(),
            content: rec.content.clone(),
            ttl,
            priority: rec.priority,
            proxied,
        }
    }

    /// Convert a Cloudflare DNS record to `DnsRecordShape`.
    fn cf_record_to_shape(rec: CfDnsRecord) -> DnsRecordShape {
        let r#type = match rec.r#type.as_str() {
            "A" => DnsRecordType::A,
            "AAAA" => DnsRecordType::AAAA,
            "CNAME" => DnsRecordType::CNAME,
            "TXT" => DnsRecordType::TXT,
            "MX" => DnsRecordType::MX,
            "NS" => DnsRecordType::NS,
            "CAA" => DnsRecordType::CAA,
            "_SRV" => DnsRecordType::SRV,
            "DS" => DnsRecordType::DS,
            _ => DnsRecordType::TXT,
        };

        // Cloudflare TTL=1 indicates proxied/automatic; real TTL for dns-only.
        let proxied = rec.proxied;

        DnsRecordShape {
            name: rec.name,
            r#type,
            content: rec.content,
            ttl: if rec.ttl == 1 { 300 } else { rec.ttl as u32 },
            priority: rec.priority,
            proxied,
        }
    }

    /// Query DoH for NS records of a domain.
    fn doh_query_ns(&self, domain: &str) -> Vec<String> {
        let url = format!("{DOH_ENDPOINT}?name={domain}&type=NS");
        let resp: DohResponse = match self
            .dns_resolver
            .get(&url)
            .header("Accept", "application/dns-json")
            .send()
            .ok()
            .and_then(|r| r.json().ok())
        {
            Some(r) => r,
            None => return Vec::new(),
        };

        // DoH Status 0 = NOERROR. Non-zero means the query failed.
        if resp.status != 0 {
            return Vec::new();
        }

        resp.answer
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.r#type == 2) // NS type
            .filter_map(|a| a.ns)
            .map(|ns| ns.trim_end_matches('.').to_lowercase())
            .collect()
    }

    /// HTTP probe with IP resolution override (--resolve equivalent).
    fn probe_http(
        &self,
        fqdn: &str,
        resolve_to: &str,
        expected_status: Option<u16>,
        follow_redirects: bool,
    ) -> EdgeProbeResult {
        let ip = match resolve_to.parse::<std::net::IpAddr>() {
            Ok(ip) => ip,
            Err(_) => {
                return EdgeProbeResult {
                    schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
                    fqdn: fqdn.to_string(),
                    expected_status,
                    observed_status: None,
                    observed_redirect: None,
                    passed: false,
                };
            }
        };

        let addr = SocketAddr::new(ip, 443);
        let probe_client = Client::builder()
            .user_agent("cynic-edge-probe/0.1.0")
            .resolve(fqdn, addr)
            .redirect(if follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            })
            .build();

        let client = match probe_client {
            Ok(c) => c,
            Err(_) => {
                return EdgeProbeResult {
                    schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
                    fqdn: fqdn.to_string(),
                    expected_status,
                    observed_status: None,
                    observed_redirect: None,
                    passed: false,
                };
            }
        };

        let url = format!("https://{fqdn}");
        let response = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => {
                return EdgeProbeResult {
                    schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
                    fqdn: fqdn.to_string(),
                    expected_status,
                    observed_status: None,
                    observed_redirect: None,
                    passed: false,
                };
            }
        };

        let observed_status = Some(response.status().as_u16());
        let observed_redirect = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let passed = match expected_status {
            Some(expected) => observed_status == Some(expected),
            None => response.status().is_success(),
        };

        EdgeProbeResult {
            schema: DOMAIN_OPS_EDGE_PROBE_SCHEMA.to_string(),
            fqdn: fqdn.to_string(),
            expected_status,
            observed_status,
            observed_redirect,
            passed,
        }
    }
}

impl CloudflareDnsPort for CloudflareClient {
    fn zone_exists(&self, zone: &str) -> bool {
        self.find_zone_id(zone).is_some()
    }

    fn read_zone_records(&self, zone: &str) -> Vec<DnsRecordShape> {
        let zone_id = match self.find_zone_id(zone) {
            Some(id) => id,
            None => return Vec::new(),
        };

        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records?per_page=5000");
        let resp: CfResponse<Vec<CfDnsRecord>> = match self.http.get(&url).send() {
            Ok(r) => match r.json() {
                Ok(data) => data,
                Err(_) => return Vec::new(),
            },
            Err(_) => return Vec::new(),
        };

        match resp.result {
            Some(records) => records.into_iter().map(Self::cf_record_to_shape).collect(),
            None => Vec::new(),
        }
    }

    fn create_zone(&self, zone: &str) -> String {
        let url = format!("{CF_API_BASE}/zones");
        let payload = CfZonePayload {
            name: zone.to_string(),
            account_id: self.account_id.clone(),
            jump_start: false,
        };

        let resp: CfResponse<CfZone> = match self.http.post(&url).json(&payload).send() {
            Ok(r) => match r.json() {
                Ok(data) => data,
                Err(_) => return String::new(),
            },
            Err(_) => return String::new(),
        };

        if !resp.success {
            return String::new();
        }

        resp.result.map(|z| z.id).unwrap_or_default()
    }

    fn upsert_records(
        &self,
        zone_id: &str,
        records: &[DnsRecordShape],
        idempotency_key: &str,
    ) -> Vec<String> {
        let _ = idempotency_key;
        let mut record_ids = Vec::new();

        for rec in records {
            let payload = Self::record_to_payload(rec);
            let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");

            // Try POST (create). If conflict (exists), find and PUT.
            let result = match self.http.post(&url).json(&payload).send() {
                Ok(r) => r.json::<CfResponse<CfDnsRecord>>(),
                Err(_) => continue,
            };

            let record_id = match result {
                Ok(resp) if resp.success && resp.result.is_some() => {
                    resp.result.map(|r| r.id).unwrap_or_default()
                }
                _ => {
                    // Try finding existing record and updating
                    let find_url = format!(
                        "{CF_API_BASE}/zones/{zone_id}/dns_records?type={}&name={}",
                        payload.r#type.as_str().trim_start_matches('_'),
                        payload.name,
                    );
                    let find_resp = match self.http.get(&find_url).send() {
                        Ok(r) => r.json::<CfResponse<Vec<CfDnsRecord>>>(),
                        Err(_) => continue,
                    };

                    if let Ok(existing) = find_resp {
                        if let Some(existing_rec) = existing.result.and_then(|mut v| v.pop()) {
                            let put_url = format!(
                                "{CF_API_BASE}/zones/{zone_id}/dns_records/{}",
                                existing_rec.id
                            );
                            match self
                                .http
                                .put(&put_url)
                                .json(&payload)
                                .send()
                                .ok()
                                .and_then(|r| r.json::<CfResponse<CfDnsRecord>>().ok())
                            {
                                Some(resp) if resp.success => {
                                    resp.result.map(|r| r.id).unwrap_or_default()
                                }
                                _ => continue,
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            };

            if !record_id.is_empty() {
                record_ids.push(record_id);
            }
        }

        record_ids
    }

    fn export_zone_backup(&self, zone_id: &str) -> String {
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records?per_page=5000");
        match self.http.get(&url).send() {
            Ok(resp) => match resp.json::<serde_json::Value>() {
                Ok(json) => serde_json::to_string(&json).unwrap_or_default(),
                Err(_) => String::new(),
            },
            Err(_) => String::new(),
        }
    }
}

impl EdgeProbePort for CloudflareClient {
    fn query_authoritative_ns(&self, domain: &str) -> Vec<String> {
        self.doh_query_ns(domain)
    }

    fn http_probe(
        &self,
        fqdn: &str,
        resolve_to: &str,
        expected_status: Option<u16>,
        follow_redirects: bool,
    ) -> EdgeProbeResult {
        self.probe_http(fqdn, resolve_to, expected_status, follow_redirects)
    }

    fn check_tld_delegation(&self, domain: &str) -> DelegationProof {
        let observed = self.doh_query_ns(domain);
        let expected = self.expected_nameservers.clone();

        let ns_set_matches = if expected.is_empty() {
            false
        } else {
            let observed_lower: std::collections::HashSet<String> =
                observed.iter().map(|s| s.to_lowercase()).collect();
            let expected_lower: std::collections::HashSet<String> =
                expected.iter().map(|s| s.to_lowercase()).collect();
            observed_lower == expected_lower
        };

        DelegationProof {
            schema: DOMAIN_OPS_DELEGATION_SCHEMA.to_string(),
            domain: domain.to_string(),
            delegated: !observed.is_empty(),
            observed_nameservers: observed,
            expected_nameservers: expected,
            ns_set_matches,
        }
    }
}

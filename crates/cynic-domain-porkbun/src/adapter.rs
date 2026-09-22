//! Porkbun API adapter — implements `RegistrarPort`.

use cynic_domain_ops::{RegistrarPort, SecretProvider};
use cynic_protocol::domain_ops::*;
use cynic_protocol::secret_reference::SecretReference;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

use super::error::PorkbunError;
use super::secret::EnvSecretProvider;

/// Porkbun API base URL.
const PORKBUN_API_BASE: &str = "https://api.porkbun.com/api/json/v2";

/// Authentication payload sent in the body of every Porkbun API request.
#[derive(Debug, Clone, serde::Serialize)]
struct PorkbunAuth {
    id: String,
    secret: String,
}

/// Wrapper around Porkbun API responses.
#[derive(Debug, Deserialize)]
struct PorkbunResponse<T> {
    status: String,
    data: Option<T>,
    message: Option<String>,
}

/// Porkbun availability data.
#[derive(Debug, Deserialize)]
struct PorkbunAvailability {
    domain: String,
    available: String, // "yes", "no", or "premium"
    price: String,
    currency: String,
}

/// Porkbun DNS record (as returned by the list-records endpoint).
#[derive(Debug, Deserialize)]
struct PorkbunDnsRecord {
    name: String, // relative to zone, e.g. "www" or "@"
    #[serde(rename = "type")]
    r#type: String,
    content: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    ttl: Option<String>,
    #[serde(default)]
    priority: Option<String>,
}

/// Porkbun DNS list response with pagination.
#[derive(Debug, Deserialize)]
struct PorkbunRecordListResponse {
    #[serde(default)]
    records: Vec<PorkbunDnsRecord>,
}

/// Porkbun nameserver list.
#[derive(Debug, Deserialize)]
struct PorkbunNsResponse {
    ns: Vec<String>,
}

/// Porkbun registration request body.
#[derive(Debug, serde::Serialize)]
struct PorkbunRegisterRequest {
    id: String,
    secret: String,
    domain: String,
    years: u8,
    #[serde(rename = "autoRenew")]
    auto_renew: bool,
}

/// Porkbun NS update request body.
#[derive(Debug, serde::Serialize)]
struct PorkbunNsUpdateRequest {
    id: String,
    secret: String,
    ns: Vec<String>,
}

/// Registrar adapter backed by the Porkbun API v2.
///
/// Credentials (API key + secret) are injected at construction — they are
/// resolved from `SecretReference` by the caller (typically the runner via
/// `EnvSecretProvider`). No credential material is read from disk or env
/// inside this struct after construction.
#[derive(Debug)]
pub struct PorkbunRegistrar {
    client: Client,
    api_key: String,
    api_secret: String,
}

impl PorkbunRegistrar {
    /// Build a Porkbun registrar from an API key and secret.
    ///
    /// In production, the caller resolves these from `SecretReference`
    /// using an `EnvSecretProvider` or `PrincipalOwnedFileProvider` adapter.
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            client: Client::builder()
                .user_agent("cynic-domain-porkbun/0.1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|e| {
                    // reqwest::Error is not available for unwrap in deny mode;
                    // this only fails on impossible config (no global tls provider).
                    panic!("reqwest client init failed: {e}")
                }),
            api_key,
            api_secret,
        }
    }

    /// Build from a `SecretReference` pair — resolved by an injected `SecretProvider`.
    ///
    /// The `key_ref` resolves the API key env var; `secret_ref` resolves the
    /// API secret env var. This is the standard V1 credential injection path.
    /// Accepts any `SecretProviderPort` implementation (env, composite, keyring).
    pub fn from_secret_refs(
        key_ref: &SecretReference,
        secret_ref: &SecretReference,
        provider: &impl SecretProvider,
    ) -> Result<Self, PorkbunError> {
        let api_key = provider
            .resolve(key_ref)
            .ok_or_else(|| PorkbunError::CredentialNotFound(key_ref.id.clone()))?;
        let api_secret = provider
            .resolve(secret_ref)
            .ok_or_else(|| PorkbunError::CredentialNotFound(secret_ref.id.clone()))?;
        Ok(Self::new(api_key, api_secret))
    }

    fn auth(&self) -> PorkbunAuth {
        PorkbunAuth {
            id: self.api_key.clone(),
            secret: self.api_secret.clone(),
        }
    }

    fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, PorkbunError> {
        let url = format!("{PORKBUN_API_BASE}{endpoint}");
        let resp = self.client.post(&url).json(body).send()?;
        let wrapper: PorkbunResponse<T> = resp.json()?;
        if wrapper.status == "ERROR" || wrapper.data.is_none() {
            return Err(PorkbunError::ApiError {
                status: 0,
                message: wrapper.message.unwrap_or_default(),
            });
        }
        Ok(wrapper.data.unwrap())
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T, PorkbunError> {
        // Porkbun requires auth body even on GET — use POST with auth.
        let url = format!("{PORKBUN_API_BASE}{endpoint}");
        let resp = self.client.post(&url).json(&self.auth()).send()?;
        let wrapper: PorkbunResponse<T> = resp.json()?;
        if wrapper.status == "ERROR" || wrapper.data.is_none() {
            return Err(PorkbunError::ApiError {
                status: 0,
                message: wrapper.message.unwrap_or_default(),
            });
        }
        Ok(wrapper.data.unwrap())
    }

    /// Convert a relative Porkbun record name (e.g. "www", "@") to FQDN.
    fn relative_to_fqdn(relative: &str, domain: &str) -> String {
        if relative.is_empty() || relative == "@" {
            domain.to_string()
        } else {
            format!("{relative}.{domain}")
        }
    }

    /// Parse a Porkbun DNS record into `DnsRecordShape`.
    fn parse_record(rec: PorkbunDnsRecord, domain: &str) -> DnsRecordShape {
        let r#type = match rec.r#type.as_str() {
            "A" => DnsRecordType::A,
            "AAAA" => DnsRecordType::AAAA,
            "CNAME" => DnsRecordType::CNAME,
            "TXT" => DnsRecordType::TXT,
            "MX" => DnsRecordType::MX,
            "NS" => DnsRecordType::NS,
            "CAA" => DnsRecordType::CAA,
            "SRV" => DnsRecordType::SRV,
            "DS" => DnsRecordType::DS,
            _ => {
                return DnsRecordShape {
                    name: Self::relative_to_fqdn(&rec.name, domain),
                    r#type: DnsRecordType::TXT,
                    content: rec.content.clone(),
                    ttl: rec.ttl.and_then(|t| t.parse().ok()).unwrap_or(300),
                    priority: None,
                    proxied: false,
                };
            }
        };

        let content = rec.value.unwrap_or(rec.content);
        DnsRecordShape {
            name: Self::relative_to_fqdn(&rec.name, domain),
            r#type,
            content,
            ttl: rec.ttl.and_then(|t| t.parse().ok()).unwrap_or(300),
            priority: rec.priority.and_then(|p| p.parse().ok()),
            proxied: false, // Porkbun does not track proxy status
        }
    }
}

impl RegistrarPort for PorkbunRegistrar {
    fn check_availability(&self, domain: &str) -> DomainAvailability {
        let endpoint = format!("/domain/{domain}/available");
        let result: PorkbunAvailability = match self.get_json(&endpoint) {
            Ok(v) => v,
            Err(_) => {
                return DomainAvailability {
                    schema: DOMAIN_OPS_AVAILABILITY_SCHEMA.to_string(),
                    domain: domain.to_string(),
                    registrar: REGISTRAR_NAME.to_string(),
                    available: false,
                    premium: false,
                    price: None,
                    currency: None,
                };
            }
        };

        let available = result.available == "yes" || result.available == "premium";
        let premium = result.available == "premium";

        DomainAvailability {
            schema: DOMAIN_OPS_AVAILABILITY_SCHEMA.to_string(),
            domain: result.domain,
            registrar: REGISTRAR_NAME.to_string(),
            available,
            premium,
            price: result.price.parse().ok(),
            currency: Some(result.currency),
        }
    }

    fn read_nameservers(&self, domain: &str) -> Vec<String> {
        let endpoint = format!("/domain/{domain}/ns");
        match self.get_json::<PorkbunNsResponse>(&endpoint) {
            Ok(resp) => resp.ns,
            Err(_) => Vec::new(),
        }
    }

    fn read_zone_records(&self, domain: &str) -> Vec<DnsRecordShape> {
        let mut all_records = Vec::new();
        let mut page: u32 = 1;
        loop {
            let endpoint = format!("/domain/{domain}/dns?page={page}&include=defaultRecords=true");
            let resp: PorkbunRecordListResponse = match self.get_json(&endpoint) {
                Ok(r) => r,
                Err(_) => break,
            };
            if resp.records.is_empty() {
                break;
            }
            for rec in resp.records {
                all_records.push(Self::parse_record(rec, domain));
            }
            page += 1;
            if page > 100 {
                break; // safety limit
            }
        }
        all_records
    }

    fn check_pricing(&self, domain: &str, _years: u8) -> Option<f64> {
        let endpoint = format!("/domain/{domain}/available");
        let result: PorkbunAvailability = match self.get_json(&endpoint) {
            Ok(v) => v,
            Err(_) => return None,
        };
        result.price.parse().ok()
    }

    fn register_domain(
        &self,
        domain: &str,
        years: u8,
        auto_renew: bool,
        idempotency_key: &str,
    ) -> RegistrationReceipt {
        let body = PorkbunRegisterRequest {
            id: self.api_key.clone(),
            secret: self.api_secret.clone(),
            domain: domain.to_string(),
            years,
            auto_renew,
        };
        let _ = idempotency_key; // Porkbun does not support idempotency keys;
        // the runner uses the key for local reconciliation before calling this.
        let endpoint = "/domain".to_string();
        let result: HashMap<String, String> = match self.post_json(&endpoint, &body) {
            Ok(v) => v,
            Err(e) => {
                return RegistrationReceipt {
                    schema: DOMAIN_OPS_REGISTRATION_SCHEMA.to_string(),
                    domain: domain.to_string(),
                    registrar: REGISTRAR_NAME.to_string(),
                    status: format!("error: {e}"),
                    expiry_date: None,
                    eap_code: None,
                    idempotency_key: idempotency_key.to_string(),
                };
            }
        };

        let status = result.get("status").cloned().unwrap_or_default();

        RegistrationReceipt {
            schema: DOMAIN_OPS_REGISTRATION_SCHEMA.to_string(),
            domain: domain.to_string(),
            registrar: REGISTRAR_NAME.to_string(),
            status,
            expiry_date: result.get("expireDate").cloned(),
            eap_code: result.get("id").cloned(),
            idempotency_key: idempotency_key.to_string(),
        }
    }

    fn update_nameservers(
        &self,
        domain: &str,
        nameservers: &[String],
        idempotency_key: &str,
    ) -> NameserverUpdateReceipt {
        let previous = self.read_nameservers(domain);
        let body = PorkbunNsUpdateRequest {
            id: self.api_key.clone(),
            secret: self.api_secret.clone(),
            ns: nameservers.to_vec(),
        };
        let endpoint = format!("/domain/{domain}/ns");
        match self.post_json::<HashMap<String, String>>(&endpoint, &body) {
            Ok(_) => NameserverUpdateReceipt {
                schema: DOMAIN_OPS_NS_UPDATE_SCHEMA.to_string(),
                domain: domain.to_string(),
                registrar: REGISTRAR_NAME.to_string(),
                previous_nameservers: previous,
                current_nameservers: nameservers.to_vec(),
                idempotency_key: idempotency_key.to_string(),
            },
            Err(_) => NameserverUpdateReceipt {
                schema: DOMAIN_OPS_NS_UPDATE_SCHEMA.to_string(),
                domain: domain.to_string(),
                registrar: REGISTRAR_NAME.to_string(),
                previous_nameservers: previous.clone(),
                current_nameservers: previous,
                idempotency_key: idempotency_key.to_string(),
            },
        }
    }

    fn export_zone_backup(&self, domain: &str) -> String {
        let records = self.read_zone_records(domain);
        serde_json::to_string(&records).unwrap_or_default()
    }
}

/// Constant: registrar name string used in all receipts.
const REGISTRAR_NAME: &str = "porkbun";

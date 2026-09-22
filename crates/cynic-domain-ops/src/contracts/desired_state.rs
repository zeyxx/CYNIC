//! Desired-state contract deserialization and validation.
//!
//! Parses `cynic.domain_ops.desired_state.v1` JSON into a typed
//! `DomainOpsDesiredState`, validating schema, domain name, provider
//! configuration, and edge probe specifications.
//!
//! This module is the boundary between untrusted JSON input (files,
//! stdin, registry) and the pure domain layer. All external input
//! must pass through `parse_desired_state` before reaching the runner.

use cynic_protocol::domain_ops::{
    DOMAIN_OPS_DESIRED_STATE_SCHEMA, DomainOpsDesiredState, EdgeProbeSpec, ProxyPolicy,
    ProviderConfig,
};

use super::error::DomainOpsError;

/// Parse and validate a desired-state JSON string into `DomainOpsDesiredState`.
///
/// Validation steps (all pure, no I/O):
/// 1. Deserialize JSON → `DomainOpsDesiredState`
/// 2. Validate `schema` matches `DOMAIN_OPS_DESIRED_STATE_SCHEMA`
/// 3. Validate `domain` via `validate_domain_name`
/// 4. Validate `registrar.provider` and `dns_authority.provider` are non-empty
/// 5. Validate each `edge_probes` entry has a valid `fqdn`
///
/// # Errors
/// Returns `DomainOpsError` if any validation step fails.
pub fn parse_desired_state(json: &str) -> Result<DomainOpsDesiredState, DomainOpsError> {
    let state: DomainOpsDesiredState =
        serde_json::from_str(json).map_err(|e| DomainOpsError::json_parse("desired_state", e))?;

    // Schema validation
    if state.schema != DOMAIN_OPS_DESIRED_STATE_SCHEMA {
        return Err(DomainOpsError::schema_mismatch(&state.schema));
    }

    // Domain name validation
    if let Err(reason) = cynic_protocol::domain_ops::validate_domain_name(&state.domain) {
        return Err(DomainOpsError::InvalidDomain {
            domain: state.domain,
            reason,
        });
    }

    // Provider configuration validation
    validate_provider_config(&state.registrar, "registrar")?;
    validate_provider_config(&state.dns_authority, "dns_authority")?;

    // Edge probe validation
    for probe in &state.edge_probes {
        validate_edge_probe(probe)?;
    }

    // Proxy policy validation: at least one field must be non-default
    // (we don't reject default, just validate it's internally consistent)
    validate_proxy_policy(&state.proxy_policy)?;

    Ok(state)
}

/// Validate a provider configuration — provider name must be non-empty.
fn validate_provider_config(config: &ProviderConfig, field: &str) -> Result<(), DomainOpsError> {
    if config.provider.trim().is_empty() {
        return Err(DomainOpsError::ProviderConfig {
            provider: field.to_string(),
            reason: "provider name is empty".to_string(),
        });
    }

    // Credential must have a non-empty locator
    if config.credential.locator.trim().is_empty() {
        return Err(DomainOpsError::ProviderConfig {
            provider: config.provider.clone(),
            reason: "credential locator is empty".to_string(),
        });
    }

    Ok(())
}

/// Validate an edge probe specification.
fn validate_edge_probe(probe: &EdgeProbeSpec) -> Result<(), DomainOpsError> {
    if probe.fqdn.trim().is_empty() {
        return Err(DomainOpsError::EdgeProbe(format!(
            "probe fqdn is empty"
        )));
    }

    // FQDN must be a valid domain or subdomain (strip trailing dot if present)
    let fqdn = probe.fqdn.trim_end_matches('.');
    if let Err(reason) = cynic_protocol::domain_ops::validate_domain_name(fqdn) {
        // Allow bare hostnames by checking if it's at least non-empty and
        // contains valid characters — but for DNS, we require a dot.
        return Err(DomainOpsError::EdgeProbe(format!(
            "invalid probe fqdn '{}': {}",
            probe.fqdn, reason
        )));
    }

    Ok(())
}

/// Validate a proxy policy — all fields must be internally consistent.
fn validate_proxy_policy(policy: &ProxyPolicy) -> Result<(), DomainOpsError> {
    // ProxyMode::Auto is always valid — it delegates to record-type rules.
    // ProxyMode::Proxied is valid for web_records but nonsensical for
    // mail_records, verification_records, dnssec_records (they can never
    // actually be proxied by Cloudflare). We warn but don't reject —
    // the resolver will silently downgrade them.
    if matches!(policy.mail_records, cynic_protocol::domain_ops::ProxyMode::Proxied) {
        return Err(DomainOpsError::ProxyPolicy(
            "mail_records cannot be Proxied (Cloudflare does not proxy MX)".to_string(),
        ));
    }

    if matches!(
        policy.verification_records,
        cynic_protocol::domain_ops::ProxyMode::Proxied
    ) {
        return Err(DomainOpsError::ProxyPolicy(
            "verification_records cannot be Proxied (TXT/SPF/DKIM)".to_string(),
        ));
    }

    if matches!(policy.dnssec_records, cynic_protocol::domain_ops::ProxyMode::Proxied) {
        return Err(DomainOpsError::ProxyPolicy(
            "dnssec_records cannot be Proxied (DS/CAA are DNS-only)".to_string(),
        ));
    }

    Ok(())
}

/// Serialize a `DomainOpsDesiredState` back to JSON for round-trip verification.
pub fn desired_state_to_json(state: &DomainOpsDesiredState) -> Result<String, DomainOpsError> {
    serde_json::to_string_pretty(state)
        .map_err(|e| DomainOpsError::json_parse("desired_state_to_json", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::domain_ops::ProxyMode;

    fn valid_desired_state_json() -> String {
        r#"{
            "schema": "cynic.domain_ops.desired_state.v1",
            "domain": "example.com",
            "registrar": {
                "provider": "porkbun",
                "credential": {
                    "id": "porkbun-key",
                    "class": "RuntimeMachineIdentity",
                    "provider": "process_environment",
                    "locator": "PORKBUN_API_KEY"
                }
            },
            "dns_authority": {
                "provider": "cloudflare",
                "credential": {
                    "id": "cf-token",
                    "class": "RuntimeMachineIdentity",
                    "provider": "principal_owned_file",
                    "locator": "/etc/cynic/secrets/cf-api-token"
                }
            },
            "proxy_policy": {
                "web_records": "proxied",
                "mail_records": "dns_only",
                "verification_records": "dns_only",
                "dnssec_records": "dns_only"
            },
            "expected_nameservers": ["ns1.cloudflare.com", "ns2.cloudflare.com"],
            "edge_probes": [
                {
                    "fqdn": "www.example.com",
                    "expected_status": 200,
                    "expected_redirect": null,
                    "follow_redirects": true,
                    "preserve_path": false
                }
            ]
        }"#.to_string()
    }

    #[test]
    fn parses_valid_desired_state() {
        let json = valid_desired_state_json();
        let state = parse_desired_state(&json).expect("valid desired state should parse");
        assert_eq!(state.domain, "example.com");
        assert_eq!(state.registrar.provider, "porkbun");
        assert_eq!(state.dns_authority.provider, "cloudflare");
        assert_eq!(state.proxy_policy.web_records, ProxyMode::Proxied);
        assert_eq!(state.expected_nameservers.len(), 2);
        assert_eq!(state.edge_probes.len(), 1);
    }

    #[test]
    fn rejects_wrong_schema() {
        let mut json = valid_desired_state_json();
        json = json.replace(
            "cynic.domain_ops.desired_state.v1",
            "cynic.domain_ops.desired_state.v0",
        );
        let result = parse_desired_state(&json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_schema_mismatch());
        assert!(err.is_json_parse_error() == false);
    }

    #[test]
    fn rejects_invalid_domain() {
        let mut json = valid_desired_state_json();
        json = json.replace("example.com", "no-dot-domain");
        let result = parse_desired_state(&json);
        assert!(result.is_err());
        match &result.unwrap_err() {
            DomainOpsError::InvalidDomain { domain, .. } => {
                assert_eq!(domain, "no-dot-domain");
            }
            _ => panic!("expected InvalidDomain error"),
        }
    }

    #[test]
    fn rejects_malformed_json() {
        let result = parse_desired_state("{ not valid json }");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_json_parse_error());
    }

    #[test]
    fn rejects_empty_provider() {
        let json = r#"{
            "schema": "cynic.domain_ops.desired_state.v1",
            "domain": "example.com",
            "registrar": {
                "provider": "",
                "credential": {
                    "id": "k", "class": "RuntimeMachineIdentity",
                    "provider": "process_environment", "locator": "VAR"
                }
            },
            "dns_authority": {
                "provider": "cloudflare",
                "credential": {
                    "id": "k", "class": "RuntimeMachineIdentity",
                    "provider": "process_environment", "locator": "VAR"
                }
            },
            "proxy_policy": {
                "web_records": "proxied", "mail_records": "dns_only",
                "verification_records": "dns_only", "dnssec_records": "dns_only"
            },
            "expected_nameservers": ["ns1.cloudflare.com"],
            "edge_probes": []
        }"#;
        let result = parse_desired_state(json);
        assert!(result.is_err());
        match &result.unwrap_err() {
            DomainOpsError::ProviderConfig { provider, .. } => {
                assert_eq!(provider, "registrar");
            }
            _ => panic!("expected ProviderConfig error"),
        }
    }

    #[test]
    fn rejects_proxied_mail_policy() {
        let mut json = valid_desired_state_json();
        json = json.replace("\"mail_records\": \"dns_only\"", "\"mail_records\": \"proxied\"");
        let result = parse_desired_state(&json);
        assert!(result.is_err());
        match &result.unwrap_err() {
            DomainOpsError::ProxyPolicy(msg) => {
                assert!(msg.contains("mail_records"));
            }
            _ => panic!("expected ProxyPolicy error"),
        }
    }

    #[test]
    fn rejects_empty_probe_fqdn() {
        let json = r#"{
            "schema": "cynic.domain_ops.desired_state.v1",
            "domain": "example.com",
            "registrar": {
                "provider": "porkbun",
                "credential": {
                    "id": "k", "class": "RuntimeMachineIdentity",
                    "provider": "process_environment", "locator": "VAR"
                }
            },
            "dns_authority": {
                "provider": "cloudflare",
                "credential": {
                    "id": "k", "class": "RuntimeMachineIdentity",
                    "provider": "process_environment", "locator": "VAR"
                }
            },
            "proxy_policy": {
                "web_records": "proxied", "mail_records": "dns_only",
                "verification_records": "dns_only", "dnssec_records": "dns_only"
            },
            "expected_nameservers": ["ns1.cloudflare.com"],
            "edge_probes": [
                { "fqdn": "", "expected_status": null, "expected_redirect": null,
                  "follow_redirects": false, "preserve_path": false }
            ]
        }"#;
        let result = parse_desired_state(json);
        assert!(result.is_err());
        match &result.unwrap_err() {
            DomainOpsError::EdgeProbe(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("expected EdgeProbe error"),
        }
    }

    #[test]
    fn desired_state_roundtrips() {
        let json = valid_desired_state_json();
        let state = parse_desired_state(&json).unwrap();
        let reserialized = desired_state_to_json(&state).unwrap();
        let state2 = parse_desired_state(&reserialized).unwrap();
        assert_eq!(state.domain, state2.domain);
        assert_eq!(state.registrar.provider, state2.registrar.provider);
        assert_eq!(state.proxy_policy, state2.proxy_policy);
    }

    #[test]
    fn accepts_auto_proxy_policy() {
        let mut json = valid_desired_state_json();
        json = json.replace("\"proxied\"", "\"auto\"");
        let state = parse_desired_state(&json).expect("Auto policy should be valid");
        assert_eq!(state.proxy_policy.web_records, ProxyMode::Auto);
    }
}

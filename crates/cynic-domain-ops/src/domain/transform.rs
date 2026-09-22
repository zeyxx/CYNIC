//! Record shape transformation: registrar → Cloudflare.
//!
//! Pure, zero-I/O functions that classify DNS records and apply the proxy
//! policy. Mirrors the article's migration rules:
//!   - A/AAAA/CNAME/TXT/MX pass through as-is
//!   - MX, TXT, and anything that can't be proxied stays DNS-only
//!   - Web records follow the configured Cloudflare proxy policy
//!   - Registrar-hosted NS records are ignored
//!
//! Provider-specific alias record flattening (ANAME → A/AAAA) happens in
//! the registrar adapter, not here — the domain layer only sees normalized
//! `DnsRecordShape` entries.

use cynic_protocol::domain_ops::{
    DnsRecordShape, DnsRecordType, ProxyMode, ProxyPolicy, resolve_proxy_flag,
};

/// Classification of a DNS record into the policy category that governs its
/// proxy behavior during zone migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordCategory {
    /// A, AAAA, CNAME — web-facing records. Follows `web_records` policy.
    Web,
    /// MX — mail exchanger. Follows `mail_records` policy.
    Mail,
    /// TXT — verification, SPF, DKIM, etc. Follows `verification_records` policy.
    Verification,
    /// DS, CAA — DNSSEC and certificate authority authorization.
    /// Always DNS-only (never proxied).
    Dnssec,
    /// NS — registrar-hosted nameserver delegation. Always skipped
    /// (registrar-owned, delegated to Cloudflare via NS cutover).
    RegistrarOwned,
    /// SRV and any other type not explicitly covered. Defaults to DNS-only.
    Other,
}

impl RecordCategory {
    pub fn name(self) -> &'static str {
        match self {
            RecordCategory::Web => "web",
            RecordCategory::Mail => "mail",
            RecordCategory::Verification => "verification",
            RecordCategory::Dnssec => "dnssec",
            RecordCategory::RegistrarOwned => "registrar_owned",
            RecordCategory::Other => "other",
        }
    }
}

/// Classify a DNS record type into its policy category.
pub fn classify_record(r#type: DnsRecordType) -> RecordCategory {
    match r#type {
        DnsRecordType::A | DnsRecordType::AAAA | DnsRecordType::CNAME => RecordCategory::Web,
        DnsRecordType::MX => RecordCategory::Mail,
        DnsRecordType::TXT => RecordCategory::Verification,
        DnsRecordType::DS | DnsRecordType::CAA => RecordCategory::Dnssec,
        DnsRecordType::NS => RecordCategory::RegistrarOwned,
        DnsRecordType::SRV => RecordCategory::Other,
    }
}

/// Resolve the `ProxyMode` for a given record type using the proxy policy.
///
/// DNSSEC, verification, mail, and registrar-owned records default to
/// DNS-only even if the policy says `Auto` — they can never be proxied.
pub fn policy_for_record_type(r#type: DnsRecordType, policy: &ProxyPolicy) -> ProxyMode {
    let category = classify_record(r#type);
    match category {
        RecordCategory::Web => policy.web_records,
        RecordCategory::Mail => policy.mail_records,
        RecordCategory::Verification => policy.verification_records,
        RecordCategory::Dnssec | RecordCategory::Other | RecordCategory::RegistrarOwned => {
            ProxyMode::DnsOnly
        }
    }
}

/// Result of transforming a record for Cloudflare ingestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformResult {
    /// Record transformed with resolved proxy flag.
    Transformed(DnsRecordShape),
    /// Record intentionally skipped during migration.
    Skipped { reason: &'static str },
}

/// Transform a registrar DNS record for Cloudflare ingestion.
///
/// Applies the proxy policy to set the `proxied` flag correctly, and skips
/// registrar-owned NS records (they are delegated via NS cutover, not
/// replicated as Cloudflare records).
pub fn transform_record_for_cloudflare(
    record: &DnsRecordShape,
    policy: &ProxyPolicy,
) -> TransformResult {
    let category = classify_record(record.r#type);

    if category == RecordCategory::RegistrarOwned {
        return TransformResult::Skipped {
            reason: "registrar-owned NS record — delegation handled via NS cutover",
        };
    }

    let mode = policy_for_record_type(record.r#type, policy);
    let proxied = resolve_proxy_flag(record.r#type, mode);

    TransformResult::Transformed(DnsRecordShape {
        name: record.name.clone(),
        r#type: record.r#type,
        content: record.content.clone(),
        ttl: record.ttl,
        priority: record.priority,
        proxied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::domain_ops::DnsRecordType;

    fn sample_record(name: &str, r#type: DnsRecordType) -> DnsRecordShape {
        DnsRecordShape {
            name: name.to_string(),
            r#type,
            content: "1.2.3.4".to_string(),
            ttl: 300,
            priority: None,
            proxied: false,
        }
    }

    #[test]
    fn classify_a_aaaa_cname_as_web() {
        assert_eq!(classify_record(DnsRecordType::A), RecordCategory::Web);
        assert_eq!(classify_record(DnsRecordType::AAAA), RecordCategory::Web);
        assert_eq!(classify_record(DnsRecordType::CNAME), RecordCategory::Web);
    }

    #[test]
    fn classify_mx_as_mail() {
        assert_eq!(classify_record(DnsRecordType::MX), RecordCategory::Mail);
    }

    #[test]
    fn classify_txt_as_verification() {
        assert_eq!(
            classify_record(DnsRecordType::TXT),
            RecordCategory::Verification
        );
    }

    #[test]
    fn classify_ds_caa_as_dnssec() {
        assert_eq!(classify_record(DnsRecordType::DS), RecordCategory::Dnssec);
        assert_eq!(classify_record(DnsRecordType::CAA), RecordCategory::Dnssec);
    }

    #[test]
    fn classify_ns_as_registrar_owned() {
        assert_eq!(
            classify_record(DnsRecordType::NS),
            RecordCategory::RegistrarOwned
        );
    }

    #[test]
    fn classify_srv_as_other() {
        assert_eq!(classify_record(DnsRecordType::SRV), RecordCategory::Other);
    }

    #[test]
    fn policy_for_type_auto_follows_classification() {
        let policy = ProxyPolicy::default();
        // Web records: Auto → proxied (because default is Proxied)
        assert_eq!(
            policy_for_record_type(DnsRecordType::A, &policy),
            ProxyMode::Proxied
        );
        // Mail records: Auto → DnsOnly (default DnsOnly)
        assert_eq!(
            policy_for_record_type(DnsRecordType::MX, &policy),
            ProxyMode::DnsOnly
        );
        // TXT: Auto → DnsOnly
        assert_eq!(
            policy_for_record_type(DnsRecordType::TXT, &policy),
            ProxyMode::DnsOnly
        );
        // DS: Auto → DnsOnly
        assert_eq!(
            policy_for_record_type(DnsRecordType::DS, &policy),
            ProxyMode::DnsOnly
        );
        // NS: always DnsOnly regardless of policy
        assert_eq!(
            policy_for_record_type(DnsRecordType::NS, &policy),
            ProxyMode::DnsOnly
        );
    }

    #[test]
    fn transform_skips_registrar_ns_records() {
        let record = sample_record("ns1.example.com", DnsRecordType::NS);
        let policy = ProxyPolicy::default();
        let result = transform_record_for_cloudflare(&record, &policy);
        assert!(matches!(result, TransformResult::Skipped { .. }));
    }

    #[test]
    fn transform_proxies_a_record_with_proxied_policy() {
        let record = sample_record("www.example.com", DnsRecordType::A);
        let policy = ProxyPolicy::default();
        let result = transform_record_for_cloudflare(&record, &policy);
        match result {
            TransformResult::Transformed(t) => {
                assert!(t.proxied);
            }
            _ => panic!("expected Transformed, got Skipped"),
        }
    }

    #[test]
    fn transform_does_not_proxy_mx_record_even_with_proxied_policy() {
        let mut record = sample_record("mail.example.com", DnsRecordType::MX);
        record.priority = Some(10);
        let policy = ProxyPolicy::default();
        let result = transform_record_for_cloudflare(&record, &policy);
        match result {
            TransformResult::Transformed(t) => {
                assert!(!t.proxied);
                assert_eq!(t.priority, Some(10));
            }
            _ => panic!("expected Transformed, got Skipped"),
        }
    }

    #[test]
    fn transform_with_dns_only_policy_proxies_nothing() {
        let policy = ProxyPolicy {
            web_records: ProxyMode::DnsOnly,
            mail_records: ProxyMode::DnsOnly,
            verification_records: ProxyMode::DnsOnly,
            dnssec_records: ProxyMode::DnsOnly,
        };
        let record = sample_record("www.example.com", DnsRecordType::A);
        let result = transform_record_for_cloudflare(&record, &policy);
        match result {
            TransformResult::Transformed(t) => {
                assert!(!t.proxied);
            }
            _ => panic!("expected Transformed"),
        }
    }

    #[test]
    fn transform_preserves_content_and_ttl() {
        let mut record = sample_record("www.example.com", DnsRecordType::A);
        record.content = "203.0.113.42".to_string();
        record.ttl = 3600;
        let policy = ProxyPolicy::default();
        let result = transform_record_for_cloudflare(&record, &policy);
        match result {
            TransformResult::Transformed(t) => {
                assert_eq!(t.content, "203.0.113.42");
                assert_eq!(t.ttl, 3600);
                assert_eq!(t.r#type, DnsRecordType::A);
            }
            _ => panic!("expected Transformed"),
        }
    }
}

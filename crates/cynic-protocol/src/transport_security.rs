use serde::{Deserialize, Serialize};

pub const NETWORK_PATH_EVIDENCE_SCHEMA: &str = "cynic.network_path.evidence.v1";
pub const TRANSPORT_SECURITY_REPORT_SCHEMA: &str = "cynic.transport_security.report.v1";
pub const CERTIFICATE_REF_SCHEMA: &str = "cynic.transport_security.certificate_ref.v1";
/// Default renewal window: refresh a leaf 30 days before expiry.
pub const DEFAULT_RENEW_BEFORE_EXPIRY_SECS: u64 = 30 * 24 * 3600;
/// Maximum compliant leaf lifetime: 90 days (short-lived by default).
pub const MAX_LEAF_LIFETIME_SECS: u64 = 90 * 24 * 3600;

/// Shared proof-evidence vocabulary used across build, cli, host, node and vcs
/// surfaces. Kept in the protocol residue because it is cross-family vocabulary,
/// not transport-specific (40+ consumer sites across 5 crates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEvidenceState {
    Proven,
    Refuted,
    Declared,
    Absent,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportScope {
    LocalProcess,
    LocalPrivilegeBoundary,
    HypervisorGuest,
    RemoteNetwork,
    ExternalProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPathEvidence {
    pub schema: String,
    pub id: String,
    pub service_ref: String,
    pub name_resolution: SecurityEvidenceState,
    pub network_route: SecurityEvidenceState,
    pub transport_reachability: SecurityEvidenceState,
    pub confidentiality: SecurityEvidenceState,
    pub peer_identity: SecurityEvidenceState,
    pub application_identity: SecurityEvidenceState,
    #[serde(default)]
    pub resolved_addresses: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl NetworkPathEvidence {
    pub fn admits_authenticated_application(&self) -> bool {
        self.schema == NETWORK_PATH_EVIDENCE_SCHEMA
            && self.name_resolution == SecurityEvidenceState::Proven
            && self.network_route == SecurityEvidenceState::Proven
            && self.transport_reachability == SecurityEvidenceState::Proven
            && self.confidentiality == SecurityEvidenceState::Proven
            && self.peer_identity == SecurityEvidenceState::Proven
            && self.application_identity == SecurityEvidenceState::Proven
            && !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecuritySurface {
    pub id: String,
    pub scope: TransportScope,
    pub mutates: bool,
    pub confidentiality: SecurityEvidenceState,
    pub peer_identity: SecurityEvidenceState,
    pub mutual_authentication: SecurityEvidenceState,
    pub authorization_binding: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub residual_risks: Vec<String>,
}

impl TransportSecuritySurface {
    pub fn admits_remote_mutation(&self) -> bool {
        self.scope == TransportScope::RemoteNetwork
            && self.mutates
            && self.confidentiality == SecurityEvidenceState::Proven
            && self.peer_identity == SecurityEvidenceState::Proven
            && self.mutual_authentication == SecurityEvidenceState::Proven
            && self.authorization_binding == SecurityEvidenceState::Proven
            && !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecurityReport {
    pub schema: String,
    pub status: String,
    pub surfaces: Vec<TransportSecuritySurface>,
    pub remote_mutation_blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

impl TransportSecurityReport {
    pub fn from_surfaces(surfaces: Vec<TransportSecuritySurface>) -> Self {
        let mut remote_mutation_blockers = Vec::new();
        let mut residual_unknowns = Vec::new();
        for surface in &surfaces {
            if surface.scope == TransportScope::RemoteNetwork
                && surface.mutates
                && !surface.admits_remote_mutation()
            {
                remote_mutation_blockers
                    .push(format!("{}:remote_mutation_identity_unproven", surface.id));
            }
            for (field, state) in [
                ("confidentiality", surface.confidentiality),
                ("peer_identity", surface.peer_identity),
                ("mutual_authentication", surface.mutual_authentication),
                ("authorization_binding", surface.authorization_binding),
            ] {
                if state == SecurityEvidenceState::Unknown {
                    residual_unknowns.push(format!("{}:{field}", surface.id));
                }
            }
        }
        let status = if !remote_mutation_blockers.is_empty() {
            "blocked"
        } else if !residual_unknowns.is_empty() {
            "partial"
        } else {
            "proved"
        };
        Self {
            schema: TRANSPORT_SECURITY_REPORT_SCHEMA.to_string(),
            status: status.to_string(),
            surfaces,
            remote_mutation_blockers,
            residual_unknowns,
        }
    }
}

/// Reference to one X.509 certificate in the fleet PKI (D-TLS1..3).
///
/// A ref, not a certificate: the bytes live with their custodian (root
/// offline, intermediate on kernel primary, leaves on services). Only
/// fingerprints and policy travel. All predicates below are pure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateRef {
    pub schema: String,
    pub fingerprint_sha256: String,
    pub issuer_fingerprint_sha256: String,
    pub subject_cn: String,
    #[serde(default)]
    pub sans: Vec<String>,
    pub not_before_unix_s: u64,
    pub not_after_unix_s: u64,
}

/// Renewal policy for leaf certificates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewalPolicy {
    pub renew_before_expiry_secs: u64,
    pub max_lifetime_secs: u64,
}

impl RenewalPolicy {
    pub fn fleet_default() -> Self {
        Self {
            renew_before_expiry_secs: DEFAULT_RENEW_BEFORE_EXPIRY_SECS,
            max_lifetime_secs: MAX_LEAF_LIFETIME_SECS,
        }
    }
}

/// Normalize a SHA256 fingerprint for comparison: strip separators,
/// uppercase. Accepts `5E:A0:..`, `5e-a0-..`, or bare hex.
pub fn normalize_fingerprint(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase()
}

impl CertificateRef {
    /// True when `now` falls inside the validity window.
    pub fn valid_at(&self, now_unix_s: u64) -> bool {
        self.schema == CERTIFICATE_REF_SCHEMA
            && self.not_before_unix_s <= now_unix_s
            && now_unix_s < self.not_after_unix_s
    }

    /// True when the leaf must be renewed now: expired, inside the
    /// renewal window, or violating the maximum lifetime.
    pub fn needs_renewal(&self, now_unix_s: u64, policy: &RenewalPolicy) -> bool {
        if !self.valid_at(now_unix_s) {
            return true;
        }
        if self.not_after_unix_s.saturating_sub(self.not_before_unix_s) > policy.max_lifetime_secs {
            return true;
        }
        self.not_after_unix_s.saturating_sub(now_unix_s) <= policy.renew_before_expiry_secs
    }

    /// True when `name` is covered: exact SAN match, or a single-level
    /// wildcard (`*.service.cynic` covers `a.service.cynic`, never
    /// `a.b.service.cynic` or the bare zone).
    pub fn matches_san(&self, name: &str) -> bool {
        for san in &self.sans {
            if san == name {
                return true;
            }
            if let Some(suffix) = san.strip_prefix("*.") {
                if !suffix.is_empty()
                    && name.len() > suffix.len() + 1
                    && name.ends_with(suffix)
                    && name.as_bytes()[name.len() - suffix.len() - 1] == b'.'
                {
                    let stem = &name[..name.len() - suffix.len() - 1];
                    if !stem.is_empty() && !stem.contains('.') {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// True when a leaf chains to the fleet root through the known
/// intermediate: leaf issued by intermediate, intermediate issued by
/// root. Fingerprints compared normalized.
pub fn chain_links_to_root(
    leaf: &CertificateRef,
    intermediate_fingerprint_sha256: &str,
    intermediate_issuer_fingerprint_sha256: &str,
    root_fingerprint_sha256: &str,
) -> bool {
    normalize_fingerprint(&leaf.issuer_fingerprint_sha256)
        == normalize_fingerprint(intermediate_fingerprint_sha256)
        && normalize_fingerprint(intermediate_issuer_fingerprint_sha256)
            == normalize_fingerprint(root_fingerprint_sha256)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_mutation_requires_mutual_identity_and_authorization_proof() {
        let report = TransportSecurityReport::from_surfaces(vec![TransportSecuritySurface {
            id: "fleet_node_exec".to_string(),
            scope: TransportScope::RemoteNetwork,
            mutates: true,
            confidentiality: SecurityEvidenceState::Declared,
            peer_identity: SecurityEvidenceState::Unknown,
            mutual_authentication: SecurityEvidenceState::Unknown,
            authorization_binding: SecurityEvidenceState::Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec!["machine identity enrollment absent".to_string()],
        }]);

        assert_eq!(report.status, "blocked");
        assert_eq!(
            report.remote_mutation_blockers,
            vec!["fleet_node_exec:remote_mutation_identity_unproven"]
        );
        assert!(report.residual_unknowns.len() >= 2);
    }

    #[test]
    fn open_transport_is_not_authenticated_application_evidence() {
        let path = NetworkPathEvidence {
            schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
            id: "forgejo-admin".to_string(),
            service_ref: "forgejo".to_string(),
            name_resolution: SecurityEvidenceState::Proven,
            network_route: SecurityEvidenceState::Proven,
            transport_reachability: SecurityEvidenceState::Proven,
            confidentiality: SecurityEvidenceState::Absent,
            peer_identity: SecurityEvidenceState::Unknown,
            application_identity: SecurityEvidenceState::Unknown,
            resolved_addresses: vec!["10.10.10.101:3000".to_string()],
            evidence_refs: vec!["tcp-probe.json".to_string()],
        };
        assert!(!path.admits_authenticated_application());
    }

    fn leaf(sans: &[&str], not_before: u64, not_after: u64) -> CertificateRef {
        CertificateRef {
            schema: CERTIFICATE_REF_SCHEMA.to_string(),
            fingerprint_sha256: "AA".to_string(),
            issuer_fingerprint_sha256: "BB".to_string(),
            subject_cn: "forgejo.service.cynic".to_string(),
            sans: sans.iter().map(|s| s.to_string()).collect(),
            not_before_unix_s: not_before,
            not_after_unix_s: not_after,
        }
    }

    #[test]
    fn certificate_validity_windows_behave() {
        let valid = leaf(&[], 1000, 2000);
        for (now, expected) in [
            (999, false),
            (1000, true),
            (1500, true),
            (1999, true),
            (2000, false),
        ] {
            assert_eq!(valid.valid_at(now), expected, "now={now}");
        }
        let wrong_schema = CertificateRef {
            schema: "wrong".to_string(),
            ..leaf(&[], 1000, 2000)
        };
        assert!(!wrong_schema.valid_at(1500));
    }

    #[test]
    fn renewal_triggers_on_expiry_window_and_lifetime() {
        let policy = RenewalPolicy::fleet_default();
        let fresh = leaf(&[], 0, 90 * 24 * 3600);
        assert!(!fresh.needs_renewal(24 * 3600, &policy));
        assert!(fresh.needs_renewal(61 * 24 * 3600, &policy));
        assert!(fresh.needs_renewal(100 * 24 * 3600, &policy));
        let eternal = leaf(&[], 0, 365 * 24 * 3600);
        assert!(eternal.needs_renewal(24 * 3600, &policy));
    }

    #[test]
    fn san_matching_covers_exact_and_single_level_wildcard() {
        let cert = leaf(&["forgejo.service.cynic", "*.service.cynic"], 0, 100);
        for (name, expected) in [
            ("forgejo.service.cynic", true),
            ("kernel.service.cynic", true),
            ("a.b.service.cynic", false),
            ("service.cynic", false),
            ("forgejo.legacy.invalid", false),
        ] {
            assert_eq!(cert.matches_san(name), expected, "name={name}");
        }
    }

    #[test]
    fn chain_verification_normalizes_fingerprints() {
        let leaf_cert = leaf(&[], 0, 100);
        assert!(chain_links_to_root(&leaf_cert, "bb", "CC", "c-c"));
        assert!(!chain_links_to_root(&leaf_cert, "DD", "CC", "c-c"));
        assert!(!chain_links_to_root(&leaf_cert, "bb", "CC", "EE"));
    }
}

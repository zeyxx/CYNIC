//! Intent transmission record (H1 slice, Phase 1).
//!
//! WHY: Human intent crossing agent boundaries was prose_only debt (H1,
//! debt-audit Tier1 avéré). This type is the smallest shippable record that
//! makes one inter-agent transmission explicit, addressed, sourced, and
//! integrity-sealed.
//!
//! SCOPE: Structural validation only (field constraints, digest match).
//! Domain predicates (conflict, admission, routing) belong to the Judicial
//! layer and do not exist yet — their absence is stated, not hidden.
//!
//! Digest law: internal integrity hashing is SHA3-512 via the QUALIFIED path
//! `crate::digest::digest_json`. Never the crate-root `crate::digest_json`
//! re-export (that resolves to `integrity::digest_json`, SHA-256 for external
//! contracts). See protocols/h1-intent.md §Digest ambiguity.
//!
//! Conventions (strings until a dispatcher consumes the namespace):
//! emitter/promoter: "pole:<name>" | "human:<name>"
//! recipient: "agent:<id>" | "node:<id>" | "role:<name>"
//! Prefix inventory (U5, documentation only): `node:<id>` maps to
//! `NodeIdentityDesiredState.node_id` (node_identity.rs); `agent:<id>` maps
//! to the workload/agent identity registries; `role:`/`pole:`/`human:` are
//! team-topology conventions, out of protocol scope.

use serde::{Deserialize, Serialize};

/// Schema identifier for [`IntentTransmission`].
pub const INTENT_TRANSMISSION_SCHEMA: &str = "cynic.intent_transmission.v1";

/// Maximum content length in characters (shape bound, not a quality judgment).
pub const MAX_CONTENT_CHARS: usize = 2000;

/// Provenance class for a transmission claim.
///
/// Bootcamp values ("Sujet.xlsm", kit PDF, team proposal) are INSTANCES of
/// these classes, never new variants. `observed_fact` exists because field
/// evidence (e.g. a scan result) is neither official nor speculative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceClass {
    OfficialSource,
    ProvidedKit,
    ObservedFact,
    UnverifiedProposal,
}

/// One explicit inter-agent transmission: addressed, sourced, sealed.
///
/// Cross-references target `id`, never `digest` (a digest cannot be referenced
/// before it is computed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentTransmission {
    /// Unique record identifier (uuid v4 by convention, pre-persistence).
    pub id: String,
    /// Must equal [`INTENT_TRANSMISSION_SCHEMA`].
    pub schema: String,
    /// Creation time, RFC 3339 UTC (shape-checked only, no chrono dep).
    pub created_at: String,
    /// Sender address, e.g. "pole:cyber" or "human:coordinator".
    pub emitter: String,
    /// Recipient address, e.g. "role:pole_infra".
    pub recipient: String,
    /// Mission/deliverable tag, e.g. "M2-carto".
    pub deliverable_ref: String,
    /// Payload, 1..=MAX_CONTENT_CHARS chars.
    pub content: String,
    /// Claim class (see [`ProvenanceClass`]).
    pub provenance_class: ProvenanceClass,
    /// Who/what produced the claim (non-empty).
    pub provenance_source: String,
    /// SHA3-512 digest over all fields above, via `crate::digest::digest_json`.
    pub digest: String,
}

impl IntentTransmission {
    /// Canonical digest payload: every field except `digest` itself.
    fn digest_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "schema": self.schema,
            "created_at": self.created_at,
            "emitter": self.emitter,
            "recipient": self.recipient,
            "deliverable_ref": self.deliverable_ref,
            "content": self.content,
            "provenance_class": self.provenance_class,
            "provenance_source": self.provenance_source,
        })
    }

    /// Compute the expected digest for this record.
    pub fn expected_digest(&self) -> Result<String, String> {
        crate::digest::digest_json(&self.digest_payload())
            .map_err(|e| format!("digest_serialization_failed: {e}"))
    }

    /// Structural validation only. Returns every violation found.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();

        if self.id.trim().is_empty() {
            violations.push("id_must_be_non_empty".to_string());
        }
        if self.schema != INTENT_TRANSMISSION_SCHEMA {
            violations.push(format!("schema_must_equal_{INTENT_TRANSMISSION_SCHEMA}"));
        }
        if !is_rfc3339_shape(&self.created_at) {
            violations.push("created_at_must_be_rfc3339_utc_shape".to_string());
        }
        if !has_address_prefix(&self.emitter) {
            violations.push("emitter_must_carry_namespace_prefix".to_string());
        }
        if !has_address_prefix(&self.recipient) {
            violations.push("recipient_must_carry_namespace_prefix".to_string());
        }
        if self.deliverable_ref.trim().is_empty() {
            violations.push("deliverable_ref_must_be_non_empty".to_string());
        }
        let content_len = self.content.chars().count();
        if content_len == 0 || content_len > MAX_CONTENT_CHARS {
            violations.push(format!("content_must_be_1_to_{MAX_CONTENT_CHARS}_chars"));
        }
        if self.provenance_source.trim().is_empty() {
            violations.push("provenance_source_must_be_non_empty".to_string());
        }
        match self.expected_digest() {
            Ok(expected) => {
                if expected != self.digest {
                    violations.push("digest_mismatch".to_string());
                }
                if !self.digest.starts_with(crate::digest::SHA3_PREFIX) {
                    violations.push("digest_must_carry_sha3_512_prefix".to_string());
                }
            }
            Err(e) => violations.push(e),
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

/// Shape check for RFC 3339 UTC timestamps without a time dependency.
///
/// Accepts `YYYY-MM-DDTHH:MM:SSZ` (and fractional/offset forms sharing the
/// `T` separator and a trailing zone designator). Shape only: a well-formed
/// but impossible date (e.g. month 13) passes — full calendar validation is
/// deferred to the persistence slice. Stating the limit is the point.
fn is_rfc3339_shape(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 20 {
        return false;
    }
    let bytes = value.as_bytes();
    // Minimal skeleton: digits at date/time slots, 'T' separator, zone tail.
    let digits = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let skeleton = digits(0)
        && digits(1)
        && digits(2)
        && digits(3)
        && bytes.get(4) == Some(&b'-')
        && digits(5)
        && digits(6)
        && bytes.get(7) == Some(&b'-')
        && digits(8)
        && digits(9)
        && bytes.get(10) == Some(&b'T')
        && digits(11)
        && digits(12)
        && bytes.get(13) == Some(&b':')
        && digits(14)
        && digits(15)
        && bytes.get(16) == Some(&b':')
        && digits(17)
        && digits(18);
    if !skeleton {
        return false;
    }
    value.ends_with('Z') || value.contains('+') || value[10..].contains('-')
}

/// Address convention: `<namespace>:<id>` with a known namespace prefix.
fn has_address_prefix(value: &str) -> bool {
    let Some((prefix, id)) = value.split_once(':') else {
        return false;
    };
    !id.trim().is_empty() && matches!(prefix, "pole" | "human" | "agent" | "node" | "role")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_transmission() -> IntentTransmission {
        let mut tx = IntentTransmission {
            id: "0193f2a1-4b5c-4d6e-8f90-123456789abc".to_string(),
            schema: INTENT_TRANSMISSION_SCHEMA.to_string(),
            created_at: "2026-09-09T14:30:00Z".to_string(),
            emitter: "pole:cyber".to_string(),
            recipient: "role:pole_infra".to_string(),
            deliverable_ref: "M2-carto".to_string(),
            content: "Scan found 3 hosts on undocumented VLAN 42. Need physical confirmation."
                .to_string(),
            provenance_class: ProvenanceClass::ObservedFact,
            provenance_source: "Cyber pole scan, 2026-09-09T14:30Z, operator cyber-1".to_string(),
            digest: String::new(),
        };
        tx.digest = tx.expected_digest().expect("digest must compute");
        tx
    }

    #[test]
    fn valid_transmission_passes_with_sha3_prefix() {
        let tx = valid_transmission();
        assert!(tx.validate().is_ok(), "valid record must pass");
        assert!(
            tx.digest.starts_with(crate::digest::SHA3_PREFIX),
            "digest must be sha3-512, never the root sha256 re-export"
        );
    }

    #[test]
    fn empty_emitter_fails() {
        let mut tx = valid_transmission();
        tx.emitter = String::new();
        // Re-seal so the failure under test is the emitter, not the digest.
        tx.digest = tx.expected_digest().expect("digest must compute");
        let err = tx.validate().expect_err("empty emitter must fail");
        assert!(err.contains(&"emitter_must_carry_namespace_prefix".to_string()));
    }

    #[test]
    fn tampered_content_fails_digest() {
        let mut tx = valid_transmission();
        tx.content = "Altered after sealing.".to_string();
        let err = tx.validate().expect_err("tampered content must fail");
        assert!(err.contains(&"digest_mismatch".to_string()));
    }

    #[test]
    fn malformed_timestamp_fails() {
        let mut tx = valid_transmission();
        tx.created_at = "09/09/2026 14h30".to_string();
        tx.digest = tx.expected_digest().expect("digest must compute");
        let err = tx.validate().expect_err("bad timestamp must fail");
        assert!(err.contains(&"created_at_must_be_rfc3339_utc_shape".to_string()));
    }

    #[test]
    fn oversize_content_fails() {
        let mut tx = valid_transmission();
        tx.content = "x".repeat(MAX_CONTENT_CHARS + 1);
        tx.digest = tx.expected_digest().expect("digest must compute");
        let err = tx.validate().expect_err("oversize content must fail");
        assert!(err.iter().any(|v| v.starts_with("content_must_be_1_to_")));
    }

    #[test]
    fn sha256_root_digest_is_rejected() {
        // Guard against the glob-shadow trap: a digest sealed with the
        // crate-root (sha256) function must NOT validate.
        let mut tx = valid_transmission();
        let payload = tx.digest_payload();
        tx.digest = crate::integrity::digest_json(&payload).expect("root digest must compute");
        assert!(tx.digest.starts_with("sha256:"));
        let err = tx.validate().expect_err("sha256 digest must fail");
        assert!(err.contains(&"digest_mismatch".to_string()));
    }

    #[test]
    fn bare_string_addresses_are_rejected() {
        let mut tx = valid_transmission();
        tx.recipient = "team".to_string();
        tx.digest = tx.expected_digest().expect("digest must compute");
        let err = tx.validate().expect_err("prefix-less recipient must fail");
        assert!(err.contains(&"recipient_must_carry_namespace_prefix".to_string()));
    }
}

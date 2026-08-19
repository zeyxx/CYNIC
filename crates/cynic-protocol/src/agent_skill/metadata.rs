//! Immutable metadata and append-only lifecycle records for skill artifacts.

use serde::{Deserialize, Serialize};

use crate::{AgentEvidenceRef, digest_json, validate_evidence_refs};

use super::{AgentSkillBundle, AgentSkillBundleViolation, violation};

pub const AGENT_SKILL_LIFECYCLE_RECORD_SCHEMA: &str = "cynic.agent_skill_lifecycle_record.v1";
pub const AGENT_SKILL_PROJECTION_SCHEMA: &str = "cynic.agent_skill_projection.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSkillDataClassification {
    Public,
    Private,
    Sensitive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSkillRetentionPolicy {
    Ephemeral,
    RetainUntilRevoked,
    RetainUntilUnixMs { unix_ms: u64 },
}

/// Metadata admitted together with immutable skill content. It describes the
/// artifact; it is not mutable execution state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillMetadata {
    pub source_evidence: AgentEvidenceRef,
    pub issued_at_ms: u64,
    pub data_classification: AgentSkillDataClassification,
    pub retention: AgentSkillRetentionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSkillLifecycleState {
    Admitted,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSkillAvailability {
    Admitted,
    Revoked,
    Superseded,
    Unknown,
}

/// Query-friendly data projection derived from immutable bundles and validated
/// lifecycle events. Stores may index it but must be able to rebuild it from
/// the source records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillProjection {
    pub schema: String,
    pub skill_id: String,
    pub version: String,
    pub content_digest: String,
    pub source_digest: String,
    pub approval_digest: String,
    pub data_classification: AgentSkillDataClassification,
    pub retention: AgentSkillRetentionPolicy,
    pub compatible_runtime_ids: Vec<String>,
    pub required_capability_ids: Vec<String>,
    pub availability: AgentSkillAvailability,
    pub latest_lifecycle_digest: Option<String>,
    pub lifecycle_record_count: u32,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

/// An immutable event that changes whether a bundle may be newly bound. It
/// never changes the bundle bytes or rewrites prior delivery evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillLifecycleRecord {
    pub schema: String,
    pub skill_id: String,
    pub content_digest: String,
    pub state: AgentSkillLifecycleState,
    pub changed_at_ms: u64,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_content_digest: Option<String>,
    pub authority_evidence: AgentEvidenceRef,
}

pub(crate) fn validate_agent_skill_metadata(
    metadata: &AgentSkillMetadata,
) -> Vec<AgentSkillBundleViolation> {
    let mut violations = Vec::new();
    if metadata.issued_at_ms == 0 {
        violations.push(violation(
            "skill_metadata_issued_at_missing",
            "skill metadata requires a non-zero issued_at_ms",
        ));
    }
    if let AgentSkillRetentionPolicy::RetainUntilUnixMs { unix_ms } = metadata.retention {
        if unix_ms <= metadata.issued_at_ms {
            violations.push(violation(
                "skill_retention_deadline_invalid",
                "retention deadline must be later than the bundle issue time",
            ));
        }
    }
    violations.extend(
        validate_evidence_refs(std::slice::from_ref(&metadata.source_evidence))
            .into_iter()
            .map(|issue| violation("invalid_skill_source_evidence", issue.message)),
    );
    violations
}

pub fn validate_agent_skill_lifecycle_record(
    record: &AgentSkillLifecycleRecord,
) -> Vec<AgentSkillBundleViolation> {
    let mut violations = Vec::new();
    if record.schema != AGENT_SKILL_LIFECYCLE_RECORD_SCHEMA {
        violations.push(violation(
            "skill_lifecycle_schema_invalid",
            "skill lifecycle record schema is not supported",
        ));
    }
    for (field, value) in [
        ("skill_id", record.skill_id.as_str()),
        ("content_digest", record.content_digest.as_str()),
        ("reason", record.reason.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "skill_lifecycle_required_field_missing",
                format!("skill lifecycle record requires {field}"),
            ));
        }
    }
    if !valid_sha256(&record.content_digest) || record.changed_at_ms == 0 {
        violations.push(violation(
            "skill_lifecycle_identity_invalid",
            "skill lifecycle record requires a SHA-256 digest and non-zero change time",
        ));
    }
    match (record.state, record.replacement_content_digest.as_deref()) {
        (AgentSkillLifecycleState::Superseded, Some(digest))
            if valid_sha256(digest) && digest != record.content_digest => {}
        (AgentSkillLifecycleState::Superseded, _) => violations.push(violation(
            "skill_supersedence_replacement_invalid",
            "a superseded skill requires a distinct SHA-256 replacement digest",
        )),
        (_, Some(_)) => violations.push(violation(
            "skill_lifecycle_replacement_unexpected",
            "only a superseded skill may name a replacement digest",
        )),
        _ => {}
    }
    if record.authority_evidence.kind != "human_approval" {
        violations.push(violation(
            "skill_lifecycle_human_authority_missing",
            "skill lifecycle changes require human_approval evidence",
        ));
    }
    violations.extend(
        validate_evidence_refs(std::slice::from_ref(&record.authority_evidence))
            .into_iter()
            .map(|issue| violation("invalid_skill_lifecycle_evidence", issue.message)),
    );
    violations
}

pub fn project_agent_skill(
    bundle: &AgentSkillBundle,
    lifecycle_records: &[AgentSkillLifecycleRecord],
) -> AgentSkillProjection {
    let mut matching = lifecycle_records
        .iter()
        .filter(|record| {
            record.skill_id == bundle.skill_id && record.content_digest == bundle.content_digest
        })
        .collect::<Vec<_>>();
    matching.sort_by(|left, right| left.changed_at_ms.cmp(&right.changed_at_ms));

    let mut unknowns = Vec::new();
    let mut availability = AgentSkillAvailability::Admitted;
    let mut latest_lifecycle_digest = None;
    if let Some(latest) = matching.last() {
        let conflicting_latest_state = matching.iter().any(|record| {
            record.changed_at_ms == latest.changed_at_ms && record.state != latest.state
        });
        if conflicting_latest_state {
            availability = AgentSkillAvailability::Unknown;
            unknowns.push("skill_lifecycle_latest_event_conflict".to_string());
        } else {
            availability = match latest.state {
                AgentSkillLifecycleState::Admitted => AgentSkillAvailability::Admitted,
                AgentSkillLifecycleState::Revoked => AgentSkillAvailability::Revoked,
                AgentSkillLifecycleState::Superseded => AgentSkillAvailability::Superseded,
            };
        }
        latest_lifecycle_digest = digest_json(latest).ok();
    }
    if matching
        .iter()
        .any(|record| !validate_agent_skill_lifecycle_record(record).is_empty())
    {
        availability = AgentSkillAvailability::Unknown;
        unknowns.push("skill_lifecycle_record_invalid".to_string());
    }
    unknowns.sort();
    unknowns.dedup();
    AgentSkillProjection {
        schema: AGENT_SKILL_PROJECTION_SCHEMA.to_string(),
        skill_id: bundle.skill_id.clone(),
        version: bundle.version.clone(),
        content_digest: bundle.content_digest.clone(),
        source_digest: bundle.metadata.source_evidence.digest.clone(),
        approval_digest: bundle.approval_evidence.digest.clone(),
        data_classification: bundle.metadata.data_classification,
        retention: bundle.metadata.retention.clone(),
        compatible_runtime_ids: bundle.compatible_runtime_ids.clone(),
        required_capability_ids: bundle.required_capability_ids.clone(),
        availability,
        latest_lifecycle_digest,
        lifecycle_record_count: matching.len().min(u32::MAX as usize) as u32,
        unknowns,
    }
}

/// Whether `value` is a `sha256:<64 lowercase hex>` digest reference.
///
/// Deliberately STRICTER than the crate-wide digest check: skill digests must
/// be lowercase, because a skill is addressed by its digest string and
/// `sha256:AB…` / `sha256:ab…` would otherwise name the same skill twice.
/// Do not replace this with the shared helper — `uppercase_digest_is_rejected`
/// locks the difference. See `evidence::valid_sha256_digest` for the
/// case-insensitive variant used everywhere else.
fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skill digests are case-sensitive on purpose. The crate-wide helper
    /// (`evidence::valid_sha256_digest`) accepts uppercase hex; this one must
    /// not, or the same skill could be addressed by two different strings.
    ///
    /// This test exists because 20 other copies of this check in the crate DO
    /// accept uppercase. Anyone consolidating them must not fold this one in.
    #[test]
    fn uppercase_digest_is_rejected() {
        let lower = format!("sha256:{}", "a".repeat(64));
        let upper = format!("sha256:{}", "A".repeat(64));

        assert!(valid_sha256(&lower));
        assert!(
            !valid_sha256(&upper),
            "skill digests must stay lowercase-only; the shared helper accepts \
             uppercase and folding this one into it would make one skill \
             addressable by two digests"
        );
        // The shared helper genuinely differs — proof the divergence is real.
        assert!(crate::evidence::valid_sha256_digest(&upper));
    }

    fn evidence() -> AgentEvidenceRef {
        AgentEvidenceRef {
            kind: "human_approval".to_string(),
            subject: "skill:test".to_string(),
            digest: format!("sha256:{}", "a".repeat(64)),
            lineage: None,
        }
    }

    #[test]
    fn supersedence_requires_a_distinct_replacement_digest() {
        let record = AgentSkillLifecycleRecord {
            schema: AGENT_SKILL_LIFECYCLE_RECORD_SCHEMA.to_string(),
            skill_id: "test".to_string(),
            content_digest: format!("sha256:{}", "a".repeat(64)),
            state: AgentSkillLifecycleState::Superseded,
            changed_at_ms: 1,
            reason: "replacement approved".to_string(),
            replacement_content_digest: Some(format!("sha256:{}", "b".repeat(64))),
            authority_evidence: evidence(),
        };
        assert!(validate_agent_skill_lifecycle_record(&record).is_empty());
    }
}

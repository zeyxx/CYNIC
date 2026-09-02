//! Publication / disclosure consent policy — pure law for what may leave the
//! private membrane toward a public audience.
//!
//! Authority: AGENTS.md §Long-Horizon Direction — "disclose user activity
//! only through explicit opt-in publication policy, minimization, consent,
//! and evidence-preserving projection". This module turns that guardrail
//! into refutable types; it performs no I/O and owns no transport.
//!
//! Core laws (each enforced by a named test):
//! 1. hold_by_default            — no explicit consent means nothing ships.
//! 2. consent_scope_is_not_amplifiable — a narrow consent never covers more.
//! 3. minimization_is_whitelist  — undeclared fields can never be projected,
//!   even under a standing policy.
//! 4. stale_consent_blocks       — a different policy digest voids consent.
//! 5. projection_cites_sources   — every published field keeps its source
//!   digest; the projection derives, it never fabricates.
//!
//! Schema: `cynic.publication_policy.v1`

use crate::error_catalog::CynicErrorCode;
use serde::{Deserialize, Serialize};

pub const PUBLICATION_POLICY_SCHEMA: &str = "cynic.publication_policy.v1";
pub const PUBLIC_PROJECTION_SCHEMA: &str = "cynic.public_projection.v1";

/// The only two states a publication decision can take. Absence of a consent
/// record is Hold by construction at the call site — there is no Maybe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationDecision {
    Publish,
    Hold,
}

/// What exactly was consented to. A scope is a boundary, not a grant of
/// discretion: whatever it does not name is not covered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentScope {
    /// Exactly one artifact, named by its canonical digest.
    SingleArtifact { artifact_digest: String },
    /// Any artifact tagged with this topic id.
    Topic { topic_id: String },
    /// Any artifact admitted under the standing policy named by digest.
    StandingPolicy { policy_digest: String },
}

/// A recorded, refutable consent act. `policy_digest` binds the consent to one
/// exact version of the disclosure policy that interpreted it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationConsent {
    pub schema: String,
    pub consent_id: String,
    /// Principal or human admin that granted it. Never "system".
    pub granted_by: String,
    pub scope: ConsentScope,
    /// sha256 of the disclosure policy version in force at grant time.
    pub policy_digest: String,
    pub granted_at_unix_s: i64,
}

/// One field of a private artifact and whether the disclosure policy declares
/// it publishable. Whitelist semantics: absent means never publishable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureField {
    pub field_name: String,
    pub publishable: bool,
    /// Digest of the field content in the private artifact.
    pub source_digest: String,
}

impl DisclosureField {
    pub fn publishable(field_name: &str, source_digest: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            publishable: true,
            source_digest: source_digest.to_string(),
        }
    }

    pub fn private(field_name: &str, source_digest: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            publishable: false,
            source_digest: source_digest.to_string(),
        }
    }
}

/// The minimized, evidence-preserving projection that may leave the membrane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicProjection {
    pub schema: String,
    pub artifact_digest: String,
    pub consent_id: String,
    /// Published fields only, with their source digests.
    pub fields: Vec<(String, String)>,
    pub projected_at_unix_s: i64,
}

/// Named refusal reasons. Unknown is data; blockers are stable identifiers an
/// agent can match on without parsing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationBlocker {
    DecisionHold,
    ConsentMissing,
    ConsentScopeMismatch { required: &'static str },
    ConsentPolicyStale { expected: String, granted: String },
    FieldNotDisclosable { field_name: String },
    EmptyProjection,
}

impl PublicationBlocker {
    pub fn code(&self) -> String {
        match self {
            Self::DecisionHold => "publication_decision_hold".to_string(),
            Self::ConsentMissing => "publication_consent_missing".to_string(),
            Self::ConsentScopeMismatch { .. } => "publication_consent_scope_mismatch".to_string(),
            Self::ConsentPolicyStale { .. } => "publication_consent_policy_stale".to_string(),
            Self::FieldNotDisclosable { field_name } => {
                format!("publication_field_not_disclosable:{field_name}")
            }
            Self::EmptyProjection => "publication_projection_empty".to_string(),
        }
    }

    pub fn to_cynic_error_code(&self) -> CynicErrorCode {
        match self {
            Self::DecisionHold | Self::ConsentMissing => CynicErrorCode::PublicationConsentMissing,
            Self::ConsentScopeMismatch { .. } | Self::ConsentPolicyStale { .. } => {
                CynicErrorCode::PublicationConsentExpired
            }
            Self::FieldNotDisclosable { .. } | Self::EmptyProjection => {
                CynicErrorCode::PublicationFieldUnwhitelisted
            }
        }
    }
}

/// Does this consent authorize `artifact_digest` under the given policy?
/// Pure scope interpretation — the single place scope meaning lives.
fn consent_covers(
    consent: &PublicationConsent,
    artifact_digest: &str,
    topic_id: Option<&str>,
    policy_digest: &str,
) -> Result<(), PublicationBlocker> {
    if consent.policy_digest != policy_digest {
        return Err(PublicationBlocker::ConsentPolicyStale {
            expected: policy_digest.to_string(),
            granted: consent.policy_digest.clone(),
        });
    }
    let covered = match &consent.scope {
        ConsentScope::SingleArtifact { artifact_digest: d } => d == artifact_digest,
        ConsentScope::Topic { topic_id: t } => topic_id == Some(t.as_str()),
        ConsentScope::StandingPolicy { policy_digest: p } => p == policy_digest,
    };
    if covered {
        Ok(())
    } else {
        Err(PublicationBlocker::ConsentScopeMismatch {
            required: "scope matching artifact, topic, or standing policy",
        })
    }
}

/// Derive the public projection of one artifact — pure, whitelist-driven,
/// evidence-preserving. This is the ONLY door from private fields to a
/// publishable projection.
///
/// * `artifact_digest` — canonical digest of the whole private artifact.
/// * `topic_id` — optional audience topic the artifact belongs to.
/// * `fields` — every candidate field with its disclosure classification.
/// * `consent`/`decision`/`policy_digest` — the recorded authority to ship.
pub fn project_for_publication(
    artifact_digest: &str,
    topic_id: Option<&str>,
    fields: &[DisclosureField],
    consent: Option<&PublicationConsent>,
    decision: PublicationDecision,
    policy_digest: &str,
    now_unix_s: i64,
) -> Result<PublicProjection, Vec<PublicationBlocker>> {
    let mut blockers = Vec::new();

    if decision != PublicationDecision::Publish {
        blockers.push(PublicationBlocker::DecisionHold);
    }

    let Some(consent) = consent else {
        blockers.push(PublicationBlocker::ConsentMissing);
        return Err(blockers);
    };

    if let Err(scope_blocker) = consent_covers(consent, artifact_digest, topic_id, policy_digest) {
        blockers.push(scope_blocker);
    }

    if consent.granted_by.is_empty() || consent.consent_id.is_empty() {
        blockers.push(PublicationBlocker::ConsentMissing);
    }

    if !blockers.is_empty() {
        return Err(blockers);
    }

    // Whitelist projection: only fields declared publishable AND covered by
    // an explicit per-field pass survive. Undeclared = private forever.
    let mut projected = Vec::new();
    for field in fields {
        if field.publishable {
            if field.source_digest.is_empty() {
                blockers.push(PublicationBlocker::FieldNotDisclosable {
                    field_name: field.field_name.clone(),
                });
                continue;
            }
            projected.push((field.field_name.clone(), field.source_digest.clone()));
        } else {
            blockers.push(PublicationBlocker::FieldNotDisclosable {
                field_name: field.field_name.clone(),
            });
        }
    }

    if projected.is_empty() {
        blockers.push(PublicationBlocker::EmptyProjection);
        return Err(blockers);
    }

    Ok(PublicProjection {
        schema: PUBLIC_PROJECTION_SCHEMA.to_string(),
        artifact_digest: artifact_digest.to_string(),
        consent_id: consent.consent_id.clone(),
        fields: projected,
        projected_at_unix_s: now_unix_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = "sha256:policy-v1-digest";

    fn consent(scope: ConsentScope) -> PublicationConsent {
        PublicationConsent {
            schema: PUBLICATION_POLICY_SCHEMA.to_string(),
            consent_id: "consent-1".to_string(),
            granted_by: "human-admin".to_string(),
            scope,
            policy_digest: POLICY.to_string(),
            granted_at_unix_s: 1_000,
        }
    }

    fn fields() -> Vec<DisclosureField> {
        vec![
            DisclosureField::publishable("verdict_summary", "sha256:f1"),
            DisclosureField::publishable("market_consequence", "sha256:f2"),
            DisclosureField::private("operator_notes", "sha256:f3"),
            DisclosureField::private("agent_identity", "sha256:f4"),
        ]
    }

    #[test]
    fn hold_by_default_blocks_everything() {
        let err = project_for_publication(
            "sha256:artifact",
            None,
            &fields(),
            Some(&consent(ConsentScope::StandingPolicy {
                policy_digest: POLICY.to_string(),
            })),
            PublicationDecision::Hold,
            POLICY,
            2_000,
        )
        .expect_err("hold must block");
        assert!(err.contains(&PublicationBlocker::DecisionHold));
    }

    #[test]
    fn missing_consent_blocks_with_named_reason() {
        let err = project_for_publication(
            "sha256:artifact",
            None,
            &fields(),
            None,
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect_err("no consent must block");
        assert_eq!(err, vec![PublicationBlocker::ConsentMissing]);
    }

    #[test]
    fn consent_scope_is_not_amplifiable() {
        // SingleArtifact consent must not cover a different artifact…
        let err = project_for_publication(
            "sha256:OTHER-artifact",
            None,
            &fields(),
            Some(&consent(ConsentScope::SingleArtifact {
                artifact_digest: "sha256:artifact".to_string(),
            })),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect_err("scope mismatch must block");
        assert!(
            err.iter()
                .any(|b| matches!(b, PublicationBlocker::ConsentScopeMismatch { .. }))
        );

        // …and a Topic consent must not fire without the topic.
        let err = project_for_publication(
            "sha256:artifact",
            None,
            &fields(),
            Some(&consent(ConsentScope::Topic {
                topic_id: "futarchic-audience".to_string(),
            })),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect_err("topic consent needs the topic present");
        assert!(
            err.iter()
                .any(|b| matches!(b, PublicationBlocker::ConsentScopeMismatch { .. }))
        );
    }

    #[test]
    fn minimization_is_whitelist() {
        let projection = project_for_publication(
            "sha256:artifact",
            Some("futarchic-audience"),
            &fields(),
            Some(&consent(ConsentScope::Topic {
                topic_id: "futarchic-audience".to_string(),
            })),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect("publishable fields project");

        let names: Vec<&str> = projection.fields.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["verdict_summary", "market_consequence"]);
        assert!(!names.contains(&"operator_notes"));
        assert!(!names.contains(&"agent_identity"));
    }

    #[test]
    fn stale_consent_blocks() {
        let mut c = consent(ConsentScope::StandingPolicy {
            policy_digest: POLICY.to_string(),
        });
        c.policy_digest = "sha256:old-policy".to_string();
        let err = project_for_publication(
            "sha256:artifact",
            None,
            &fields(),
            Some(&c),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect_err("stale policy digest must block");
        assert!(matches!(
            err.first(),
            Some(PublicationBlocker::ConsentPolicyStale { .. })
        ));
    }

    #[test]
    fn projection_cites_sources_and_stays_pure() {
        let a = project_for_publication(
            "sha256:artifact",
            Some("futarchic-audience"),
            &fields(),
            Some(&consent(ConsentScope::Topic {
                topic_id: "futarchic-audience".to_string(),
            })),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect("projection");
        let b = project_for_publication(
            "sha256:artifact",
            Some("futarchic-audience"),
            &fields(),
            Some(&consent(ConsentScope::Topic {
                topic_id: "futarchic-audience".to_string(),
            })),
            PublicationDecision::Publish,
            POLICY,
            2_000,
        )
        .expect("projection");
        assert_eq!(a, b, "same inputs, same projection (pure law)");
        for (name, digest) in &a.fields {
            assert!(digest.starts_with("sha256:"), "{name} cites its source");
        }
        assert_eq!(a.artifact_digest, "sha256:artifact");
        assert_eq!(a.consent_id, "consent-1");
    }
}

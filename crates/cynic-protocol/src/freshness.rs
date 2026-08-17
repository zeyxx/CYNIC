use serde::{Deserialize, Serialize};

/// Portable freshness policy for read-only evidence consumers.
///
/// Freshness is deliberately a policy input rather than a property inferred
/// from a producer name. Consumers choose the acceptable age for their risk
/// boundary and retain `Unknown` when the clock window cannot be proven.
pub const EVIDENCE_FRESHNESS_POLICY_SCHEMA: &str = "cynic.evidence.freshness_policy.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFreshnessPolicy {
    pub schema: String,
    pub max_age_ms: u128,
    pub allow_future_skew_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshnessStatus {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFreshnessAssessment {
    pub status: EvidenceFreshnessStatus,
    pub observed_at_unix_ms: u128,
    pub assessed_at_unix_ms: u128,
    pub age_ms: Option<u128>,
    pub residual_unknowns: Vec<String>,
}

pub fn assess_evidence_freshness(
    observed_at_unix_ms: u128,
    assessed_at_unix_ms: u128,
    policy: EvidenceFreshnessPolicy,
) -> EvidenceFreshnessAssessment {
    let mut residual_unknowns = Vec::new();
    if policy.schema != EVIDENCE_FRESHNESS_POLICY_SCHEMA {
        residual_unknowns.push("evidence_freshness_policy_schema_mismatch".to_string());
    }
    let age_ms = assessed_at_unix_ms.checked_sub(observed_at_unix_ms);
    let status = if !residual_unknowns.is_empty() {
        EvidenceFreshnessStatus::Unknown
    } else if observed_at_unix_ms == 0 {
        residual_unknowns.push("evidence_observed_at_missing".to_string());
        EvidenceFreshnessStatus::Unknown
    } else if assessed_at_unix_ms == 0 || policy.max_age_ms == 0 {
        residual_unknowns.push("evidence_freshness_clock_or_policy_invalid".to_string());
        EvidenceFreshnessStatus::Unknown
    } else if observed_at_unix_ms > assessed_at_unix_ms.saturating_add(policy.allow_future_skew_ms)
    {
        residual_unknowns.push("evidence_observed_at_in_future".to_string());
        EvidenceFreshnessStatus::Unknown
    } else if age_ms.is_some_and(|age| age <= policy.max_age_ms) {
        EvidenceFreshnessStatus::Fresh
    } else {
        EvidenceFreshnessStatus::Stale
    };

    EvidenceFreshnessAssessment {
        status,
        observed_at_unix_ms,
        assessed_at_unix_ms,
        age_ms,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> EvidenceFreshnessPolicy {
        EvidenceFreshnessPolicy {
            schema: EVIDENCE_FRESHNESS_POLICY_SCHEMA.to_string(),
            max_age_ms: 100,
            allow_future_skew_ms: 10,
        }
    }

    #[test]
    fn classifies_fresh_and_stale_evidence() {
        assert_eq!(
            assess_evidence_freshness(1_000, 1_050, policy()).status,
            EvidenceFreshnessStatus::Fresh
        );
        assert_eq!(
            assess_evidence_freshness(1_000, 1_101, policy()).status,
            EvidenceFreshnessStatus::Stale
        );
    }

    #[test]
    fn retains_unknown_for_invalid_clock_or_future_observation() {
        let missing = assess_evidence_freshness(0, 1_000, policy());
        assert_eq!(missing.status, EvidenceFreshnessStatus::Unknown);
        assert!(
            missing
                .residual_unknowns
                .contains(&"evidence_observed_at_missing".to_string())
        );

        let future = assess_evidence_freshness(1_011, 1_000, policy());
        assert_eq!(future.status, EvidenceFreshnessStatus::Unknown);
        assert!(
            future
                .residual_unknowns
                .contains(&"evidence_observed_at_in_future".to_string())
        );
    }
}

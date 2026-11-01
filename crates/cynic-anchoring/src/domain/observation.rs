//! Observation classification for personal anchoring.
//!
//! The universal `Observation` type lives in `cynic_protocol::observation_record`.
//! This module provides anchoring-specific classification helpers.

use cynic_protocol::observation_record::Observation;

/// The domains of personal anchoring (business-logic level, not storage level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorDomain {
    Sport,
    Meditation,
    Nutrition,
    Sobriety,
    ProjectMaterialized,
    GeneralReturn,
}

impl AnchorDomain {
    /// Map an observation tool string to an anchor domain.
    pub fn from_tool(tool: &str) -> Option<Self> {
        match tool {
            "sport" => Some(AnchorDomain::Sport),
            "meditation" => Some(AnchorDomain::Meditation),
            "nutrition" => Some(AnchorDomain::Nutrition),
            "sobriety" => Some(AnchorDomain::Sobriety),
            "project_materialized" => Some(AnchorDomain::ProjectMaterialized),
            "general_return" => Some(AnchorDomain::GeneralReturn),
            _ => None,
        }
    }

    pub fn as_tool(&self) -> &'static str {
        match self {
            AnchorDomain::Sport => "sport",
            AnchorDomain::Meditation => "meditation",
            AnchorDomain::Nutrition => "nutrition",
            AnchorDomain::Sobriety => "sobriety",
            AnchorDomain::ProjectMaterialized => "project_materialized",
            AnchorDomain::GeneralReturn => "general_return",
        }
    }
}

/// Check if an observation belongs to the personal-anchoring domain.
pub fn is_anchor_observation(obs: &Observation) -> bool {
    obs.domain == "personal-anchoring"
}

/// Extract the anchor domain from an observation, if applicable.
pub fn anchor_domain_of(obs: &Observation) -> Option<AnchorDomain> {
    if !is_anchor_observation(obs) {
        return None;
    }
    AnchorDomain::from_tool(&obs.tool)
}

/// Extract duration in ms from an observation's value field.
pub fn duration_ms_of(obs: &Observation) -> Option<i64> {
    obs.value
        .as_ref()
        .and_then(|v| v.get("duration_ms"))
        .and_then(|v| v.as_i64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::observation_record::Observation;

    fn make_obs(tool: &str, domain: &str) -> Observation {
        Observation {
            project: "CYNIC".to_string(),
            agent_id: "kyon-device-local".to_string(),
            tool: tool.to_string(),
            target: "test".to_string(),
            domain: domain.to_string(),
            status: "observed".to_string(),
            context: "{}".to_string(),
            session_id: "s1".to_string(),
            timestamp: 1000,
            tags: vec![],
            source_tier: String::new(),
            value: None,
            confidence: None,
            consumer: None,
            action: None,
            depends_on: vec![],
            maturity: None,
            hash: String::new(),
            prev_hash: String::new(),
            observers: vec![],
            consensus_score: None,
        }
    }

    #[test]
    fn classify_anchor_observation() {
        let obs = make_obs("sport", "personal-anchoring");
        assert!(is_anchor_observation(&obs));
        assert_eq!(anchor_domain_of(&obs), Some(AnchorDomain::Sport));
    }

    #[test]
    fn reject_non_anchor() {
        let obs = make_obs("sport", "construction-site");
        assert!(!is_anchor_observation(&obs));
        assert_eq!(anchor_domain_of(&obs), None);
    }

    #[test]
    fn extract_duration() {
        let mut obs = make_obs("sport", "personal-anchoring");
        obs.value = Some(serde_json::json!({"duration_ms": 1800000}));
        assert_eq!(duration_ms_of(&obs), Some(1800000));
    }
}
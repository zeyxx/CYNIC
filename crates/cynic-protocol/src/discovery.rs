use serde::{Deserialize, Serialize};

pub const TARGET_DISCOVERY_REPORT_SCHEMA: &str = "cynic.target_discovery.report.v1";
pub const DISCOVERY_PROMOTION_ADMISSION_SCHEMA: &str =
    "cynic.target_discovery.promotion_admission.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryState {
    Confirmed,
    Refuted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetCandidate {
    pub id: String,
    pub provider: String,
    pub resource_id: Option<String>,
    pub kind: Option<String>,
    pub declared: DiscoveryState,
    pub discovered: DiscoveryState,
    pub reachable: DiscoveryState,
    pub attestable: DiscoveryState,
    #[serde(default)]
    pub addresses: Vec<String>,
    #[serde(default)]
    pub transports: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetDiscoveryReport {
    pub schema: String,
    pub status: String,
    pub candidates: Vec<TargetCandidate>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryPromotionAdmission {
    pub schema: String,
    pub candidate_id: String,
    pub status: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn admit_candidate_declaration(candidate: &TargetCandidate) -> DiscoveryPromotionAdmission {
    let mut blockers = Vec::new();
    for (name, state) in [
        ("discovered", candidate.discovered),
        ("reachable", candidate.reachable),
        ("attestable", candidate.attestable),
    ] {
        match state {
            DiscoveryState::Confirmed => {}
            DiscoveryState::Refuted => blockers.push(format!("{name}_refuted")),
            DiscoveryState::Unknown => blockers.push(format!("{name}_unknown")),
        }
    }
    if candidate.provider.trim().is_empty() {
        blockers.push("provider_missing".to_string());
    }
    if candidate.resource_id.as_deref().is_none_or(str::is_empty) {
        blockers.push("resource_identity_missing".to_string());
    }
    if candidate.evidence_refs.is_empty() {
        blockers.push("discovery_evidence_missing".to_string());
    }
    DiscoveryPromotionAdmission {
        schema: DISCOVERY_PROMOTION_ADMISSION_SCHEMA.to_string(),
        candidate_id: candidate.id.clone(),
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        }
        .to_string(),
        blockers,
        evidence_refs: candidate.evidence_refs.clone(),
    }
}

impl TargetDiscoveryReport {
    pub fn from_candidates(candidates: Vec<TargetCandidate>) -> Self {
        let residual_unknowns = candidates
            .iter()
            .flat_map(|target| {
                [
                    ("declared", target.declared),
                    ("discovered", target.discovered),
                    ("reachable", target.reachable),
                    ("attestable", target.attestable),
                ]
                .into_iter()
                .filter(|(_, state)| *state == DiscoveryState::Unknown)
                .map(|(field, _)| format!("{}:{field}", target.id))
            })
            .collect::<Vec<_>>();
        Self {
            schema: TARGET_DISCOVERY_REPORT_SCHEMA.to_string(),
            status: if residual_unknowns.is_empty() {
                "complete".to_string()
            } else {
                "partial".to_string()
            },
            candidates,
            residual_unknowns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DiscoveryState, TargetCandidate, TargetDiscoveryReport, admit_candidate_declaration,
    };

    #[test]
    fn unknown_state_is_preserved_as_residual_risk() {
        let report = TargetDiscoveryReport::from_candidates(vec![TargetCandidate {
            id: "guest-1".to_string(),
            provider: "qemu-libvirt".to_string(),
            resource_id: Some("guest-1".to_string()),
            kind: Some("vm".to_string()),
            declared: DiscoveryState::Confirmed,
            discovered: DiscoveryState::Confirmed,
            reachable: DiscoveryState::Unknown,
            attestable: DiscoveryState::Unknown,
            addresses: Vec::new(),
            transports: vec!["qemu-guest-agent".to_string()],
            evidence_refs: Vec::new(),
        }]);
        assert_eq!(report.status, "partial");
        assert!(
            report
                .residual_unknowns
                .contains(&"guest-1:reachable".to_string())
        );
    }

    #[test]
    fn discovery_does_not_self_promote_without_attestation_and_evidence() {
        let candidate = TargetCandidate {
            id: "forgejo".to_string(),
            provider: "proxmox".to_string(),
            resource_id: Some("lxc/101".to_string()),
            kind: Some("lxc".to_string()),
            declared: DiscoveryState::Unknown,
            discovered: DiscoveryState::Confirmed,
            reachable: DiscoveryState::Confirmed,
            attestable: DiscoveryState::Unknown,
            addresses: vec!["10.10.10.101".to_string()],
            transports: vec!["proxmox-api".to_string()],
            evidence_refs: Vec::new(),
        };

        let admission = admit_candidate_declaration(&candidate);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"attestable_unknown".to_string())
        );
        assert!(
            admission
                .blockers
                .contains(&"discovery_evidence_missing".to_string())
        );
    }
}

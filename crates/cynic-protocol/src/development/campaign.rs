//! Replayable development campaign planning, independent of agent execution.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AgentEvidenceRef, validate_evidence_refs};

pub const DEVELOPMENT_CAMPAIGN_SCHEMA: &str = "cynic.development_campaign.v1";
pub const DEVELOPMENT_CAMPAIGN_EVENT_SCHEMA: &str = "cynic.development_campaign_event.v1";
pub const DEVELOPMENT_CAMPAIGN_PROJECTION_SCHEMA: &str = "cynic.development_campaign_projection.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentCampaign {
    pub schema: String,
    pub campaign_id: String,
    pub objective: String,
    pub milestones: Vec<DevelopmentCampaignMilestone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentCampaignMilestone {
    pub id: String,
    pub owner: String,
    pub objective: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub acceptance_evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentCampaignEventKind {
    Observed,
    DecisionRecorded,
    ChangeApplied,
    Validated,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentCampaignEvent {
    pub schema: String,
    pub campaign_id: String,
    pub milestone_id: String,
    pub kind: DevelopmentCampaignEventKind,
    pub recorded_at_ms: u64,
    #[serde(default)]
    pub evidence_refs: Vec<AgentEvidenceRef>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentMilestoneStatus {
    Planned,
    Ready,
    Blocked,
    Proven,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentMilestoneProjection {
    pub id: String,
    pub owner: String,
    pub status: DevelopmentMilestoneStatus,
    pub missing_dependencies: Vec<String>,
    pub missing_acceptance_evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<AgentEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentCampaignProjection {
    pub schema: String,
    pub campaign_id: String,
    pub objective: String,
    pub milestones: Vec<DevelopmentMilestoneProjection>,
    pub unknowns: Vec<String>,
}

/// Reconciles declared campaign intent with append-only observed events. It
/// never schedules work, writes state, or treats a chat plan as evidence.
pub fn reconcile_development_campaign(
    campaign: &DevelopmentCampaign,
    events: &[DevelopmentCampaignEvent],
) -> DevelopmentCampaignProjection {
    let mut unknowns = validate_campaign(campaign);
    let declared_milestones = campaign
        .milestones
        .iter()
        .map(|milestone| milestone.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let invalid_dependency_milestones = campaign
        .milestones
        .iter()
        .enumerate()
        .filter_map(|(index, milestone)| {
            let declared_before = campaign.milestones[..index]
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let invalid = milestone.depends_on.iter().any(|dependency| {
                dependency == &milestone.id
                    || !declared_milestones.contains(dependency.as_str())
                    || !declared_before.contains(dependency.as_str())
            });
            invalid.then(|| milestone.id.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>();
    if !invalid_dependency_milestones.is_empty() {
        unknowns.push("development_campaign_dependency_invalid".to_string());
    }
    let valid_events = events
        .iter()
        .filter(|event| {
            let valid_shape = event.schema == DEVELOPMENT_CAMPAIGN_EVENT_SCHEMA
                && event.campaign_id == campaign.campaign_id
                && event.recorded_at_ms > 0
                && validate_evidence_refs(&event.evidence_refs).is_empty();
            if !valid_shape {
                unknowns.push("development_campaign_event_invalid".to_string());
            }
            if !declared_milestones.contains(event.milestone_id.as_str()) {
                unknowns.push("development_campaign_event_milestone_unknown".to_string());
            }
            valid_shape && declared_milestones.contains(event.milestone_id.as_str())
        })
        .collect::<Vec<_>>();
    let mut projections: BTreeMap<String, DevelopmentMilestoneProjection> = BTreeMap::new();
    for milestone in &campaign.milestones {
        let milestone_events = valid_events
            .iter()
            .filter(|event| event.milestone_id == milestone.id)
            .copied()
            .collect::<Vec<_>>();
        let dependencies = milestone
            .depends_on
            .iter()
            .filter(|id| {
                !matches!(
                    projections.get(*id).map(|item| item.status),
                    Some(DevelopmentMilestoneStatus::Proven)
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let blockers = milestone_events
            .iter()
            .filter(|event| event.kind == DevelopmentCampaignEventKind::Blocked)
            .flat_map(|event| event.blockers.iter().cloned())
            .collect::<Vec<_>>();
        let evidence_refs = milestone_events
            .iter()
            .flat_map(|event| event.evidence_refs.iter().cloned())
            .collect::<Vec<_>>();
        let evidence_subjects = evidence_refs
            .iter()
            .map(|evidence| evidence.subject.as_str())
            .collect::<Vec<_>>();
        let missing_acceptance_evidence = milestone
            .acceptance_evidence
            .iter()
            .filter(|required| !evidence_subjects.contains(&required.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let validated = milestone_events
            .iter()
            .any(|event| event.kind == DevelopmentCampaignEventKind::Validated);
        let status = if invalid_dependency_milestones.contains(milestone.id.as_str()) {
            DevelopmentMilestoneStatus::Unknown
        } else if !blockers.is_empty() {
            DevelopmentMilestoneStatus::Blocked
        } else if !dependencies.is_empty() {
            DevelopmentMilestoneStatus::Planned
        } else if validated && missing_acceptance_evidence.is_empty() {
            DevelopmentMilestoneStatus::Proven
        } else if milestone_events.is_empty() {
            DevelopmentMilestoneStatus::Ready
        } else {
            DevelopmentMilestoneStatus::Unknown
        };
        projections.insert(
            milestone.id.clone(),
            DevelopmentMilestoneProjection {
                id: milestone.id.clone(),
                owner: milestone.owner.clone(),
                status,
                missing_dependencies: dependencies,
                missing_acceptance_evidence,
                blockers,
                evidence_refs,
            },
        );
    }
    unknowns.sort();
    unknowns.dedup();
    DevelopmentCampaignProjection {
        schema: DEVELOPMENT_CAMPAIGN_PROJECTION_SCHEMA.to_string(),
        campaign_id: campaign.campaign_id.clone(),
        objective: campaign.objective.clone(),
        milestones: campaign
            .milestones
            .iter()
            .filter_map(|milestone| projections.remove(&milestone.id))
            .collect(),
        unknowns,
    }
}

fn validate_campaign(campaign: &DevelopmentCampaign) -> Vec<String> {
    let mut unknowns = Vec::new();
    if campaign.schema != DEVELOPMENT_CAMPAIGN_SCHEMA
        || campaign.campaign_id.trim().is_empty()
        || campaign.objective.trim().is_empty()
    {
        unknowns.push("development_campaign_identity_invalid".to_string());
    }
    let mut ids = std::collections::BTreeSet::new();
    for milestone in &campaign.milestones {
        if milestone.id.trim().is_empty()
            || milestone.owner.trim().is_empty()
            || milestone.objective.trim().is_empty()
            || milestone.acceptance_evidence.is_empty()
            || !ids.insert(milestone.id.as_str())
        {
            unknowns.push("development_campaign_milestone_invalid".to_string());
        }
    }
    unknowns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campaign() -> DevelopmentCampaign {
        DevelopmentCampaign {
            schema: DEVELOPMENT_CAMPAIGN_SCHEMA.to_string(),
            campaign_id: "foundation".to_string(),
            objective: "prove one bounded development vertical".to_string(),
            milestones: vec![
                DevelopmentCampaignMilestone {
                    id: "contract".to_string(),
                    owner: "cynic-protocol".to_string(),
                    objective: "define contract".to_string(),
                    depends_on: Vec::new(),
                    acceptance_evidence: vec!["contract_test".to_string()],
                },
                DevelopmentCampaignMilestone {
                    id: "adapter".to_string(),
                    owner: "cynic-node".to_string(),
                    objective: "bind adapter".to_string(),
                    depends_on: vec!["contract".to_string()],
                    acceptance_evidence: vec!["adapter_proof".to_string()],
                },
            ],
        }
    }

    #[test]
    fn dependencies_stay_planned_until_their_acceptance_evidence_is_validated() {
        let projection = reconcile_development_campaign(&campaign(), &[]);
        assert_eq!(
            projection.milestones[0].status,
            DevelopmentMilestoneStatus::Ready
        );
        assert_eq!(
            projection.milestones[1].status,
            DevelopmentMilestoneStatus::Planned
        );
    }
}

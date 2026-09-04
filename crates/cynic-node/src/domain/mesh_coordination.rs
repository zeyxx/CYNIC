//! Mesh coordination domain — reconciliation between declared registry peers and provider members.

use serde::{Deserialize, Serialize};

use crate::ports::mesh_coordination_port::MeshMember;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshCoordinationReport {
    pub schema: &'static str,
    pub provider: String,
    pub desired_peers: usize,
    pub observed_members: usize,
    pub active_members: Vec<String>,
    pub missing_members: Vec<String>,
    pub unmanaged_members: Vec<String>,
}

impl MeshCoordinationReport {
    pub const SCHEMA: &'static str = "cynic.mesh_coordination_report.v1";
}

/// Pure decision: reconcile declared node names against observed provider members.
pub fn reconcile_mesh_members(
    provider: &str,
    desired_node_names: &[String],
    observed_members: &[MeshMember],
) -> MeshCoordinationReport {
    let mut active_members = Vec::new();
    let mut missing_members = Vec::new();
    let mut unmanaged_members = Vec::new();

    let observed_map: std::collections::BTreeMap<_, _> = observed_members
        .iter()
        .map(|m| (m.name.as_str(), m))
        .collect();

    for desired in desired_node_names {
        if observed_map.contains_key(desired.as_str()) {
            active_members.push(desired.clone());
        } else {
            missing_members.push(desired.clone());
        }
    }

    let desired_set: std::collections::BTreeSet<_> =
        desired_node_names.iter().map(String::as_str).collect();

    for member in observed_members {
        if !desired_set.contains(member.name.as_str()) {
            unmanaged_members.push(member.name.clone());
        }
    }

    active_members.sort();
    missing_members.sort();
    unmanaged_members.sort();

    MeshCoordinationReport {
        schema: MeshCoordinationReport::SCHEMA,
        provider: provider.to_string(),
        desired_peers: desired_node_names.len(),
        observed_members: observed_members.len(),
        active_members,
        missing_members,
        unmanaged_members,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciles_matching_missing_and_unmanaged() {
        let desired = vec![
            "builder".to_string(),
            "cortex".to_string(),
            "forgejo".to_string(),
        ];
        let observed = vec![
            MeshMember {
                member_id: "1".to_string(),
                name: "builder".to_string(),
                addresses: vec!["100.64.0.1".to_string()],
                provider_reports_online: Some(true),
            },
            MeshMember {
                member_id: "2".to_string(),
                name: "ephemeral-vm-42".to_string(),
                addresses: vec!["100.64.0.99".to_string()],
                provider_reports_online: Some(false),
            },
        ];

        let report = reconcile_mesh_members("headscale", &desired, &observed);
        assert_eq!(report.schema, MeshCoordinationReport::SCHEMA);
        assert_eq!(report.provider, "headscale");
        assert_eq!(report.desired_peers, 3);
        assert_eq!(report.observed_members, 2);
        assert_eq!(report.active_members, vec!["builder"]);
        assert_eq!(report.missing_members, vec!["cortex", "forgejo"]);
        assert_eq!(report.unmanaged_members, vec!["ephemeral-vm-42"]);
    }
}

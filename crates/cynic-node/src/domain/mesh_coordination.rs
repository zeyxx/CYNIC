//! Mesh coordination domain — reconciliation between declared registry peers and provider members.

use serde::{Deserialize, Serialize};

use crate::ports::mesh_coordination_port::MeshMember;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredMeshPeer {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl DesiredMeshPeer {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            aliases: Vec::new(),
        }
    }

    pub fn with_aliases(id: impl Into<String>, name: Option<String>, aliases: Vec<String>) -> Self {
        Self {
            id: id.into(),
            name,
            aliases,
        }
    }

    /// Check if an observed member name matches this peer's id, name, or aliases.
    pub fn matches_name(&self, candidate: &str) -> bool {
        let norm = candidate.trim().to_lowercase();
        if self.id.to_lowercase() == norm {
            return true;
        }
        if let Some(ref name) = self.name {
            if name.to_lowercase() == norm {
                return true;
            }
        }
        self.aliases.iter().any(|a| a.to_lowercase() == norm)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfMeshStatus {
    pub node_id: String,
    pub enrolled: bool,
    pub provider_online: Option<bool>,
    pub assigned_ips: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshCoordinationReport {
    pub schema: &'static str,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_node: Option<SelfMeshStatus>,
    pub desired_peers: usize,
    pub observed_members: usize,
    pub active_members: Vec<String>,
    pub missing_members: Vec<String>,
    pub unmanaged_members: Vec<String>,
}

impl MeshCoordinationReport {
    pub const SCHEMA: &'static str = "cynic.mesh_coordination_report.v1";
}

/// Reconcile declared peers with alias matching and self-node detection.
pub fn reconcile_mesh_coordination(
    provider: &str,
    desired_peers: &[DesiredMeshPeer],
    observed_members: &[MeshMember],
    self_node_id: Option<&str>,
) -> MeshCoordinationReport {
    let mut active_members = Vec::new();
    let mut missing_members = Vec::new();
    let mut matched_member_indices = std::collections::BTreeSet::new();

    for peer in desired_peers {
        let matched = observed_members
            .iter()
            .enumerate()
            .find(|(_, m)| peer.matches_name(&m.name));

        if let Some((idx, _)) = matched {
            active_members.push(peer.id.clone());
            matched_member_indices.insert(idx);
        } else {
            missing_members.push(peer.id.clone());
        }
    }

    let mut unmanaged_members = Vec::new();
    for (idx, member) in observed_members.iter().enumerate() {
        if !matched_member_indices.contains(&idx) {
            unmanaged_members.push(member.name.clone());
        }
    }

    active_members.sort();
    missing_members.sort();
    unmanaged_members.sort();

    let self_node = self_node_id.map(|self_id| {
        let found = observed_members.iter().find(|m| {
            m.name.eq_ignore_ascii_case(self_id)
                || desired_peers
                    .iter()
                    .find(|p| p.matches_name(self_id))
                    .map(|p| p.matches_name(&m.name))
                    .unwrap_or(false)
        });

        match found {
            Some(member) => SelfMeshStatus {
                node_id: self_id.to_string(),
                enrolled: true,
                provider_online: member.provider_reports_online,
                assigned_ips: member.addresses.clone(),
            },
            None => SelfMeshStatus {
                node_id: self_id.to_string(),
                enrolled: false,
                provider_online: None,
                assigned_ips: Vec::new(),
            },
        }
    });

    MeshCoordinationReport {
        schema: MeshCoordinationReport::SCHEMA,
        provider: provider.to_string(),
        self_node,
        desired_peers: desired_peers.len(),
        observed_members: observed_members.len(),
        active_members,
        missing_members,
        unmanaged_members,
    }
}

/// Convenience facade for simple string-based desired nodes.
pub fn reconcile_mesh_members(
    provider: &str,
    desired_node_names: &[String],
    observed_members: &[MeshMember],
) -> MeshCoordinationReport {
    let desired_peers: Vec<DesiredMeshPeer> = desired_node_names
        .iter()
        .map(DesiredMeshPeer::new)
        .collect();
    reconcile_mesh_coordination(provider, &desired_peers, observed_members, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_peers_by_id_or_alias() {
        let desired = vec![
            DesiredMeshPeer::with_aliases(
                "cynic-builder",
                Some("cynic-builder".to_string()),
                vec!["builder".to_string(), "build-runner".to_string()],
            ),
            DesiredMeshPeer::new("cynic-host-cortex"),
        ];

        // Member registered with short alias "builder"
        let observed = vec![MeshMember {
            member_id: "1".to_string(),
            name: "builder".to_string(),
            addresses: vec!["100.64.0.5".to_string()],
            provider_reports_online: Some(true),
        }];

        let report =
            reconcile_mesh_coordination("headscale", &desired, &observed, Some("cynic-builder"));

        assert_eq!(report.desired_peers, 2);
        assert_eq!(report.observed_members, 1);
        assert_eq!(report.active_members, vec!["cynic-builder"]);
        assert_eq!(report.missing_members, vec!["cynic-host-cortex"]);
        assert!(report.unmanaged_members.is_empty());

        let self_status = report.self_node.expect("self_node present");
        assert_eq!(self_status.node_id, "cynic-builder");
        assert!(self_status.enrolled);
        assert_eq!(self_status.provider_online, Some(true));
        assert_eq!(self_status.assigned_ips, vec!["100.64.0.5"]);
    }

    #[test]
    fn reports_self_not_enrolled_when_absent() {
        let desired = vec![DesiredMeshPeer::new("cynic-builder")];
        let observed = vec![];

        let report =
            reconcile_mesh_coordination("headscale", &desired, &observed, Some("cynic-builder"));

        let self_status = report.self_node.expect("self_node present");
        assert_eq!(self_status.node_id, "cynic-builder");
        assert!(!self_status.enrolled);
        assert!(self_status.assigned_ips.is_empty());
    }
}

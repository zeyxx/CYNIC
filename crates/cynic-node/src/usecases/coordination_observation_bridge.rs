//! Observation bridge: federated mesh liveness -> coordination authority
//! observation (design D).
//!
//! Consumes the existing `FederatedMeshProbeReport` (TCP/WireGuard reachability
//! measured by `federated_mesh_probe_uc`) and projects it into the
//! `CoordinationAuthorityObservation` contract that
//! `coordination_posture_uc` reconciles. Nothing here talks to a kernel.
//!
//! Honesty boundary (explicit, load-bearing):
//! - `active` / `healthy` are MEASURED liveness from the mesh probe;
//! - `term` is reported as the locally known desired term — absence of
//!   evidence of a higher term is not evidence of one (reconcile only blocks
//!   on terms strictly AHEAD of desired);
//! - fencing tokens carry the LOCAL high-water mark from the node's own lease
//!   store. A future gossip envelope may raise this fidelity without changing
//!   this contract; until then the bound is declared, not hidden.

use std::collections::BTreeMap;
use std::path::Path;

use cynic_protocol::FederatedMeshProbeReport;
use cynic_protocol::coordination_authority::{
    COORDINATION_AUTHORITY_OBSERVATION_SCHEMA, CoordinationAuthorityMemberObservation,
    CoordinationAuthorityObservation,
};

/// Project a mesh probe report onto the authority observation contract.
///
/// Every desired member yields exactly one member entry; a declared member
/// missing from the probe report is recorded as present-but-inactive (it must
/// count against quorum loss, not silently disappear).
pub fn mesh_to_authority_observation(
    authority_id: &str,
    desired_member_ids: &[String],
    desired_term: u64,
    local_fence_high_water: u64,
    mesh: &FederatedMeshProbeReport,
) -> CoordinationAuthorityObservation {
    let statuses: BTreeMap<&str, &cynic_protocol::FederatedMeshNodeStatus> = mesh
        .node_statuses
        .iter()
        .map(|status| (status.node_id.as_str(), status))
        .collect();

    let members = desired_member_ids
        .iter()
        .map(|member_id| match statuses.get(member_id.as_str()) {
            Some(status) => {
                // Reachable overlay = active. Healthy additionally requires at
                // least one probed service answering (a forced-reachable
                // loopback entry with every port down is present but degraded).
                let active = status.wireguard_reachable;
                let healthy = active && !status.active_services.is_empty();
                CoordinationAuthorityMemberObservation {
                    node_id: member_id.clone(),
                    active,
                    healthy,
                    term: desired_term,
                    fencing_token: local_fence_high_water,
                    last_committed_fencing_token: local_fence_high_water,
                }
            }
            None => CoordinationAuthorityMemberObservation {
                node_id: member_id.clone(),
                active: false,
                healthy: false,
                term: desired_term,
                fencing_token: 0,
                last_committed_fencing_token: 0,
            },
        })
        .collect();

    CoordinationAuthorityObservation {
        schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
        authority_id: authority_id.to_string(),
        members,
    }
}

/// High-water mark over fencing tokens found in the node-local lease store.
/// Pure fold so the rule is testable without a filesystem.
pub fn fence_high_water_from_tokens<'a>(tokens: impl Iterator<Item = &'a str>) -> u64 {
    tokens
        .filter_map(|raw| raw.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

/// Scan a lease-store directory (`<root>/<resource>.json` grant files) and
/// return the highest fencing token ever issued locally.
pub fn local_fence_high_water(lease_root: &Path) -> Result<u64, String> {
    let mut best: u64 = 0;
    let entries =
        std::fs::read_dir(lease_root).map_err(|e| format!("lease_root_unreadable:{e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("lease_file_unreadable:{e}"))?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("lease_file_decode_failed:{e}"))?;
        let token = value
            .pointer("/guard/fencing_token")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        best = best.max(token);
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{FEDERATED_MESH_PROBE_SCHEMA, FederatedMeshNodeStatus};

    fn mesh_status(node_id: &str, reachable: bool, services: &[&str]) -> FederatedMeshNodeStatus {
        FederatedMeshNodeStatus {
            node_id: node_id.into(),
            role: "peer".into(),
            synapse_ip: "10.10.10.x".into(),
            wireguard_reachable: reachable,
            active_services: services.iter().map(|s| s.to_string()).collect(),
            rtt_ms: None,
        }
    }

    fn mesh_report(statuses: Vec<FederatedMeshNodeStatus>) -> FederatedMeshProbeReport {
        FederatedMeshProbeReport {
            schema: FEDERATED_MESH_PROBE_SCHEMA,
            status: "observed",
            local_node_id: "n1".into(),
            total_declared_nodes: statuses.len(),
            reachable_nodes_count: statuses.iter().filter(|s| s.wireguard_reachable).count(),
            node_statuses: statuses,
        }
    }

    fn member_ids() -> Vec<String> {
        vec!["n1".into(), "n2".into(), "n3".into()]
    }

    #[test]
    fn all_peers_reachable_maps_active_and_healthy() {
        let report = mesh_report(vec![
            mesh_status("n1", true, &["port_8000_tcp"]),
            mesh_status("n2", true, &["port_22_tcp"]),
            mesh_status("n3", true, &["port_3000_tcp"]),
        ]);
        let observed = mesh_to_authority_observation("coord-main", &member_ids(), 3, 8, &report);
        assert_eq!(observed.schema, COORDINATION_AUTHORITY_OBSERVATION_SCHEMA);
        assert_eq!(observed.members.len(), 3);
        assert!(observed.members.iter().all(|m| m.active && m.healthy));
        assert!(
            observed
                .members
                .iter()
                .all(|m| m.last_committed_fencing_token == 8)
        );
    }

    #[test]
    fn dead_peer_counts_against_quorum_instead_of_disappearing() {
        let report = mesh_report(vec![
            mesh_status("n1", true, &["port_8000_tcp"]),
            mesh_status("n2", false, &[]),
        ]);
        let observed = mesh_to_authority_observation("coord-main", &member_ids(), 3, 8, &report);
        let n3 = observed
            .members
            .iter()
            .find(|m| m.node_id == "n3")
            .expect("missing declared member must still be projected");
        assert!(!n3.active);
        assert!(!n3.healthy);
        let n2 = observed.members.iter().find(|m| m.node_id == "n2").unwrap();
        assert!(!n2.active && !n2.healthy);
    }

    #[test]
    fn loopback_present_but_portless_is_active_not_healthy() {
        // Mesh probe forces 127.0.0.1 entries reachable even with zero open
        // ports; the bridge must classify that as present-but-degraded.
        let report = mesh_report(vec![mesh_status("n1", true, &[])]);
        let observed = mesh_to_authority_observation("coord-main", &member_ids(), 3, 0, &report);
        let n1 = observed.members.iter().find(|m| m.node_id == "n1").unwrap();
        assert!(n1.active);
        assert!(!n1.healthy);
    }

    #[test]
    fn fence_high_water_takes_max_and_tolerates_garbage() {
        assert_eq!(
            fence_high_water_from_tokens(["4", "17", "9"].iter().copied()),
            17
        );
        assert_eq!(fence_high_water_from_tokens(std::iter::empty()), 0);
        assert_eq!(fence_high_water_from_tokens(["abc", ""].iter().copied()), 0);
    }

    #[test]
    fn local_fence_high_water_reads_real_lease_store() {
        let dir = std::env::temp_dir().join(format!("cynic-fence-hw-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let grant = serde_json::json!({
            "schema": "cynic.storage_lease.v1",
            "guard": {
                "agent_id": "agent-a",
                "target": "storage:res",
                "lease_id": "l1",
                "fencing_token": 42u64
            },
            "expires_at_ms": 999u64
        });
        std::fs::write(dir.join("res.json"), serde_json::to_vec(&grant).unwrap()).unwrap();
        std::fs::write(dir.join("notes.txt"), b"ignore me").unwrap();

        let hw = local_fence_high_water(&dir).expect("scan must succeed");
        assert_eq!(hw, 42);

        let missing = local_fence_high_water(&dir.join("does-not-exist"));
        assert!(missing.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// End-to-end SPOF scenario: every peer dead + empty lease history ->
    /// observation that drives `decide_coordination_posture` to explicit
    /// local autonomy (the kernel-optional guarantee, proven through the
    /// real bridge).
    #[test]
    fn total_mesh_loss_yields_local_autonomy_through_the_full_pipeline() {
        use crate::usecases::coordination_posture_uc::decide_coordination_posture;
        use cynic_protocol::coordination_authority::{
            COORDINATION_AUTHORITY_DESIRED_SCHEMA, CoordinationAuthorityDesiredState,
        };

        let report = mesh_report(vec![mesh_status("n1", false, &[])]);
        let observed = mesh_to_authority_observation("coord-main", &member_ids(), 3, 0, &report);
        let desired = CoordinationAuthorityDesiredState {
            schema: COORDINATION_AUTHORITY_DESIRED_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            member_ids: member_ids(),
            required_member_count: 2,
            term: 3,
            next_fencing_token: 1,
        };
        let posture = decide_coordination_posture(&desired, &observed);
        assert_eq!(posture.state, "local_autonomous");
        assert_eq!(posture.blockers, vec!["healthy_quorum_not_observed"]);
    }
}

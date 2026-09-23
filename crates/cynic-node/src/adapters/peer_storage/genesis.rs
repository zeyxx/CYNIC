//! Genesis bootstrap seeding the CynicPeer SMR from the live infrastructure registry.

use serde_json::Value;
use std::fs;
use std::path::Path;

use cynic_consensus::peer::{
    CitizenAccount, LedgerTransaction, LedgerTransactionPayload, NODE_ACCOUNT_SCHEMA, NodeAccount,
    NodeMembershipStatus, OFFICE_ACCOUNT_SCHEMA, OfficeAccount,
};

use super::manager::{PeerStorageError, PeerStorageManager};

/// Bootstrap the peer's genesis block from `registry.json` if the WAL is empty.
pub fn bootstrap_genesis_from_registry(
    storage: &mut PeerStorageManager,
    registry_path: impl AsRef<Path>,
) -> Result<usize, PeerStorageError> {
    if storage.state().last_sequence > 0 {
        // Already initialized, no-op
        return Ok(0);
    }

    let registry_path = registry_path.as_ref();
    let raw = fs::read_to_string(registry_path)
        .map_err(|e| PeerStorageError::Wal(format!("failed to read registry for genesis: {e}")))?;
    let val: Value = serde_json::from_str(&raw)
        .map_err(|e| PeerStorageError::Wal(format!("failed to parse registry json: {e}")))?;

    let mut tx_count = 0;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1724000000);

    // 1. Tx 1: Sovereign Human Admin Citizen
    let admin = CitizenAccount::new_admin(
        "cit_admin",
        "Sovereign Operator",
        "sovereign_operator_key",
        now,
    );
    let tx1 = LedgerTransaction::new(
        "genesis_tx_01",
        1,
        now,
        "cit_admin",
        0,
        LedgerTransactionPayload::CitizenRegistered { citizen: admin },
    );
    storage.commit(tx1)?;
    tx_count += 1;

    // 2. Standard Institutional Offices
    let standard_offices = vec![
        (
            "builder",
            "Build Runner",
            vec!["cargo_check", "cargo_build", "artifact_install"],
        ),
        (
            "auditor",
            "Architecture and Security Auditor",
            vec!["read_only_audit", "metrics_evaluation"],
        ),
        (
            "scheduler",
            "Workload and Task Scheduler",
            vec!["workload_dispatch", "lease_coordination"],
        ),
        (
            "coordinator",
            "Mesh and Network Coordinator",
            vec!["preauthkey_generation", "node_revocation"],
        ),
    ];

    for (office_id, title, caps) in standard_offices {
        let seq = storage.state().last_sequence + 1;
        let office = OfficeAccount {
            schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
            office_id: office_id.to_string(),
            title: title.to_string(),
            allowed_capabilities: caps.into_iter().map(String::from).collect(),
            forbidden_actions: vec!["unbounded_mutation".to_string()],
        };
        let tx = LedgerTransaction::new(
            format!("genesis_office_{office_id}"),
            seq,
            now,
            "cit_admin",
            seq,
            LedgerTransactionPayload::OfficeDefined { office },
        );
        storage.commit(tx)?;
        tx_count += 1;
    }

    // 3. Bind Nodes from registry.json
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            let Some(node_id) = node
                .get("id")
                .or_else(|| node.get("name"))
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let hostname = node.get("name").and_then(|v| v.as_str()).unwrap_or(node_id);
            let host_family = node
                .get("host_family")
                .and_then(|v| v.as_str())
                .unwrap_or("linux.debian.systemd");
            let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("node");

            let seq = storage.state().last_sequence + 1;
            let node_account = NodeAccount {
                schema: NODE_ACCOUNT_SCHEMA.to_string(),
                node_id: node_id.to_string(),
                owner_citizen_id: "cit_admin".to_string(),
                hostname: hostname.to_string(),
                host_family: host_family.to_string(),
                capabilities: vec![role.to_string()],
                status: NodeMembershipStatus::Enrolled,
                last_liveness_proof_unix_s: Some(now),
            };

            let tx = LedgerTransaction::new(
                format!("genesis_node_{node_id}"),
                seq,
                now,
                "cit_admin",
                seq,
                LedgerTransactionPayload::NodeBound { node: node_account },
            );
            storage.commit(tx)?;
            tx_count += 1;
        }
    }

    Ok(tx_count)
}

//! Peer local storage adapter — Double-layer WAL (SSOT) + SQLite (State Projection).
//!
//! Invariants:
//! 1. Every transaction is appended to `cynic-peer.wal` and fsync'd first.
//! 2. In-memory `PeerState` is transitioned via `apply_transaction`.
//! 3. Current account state is projected into `cynic-peer.db` (SQLite).
//! 4. Complete crash-recovery: `rebuild_projection_from_wal` restores exact state.

pub mod manager;
pub mod projection;
pub mod wal_file;

pub use manager::{PeerStorageError, PeerStorageManager};
pub use projection::PeerSqliteProjection;
pub use wal_file::PeerWalFile;

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::peer::{
        CITIZEN_ACCOUNT_SCHEMA, CitizenAccount, LedgerTransaction, LedgerTransactionPayload,
        MANDATE_ACCOUNT_SCHEMA, MandateAccount, MandateStatus, NODE_ACCOUNT_SCHEMA, NodeAccount,
        NodeMembershipStatus, OFFICE_ACCOUNT_SCHEMA, OfficeAccount,
    };

    #[test]
    fn double_layer_storage_commit_and_crash_recovery() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut storage = PeerStorageManager::open(dir.path()).expect("open storage");

        let now = 1000;

        // 1. Commit CitizenRegistered
        let admin = CitizenAccount::new_admin("cit_admin", "Human Operator", "pubkey_hex", now);
        let tx1 = LedgerTransaction::new(
            "tx_01",
            1,
            now,
            "cit_admin",
            0,
            LedgerTransactionPayload::CitizenRegistered {
                citizen: admin.clone(),
            },
        );
        storage.commit(tx1).expect("commit tx1");

        // Verify SQLite index
        let citizen = storage
            .projection()
            .get_citizen("cit_admin")
            .expect("query citizen")
            .expect("citizen found");
        assert_eq!(citizen.citizen_id, "cit_admin");
        assert_eq!(citizen.display_name, "Human Operator");

        // 2. Commit NodeBound
        let node = NodeAccount {
            schema: NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: "node_beta".to_string(),
            owner_citizen_id: "cit_admin".to_string(),
            hostname: "beta".to_string(),
            host_family: "linux.debian.systemd".to_string(),
            capabilities: vec!["builder".to_string()],
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: Some(now),
        };
        let tx2 = LedgerTransaction::new(
            "tx_02",
            2,
            now + 1,
            "cit_admin",
            1,
            LedgerTransactionPayload::NodeBound { node: node.clone() },
        );
        storage.commit(tx2).expect("commit tx2");

        let fetched_node = storage
            .projection()
            .get_node("node_beta")
            .expect("query node")
            .expect("node found");
        assert_eq!(fetched_node.hostname, "beta");

        // 3. Commit Office and Mandate
        let office = OfficeAccount {
            schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
            office_id: "auditor".to_string(),
            title: "Auditor".to_string(),
            allowed_capabilities: vec!["audit".to_string()],
            forbidden_actions: vec!["write".to_string()],
        };
        let tx3 = LedgerTransaction::new(
            "tx_03",
            3,
            now + 2,
            "cit_admin",
            2,
            LedgerTransactionPayload::OfficeDefined { office },
        );
        storage.commit(tx3).expect("commit tx3");

        let mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
            mandate_id: "mnd_audit".to_string(),
            agent_id: "agt_01".to_string(),
            office_id: "auditor".to_string(),
            scope: "all".to_string(),
            fencing_token: 1,
            started_at_unix_s: now,
            expires_at_unix_s: now + 3600,
            max_actions: 50,
            consumed_actions: 0,
            max_wall_seconds: 3600,
            consumed_wall_seconds: 0,
            status: MandateStatus::Active,
        };
        let tx4 = LedgerTransaction::new(
            "tx_04",
            4,
            now + 3,
            "cit_admin",
            3,
            LedgerTransactionPayload::MandateGranted { mandate },
        );
        storage.commit(tx4).expect("commit tx4");

        let pre_crash_state = storage.state().clone();
        assert_eq!(pre_crash_state.last_sequence, 4);
        assert_eq!(pre_crash_state.citizens.len(), 1);
        assert_eq!(pre_crash_state.nodes.len(), 1);
        assert_eq!(pre_crash_state.mandates.len(), 1);

        // --- SIMULATE CATASTROPHIC SQLITE CORRUPTION / DESTRUCTION ---
        let sqlite_file = dir.path().join("cynic-peer.db");
        assert!(sqlite_file.exists());
        std::fs::remove_file(&sqlite_file).expect("destroy sqlite database file");
        assert!(!sqlite_file.exists());

        // --- CRASH RECOVERY FROM WAL ---
        storage
            .recover_projection_from_wal()
            .expect("recover from wal");

        // Verify SQLite database was recreated and fully repopulated
        assert!(sqlite_file.exists());
        let recovered_citizen = storage
            .projection()
            .get_citizen("cit_admin")
            .expect("query citizen after recovery")
            .expect("citizen exists after recovery");
        assert_eq!(recovered_citizen.citizen_id, "cit_admin");

        let recovered_mandate = storage
            .projection()
            .get_mandate("mnd_audit")
            .expect("query mandate after recovery")
            .expect("mandate exists after recovery");
        assert_eq!(recovered_mandate.mandate_id, "mnd_audit");

        // State parity check
        assert_eq!(storage.state(), &pre_crash_state);
    }
}

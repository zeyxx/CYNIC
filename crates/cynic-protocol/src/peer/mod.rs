//! Canonical CynicPeer constitutional state and ledger models.
//!
//! Subordinate to `contracts/foundation.json`, `contracts/framework.json` and
//! `protocols/cynic-stage2-peer-and-ledger.md`.

pub mod accounts;
pub mod transactions;

pub use accounts::*;
pub use transactions::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citizen_account_schema_and_admin_constructor() {
        let admin =
            CitizenAccount::new_admin("cit_001", "Human Operator", "aabbcc1122", 1724000000);
        assert_eq!(admin.schema, CITIZEN_ACCOUNT_SCHEMA);
        assert!(admin.is_sovereign_admin);
        assert_eq!(admin.status, AccountStatus::Active);

        let json = serde_json::to_string(&admin).expect("serialize citizen");
        let decoded: CitizenAccount = serde_json::from_str(&json).expect("deserialize citizen");
        assert_eq!(decoded, admin);
    }

    #[test]
    fn mandate_account_is_valid_at_enforces_temporal_and_budget_bounds() {
        let mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA,
            mandate_id: "mnd_001".to_string(),
            agent_id: "agt_001".to_string(),
            office_id: "builder".to_string(),
            scope: "crates/cynic-node".to_string(),
            fencing_token: 42,
            started_at_unix_s: 1000,
            expires_at_unix_s: 2000,
            max_actions: 10,
            consumed_actions: 5,
            max_wall_seconds: 600,
            consumed_wall_seconds: 300,
            status: MandateStatus::Active,
        };

        // Within bounds
        assert!(mandate.is_valid_at(1500));

        // Before start
        assert!(!mandate.is_valid_at(999));

        // Expired
        assert!(!mandate.is_valid_at(2001));

        // Actions exhausted
        let mut exhausted_actions = mandate.clone();
        exhausted_actions.consumed_actions = 10;
        assert!(!exhausted_actions.is_valid_at(1500));

        // Wall time exhausted
        let mut exhausted_time = mandate.clone();
        exhausted_time.consumed_wall_seconds = 600;
        assert!(!exhausted_time.is_valid_at(1500));

        // Revoked
        let mut revoked = mandate.clone();
        revoked.status = MandateStatus::Revoked;
        assert!(!revoked.is_valid_at(1500));
    }

    #[test]
    fn ledger_transaction_roundtrip_all_18_variants() {
        let now = 1724000000;
        let admin = CitizenAccount::new_admin("cit_001", "Operator", "hex", now);
        let node = NodeAccount {
            schema: NODE_ACCOUNT_SCHEMA,
            node_id: "cynic-builder".to_string(),
            owner_citizen_id: "cit_001".to_string(),
            hostname: "builder".to_string(),
            host_family: "linux.ubuntu.systemd".to_string(),
            capabilities: vec!["build_runner".to_string()],
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: Some(now),
        };
        let resource = ResourceAccount {
            schema: RESOURCE_ACCOUNT_SCHEMA,
            resource_id: "res_001".to_string(),
            node_id: "cynic-builder".to_string(),
            cpu_cores_available: 16,
            ram_bytes_available: 34359738368,
            disk_bytes_available: 500000000000,
            gpu_vram_bytes_available: 0,
            observed_at_unix_s: now,
            observation_proof_digest: Some("sha256:abcd".to_string()),
        };
        let office = OfficeAccount {
            schema: OFFICE_ACCOUNT_SCHEMA,
            office_id: "builder".to_string(),
            title: "Software Build Runner".to_string(),
            allowed_capabilities: vec!["cargo_check".to_string(), "cargo_build".to_string()],
            forbidden_actions: vec!["git_push_main".to_string()],
        };
        let mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA,
            mandate_id: "mnd_001".to_string(),
            agent_id: "agt_hermes".to_string(),
            office_id: "builder".to_string(),
            scope: "crates/cynic-node".to_string(),
            fencing_token: 1,
            started_at_unix_s: now,
            expires_at_unix_s: now + 3600,
            max_actions: 50,
            consumed_actions: 0,
            max_wall_seconds: 3600,
            consumed_wall_seconds: 0,
            status: MandateStatus::Active,
        };

        let transactions = vec![
            LedgerTransactionPayload::CitizenRegistered { citizen: admin },
            LedgerTransactionPayload::NodeBound { node },
            LedgerTransactionPayload::ResourceObserved { resource },
            LedgerTransactionPayload::OfficeDefined { office },
            LedgerTransactionPayload::MandateGranted { mandate },
            LedgerTransactionPayload::WorkloadAdmitted {
                workload_id: "wk_01".to_string(),
                mandate_id: "mnd_001".to_string(),
                target_node_id: "cynic-builder".to_string(),
                action: "cargo_check".to_string(),
            },
            LedgerTransactionPayload::WorkloadStarted {
                workload_id: "wk_01".to_string(),
                started_at_unix_s: now + 10,
            },
            LedgerTransactionPayload::ResourceReserved {
                workload_id: "wk_01".to_string(),
                node_id: "cynic-builder".to_string(),
                cpu_cores: 4,
                ram_bytes: 8589934592,
            },
            LedgerTransactionPayload::ResourceConsumed {
                workload_id: "wk_01".to_string(),
                node_id: "cynic-builder".to_string(),
                actions_used: 1,
                wall_seconds_used: 15,
            },
            LedgerTransactionPayload::WorkloadCompleted {
                workload_id: "wk_01".to_string(),
                success: true,
                completed_at_unix_s: now + 25,
                proof_digest: "sha256:proof001".to_string(),
            },
            LedgerTransactionPayload::OutcomeObserved {
                workload_id: "wk_01".to_string(),
                outcome_kind: "test_suite".to_string(),
                score_percent: 100,
            },
            LedgerTransactionPayload::EvidenceAttested {
                evidence_id: "evi_01".to_string(),
                digest: "sha256:digest01".to_string(),
                source: "cynic-node observe".to_string(),
            },
            LedgerTransactionPayload::CalibrationCreditRecorded {
                treasury_id: "trs_main".to_string(),
                amount: 100,
                reason: "green build outcome proof".to_string(),
            },
            LedgerTransactionPayload::DisputeOpened {
                dispute_id: "dsp_01".to_string(),
                target_id: "wk_01".to_string(),
                reason: "suspected non-deterministic flaky failure".to_string(),
                opened_by_citizen_id: "cit_001".to_string(),
            },
            LedgerTransactionPayload::CreditSuspended {
                treasury_id: "trs_main".to_string(),
                amount: 100,
                dispute_id: "dsp_01".to_string(),
            },
            LedgerTransactionPayload::MandateRevoked {
                mandate_id: "mnd_001".to_string(),
                revocation_reason: "emergency operator breakglass".to_string(),
                revoked_at_unix_s: now + 50,
            },
            LedgerTransactionPayload::PeerJoined {
                peer_id: "peer_b".to_string(),
                public_key_hex: "11223344".to_string(),
                joined_at_unix_s: now + 60,
            },
            LedgerTransactionPayload::PeerCheckpointed {
                epoch: 1,
                state_merkle_root_hex: "root_hash_01".to_string(),
                checkpoint_unix_s: now + 100,
            },
        ];

        assert_eq!(transactions.len(), 18);

        for (i, payload) in transactions.into_iter().enumerate() {
            let tx = LedgerTransaction::new(
                format!("tx_{i}"),
                i as u64 + 1,
                now + i as u64,
                "cit_001",
                i as u64,
                payload,
            );
            assert_eq!(tx.schema, LEDGER_TRANSACTION_SCHEMA);
            let json = serde_json::to_string(&tx).expect("serialize tx");
            let decoded: LedgerTransaction = serde_json::from_str(&json).expect("deserialize tx");
            assert_eq!(decoded, tx);
        }
    }
}

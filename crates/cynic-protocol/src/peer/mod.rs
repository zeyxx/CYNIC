//! Canonical CynicPeer constitutional state and ledger models.
//!
//! Subordinate to `contracts/foundation.json`, `contracts/framework.json` and
//! `protocols/cynic-stage2-peer-and-ledger.md`.

pub mod accounts;
pub mod engine;
pub mod state;
pub mod transactions;
pub mod wal;

pub use accounts::*;
pub use engine::*;
pub use state::*;
pub use transactions::*;
pub use wal::*;

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
            schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
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
            schema: NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            owner_citizen_id: "cit_001".to_string(),
            hostname: "builder".to_string(),
            host_family: "linux.ubuntu.systemd".to_string(),
            capabilities: vec!["build_runner".to_string()],
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: Some(now),
        };
        let resource = ResourceAccount {
            schema: RESOURCE_ACCOUNT_SCHEMA.to_string(),
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
            schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
            office_id: "builder".to_string(),
            title: "Software Build Runner".to_string(),
            allowed_capabilities: vec!["cargo_check".to_string(), "cargo_build".to_string()],
            forbidden_actions: vec!["git_push_main".to_string()],
        };
        let mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
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

    #[test]
    fn happy_path_peer_state_transitions() {
        let mut state = PeerState::new();
        let now = 1000;

        // Tx 1: Register Admin Citizen
        let admin = CitizenAccount::new_admin("cit_admin", "Sovereign Operator", "pubkey_hex", now);
        let tx1 = LedgerTransaction::new(
            "tx_01",
            1,
            now,
            "cit_admin",
            0,
            LedgerTransactionPayload::CitizenRegistered { citizen: admin },
        );
        let (next, receipt1) = apply_transaction(&state, &tx1).expect("apply tx1");
        assert_eq!(receipt1.sequence, 1);
        assert_eq!(next.citizens.len(), 1);
        state = next;

        // Tx 2: Bind Node
        let node = NodeAccount {
            schema: NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: "node_alpha".to_string(),
            owner_citizen_id: "cit_admin".to_string(),
            hostname: "alpha".to_string(),
            host_family: "linux.ubuntu.systemd".to_string(),
            capabilities: vec!["compute".to_string()],
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: Some(now),
        };
        let tx2 = LedgerTransaction::new(
            "tx_02",
            2,
            now + 1,
            "cit_admin",
            1,
            LedgerTransactionPayload::NodeBound { node },
        );
        let (next, _) = apply_transaction(&state, &tx2).expect("apply tx2");
        assert_eq!(next.nodes.len(), 1);
        state = next;

        // Tx 3: Observe Resources
        let resource = ResourceAccount {
            schema: RESOURCE_ACCOUNT_SCHEMA.to_string(),
            resource_id: "res_alpha".to_string(),
            node_id: "node_alpha".to_string(),
            cpu_cores_available: 16,
            ram_bytes_available: 34359738368,
            disk_bytes_available: 1000000000000,
            gpu_vram_bytes_available: 0,
            observed_at_unix_s: now + 2,
            observation_proof_digest: Some("sha256:digest01".to_string()),
        };
        let tx3 = LedgerTransaction::new(
            "tx_03",
            3,
            now + 2,
            "cit_admin",
            2,
            LedgerTransactionPayload::ResourceObserved { resource },
        );
        let (next, _) = apply_transaction(&state, &tx3).expect("apply tx3");
        state = next;

        // Tx 4: Define Office
        let office = OfficeAccount {
            schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
            office_id: "builder".to_string(),
            title: "Build Runner".to_string(),
            allowed_capabilities: vec!["cargo_check".to_string()],
            forbidden_actions: vec!["root_shell".to_string()],
        };
        let tx4 = LedgerTransaction::new(
            "tx_04",
            4,
            now + 3,
            "cit_admin",
            3,
            LedgerTransactionPayload::OfficeDefined { office },
        );
        let (next, _) = apply_transaction(&state, &tx4).expect("apply tx4");
        state = next;

        // Tx 5: Grant Mandate with fencing token 1
        let mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
            mandate_id: "mnd_01".to_string(),
            agent_id: "agt_hermes".to_string(),
            office_id: "builder".to_string(),
            scope: "crates/cynic-node".to_string(),
            fencing_token: 1,
            started_at_unix_s: now,
            expires_at_unix_s: now + 3600,
            max_actions: 100,
            consumed_actions: 0,
            max_wall_seconds: 3600,
            consumed_wall_seconds: 0,
            status: MandateStatus::Active,
        };
        let tx5 = LedgerTransaction::new(
            "tx_05",
            5,
            now + 4,
            "cit_admin",
            4,
            LedgerTransactionPayload::MandateGranted { mandate },
        );
        let (next, _) = apply_transaction(&state, &tx5).expect("apply tx5");
        state = next;

        // Tx 6: Admit Workload
        let tx6 = LedgerTransaction::new(
            "tx_06",
            6,
            now + 5,
            "cit_admin",
            5,
            LedgerTransactionPayload::WorkloadAdmitted {
                workload_id: "wk_01".to_string(),
                mandate_id: "mnd_01".to_string(),
                target_node_id: "node_alpha".to_string(),
                action: "cargo_check".to_string(),
            },
        );
        let (next, _) = apply_transaction(&state, &tx6).expect("apply tx6");
        assert_eq!(next.workloads.len(), 1);
        state = next;

        // Tx 7: Start Workload
        let tx7 = LedgerTransaction::new(
            "tx_07",
            7,
            now + 6,
            "cit_admin",
            6,
            LedgerTransactionPayload::WorkloadStarted {
                workload_id: "wk_01".to_string(),
                started_at_unix_s: now + 6,
            },
        );
        let (next, _) = apply_transaction(&state, &tx7).expect("apply tx7");
        assert_eq!(
            next.workloads["wk_01"].phase,
            WorkloadExecutionPhase::Started
        );
        state = next;

        // Tx 8: Reserve Resources
        let tx8 = LedgerTransaction::new(
            "tx_08",
            8,
            now + 7,
            "cit_admin",
            7,
            LedgerTransactionPayload::ResourceReserved {
                workload_id: "wk_01".to_string(),
                node_id: "node_alpha".to_string(),
                cpu_cores: 4,
                ram_bytes: 8589934592,
            },
        );
        let (next, _) = apply_transaction(&state, &tx8).expect("apply tx8");
        assert_eq!(next.resources["res_alpha"].cpu_cores_available, 12);
        state = next;

        // Tx 9: Complete Workload
        let tx9 = LedgerTransaction::new(
            "tx_09",
            9,
            now + 30,
            "cit_admin",
            8,
            LedgerTransactionPayload::WorkloadCompleted {
                workload_id: "wk_01".to_string(),
                success: true,
                completed_at_unix_s: now + 30,
                proof_digest: "sha256:proof_verified".to_string(),
            },
        );
        let (next, _) = apply_transaction(&state, &tx9).expect("apply tx9");
        assert_eq!(
            next.workloads["wk_01"].phase,
            WorkloadExecutionPhase::Completed
        );
        state = next;

        // Tx 10: Seal Checkpoint
        let tx10 = LedgerTransaction::new(
            "tx_10",
            10,
            now + 60,
            "cit_admin",
            9,
            LedgerTransactionPayload::PeerCheckpointed {
                epoch: 1,
                state_merkle_root_hex: state.state_root_digest(),
                checkpoint_unix_s: now + 60,
            },
        );
        let (next, receipt10) = apply_transaction(&state, &tx10).expect("apply tx10");
        assert_eq!(next.epoch, 1);
        assert_eq!(
            receipt10.outcome,
            TransitionOutcome::Checkpointed { epoch: 1 }
        );
    }

    #[test]
    fn rejects_out_of_order_sequence() {
        let state = PeerState::new();
        let tx = LedgerTransaction::new(
            "tx_01",
            5,
            1000,
            "cit_01",
            0,
            LedgerTransactionPayload::CitizenRegistered {
                citizen: CitizenAccount::new_admin("cit_01", "Admin", "pub", 1000),
            },
        );
        match apply_transaction(&state, &tx) {
            Err(TransitionError::SequenceNotMonotonic {
                expected: 1,
                got: 5,
            }) => {}
            other => panic!("expected SequenceNotMonotonic, got {other:?}"),
        }
    }

    #[test]
    fn rejects_fencing_token_regression() {
        let mut state = PeerState::new();
        state.last_sequence = 1;
        state.offices.insert(
            "builder".to_string(),
            OfficeAccount {
                schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
                office_id: "builder".to_string(),
                title: "Builder".to_string(),
                allowed_capabilities: vec![],
                forbidden_actions: vec![],
            },
        );
        state.mandates.insert(
            "mnd_01".to_string(),
            MandateAccount {
                schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
                mandate_id: "mnd_01".to_string(),
                agent_id: "agt_hermes".to_string(),
                office_id: "builder".to_string(),
                scope: "all".to_string(),
                fencing_token: 5,
                started_at_unix_s: 1000,
                expires_at_unix_s: 2000,
                max_actions: 10,
                consumed_actions: 0,
                max_wall_seconds: 100,
                consumed_wall_seconds: 0,
                status: MandateStatus::Active,
            },
        );

        // New mandate with regressed fencing token 3 (must be > 5)
        let regressed_mandate = MandateAccount {
            schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
            mandate_id: "mnd_02".to_string(),
            agent_id: "agt_hermes".to_string(),
            office_id: "builder".to_string(),
            scope: "all".to_string(),
            fencing_token: 3,
            started_at_unix_s: 1000,
            expires_at_unix_s: 2000,
            max_actions: 10,
            consumed_actions: 0,
            max_wall_seconds: 100,
            consumed_wall_seconds: 0,
            status: MandateStatus::Active,
        };
        let tx = LedgerTransaction::new(
            "tx_02",
            2,
            1001,
            "cit_admin",
            1,
            LedgerTransactionPayload::MandateGranted {
                mandate: regressed_mandate,
            },
        );
        match apply_transaction(&state, &tx) {
            Err(TransitionError::FencingTokenRegression {
                expected_gt: 5,
                got: 3,
                ..
            }) => {}
            other => panic!("expected FencingTokenRegression, got {other:?}"),
        }
    }

    #[test]
    fn rejects_workload_admission_under_expired_mandate() {
        let mut state = PeerState::new();
        state.last_sequence = 1;
        state.nodes.insert(
            "node_01".to_string(),
            NodeAccount {
                schema: NODE_ACCOUNT_SCHEMA.to_string(),
                node_id: "node_01".to_string(),
                owner_citizen_id: "cit_01".to_string(),
                hostname: "n1".to_string(),
                host_family: "linux".to_string(),
                capabilities: vec![],
                status: NodeMembershipStatus::Enrolled,
                last_liveness_proof_unix_s: None,
            },
        );
        state.mandates.insert(
            "mnd_01".to_string(),
            MandateAccount {
                schema: MANDATE_ACCOUNT_SCHEMA.to_string(),
                mandate_id: "mnd_01".to_string(),
                agent_id: "agt_01".to_string(),
                office_id: "builder".to_string(),
                scope: "all".to_string(),
                fencing_token: 1,
                started_at_unix_s: 1000,
                expires_at_unix_s: 2000,
                max_actions: 10,
                consumed_actions: 0,
                max_wall_seconds: 100,
                consumed_wall_seconds: 0,
                status: MandateStatus::Active,
            },
        );

        // Admit at timestamp 2500 (mandate expired at 2000)
        let tx = LedgerTransaction::new(
            "tx_02",
            2,
            2500,
            "cit_01",
            1,
            LedgerTransactionPayload::WorkloadAdmitted {
                workload_id: "wk_01".to_string(),
                mandate_id: "mnd_01".to_string(),
                target_node_id: "node_01".to_string(),
                action: "build".to_string(),
            },
        );
        match apply_transaction(&state, &tx) {
            Err(TransitionError::MandateInvalid { .. }) => {}
            other => panic!("expected MandateInvalid, got {other:?}"),
        }
    }

    #[test]
    fn rejects_resource_quota_over_allocation() {
        let mut state = PeerState::new();
        state.last_sequence = 1;
        state.resources.insert(
            "res_01".to_string(),
            ResourceAccount {
                schema: RESOURCE_ACCOUNT_SCHEMA.to_string(),
                resource_id: "res_01".to_string(),
                node_id: "node_01".to_string(),
                cpu_cores_available: 4,
                ram_bytes_available: 1024,
                disk_bytes_available: 1024,
                gpu_vram_bytes_available: 0,
                observed_at_unix_s: 1000,
                observation_proof_digest: None,
            },
        );
        state.workloads.insert(
            "wk_01".to_string(),
            AdmittedWorkload {
                workload_id: "wk_01".to_string(),
                mandate_id: "mnd_01".to_string(),
                target_node_id: "node_01".to_string(),
                action: "test".to_string(),
                phase: WorkloadExecutionPhase::Admitted,
                admitted_at_unix_s: 1000,
                started_at_unix_s: None,
                completed_at_unix_s: None,
                cpu_cores_reserved: 0,
                ram_bytes_reserved: 0,
                actions_used: 0,
                wall_seconds_used: 0,
                success: None,
                proof_digest: None,
            },
        );

        // Try reserving 8 cores when only 4 available
        let tx = LedgerTransaction::new(
            "tx_02",
            2,
            1001,
            "cit_01",
            1,
            LedgerTransactionPayload::ResourceReserved {
                workload_id: "wk_01".to_string(),
                node_id: "node_01".to_string(),
                cpu_cores: 8,
                ram_bytes: 512,
            },
        );
        match apply_transaction(&state, &tx) {
            Err(TransitionError::ResourceQuotaExceeded {
                requested: 8,
                available: 4,
                ..
            }) => {}
            other => panic!("expected ResourceQuotaExceeded, got {other:?}"),
        }
    }

    #[test]
    fn rejects_duplicate_citizen_registration() {
        let mut state = PeerState::new();
        state.last_sequence = 1;
        state.citizens.insert(
            "cit_01".to_string(),
            CitizenAccount::new_admin("cit_01", "Admin", "pub", 1000),
        );

        let tx = LedgerTransaction::new(
            "tx_02",
            2,
            1001,
            "cit_01",
            1,
            LedgerTransactionPayload::CitizenRegistered {
                citizen: CitizenAccount::new_admin("cit_01", "Duplicate Admin", "pub", 1001),
            },
        );
        match apply_transaction(&state, &tx) {
            Err(TransitionError::CitizenAlreadyExists(id)) => assert_eq!(id, "cit_01"),
            other => panic!("expected CitizenAlreadyExists, got {other:?}"),
        }
    }
}

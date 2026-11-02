use super::*;
use uuid::Uuid;

    use cynic_build::{
        admit_build_agent_request, canonical_argv_from_build_action,
        BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA, BuildReleasePublicationIntent,
    };

    /// Task P fix round 1: a submit verb must survive the whole chain —
    /// mapping -> composition admission -> argv construction — without a
    /// blocker. Pins the composition gate to the same action set the mapping
    /// and argv translators accept, so a narrow green that fixes only one
    /// link (e.g. mapping + argv but not the composition gate) cannot recur.
    #[test]
    fn submit_action_chains_through_admission_and_argv() {
        for verb in ["check", "build", "build-release", "audit", "audit-all", "test"] {
            let action = submit_action_to_canonical(verb).expect("known submit verb");
            let mut request = push_request(&"d".repeat(40));
            request.action = action.to_string();
            if action == "cargo:build-release" {
                // Composition requires publication intent plus the two
                // release proofs for release builds: equip the request so
                // the chain test exercises the real release path.
                request.release_publication = Some(BuildReleasePublicationIntent {
                    schema: BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                    release_id: "v9.9.9-test".to_string(),
                    release_epoch: 1,
                });
                request.proof_requirements.extend([
                    "release_binary_artifact_read_back".to_string(),
                    "signed_release_basket_read_back".to_string(),
                ]);
            }
            let admission = admit_build_agent_request(&request);
            assert!(
                !admission
                    .blockers
                    .contains(&"build_agent_request_action_not_allowed".to_string()),
                "verb {verb} -> action {action} refused by composition gate"
            );
            assert!(
                admission.blockers.is_empty(),
                "verb {verb} -> action {action} blocked: {:?}",
                admission.blockers
            );
            assert!(
                canonical_argv_from_build_action(action).is_some(),
                "action {action} must have a canonical argv"
            );
        }
    }

    #[test]
    fn record_shape_carries_resolved_and_unresolved_evidence() {
        let sha = "a".repeat(40);
        let shaped = shape_record_response(
            "wid-1",
            &AgentWorkloadState::Completed,
            Some("audit:crate"),
            &["cargo".to_string()],
            Some(sha.as_str()),
            Some("Completed"),
            vec![serde_json::json!({"kind": "k", "digest": "d", "envelope": {"exit_code": 0}})],
            vec!["missing-digest".to_string()],
        );
        assert_eq!(shaped["schema"], "cynic.builder.record.v1");
        assert_eq!(shaped["action"], "audit:crate");
        assert_eq!(shaped["evidence"][0]["envelope"]["exit_code"], 0);
        assert_eq!(shaped["unresolved_evidence"][0], "missing-digest");
    }

    #[test]
    fn record_shape_for_non_terminal_has_no_completion() {
        let shaped = shape_record_response(
            "wid-2",
            &AgentWorkloadState::Dispatched,
            None,
            &[],
            None,
            None,
            vec![],
            vec![],
        );
        assert!(shaped["completion_status"].is_null());
        assert!(shaped["evidence"].as_array().unwrap().is_empty());
    }

    #[test]
    fn exec_request_deserializes_command_and_work_dir() {
        let json = serde_json::json!({"command": "cargo check", "work_dir": "/tmp/repo"});
        let req: ExecRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.command, "cargo check");
        assert_eq!(req.work_dir, "/tmp/repo");
    }

    #[test]
    fn exec_request_work_dir_defaults_to_empty() {
        let json = serde_json::json!({"command": "ls -la"});
        let req: ExecRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.command, "ls -la");
        assert!(req.work_dir.is_empty());
    }

    #[test]
    fn exec_response_serializes_exit_code_stdout_stderr() {
        let resp = ExecResponse {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration_ms: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["exit_code"], 0);
        assert_eq!(json["stdout"], "ok");
        assert!(json["stderr"].as_str().unwrap().is_empty());
    }

    #[test]
    fn exec_response_serializes_nonzero_exit() {
        let resp = ExecResponse {
            exit_code: 2,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["exit_code"], 2);
        assert_eq!(json["stderr"], "error");
    }

    #[test]
    fn drain_response_shape_names_state_and_digest() {
        let receipt = DrainReceipt::new(
            "node-1",
            SupervisorState::Active,
            SupervisorState::Draining,
            2,
            1_700_000_000_000,
        );
        let shaped = shape_drain_response(&receipt, Some("sha256:abc"));
        assert_eq!(shaped["schema"], "cynic.node.drain_receipt.v1");
        assert_eq!(shaped["state"], "draining");
        assert_eq!(shaped["transition_from"], "active");
        assert_eq!(shaped["in_flight"], 2);
        assert_eq!(shaped["receipt_digest"], "sha256:abc");
        // Idempotent status reads persist nothing: no digest.
        let shaped = shape_drain_response(&receipt, None);
        assert!(shaped["receipt_digest"].is_null());
    }

    #[test]
    fn submit_verbs_map_to_honest_canonical_actions() {
        assert_eq!(submit_action_to_canonical("check"), Ok("cargo:check"));
        assert_eq!(submit_action_to_canonical("audit"), Ok("audit:crate"));
        assert_eq!(submit_action_to_canonical("audit-all"), Ok("audit:all"));
        assert_eq!(submit_action_to_canonical("build"), Ok("cargo:build"));
        assert_eq!(
            submit_action_to_canonical("build-release"),
            Ok("cargo:build-release")
        );
        assert_eq!(
            submit_action_to_canonical("build:release"),
            Ok("cargo:build-release")
        );
        assert_eq!(
            submit_action_to_canonical("release"),
            Ok("cargo:build-release")
        );
        assert_eq!(submit_action_to_canonical("test"), Ok("cargo:test"));
        assert!(submit_action_to_canonical("deploy").is_err());
        assert!(submit_action_to_canonical("").is_err());
    }

    #[test]
    fn workload_id_has_forgejo_prefix() {
        let id = format!("forgejo-{}", Uuid::new_v4());
        assert!(id.starts_with("forgejo-"));
    }

    #[test]
    fn workload_id_is_deterministic_per_commit() {
        let sha = "a".repeat(40);
        assert_eq!(
            forgejo_workload_id(&sha),
            format!("forgejo-{}", "a".repeat(12))
        );
        assert_eq!(forgejo_workload_id(&sha), forgejo_workload_id(&sha));
        let other = format!("b{}", "a".repeat(39));
        assert_ne!(forgejo_workload_id(&sha), forgejo_workload_id(&other));
    }

    fn push_request(commit_sha: &str) -> BuildAgentRequest {
        BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: forgejo_workload_id(commit_sha),
            requested_by: "forgejo".to_string(),
            source_commit: commit_sha.to_string(),
            branch: None,
            source: BuildSourceReference {
                schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://example/CYNIC.git".to_string(),
                revision: commit_sha.to_string(),
                authority_ref: "forgejo".to_string(),
                snapshot: None,
            },
            action: "audit:crate".to_string(),
            desired_role: BUILD_RUNNER_ROLE.to_string(),
            mutation_authorized: true,
            proof_requirements: vec![BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec![format!("forgejo:push@{commit_sha}")],
        }
    }

    #[test]
    fn identical_pushes_compose_identical_workload_ids() {
        let sha = "c".repeat(40);
        let first =
            compose_build_agent_workload(&push_request(&sha), "node-1", "/tmp").expect("compose");
        let second =
            compose_build_agent_workload(&push_request(&sha), "node-1", "/tmp").expect("compose");
        assert_eq!(first.workload_id, second.workload_id);
    }

    #[test]
    fn hmac_verification_valid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        // Compute expected HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        assert!(verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            &expected
        ));
    }

    #[test]
    fn hmac_verification_invalid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        assert!(!verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            "deadbeef"
        ));
    }

    #[test]
    fn hmac_verification_with_sha256_prefix() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            &expected
        ));
    }

    #[test]
    fn hmac_verification_rejects_non_hex() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        assert!(!verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            "not-a-valid-hex-digest"
        ));
    }

    #[test]
    fn commit_sha_validation_rejects_short() {
        let sha = "abc123";
        assert!(!sha.chars().all(|c| c.is_ascii_hexdigit()) || sha.len() != 40);
    }

    #[test]
    fn commit_sha_validation_accepts_valid() {
        let sha = "a".repeat(40);
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The webhook's request must survive the REAL admission gate. This is the
    /// regression proof for the retired dead-letter request (stale
    /// cynic.build_agent_request.v1 schema, non-canonical cargo:build action,
    /// empty evidence) that Forgejo pushes could never admit.
    #[test]
    fn webhook_request_passes_canonical_admission_and_composition() {
        let event = cynic_protocol::vcs::VcsPushEvent {
            provider: "forgejo".to_string(),
            commit_sha: "a".repeat(40),
            ref_name: "refs/heads/main".to_string(),
            pusher: "zeyxx".to_string(),
            repository: Some("cynic".to_string()),
            pushed_at_unix_ms: None,
        };
        let commit_sha = event.commit_sha.clone();
        let config_repo = crate::config::default_build_repository_id();
        let request = BuildAgentRequest {
            schema: cynic_build::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: format!("forgejo-{}", Uuid::new_v4()),
            requested_by: event.pusher.clone(),
            source_commit: commit_sha.clone(),
            branch: None,
            source: BuildSourceReference {
                schema: cynic_build::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: config_repo,
                transport_uri: format!(
                    "ssh://forgejo@forgejo.service.cynic:22/owner/{}.git",
                    "cynic"
                ),
                revision: commit_sha.clone(),
                authority_ref: "forgejo".into(),
                snapshot: None,
            },
            action: "audit:crate".to_string(),
            desired_role: BUILD_RUNNER_ROLE.to_string(),
            mutation_authorized: true,
            proof_requirements: vec![cynic_build::BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec![format!("forgejo:{}@{}", event.ref_name, commit_sha)],
        };

        let admission = cynic_build::admit_build_agent_request(&request);
        assert_eq!(
            admission.status, "admitted",
            "blockers: {:?}",
            admission.blockers
        );

        let envelope =
            cynic_build::compose_build_agent_workload(&request, "cynic-builder", "/tmp/wf")
                .expect("webhook request composes a bounded workload");
        // Slice semantics changed deliberately: `make:check` composes the native
        // audit argv (vocabulary honesty), not the Makefile wrapper.
        assert_eq!(
            envelope.action.argv,
            vec!["cargo", "run", "-q", "-p", "cynic-cli", "--", "audit", "crate"]
        );
    }

    #[test]
    fn identity_shape_reports_full_convergence() {
        let running = "a".repeat(64);
        let shaped = shape_identity_response(
            "cynic-builder",
            1_700_000_000_000,
            Some(&running),
            Some("/usr/local/bin/cynic"),
            Some(&running),
            Some("deadbeef"),
        );
        assert_eq!(shaped["schema"], "cynic.builder.identity.v1");
        assert_eq!(shaped["node_id"], "cynic-builder");
        assert_eq!(shaped["running_binary_sha256"], running);
        assert_eq!(shaped["installed_binary_path"], "/usr/local/bin/cynic");
        assert_eq!(shaped["applied_registry_revision"], "deadbeef");
        assert_eq!(shaped["binaries_match"], true);
    }

    #[test]
    fn identity_shape_reports_binary_skew() {
        let shaped = shape_identity_response(
            "cynic-builder",
            1_700_000_000_000,
            Some(&"a".repeat(64)),
            Some("/usr/local/bin/cynic"),
            Some(&"b".repeat(64)),
            Some("deadbeef"),
        );
        assert_eq!(shaped["binaries_match"], false);
    }

    #[test]
    fn identity_shape_reports_unknowns_as_null() {
        let shaped = shape_identity_response("cynic-builder", 0, None, None, None, None);
        assert!(shaped["running_binary_sha256"].is_null());
        assert!(shaped["installed_binary_path"].is_null());
        assert!(shaped["installed_binary_sha256"].is_null());
        assert!(shaped["applied_registry_revision"].is_null());
        assert!(shaped["binaries_match"].is_null());
    }

    #[test]
    fn identity_revision_extractor_reads_top_level_revision() {
        let value = serde_json::json!({"version": 1, "revision": "abc123"});
        assert_eq!(
            applied_revision_from_registry_json(&value),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn identity_revision_extractor_returns_none_without_revision() {
        let value = serde_json::json!({"version": 1});
        assert_eq!(applied_revision_from_registry_json(&value), None);
    }

    #[test]
    fn identity_revision_extractor_rejects_empty_and_non_string() {
        assert_eq!(
            applied_revision_from_registry_json(&serde_json::json!({"revision": ""})),
            None
        );
        assert_eq!(
            applied_revision_from_registry_json(&serde_json::json!({"revision": 42})),
            None
        );
    }

    #[test]
    fn identity_sha256_hashes_file_bytes() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().expect("temp fixture");
        file.write_all(b"cynic-identity").expect("write fixture");
        let digest = sha256_file(&file.path().to_path_buf()).expect("hashable fixture");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            sha256_file(&std::path::PathBuf::from("/nonexistent-identity-fixture")),
            None
        );
    }

    fn lifecycle_envelope(
        subject: &str,
        producer: &str,
        observed_at_ms: u128,
        payload: serde_json::Value,
    ) -> EvidenceEnvelope {
        cynic_protocol::build_evidence_envelope(
            subject,
            producer,
            observed_at_ms,
            vec!["lifecycle-receipt-fixture".to_string()],
            &payload,
        )
        .expect("fixture envelope")
    }

    #[test]
    fn evidence_kind_detects_restart_drain_pause() {
        let restart = lifecycle_envelope(
            "builder",
            "cynic-node:supervisor",
            3,
            serde_json::json!({"schema": "cynic.node.restart_receipt.v1"}),
        );
        let drain = lifecycle_envelope(
            "builder:drain",
            "cynic-node",
            2,
            serde_json::json!({"status": "ok"}),
        );
        let pause = lifecycle_envelope(
            "builder",
            "cynic-node",
            1,
            serde_json::json!({"operation": "pause"}),
        );
        assert_eq!(evidence_lifecycle_kind(&restart), Some("restart"));
        assert_eq!(evidence_lifecycle_kind(&drain), Some("drain"));
        assert_eq!(evidence_lifecycle_kind(&pause), Some("pause"));
    }

    #[test]
    fn evidence_kind_detects_reload_receipts() {
        let reload = lifecycle_envelope(
            "builder",
            "cynic-node",
            4,
            serde_json::json!({"schema": "cynic.node.reload_receipt.v1"}),
        );
        assert_eq!(evidence_lifecycle_kind(&reload), Some("reload"));
    }

    #[test]
    fn reload_shape_reports_applied_vs_previous_and_digest() {
        let receipt = ReloadReceipt::applied(
            "builder-1",
            "prev-digest",
            "next-digest",
            vec!["node_control.interval_secs".to_string()],
            true,
            1_700_000_000_000,
        );
        let shaped = shape_reload_response(&receipt, Some("sha256:abc"));
        assert_eq!(shaped["schema"], "cynic.node.reload_receipt.v1");
        assert_eq!(shaped["outcome"], "applied");
        assert_eq!(shaped["previous_digest"], "prev-digest");
        assert_eq!(shaped["applied_digest"], "next-digest");
        assert_eq!(shaped["changed_keys"][0], "node_control.interval_secs");
        assert_eq!(shaped["requires_restart"], true);
        assert_eq!(shaped["receipt_digest"], "sha256:abc");
    }

    #[test]
    fn reload_shape_no_change_carries_no_receipt_digest() {
        let receipt = ReloadReceipt {
            schema: crate::config::RELOAD_RECEIPT_SCHEMA.to_string(),
            node_id: "builder-1".to_string(),
            outcome: crate::config::ReloadOutcome::AlreadyConverged
                .as_str()
                .to_string(),
            previous_digest: "same".to_string(),
            applied_digest: "same".to_string(),
            changed_keys: Vec::new(),
            requires_restart: false,
            observed_at_unix_ms: 1_700_000_000_000,
        };
        let shaped = shape_reload_response(&receipt, None);
        assert_eq!(shaped["outcome"], "already-converged");
        assert!(shaped["receipt_digest"].is_null());
        assert!(shaped["changed_keys"].as_array().unwrap().is_empty());
    }

    #[test]
    fn evidence_kind_is_none_for_build_proofs() {
        let proof = lifecycle_envelope(
            "agent-workload",
            "cynic-builder",
            1,
            serde_json::json!({"schema": "cynic.builder.proof.v1", "exit_code": 0}),
        );
        assert_eq!(evidence_lifecycle_kind(&proof), None);
    }

    #[test]
    fn evidence_limit_defaults_and_caps() {
        assert_eq!(clamp_evidence_limit(None), EVIDENCE_LIST_DEFAULT_LIMIT);
        assert_eq!(clamp_evidence_limit(Some(10)), 10);
        assert_eq!(clamp_evidence_limit(Some(0)), 0);
        assert_eq!(clamp_evidence_limit(Some(10_000)), EVIDENCE_LIST_MAX_LIMIT);
    }

    #[test]
    fn evidence_list_shape_carries_schema_and_counters() {
        let shaped = shape_evidence_list_response(
            "cynic-builder",
            1_700_000_000_000,
            vec![serde_json::json!({"digest": "sha256:abc"})],
            7,
            1,
            50,
            None,
        );
        assert_eq!(shaped["schema"], "cynic.builder.evidence_list.v1");
        assert_eq!(shaped["node_id"], "cynic-builder");
        assert_eq!(shaped["returned"], 1);
        assert_eq!(shaped["total_scanned"], 7);
        assert_eq!(shaped["unparseable"], 1);
        assert!(shaped["kind"].is_null());
    }

    fn write_evidence_fixture(dir: &std::path::Path, envelope: &EvidenceEnvelope) {
        let name = format!(
            "{}.json",
            envelope
                .digest
                .strip_prefix("sha256:")
                .expect("content-addressed digest")
        );
        std::fs::write(
            dir.join(name),
            serde_json::to_vec_pretty(envelope).expect("serialize fixture"),
        )
        .expect("write fixture");
    }

    #[test]
    fn evidence_scan_lists_newest_first_bounded_and_filtered() {
        let dir = tempfile::tempdir().expect("temp evidence dir");
        let evidence_root = dir.path().join("evidence");
        std::fs::create_dir(&evidence_root).expect("evidence dir");
        let old = lifecycle_envelope(
            "builder",
            "cynic-node",
            100,
            serde_json::json!({"schema": "cynic.node.restart_receipt.v1"}),
        );
        let mid = lifecycle_envelope(
            "builder",
            "cynic-node",
            200,
            serde_json::json!({"schema": "cynic.node.drain_receipt.v1"}),
        );
        let new = lifecycle_envelope(
            "builder",
            "cynic-node",
            300,
            serde_json::json!({"schema": "cynic.builder.proof.v1"}),
        );
        write_evidence_fixture(&evidence_root, &old);
        write_evidence_fixture(&evidence_root, &mid);
        write_evidence_fixture(&evidence_root, &new);
        std::fs::write(evidence_root.join("corrupt.json"), b"{not json").expect("corrupt");
        std::fs::write(evidence_root.join("notes.txt"), b"ignored").expect("non-json");

        let (items, total, bad) = list_evidence_summaries(&evidence_root, 10, None);
        assert_eq!(total, 4);
        assert_eq!(bad, 1);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["observed_at_unix_ms"], 300);
        assert_eq!(items[2]["observed_at_unix_ms"], 100);

        let (page, _, _) = list_evidence_summaries(&evidence_root, 1, None);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0]["observed_at_unix_ms"], 300);

        let (restarts, _, _) = list_evidence_summaries(&evidence_root, 10, Some("restart"));
        assert_eq!(restarts.len(), 1);
        assert_eq!(restarts[0]["kind"], "restart");

        let (unknown, _, _) = list_evidence_summaries(&evidence_root, 10, Some("reboot"));
        assert!(unknown.is_empty());
    }

    #[test]
    fn evidence_scan_missing_dir_is_empty_not_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (items, total, bad) =
            list_evidence_summaries(&dir.path().join("no-such-evidence"), 10, None);
        assert!(items.is_empty());
        assert_eq!(total, 0);
        assert_eq!(bad, 0);
    }

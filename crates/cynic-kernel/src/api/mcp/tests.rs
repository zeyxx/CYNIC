mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;

        use crate::domain::coord::NullCoord;
    use crate::domain::dog::PHI_INV;
    use crate::domain::storage::{
        ActivityStorage, CrystalStorage, DispatchStorage, NullStorage, ProofOfHistory,
        ProofOfHistoryStorage, StateLogStorage, StorageError, SubmissionStorage, TaskStorage,
        VerdictStorage,
    };

    struct DownStorage;

    #[async_trait::async_trait]
    impl VerdictStorage for DownStorage {
        async fn store_verdict(&self, _: &crate::domain::dog::Verdict) -> Result<(), StorageError> {
            Ok(())
        }
        async fn get_verdict(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::dog::Verdict>, StorageError> {
            Ok(None)
        }
        async fn list_verdicts(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::dog::Verdict>, StorageError> {
            Err(StorageError::ConnectionFailed("down".into()))
        }
        async fn recent_max_disagreements(&self, _: usize) -> Result<Vec<f64>, StorageError> {
            Ok(vec![])
        }
        async fn list_verdicts_by_domain(
            &self,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::dog::Verdict>, StorageError> {
            Ok(vec![])
        }
        async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
        async fn count_verdicts(&self) -> Result<u64, StorageError> {
            Ok(0)
        }
        async fn count_verdicts_by_kind(
            &self,
        ) -> Result<std::collections::HashMap<String, u64>, StorageError> {
            Ok(std::collections::HashMap::new())
        }
    }

    #[async_trait::async_trait]
    impl CrystalStorage for DownStorage {
        async fn store_crystal(&self, _: &crate::domain::ccm::Crystal) -> Result<(), StorageError> {
            Ok(())
        }
        async fn get_crystal(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::ccm::Crystal>, StorageError> {
            Ok(None)
        }
        async fn list_crystals(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
            Ok(vec![])
        }
        async fn list_crystals_filtered(
            &self,
            _: u32,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
            Ok(vec![])
        }
        async fn delete_crystal(&self, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn update_crystal_relations(
            &self,
            _: &str,
            _: std::collections::BTreeMap<String, String>,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list_crystals_for_domain(
            &self,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
            Ok(vec![])
        }
        async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
            Ok(0)
        }
        async fn observe_crystal(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: f64,
            _: i64,
            _: usize,
            _: &str,
            _: &str,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn observe_crystal_hypha(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: f64,
            _: i64,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn shatter_crystal(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn store_crystal_embedding(&self, _: &str, _: &[f32]) -> Result<(), StorageError> {
            Ok(())
        }
        async fn search_crystals_semantic(
            &self,
            _: &[f32],
            _: u32,
        ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
            Ok(vec![])
        }
        async fn find_similar_crystal(
            &self,
            _: &[f32],
            _: &str,
            _: f64,
        ) -> Result<Option<(String, f64)>, StorageError> {
            Ok(None)
        }
        async fn list_crystals_missing_embedding(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
            Ok(vec![])
        }
        async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
            Ok(0)
        }
    }

    #[async_trait::async_trait]
    impl ActivityStorage for DownStorage {
        async fn store_observation(
            &self,
            _: &crate::domain::storage::Observation,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn store_event(&self, _: &crate::domain::storage::Event) -> Result<(), StorageError> {
            Ok(())
        }
        async fn fleet_stats(
            &self,
            _: u64,
            _: u32,
        ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
            Ok(vec![])
        }
        async fn list_events(
            &self,
            _: Option<&str>,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::RawEvent>, StorageError> {
            Ok(vec![])
        }
        async fn list_degraded_nodes(
            &self,
            _: u64,
            _: f64,
        ) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
            Ok(vec![])
        }
        async fn list_observations_raw(
            &self,
            _: Option<&str>,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
            Ok(vec![])
        }
        async fn list_observations_filtered(
            &self,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
            Ok(vec![])
        }
        async fn list_observations_by_target(
            &self,
            _: &str,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
            Ok(vec![])
        }
        async fn list_observations_by_tag(
            &self,
            _: &str,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
            Ok(vec![])
        }
        async fn last_observation_per_source(
            &self,
        ) -> Result<Vec<(String, String, u64)>, StorageError> {
            Ok(vec![])
        }
        async fn count_observations(&self) -> Result<u64, StorageError> {
            Ok(0)
        }
        async fn store_session_summary(
            &self,
            _: &crate::domain::ccm::SessionSummary,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list_session_summaries(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
            Ok(vec![])
        }
        async fn get_unsummarized_sessions(
            &self,
            _: u32,
            _: u32,
        ) -> Result<Vec<(String, String, u32)>, StorageError> {
            Ok(vec![])
        }
        async fn get_session_observations(
            &self,
            _: &str,
        ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
            Ok(vec![])
        }
        async fn store_session_compliance(
            &self,
            _: &crate::domain::compliance::SessionCompliance,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list_session_compliance(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
            Ok(vec![])
        }
        async fn zone_activity(
            &self,
            _: &[String],
            _: &str,
        ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
            Ok(vec![])
        }
    }

    #[async_trait::async_trait]
    impl TaskStorage for DownStorage {
        async fn store_agent_task(
            &self,
            _: &crate::domain::storage::AgentTask,
        ) -> Result<String, StorageError> {
            Ok("".into())
        }
        async fn list_pending_agent_tasks(
            &self,
            _: &str,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
            Ok(vec![])
        }
        async fn list_completed_agent_tasks(
            &self,
            _: &str,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
            Ok(vec![])
        }
        async fn list_overdue_agent_tasks(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
            Ok(vec![])
        }
        async fn claim_agent_task(&self, _: &str, _: &str) -> Result<bool, StorageError> {
            Ok(true)
        }
        async fn update_agent_task_result(
            &self,
            _: &str,
            _: Option<String>,
            _: Option<String>,
        ) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl DispatchStorage for DownStorage {
        async fn store_agent_dispatch(
            &self,
            _: &crate::domain::storage::AgentDispatch,
        ) -> Result<String, StorageError> {
            Ok("".into())
        }
        async fn get_active_dispatch_for_scope(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
            Ok(None)
        }
        async fn get_dispatch(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
            Ok(None)
        }
        async fn get_active_dispatches_for_agent(
            &self,
            _: &str,
        ) -> Result<Vec<crate::domain::storage::AgentDispatch>, StorageError> {
            Ok(vec![])
        }
        async fn update_dispatch_status(&self, _: &str, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn update_dispatch_pr(&self, _: &str, _: u32) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl StateLogStorage for DownStorage {
        async fn store_state_block(
            &self,
            _: &crate::domain::state_log::StateBlock,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn last_state_block(
            &self,
        ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
            Ok(None)
        }
        async fn list_state_blocks(
            &self,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
            Ok(vec![])
        }
        async fn latest_state_blocks(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
            Ok(vec![])
        }
    }

    #[async_trait::async_trait]
    impl SubmissionStorage for DownStorage {
        async fn enqueue_verdict(
            &self,
            _: &str,
            _: &str,
            _: f64,
            _: f64,
            _: f64,
            _: f64,
            _: f64,
            _: f64,
            _: f64,
            _: u32,
            _: &str,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn list_pending_verdicts(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::verdict_queue::QueuedVerdict>, StorageError> {
            Ok(vec![])
        }
        async fn list_submitted_verdicts(
            &self,
            _: u32,
        ) -> Result<Vec<crate::domain::verdict_queue::QueuedVerdict>, StorageError> {
            Ok(vec![])
        }
        async fn get_queued_verdict(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::verdict_queue::QueuedVerdict>, StorageError> {
            Ok(None)
        }
        async fn update_verdict_submitted(&self, _: &str, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn update_verdict_confirmed(&self, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn update_verdict_failed(&self, _: &str, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
            Ok((0, 0, 0, 0))
        }
    }

    #[async_trait::async_trait]
    impl StoragePort for DownStorage {
        async fn ping(&self) -> Result<(), StorageError> {
            Err(StorageError::ConnectionFailed("test: storage down".into()))
        }
        async fn flush_usage(
            &self,
            _: &[(String, crate::domain::usage::DogUsage)],
        ) -> Result<(), StorageError> {
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl ProofOfHistoryStorage for DownStorage {
        async fn store_proof_of_history(&self, _: &ProofOfHistory) -> Result<String, StorageError> {
            Err(StorageError::ConnectionFailed("test: storage down".into()))
        }

        async fn list_proof_of_history(&self, _: u32) -> Result<Vec<ProofOfHistory>, StorageError> {
            Ok(vec![])
        }
    }
    #[async_trait::async_trait]
    impl crate::domain::storage::MeshStorage for DownStorage {
        async fn publish_mesh_key(
            &self,
            _mesh_ip: &str,
            _public_key: &str,
        ) -> Result<crate::domain::storage::MeshNode, StorageError> {
            Err(StorageError::ConnectionFailed("test: storage down".into()))
        }
        async fn list_mesh_nodes(
            &self,
        ) -> Result<Vec<crate::domain::storage::MeshNode>, StorageError> {
            Ok(vec![])
        }
        async fn list_mesh_routes(
            &self,
        ) -> Result<Vec<crate::domain::storage::MeshRoute>, StorageError> {
            Ok(vec![])
        }
    }
    fn test_mcp() -> CynicMcp {
        let dogs: Vec<Arc<dyn crate::domain::dog::Dog>> = vec![Arc::new(DeterministicDog)];
        let breakers: Vec<Arc<dyn crate::domain::health_gate::HealthGate>> = dogs
            .iter()
            .map(|d| {
                Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                    d.id().to_string(),
                )) as Arc<dyn crate::domain::health_gate::HealthGate>
            })
            .collect();
        let judge = Arc::new(Judge::new(dogs, breakers));
        let storage = Arc::new(NullStorage) as Arc<dyn StoragePort>;
        let coord = Arc::new(NullCoord) as Arc<dyn CoordPort>;
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        let embedding = Arc::new(cynic_inference::embedding::NullEmbedding)
            as Arc<dyn cynic_inference::embedding::EmbeddingPort>;
        let verdict_cache = Arc::new(crate::domain::verdict_cache::VerdictCache::new());
        let infer = Arc::new(cynic_inference::inference::NullInfer)
            as Arc<dyn cynic_inference::inference::InferPort>;
        let metrics = Arc::new(crate::domain::metrics::Metrics::new());
        let domain_router = Arc::new(crate::infra::domain_router::DomainRouter::from_backends(&[]));
        let routing_calc = Arc::new(crate::infra::routing_calc::RoutingCalculator::new());
        let dog_perf_collector =
            Arc::new(crate::infra::dog_performance::DogPerformanceCollector::new());
        let mcp = CynicMcp::new(
            judge,
            storage,
            coord,
            usage,
            embedding,
            verdict_cache,
            infer,
            metrics,
            Arc::new(std::sync::RwLock::new(None)),
            Arc::new(crate::infra::task_health::TaskHealth::new()),
            crate::domain::contract::SystemContract::new(vec!["deterministic-dog".into()], false),
            None,
            None,
            Arc::new(crate::domain::wisdom::DomainCurations::new()),
            domain_router,
            dog_perf_collector,
            routing_calc,
        );
        mcp.authenticated.store(true, Ordering::Relaxed);
        mcp
    }

    fn text_of(result: &CallToolResult) -> &str {
        &result.content[0]
            .as_text()
            .expect("Expected text content")
            .text
    }

    #[tokio::test]
    async fn health_returns_critical_with_null_storage() {
        let mcp = test_mcp();
        let result = mcp.cynic_health().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(v["status"], "critical");
        assert_eq!(v["storage"], "down");
        assert_eq!(v["dog_count"], 1);
        assert_eq!(v["dogs"][0]["id"], "deterministic-dog");
        assert!((v["phi_max"].as_f64().unwrap() - PHI_INV).abs() < 0.001);
    }

    #[tokio::test]
    async fn judge_with_deterministic_dog_returns_verdict() {
        let mcp = test_mcp();
        let params = Parameters(JudgeParams {
            content: "The Sicilian Defense is a strong opening.".into(),
            context: Some("Chess opening theory".into()),
            domain: Some("chess".into()),
            dogs: None,
            agent_id: Some("test-agent".into()),
            crystals: None,
            sensitivity: None,
            priority: None,
        });
        let result = mcp.cynic_judge(params).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert!(v["verdict_id"].is_string());
        assert!(v["q_score"]["total"].as_f64().unwrap() > 0.0);
        assert!(v["q_score"]["total"].as_f64().unwrap() <= PHI_INV);
        assert_eq!(v["dog_count"], 1);
        let verdict_str = v["verdict"].as_str().unwrap();
        assert!(
            ["Howl", "Wag", "Growl", "Bark"].contains(&verdict_str),
            "unexpected verdict: {verdict_str}"
        );
    }

    #[tokio::test]
    async fn verdicts_returns_error_with_null_storage() {
        let mcp = test_mcp();
        let params = Parameters(ListParams {
            limit: Some(5),
            agent_id: None,
        });
        let result = mcp.cynic_verdicts(params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn crystals_returns_error_with_null_storage() {
        let mcp = test_mcp();
        let params = Parameters(ListParams {
            limit: Some(5),
            agent_id: None,
        });
        let result = mcp.cynic_crystals(params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn coord_register_claim_release_cycle() {
        let mcp = test_mcp();
        let reg = mcp
            .cynic_coord_register(Parameters(RegisterParams {
                agent_id: "test-1".into(),
                intent: "testing".into(),
                agent_type: Some("test".into()),
            }))
            .await
            .unwrap();
        assert!(text_of(&reg).contains("test-1"));

        let claim = mcp
            .cynic_coord_claim(Parameters(ClaimParams {
                agent_id: "test-1".into(),
                target: "mod.rs".into(),
                claim_type: Some("file".into()),
            }))
            .await
            .unwrap();
        assert!(text_of(&claim).contains("Claimed"));

        let rel = mcp
            .cynic_coord_release(Parameters(ReleaseParams {
                agent_id: "test-1".into(),
                target: None,
            }))
            .await
            .unwrap();
        assert!(!text_of(&rel).is_empty());
    }

    #[tokio::test]
    async fn who_returns_empty_snapshot() {
        let mcp = test_mcp();
        let result = mcp
            .cynic_coord_who(Parameters(WhoParams { agent_id: None }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(v["active_agents"], 0);
        assert_eq!(v["active_claims"], 0);
    }

    #[tokio::test]
    async fn audit_query_returns_empty() {
        let mcp = test_mcp();
        let result = mcp
            .cynic_audit_query(Parameters(AuditQueryParams {
                tool: None,
                agent_id: None,
                since: None,
                until: None,
                limit: Some(10),
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert!(v.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn judge_tracks_usage() {
        let mcp = test_mcp();
        let params = Parameters(JudgeParams {
            content: "Test stimulus".into(),
            context: None,
            domain: None,
            dogs: None,
            agent_id: Some("usage-test".into()),
            crystals: None,
            sensitivity: None,
            priority: None,
        });
        let _ = mcp.cynic_judge(params).await.unwrap();
        let usage = mcp.usage.lock().await;
        let dogs = usage.merged_dogs();
        assert!(dogs.contains_key("deterministic-dog"));
        assert!(dogs["deterministic-dog"].requests >= 1);
    }

    #[tokio::test]
    async fn health_returns_critical_when_storage_down() {
        let dogs: Vec<Arc<dyn crate::domain::dog::Dog>> = vec![Arc::new(DeterministicDog)];
        let breakers: Vec<Arc<dyn crate::domain::health_gate::HealthGate>> = dogs
            .iter()
            .map(|d| {
                Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                    d.id().to_string(),
                )) as Arc<dyn crate::domain::health_gate::HealthGate>
            })
            .collect();
        let judge = Arc::new(Judge::new(dogs, breakers));
        let storage = Arc::new(DownStorage) as Arc<dyn StoragePort>;
        let coord = Arc::new(NullCoord) as Arc<dyn CoordPort>;
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        let embedding = Arc::new(cynic_inference::embedding::NullEmbedding)
            as Arc<dyn cynic_inference::embedding::EmbeddingPort>;
        let verdict_cache = Arc::new(crate::domain::verdict_cache::VerdictCache::new());
        let infer = Arc::new(cynic_inference::inference::NullInfer)
            as Arc<dyn cynic_inference::inference::InferPort>;
        let metrics = Arc::new(crate::domain::metrics::Metrics::new());
        let domain_router = Arc::new(crate::infra::domain_router::DomainRouter::from_backends(&[]));
        let routing_calc = Arc::new(crate::infra::routing_calc::RoutingCalculator::new());
        let dog_perf_collector =
            Arc::new(crate::infra::dog_performance::DogPerformanceCollector::new());
        let mcp = CynicMcp::new(
            judge,
            storage,
            coord,
            usage,
            embedding,
            verdict_cache,
            infer,
            metrics,
            Arc::new(std::sync::RwLock::new(None)),
            Arc::new(crate::infra::task_health::TaskHealth::new()),
            crate::domain::contract::SystemContract::new(vec!["deterministic-dog".into()], false),
            None,
            None,
            Arc::new(crate::domain::wisdom::DomainCurations::new()),
            domain_router,
            dog_perf_collector,
            routing_calc,
        );
        mcp.authenticated.store(true, Ordering::Relaxed);

        let result = mcp.cynic_health().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(v["status"], "critical");
        assert_eq!(v["storage"], "down");
        assert_eq!(v["dog_count"], 1);
    }

    #[tokio::test]
    async fn server_info_is_correct() {
        let mcp = test_mcp();
        let info = mcp.get_info();
        assert_eq!(info.server_info.name, "cynic-kernel");
        assert!(info.instructions.is_some());
    }

    // ── Poison input regression tests ─────────────────────────
    // Small LLMs produce null/empty/garbage args. Every tool must
    // return an error, never panic. Tests cover:
    //   (a) serde rejects {} for structs with required fields
    //   (b) serde accepts {} for all-optional structs (safe defaults)
    //   (c) handlers reject empty-string required fields

    // -- (a) Required-field structs reject empty JSON --

    #[test]
    fn poison_judge_params_rejects_empty() {
        let r = serde_json::from_value::<JudgeParams>(serde_json::json!({}));
        assert!(r.is_err(), "JudgeParams should require 'content'");
    }

    #[test]
    fn poison_auth_params_rejects_empty() {
        let r = serde_json::from_value::<AuthParams>(serde_json::json!({}));
        assert!(r.is_err(), "AuthParams should require 'api_key'");
    }

    #[test]
    fn poison_infer_params_rejects_empty() {
        let r = serde_json::from_value::<InferParams>(serde_json::json!({}));
        assert!(r.is_err(), "InferParams should require 'prompt'");
    }

    #[test]
    fn poison_register_params_rejects_empty() {
        let r = serde_json::from_value::<RegisterParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "RegisterParams should require 'agent_id' + 'intent'"
        );
    }

    #[test]
    fn poison_claim_params_rejects_empty() {
        let r = serde_json::from_value::<ClaimParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "ClaimParams should require 'agent_id' + 'target'"
        );
    }

    #[test]
    fn poison_batch_claim_params_rejects_empty() {
        let r = serde_json::from_value::<BatchClaimParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "BatchClaimParams should require 'agent_id' + 'targets'"
        );
    }

    #[test]
    fn poison_release_params_rejects_empty() {
        let r = serde_json::from_value::<ReleaseParams>(serde_json::json!({}));
        assert!(r.is_err(), "ReleaseParams should require 'agent_id'");
    }

    #[test]
    fn poison_observe_params_rejects_empty() {
        let r = serde_json::from_value::<ObserveParams>(serde_json::json!({}));
        assert!(r.is_err(), "ObserveParams should require 'tool'");
    }

    #[test]
    fn poison_dispatch_task_params_rejects_empty() {
        let r = serde_json::from_value::<DispatchAgentTaskParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "DispatchAgentTaskParams should require 'kind' + 'domain' + 'content'"
        );
    }

    fn activation_evidence_contract() -> serde_json::Value {
        serde_json::json!({
            "patch_field": "activation_evidence_patch",
            "node_field": "node",
            "runtime_paths_field": "activation_evidence.runtime_paths",
            "services_field": "activation_evidence.services",
            "verified_requires": ["marker_exists", "sentinels_verified"],
            "service_active_requires": ["declared", "active"],
            "ingestion_mode": "validated_merge",
        })
    }

    #[test]
    fn dispatch_agent_task_content_accepts_coord_targets_for_forge() {
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "objective": "build kernel release",
            "orchestrator_organ": "organ-forge",
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(r.is_ok());
    }

    #[test]
    fn dispatch_agent_task_content_rejects_forge_without_targets() {
        let content = serde_json::json!({
            "objective": "build kernel release"
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            r.is_err(),
            "repo-affecting forge tasks should declare coordination targets"
        );
    }

    #[test]
    fn dispatch_agent_task_content_rejects_forge_without_lifecycle_contract() {
        let content = serde_json::json!({
            "objective": "build kernel release",
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            r.is_err(),
            "repo-affecting forge tasks should declare lifecycle contract"
        );
    }

    #[test]
    fn dispatch_agent_task_content_follows_dynamic_registry_contract() {
        let registry = serde_json::json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-build",
                    "build_runner_role": "builder",
                    "deploy_target_role": "primary",
                    "deploy_target_flag": "ships_kernel",
                    "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                    "activation_evidence_contract": activation_evidence_contract()
                }
            },
            "nodes": [
                {"name": "node-builder", "role": "builder"},
                {"name": "node-primary", "role": "primary", "ships_kernel": true}
            ]
        });
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "orchestrator_organ": "organ-build",
            "build_target": "node-builder",
            "build_runner_role": "builder",
            "deploy_target": "node-primary",
            "deploy_target_role": "primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["infra/registry.json"]
        });

        let result = validate_forge_lifecycle_contract(&content, Some(&registry));

        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_agent_task_content_allows_non_forge_without_targets() {
        let r = validate_agent_task_content("hermes", "telegram", r#"{"objective":"observe"}"#);

        assert!(r.is_ok());
    }

    #[test]
    fn poison_list_pending_params_rejects_empty() {
        let r = serde_json::from_value::<ListPendingAgentTasksParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "ListPendingAgentTasksParams should require 'kind'"
        );
    }

    #[test]
    fn poison_watch_pending_params_rejects_empty() {
        let r = serde_json::from_value::<WatchPendingAgentTasksParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "WatchPendingAgentTasksParams should require 'kind'"
        );
    }

    #[test]
    fn poison_claim_task_params_rejects_empty() {
        let r = serde_json::from_value::<ClaimAgentTaskParams>(serde_json::json!({}));
        assert!(r.is_err(), "ClaimAgentTaskParams should require 'task_id'");
    }

    #[test]
    fn poison_update_task_result_params_rejects_empty() {
        let r = serde_json::from_value::<UpdateAgentTaskResultParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "UpdateAgentTaskResultParams should require 'task_id'"
        );
    }

    // -- (b) All-optional structs accept empty JSON safely --

    #[test]
    fn poison_list_params_accepts_empty() {
        let r = serde_json::from_value::<ListParams>(serde_json::json!({}));
        assert!(r.is_ok(), "ListParams (all optional) should accept {{}}");
    }

    #[test]
    fn poison_audit_query_params_accepts_empty() {
        let r = serde_json::from_value::<AuditQueryParams>(serde_json::json!({}));
        assert!(
            r.is_ok(),
            "AuditQueryParams (all optional) should accept {{}}"
        );
    }

    #[test]
    fn poison_who_params_accepts_empty() {
        let r = serde_json::from_value::<WhoParams>(serde_json::json!({}));
        assert!(r.is_ok(), "WhoParams (all optional) should accept {{}}");
    }

    // -- (c) Handler-level: empty-string required fields --

    #[tokio::test]
    async fn poison_judge_empty_content_rejected() {
        let mcp = test_mcp();
        let params = Parameters(JudgeParams {
            content: "   ".into(),
            context: None,
            domain: None,
            dogs: None,
            agent_id: None,
            crystals: None,
            sensitivity: None,
            priority: None,
        });
        let result = mcp.cynic_judge(params).await;
        assert!(
            result.is_err(),
            "empty/whitespace content should be rejected"
        );
    }

    #[tokio::test]
    async fn poison_infer_empty_prompt_rejected() {
        let mcp = test_mcp();
        let params = Parameters(InferParams {
            prompt: "".into(),
            system: None,
            agent_id: None,
            temperature: None,
            max_tokens: None,
        });
        let result = mcp.cynic_infer(params).await;
        assert!(result.is_err(), "empty prompt should be rejected");
    }

    #[tokio::test]
    async fn poison_infer_whitespace_prompt_rejected() {
        let mcp = test_mcp();
        let params = Parameters(InferParams {
            prompt: "   \n\t  ".into(),
            system: None,
            agent_id: None,
            temperature: None,
            max_tokens: None,
        });
        let result = mcp.cynic_infer(params).await;
        assert!(result.is_err(), "whitespace-only prompt should be rejected");
    }

    #[tokio::test]
    async fn poison_batch_claim_empty_targets_rejected() {
        let mcp = test_mcp();
        let params = Parameters(BatchClaimParams {
            agent_id: "test".into(),
            targets: vec![],
            claim_type: None,
        });
        let result = mcp.cynic_coord_claim_batch(params).await;
        assert!(result.is_err(), "empty targets vec should be rejected");
    }

    // -- (d) Null arguments via serde (simulates rmcp unwrap_or_default) --

    #[test]
    fn poison_null_arguments_become_empty_object() {
        // rmcp does: arguments.take().unwrap_or_default()
        // null → empty JsonObject. Verify required-field structs still fail.
        let empty = serde_json::Value::Object(serde_json::Map::new());
        assert!(serde_json::from_value::<JudgeParams>(empty.clone()).is_err());
        assert!(serde_json::from_value::<InferParams>(empty.clone()).is_err());
        assert!(serde_json::from_value::<RegisterParams>(empty.clone()).is_err());
        assert!(serde_json::from_value::<ObserveParams>(empty.clone()).is_err());
        assert!(serde_json::from_value::<DispatchAgentTaskParams>(empty.clone()).is_err());
        assert!(serde_json::from_value::<ScopeParams>(empty.clone()).is_err());
        // All-optional structs succeed
        assert!(serde_json::from_value::<ListParams>(empty.clone()).is_ok());
        assert!(serde_json::from_value::<WhoParams>(empty).is_ok());
    }

    #[test]
    fn observe_params_deserialize_minimal() {
        let json = r#"{"tool":"Read"}"#;
        let params: ObserveParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tool, "Read");
        assert!(params.target.is_none());
    }

    #[test]
    fn observe_params_deserialize_full() {
        let json = r#"{"tool":"Edit","target":"src/main.rs","domain":"rust","agent_id":"claude-123","session_id":"s1"}"#;
        let params: ObserveParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tool, "Edit");
        assert_eq!(params.agent_id.as_deref(), Some("claude-123"));
    }
}

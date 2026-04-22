//! CYNIC MCP Server — Model Context Protocol interface for AI agents.
//!
//! Exposes CYNIC kernel capabilities as MCP tools consumable by
//! Claude Code, Gemini CLI, Hermes Agent, or any MCP-compatible client.
//!
//! Architecture: thin wrappers around existing domain logic (Judge, StoragePort).
//! Zero duplication with REST handlers — both call the same core.
//!
//! Tools split by concern:
//! - judge_tools.rs: auth, judge, health, verdicts, crystals, infer, audit
//! - coord_tools.rs: register, claim, claim_batch, release, who
//! - observe_tools.rs: observe, validate, git

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter, model::*, tool_handler,
};
use schemars::JsonSchema;
use serde::Deserialize;

pub mod build_tools;
mod coord_tools;
mod judge_tools;
mod observe_tools;

use crate::domain::coord::CoordPort;
use crate::domain::events::KernelEvent;
use crate::domain::inference::InferPort;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::judge::Judge;

// ── MCP Rate Limiter ──────────────────────────────────────────

pub(crate) struct McpRateLimit {
    judge_calls: AtomicU64,
    other_calls: AtomicU64,
    window_start: std::sync::Mutex<std::time::Instant>,
}

impl McpRateLimit {
    fn new() -> Self {
        Self {
            judge_calls: AtomicU64::new(0),
            other_calls: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub(crate) fn check_judge(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.judge_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 10 {
            self.judge_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 10 judge/infer calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn check_other(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.other_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 30 {
            self.other_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 30 calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    fn maybe_reset(&self) {
        if let Ok(mut start) = self.window_start.lock()
            && start.elapsed() > std::time::Duration::from_secs(60)
        {
            self.judge_calls.store(0, Ordering::Relaxed);
            self.other_calls.store(0, Ordering::Relaxed);
            *start = std::time::Instant::now();
        }
    }
}

// ── Input Validation Helpers ──────────────────────────────────

pub(crate) fn validate_agent_id(agent_id: &Option<String>) -> Result<(), McpError> {
    if let Some(id) = agent_id {
        crate::domain::coord::validate_agent_id(id)
            .map_err(|msg| McpError::invalid_params(msg, None))?;
    }
    Ok(())
}

pub(crate) fn sanitize_error(category: &str) -> McpError {
    McpError::internal_error(format!("{category} unavailable — check /health"), None)
}

// ── MCP Tool Parameters ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgeParams {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    pub dogs: Option<Vec<String>>,
    pub agent_id: Option<String>,
    pub crystals: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InferParams {
    pub system: Option<String>,
    pub prompt: String,
    pub agent_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditQueryParams {
    pub tool: Option<String>,
    pub agent_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    pub api_key: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitParams {
    pub op: build_tools::GitOp,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParams {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimParams {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchClaimParams {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    pub agent_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    pub tool: String,
    pub target: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub context: Option<String>,
    pub project: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ── MCP Server ───────────────────────────────────────────────

#[derive(Clone)]
pub struct CynicMcp {
    pub(crate) judge: Arc<arc_swap::ArcSwap<Judge>>,
    pub(crate) storage: Arc<dyn StoragePort>,
    pub(crate) coord: Arc<dyn CoordPort>,
    pub(crate) usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    pub(crate) embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    pub(crate) verdict_cache: Arc<crate::domain::verdict_cache::VerdictCache>,
    pub(crate) infer: Arc<dyn InferPort>,
    pub(crate) metrics: Arc<crate::domain::metrics::Metrics>,
    pub(crate) environment:
        Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    pub(crate) task_health: Arc<crate::infra::task_health::TaskHealth>,
    pub(crate) system_contract: crate::domain::contract::SystemContract,
    pub(crate) event_tx: Option<tokio::sync::broadcast::Sender<KernelEvent>>,
    pub(crate) rate_limit: Arc<McpRateLimit>,
    pub(crate) bg_semaphore: Arc<tokio::sync::Semaphore>,
    pub(crate) authenticated: Arc<AtomicBool>,
    pub(crate) project_root: String,
    pub(crate) enricher: Option<Arc<dyn crate::domain::enrichment::TokenEnricherPort>>,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcp").finish_non_exhaustive()
    }
}

impl CynicMcp {
    #[allow(clippy::too_many_arguments)]
    // WHY: Constructor receives 9 kernel dependencies — each is a distinct port required at the MCP
    // surface. A builder pattern would only rename the argument list; called in exactly one place.
    pub fn new(
        judge: Arc<Judge>,
        storage: Arc<dyn StoragePort>,
        coord: Arc<dyn CoordPort>,
        usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
        embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
        verdict_cache: Arc<crate::domain::verdict_cache::VerdictCache>,
        infer: Arc<dyn InferPort>,
        metrics: Arc<crate::domain::metrics::Metrics>,
        environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
        task_health: Arc<crate::infra::task_health::TaskHealth>,
        system_contract: crate::domain::contract::SystemContract,
        event_tx: Option<tokio::sync::broadcast::Sender<KernelEvent>>,
        project_root: String,
        enricher: Option<Arc<dyn crate::domain::enrichment::TokenEnricherPort>>,
    ) -> Self {
        Self {
            judge: Arc::new(arc_swap::ArcSwap::from(judge)),
            storage,
            coord,
            embedding,
            verdict_cache,
            infer,
            metrics,
            environment,
            task_health,
            system_contract,
            usage,
            event_tx,
            enricher,
            rate_limit: Arc::new(McpRateLimit::new()),
            bg_semaphore: Arc::new(tokio::sync::Semaphore::new(
                crate::domain::constants::BG_SEMAPHORE_PERMITS,
            )),
            authenticated: Arc::new(AtomicBool::new(false)),
            project_root,
            tool_router: Self::tool_router_judge()
                + Self::tool_router_coord()
                + Self::tool_router_observe(),
        }
    }

    pub(crate) fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Not authenticated — call cynic_auth first",
                None,
            ));
        }
        Ok(())
    }

    pub(crate) async fn touch(&self, agent_id: &str) {
        if !agent_id.is_empty()
            && agent_id != "unknown"
            && self.coord.heartbeat(agent_id).await.is_err()
        {}
    }

    pub(crate) async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
        if let Err(e) = self
            .coord
            .store_audit(tool_name, agent_id, &details.to_string())
            .await
        {
            tracing::debug!(error = %e, tool_name, "MCP audit store failed (non-fatal)");
        }
        self.touch(agent_id).await;
    }
}

#[tool_handler]
impl ServerHandler for CynicMcp {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cynic-kernel", env!("CYNIC_VERSION")))
            .with_instructions("CYNIC epistemic immune system — independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;

    use crate::dogs::deterministic::DeterministicDog;
    use crate::domain::coord::NullCoord;
    use crate::domain::dog::PHI_INV;
    use crate::domain::storage::{NullStorage, StorageError};

    struct DownStorage;

    #[async_trait::async_trait]
    impl StoragePort for DownStorage {
        async fn ping(&self) -> Result<(), StorageError> {
            Err(StorageError::ConnectionFailed("test: storage down".into()))
        }
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
        async fn delete_crystal(&self, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        async fn observe_crystal(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: f64,
            _: &str,
            _: usize,
            _: &str,
            _: &str,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn store_observation(
            &self,
            _: &crate::domain::storage::Observation,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn query_observations(
            &self,
            _: &str,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::ObservationFrequency>, StorageError> {
            Ok(vec![])
        }
        async fn query_session_targets(
            &self,
            _: &str,
            _: u32,
        ) -> Result<Vec<crate::domain::storage::SessionTarget>, StorageError> {
            Ok(vec![])
        }
        async fn flush_usage(
            &self,
            _: &[(String, crate::domain::usage::DogUsage)],
        ) -> Result<(), StorageError> {
            Ok(())
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
        let embedding = Arc::new(crate::domain::embedding::NullEmbedding)
            as Arc<dyn crate::domain::embedding::EmbeddingPort>;
        let verdict_cache = Arc::new(crate::domain::verdict_cache::VerdictCache::new());
        let infer = Arc::new(crate::domain::inference::NullInfer)
            as Arc<dyn crate::domain::inference::InferPort>;
        let metrics = Arc::new(crate::domain::metrics::Metrics::new());
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
            "/tmp".to_string(),
            None,
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
        let embedding = Arc::new(crate::domain::embedding::NullEmbedding)
            as Arc<dyn crate::domain::embedding::EmbeddingPort>;
        let verdict_cache = Arc::new(crate::domain::verdict_cache::VerdictCache::new());
        let infer = Arc::new(crate::domain::inference::NullInfer)
            as Arc<dyn crate::domain::inference::InferPort>;
        let metrics = Arc::new(crate::domain::metrics::Metrics::new());
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
            "/tmp".to_string(),
            None,
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

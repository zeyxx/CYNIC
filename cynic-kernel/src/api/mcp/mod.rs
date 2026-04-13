//! CYNIC MCP Server — Model Context Protocol interface for AI agents.
//!
//! Exposes CYNIC kernel capabilities as MCP tools consumable by
//! Claude Code, Gemini CLI, Hermes Agent, or any MCP-compatible client.
//!
//! Architecture: thin wrappers around existing domain logic (Judge, StoragePort).
//! Zero duplication with REST handlers — both call the same core.
//!
//! Inspiration credit: NousResearch/hermes-agent (tool architecture patterns).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::domain::ccm::build_observation;
use crate::domain::coord::{ClaimResult, CoordPort};
use crate::domain::dog::{AXIOM_NAMES, PHI_INV};
use crate::domain::events::KernelEvent;
use crate::domain::health_gate::count_healthy_dogs;
use crate::domain::inference::InferPort;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::judge::Judge;

// ── MCP Rate Limiter ──────────────────────────────────────────
// RC1 fix: MCP had zero rate limiting. Same limits as REST.
// Simple sliding window — resets every 60s. Single-client (stdio).

struct McpRateLimit {
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

    /// Check + increment. Returns Err if limit exceeded.
    fn check_judge(&self) -> Result<(), McpError> {
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

    fn check_other(&self) -> Result<(), McpError> {
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

/// RC1: Validate agent_id — delegates to domain::coord::validate_agent_id.
fn validate_agent_id(agent_id: &Option<String>) -> Result<(), McpError> {
    if let Some(id) = agent_id {
        crate::domain::coord::validate_agent_id(id)
            .map_err(|msg| McpError::invalid_params(msg, None))?;
    }
    Ok(())
}

/// RC1: Sanitize error messages — no internal URLs, model names, or stack traces.
fn sanitize_error(category: &str) -> McpError {
    McpError::internal_error(format!("{category} unavailable — check /health"), None)
}

// ── MCP Tool Parameters ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgeParams {
    /// Content to evaluate (text, code, decision description)
    pub content: String,
    /// Optional context (structured JSON or natural language)
    pub context: Option<String>,
    /// Domain hint: "chess", "trading", "code", "general"
    pub domain: Option<String>,
    /// Filter: evaluate with only these Dog IDs
    pub dogs: Option<Vec<String>>,
    /// Agent identity for audit trail (default: "unknown")
    pub agent_id: Option<String>,
    /// Disable crystal injection for A/B testing (default: true)
    pub crystals: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    /// Maximum number of items to return (default 20, max 100)
    pub limit: Option<u32>,
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InferParams {
    /// System prompt for the LLM
    pub system: Option<String>,
    /// User message / prompt
    pub prompt: String,
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
    /// Temperature (0.0-1.0, default 0.7)
    pub temperature: Option<f64>,
    /// Max tokens (default 2048, max 8192)
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditQueryParams {
    /// Filter by tool name (optional)
    pub tool: Option<String>,
    /// Filter by agent_id (optional)
    pub agent_id: Option<String>,
    /// Maximum results (default 20)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    /// API key — must match the kernel's CYNIC_API_KEY
    pub api_key: String,
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
}

// ── Coordination Tool Parameters ─────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParams {
    /// Unique agent identifier (e.g. "claude-session-abc", "gemini-1")
    pub agent_id: String,
    /// What this agent intends to do (human-readable)
    pub intent: String,
    /// Agent type: "claude", "gemini", "hermes", "human"
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimParams {
    /// Agent identifier (must match a registered session)
    pub agent_id: String,
    /// What to claim: file path, feature name, or zone (e.g. "rest.rs", "auth-system", "kernel/")
    pub target: String,
    /// Claim type: "file", "feature", "zone"
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchClaimParams {
    /// Agent identifier (must match a registered session)
    pub agent_id: String,
    /// List of targets to claim (max 20)
    pub targets: Vec<String>,
    /// Claim type: "file", "feature", "zone" (applied to all targets)
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    /// Agent identifier
    pub agent_id: String,
    /// Target to release (if omitted, releases ALL claims for this agent)
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoParams {
    /// Filter by agent_id (optional — show only this agent's state)
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    /// Tool or action name (1-64 chars)
    pub tool: String,
    /// Target file, resource, or entity
    pub target: Option<String>,
    /// Domain classification (auto-inferred from target if omitted)
    pub domain: Option<String>,
    /// Status: ok, warning, error
    pub status: Option<String>,
    /// Additional context (max 200 chars, truncated if longer)
    pub context: Option<String>,
    /// Project identifier
    pub project: Option<String>,
    /// Agent identifier for session tracking
    pub agent_id: Option<String>,
    /// Session identifier for CCM aggregation
    pub session_id: Option<String>,
}

// ── MCP Server ───────────────────────────────────────────────

/// CYNIC MCP Server — exposes kernel capabilities to AI agents.
#[derive(Clone)]
pub struct CynicMcp {
    judge: Arc<arc_swap::ArcSwap<Judge>>,
    storage: Arc<dyn StoragePort>,
    coord: Arc<dyn CoordPort>,
    usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    verdict_cache: Arc<crate::domain::verdict_cache::VerdictCache>,
    /// Sovereign inference — routed through InferPort, not raw HTTP (Rule #17).
    infer: Arc<dyn InferPort>,
    metrics: Arc<crate::domain::metrics::Metrics>,
    /// Probe environment snapshot — shared with REST for honest status.
    environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    /// Background task health — shared with REST for honest status.
    task_health: Arc<crate::infra::task_health::TaskHealth>,
    /// Self-model: expected system state. Shared with REST for K13 (one function, both surfaces).
    system_contract: crate::domain::contract::SystemContract,
    /// Kernel event bus — shared with REST SSE. None only in tests.
    event_tx: Option<tokio::sync::broadcast::Sender<KernelEvent>>,
    /// RC1: Rate limiter — shared across all tool calls.
    rate_limit: Arc<McpRateLimit>,
    /// Bound fire-and-forget spawns (observe, audit). Mirrors REST bg_semaphore.
    bg_semaphore: Arc<tokio::sync::Semaphore>,
    /// RC1-1: Session authentication state. Set to true after successful cynic_auth call.
    authenticated: Arc<AtomicBool>,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcp").finish_non_exhaustive()
    }
}

#[tool_router]
impl CynicMcp {
    #[allow(clippy::too_many_arguments)]
    // WHY: Constructor receives 9 kernel dependencies (judge, storage, coord, usage, embedding,
    // verdict_cache, infer, metrics, environment) — each is a distinct port required at the MCP
    // surface. A builder pattern would only rename the argument list; it does not reduce coupling
    // or improve readability for a constructor that is called in exactly one place (main.rs).
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
            rate_limit: Arc::new(McpRateLimit::new()),
            bg_semaphore: Arc::new(tokio::sync::Semaphore::new(
                crate::domain::constants::BG_SEMAPHORE_PERMITS,
            )),
            authenticated: Arc::new(AtomicBool::new(false)),
            tool_router: Self::tool_router(),
        }
    }

    // ── AUTH ──────────────────────────────────────────────────

    #[tool(
        name = "cynic_auth",
        description = "Authenticate this MCP session. Required before calling sensitive tools (judge, observe, validate, git, coord). Pass the CYNIC_API_KEY. Call once per session."
    )]
    async fn cynic_auth(&self, params: Parameters<AuthParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        let expected = std::env::var("CYNIC_API_KEY").unwrap_or_default();
        if expected.is_empty() {
            return Err(McpError::internal_error(
                "Kernel has no CYNIC_API_KEY configured",
                None,
            ));
        }

        if p.api_key != expected {
            tracing::warn!(agent_id, "MCP auth failed — invalid key");
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Authentication failed — invalid API key",
                None,
            ));
        }

        self.authenticated.store(true, Ordering::Relaxed);
        tracing::info!(agent_id, "MCP session authenticated");

        Ok(CallToolResult::success(vec![Content::text(
            r#"{"authenticated": true}"#,
        )]))
    }

    // ── cynic_judge ──────────────────────────────────────────

    #[tool(
        name = "cynic_judge",
        description = "Submit content for epistemic evaluation by CYNIC's independent AI validators (Dogs). Returns φ-bounded Q-Score, per-axiom breakdown (FIDELITY/PHI/VERIFY/CULTURE/BURN/SOVEREIGNTY), verdict (HOWL/WAG/GROWL/BARK), and multi-model reasoning. Max confidence: 61.8%."
    )]
    async fn cynic_judge(
        &self,
        params: Parameters<JudgeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        // Input validation — same limits as REST to prevent token drain
        if p.content.trim().is_empty() {
            return Err(McpError::invalid_params("content must not be empty", None));
        }
        if p.content.chars().count() > 4_000 {
            return Err(McpError::invalid_params("content exceeds 4000 chars", None));
        }
        if let Some(ref ctx) = p.context
            && ctx.chars().count() > 2_000
        {
            return Err(McpError::invalid_params("context exceeds 2000 chars", None));
        }
        if let Some(ref domain) = p.domain
            && domain.len() > 64
        {
            return Err(McpError::invalid_params("domain exceeds 64 chars", None));
        }

        // Shared pipeline: embed → cache → crystals → sessions → evaluate → store → CCM
        let judge = self.judge.load_full();
        let deps = crate::pipeline::PipelineDeps {
            judge: &judge,
            storage: self.storage.as_ref(),
            embedding: self.embedding.as_ref(),
            usage: &self.usage,
            verdict_cache: &self.verdict_cache,
            metrics: &self.metrics,
            event_tx: self.event_tx.as_ref(),
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            on_dog: None,
            expected_dog_count: judge.dog_ids().len(),
        };
        let result = crate::pipeline::run(
            p.content.clone(),
            p.context,
            p.domain.clone(),
            p.dogs.as_deref(),
            p.crystals.unwrap_or(true),
            &deps,
        )
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "MCP judge pipeline failed");
            sanitize_error("Judge")
        })?;

        let verdict = match result {
            crate::pipeline::PipelineResult::CacheHit {
                verdict: cached,
                similarity,
            } => {
                self.touch(&agent_id).await;
                let response = serde_json::json!({
                    "verdict": format!("{:?}", cached.kind),
                    "q_score": cached.q_score,
                    "reasoning": cached.reasoning,
                    "dogs_used": cached.dog_id,
                    "stimulus_summary": cached.stimulus_summary,
                    "cache_hit": similarity,
                    "source": "verdict_cache",
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_default(),
                )]));
            }
            crate::pipeline::PipelineResult::Evaluated { verdict } => verdict,
        };

        let _ = self
            .audit(
                // ok: fire-and-forget (audit is best-effort, verdict already committed)
                "cynic_judge",
                &agent_id,
                &serde_json::json!({
                    "stimulus": p.content.chars().take(200).collect::<String>(),
                    "dogs_used": verdict.dog_id,
                    "verdict": format!("{:?}", verdict.kind),
                    "q_score": verdict.q_score.total,
                }),
            )
            .await;

        // Format response
        let response = serde_json::json!({
            "verdict_id": verdict.id,
            "verdict": format!("{:?}", verdict.kind),
            "q_score": {
                "total": verdict.q_score.total,
                "fidelity": verdict.q_score.fidelity,
                "phi": verdict.q_score.phi,
                "verify": verdict.q_score.verify,
                "culture": verdict.q_score.culture,
                "burn": verdict.q_score.burn,
                "sovereignty": verdict.q_score.sovereignty,
            },
            "reasoning": {
                "fidelity": verdict.reasoning.fidelity,
                "phi": verdict.reasoning.phi,
                "verify": verdict.reasoning.verify,
                "culture": verdict.reasoning.culture,
                "burn": verdict.reasoning.burn,
                "sovereignty": verdict.reasoning.sovereignty,
            },
            "dogs_used": verdict.dog_id,
            "dog_count": verdict.dog_scores.len(),
            "anomaly_detected": verdict.anomaly_detected,
            "phi_max": PHI_INV,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    // ── cynic_health ─────────────────────────────────────────

    #[tool(
        name = "cynic_health",
        description = "Get CYNIC kernel health: active Dogs, circuit breaker states, storage status, axioms, φ constants. Use this to verify the kernel is operational before submitting judgments."
    )]
    async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let judge = self.judge.load_full();
        let dog_health = judge.dog_health();
        let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);
        let dogs: Vec<serde_json::Value> = dog_health.into_iter().map(|(id, circuit, failures)| {
            serde_json::json!({ "id": id, "circuit": circuit, "failures": failures })
        }).collect();

        let storage_ok = self.storage.ping().await.is_ok();

        // K13: same health logic as REST
        let live_dog_ids = judge.dog_ids();
        let contract_delta = self.system_contract.assess(&live_dog_ids);

        let probes_degraded =
            crate::domain::probe::EnvironmentSnapshot::is_degraded(&self.environment);
        let stale_tasks = self.task_health.readiness_stale_tasks();
        let assessment = crate::domain::health_gate::system_health_assessment_with_contract(
            healthy_dogs,
            total_dogs,
            storage_ok,
            probes_degraded,
            &stale_tasks,
            Some(&contract_delta),
        );

        let response = serde_json::json!({
            "status": assessment.status,
            "dogs": dogs,
            "dog_count": total_dogs,
            "healthy_dogs": healthy_dogs,
            "storage": if storage_ok { "connected" } else { "down" },
            "axioms": AXIOM_NAMES,
            "phi_max": PHI_INV,
            "contract": {
                "expected_dogs": self.system_contract.expected_dogs(),
                "expected_count": self.system_contract.expected_count(),
                "missing_dogs": &contract_delta.missing,
                "unexpected_dogs": &contract_delta.unexpected,
                "fulfilled": contract_delta.fulfilled,
            },
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    // ── cynic_verdicts ───────────────────────────────────────

    #[tool(
        name = "cynic_verdicts",
        description = "List recent CYNIC verdicts. Use to review judgment history and track quality."
    )]
    async fn cynic_verdicts(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        validate_agent_id(&params.0.agent_id)?;
        let limit = params.0.limit.unwrap_or(20).min(100);
        // Refresh heartbeat if agent_id provided (keeps session alive on read-only calls)
        if let Some(ref agent_id) = params.0.agent_id {
            self.touch(agent_id).await;
        }
        let verdicts = self.storage.list_verdicts(limit).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP storage query failed");
            sanitize_error("Storage")
        })?;

        let items: Vec<serde_json::Value> = verdicts
            .iter()
            .map(|v| {
                serde_json::json!({
                    "id": v.id,
                    "verdict": format!("{:?}", v.kind),
                    "q_score": v.q_score.total,
                    "stimulus": v.stimulus_summary.chars().take(100).collect::<String>(),
                    "dogs_used": v.dog_id,
                    "anomaly": v.anomaly_detected,
                })
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&items).unwrap_or_default(),
        )]))
    }

    // ── cynic_crystals ───────────────────────────────────────

    #[tool(
        name = "cynic_crystals",
        description = "List crystallized truths from CYNIC's CCM. States: Forming → Crystallized → Canonical → Decaying → Dissolved."
    )]
    async fn cynic_crystals(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        validate_agent_id(&params.0.agent_id)?;
        let limit = params.0.limit.unwrap_or(20).min(100);
        if let Some(ref agent_id) = params.0.agent_id {
            self.touch(agent_id).await;
        }
        let crystals = self.storage.list_crystals(limit).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP storage query failed");
            sanitize_error("Storage")
        })?;

        let items: Vec<serde_json::Value> = crystals
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "content": c.content,
                    "domain": c.domain,
                    "confidence": c.confidence,
                    "observations": c.observations,
                    "state": c.state.to_string(),
                })
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&items).unwrap_or_default(),
        )]))
    }

    // ── cynic_infer ──────────────────────────────────────────

    #[tool(
        name = "cynic_infer",
        description = "Run sovereign local inference via llama-server (no cloud API, no quota). Use for tasks that should stay local: summarization, classification, extraction."
    )]
    async fn cynic_infer(
        &self,
        params: Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?; // Same limit as judge — both use LLM resources
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        // Input validation
        if p.prompt.chars().count() > 8_000 {
            return Err(McpError::invalid_params("prompt exceeds 8000 chars", None));
        }
        if let Some(ref sys) = p.system
            && sys.chars().count() > 4_000
        {
            return Err(McpError::invalid_params(
                "system prompt exceeds 4000 chars",
                None,
            ));
        }

        let temperature = p.temperature.unwrap_or(0.7).clamp(0.0, 2.0);
        let max_tokens = p.max_tokens.unwrap_or(2048).min(8192);

        // Route through InferPort — single adapter for all sovereign LLM calls (Rule #17)
        let request = crate::domain::inference::InferRequest {
            system: p.system,
            prompt: p.prompt.clone(),
            temperature,
            max_tokens,
        };
        let infer_resp = self.infer.infer(&request, None).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP infer failed");
            sanitize_error("Inference")
        })?;

        let _ = self
            .audit(
                "cynic_infer",
                &agent_id,
                &serde_json::json!({ // ok: fire-and-forget
                    "prompt_len": p.prompt.len(),
                    "prompt_tokens": infer_resp.prompt_tokens,
                    "completion_tokens": infer_resp.completion_tokens,
                }),
            )
            .await;

        let response = serde_json::json!({
            "text": infer_resp.text,
            // RC1: model name omitted — leaks backend identity (exact GGUF, quantization)
            "prompt_tokens": infer_resp.prompt_tokens,
            "completion_tokens": infer_resp.completion_tokens,
            "sovereign": true,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    // ── cynic_audit_query ────────────────────────────────────

    #[tool(
        name = "cynic_audit_query",
        description = "Query the audit trail of all MCP actions. Every tool call is logged. Use to review agent history, detect anomalies, or coordinate between agents."
    )]
    async fn cynic_audit_query(
        &self,
        params: Parameters<AuditQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(20).min(100);

        match self
            .coord
            .query_audit(p.tool.as_deref(), p.agent_id.as_deref(), limit)
            .await
        {
            Ok(results) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into()),
            )])),
            Err(e) => {
                tracing::warn!(error = %e, "MCP audit query failed");
                Err(sanitize_error("Audit"))
            }
        }
    }

    // ── COORDINATION TOOLS ──────────────────────────────────────

    #[tool(
        name = "cynic_coord_register",
        description = "Register an agent session with CYNIC. Call at session start. Every subsequent MCP call refreshes the heartbeat. Sessions expire after 5 minutes of inactivity."
    )]
    async fn cynic_coord_register(
        &self,
        params: Parameters<RegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&Some(p.agent_id.clone()))?;
        if p.intent.is_empty() || p.intent.chars().count() > 500 {
            return Err(McpError::invalid_params(
                "intent must be 1-500 characters",
                None,
            ));
        }
        let agent_type = p.agent_type.unwrap_or_else(|| "unknown".into());
        if agent_type.len() > 64 {
            return Err(McpError::invalid_params(
                "agent_type must be under 64 chars",
                None,
            ));
        }

        self.coord
            .register_agent(&p.agent_id, &agent_type, &p.intent)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP register failed");
                sanitize_error("Coordination")
            })?;

        self.audit(
            "cynic_coord_register",
            &p.agent_id,
            &serde_json::json!({
                "intent": p.intent, "agent_type": agent_type,
            }),
        )
        .await;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(KernelEvent::SessionRegistered {
                agent_id: p.agent_id.clone(),
            });
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Agent '{}' registered. Intent: {}. Heartbeat refreshed on every MCP call.",
            p.agent_id, p.intent
        ))]))
    }

    #[tool(
        name = "cynic_coord_claim",
        description = "Claim a file, feature, or zone before working on it. Prevents other agents from conflicting. Returns existing claims if conflict detected."
    )]
    async fn cynic_coord_claim(
        &self,
        params: Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&Some(p.agent_id.clone()))?;
        if p.target.is_empty() || p.target.len() > 256 {
            return Err(McpError::invalid_params(
                "target must be 1-256 characters",
                None,
            ));
        }
        let claim_type = p.claim_type.unwrap_or_else(|| "file".into());
        if claim_type.len() > 64 {
            return Err(McpError::invalid_params(
                "claim_type must be under 64 chars",
                None,
            ));
        }

        match self
            .coord
            .claim(&p.agent_id, &p.target, &claim_type)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP claim failed");
                sanitize_error("Coordination")
            })? {
            ClaimResult::Conflict(infos) => {
                let conflict_info: Vec<String> = infos
                    .iter()
                    .map(|c| format!("{} (since {})", c.agent_id, c.claimed_at))
                    .collect();
                self.audit(
                    "cynic_coord_claim",
                    &p.agent_id,
                    &serde_json::json!({
                        "target": p.target, "claim_type": claim_type,
                        "result": "conflict", "held_by": conflict_info,
                    }),
                )
                .await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "CONFLICT: '{}' already claimed by: {}. Coordinate before proceeding.",
                    p.target,
                    conflict_info.join(", ")
                ))]))
            }
            ClaimResult::Claimed => {
                self.audit(
                    "cynic_coord_claim",
                    &p.agent_id,
                    &serde_json::json!({
                        "target": p.target, "claim_type": claim_type,
                    }),
                )
                .await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Claimed '{}' ({}) for agent '{}'.",
                    p.target, claim_type, p.agent_id
                ))]))
            }
        }
    }

    #[tool(
        name = "cynic_coord_claim_batch",
        description = "Claim multiple files/features/zones in one call. More efficient than calling cynic_coord_claim repeatedly. Returns per-target results (claimed vs conflict)."
    )]
    async fn cynic_coord_claim_batch(
        &self,
        params: Parameters<BatchClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&Some(p.agent_id.clone()))?;
        let claim_type = p.claim_type.unwrap_or_else(|| "file".into());
        if claim_type.len() > 64 {
            return Err(McpError::invalid_params(
                "claim_type must be under 64 chars",
                None,
            ));
        }

        if p.targets.is_empty() {
            return Err(McpError::invalid_params("targets must not be empty", None));
        }
        if p.targets.len() > 20 {
            return Err(McpError::invalid_params(
                "batch claim limited to 20 targets",
                None,
            ));
        }

        let result = self
            .coord
            .claim_batch(&p.agent_id, &p.targets, &claim_type)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP batch claim failed");
                sanitize_error("Coordination")
            })?;

        let mut lines = Vec::new();

        for target in &result.claimed {
            lines.push(format!(
                "CLAIMED: '{}' ({}) for agent '{}'.",
                target, claim_type, p.agent_id
            ));
        }
        for (target, infos) in &result.conflicts {
            let conflict_info: Vec<String> = infos
                .iter()
                .map(|c| format!("{} (since {})", c.agent_id, c.claimed_at))
                .collect();
            lines.push(format!(
                "CONFLICT: '{}' already claimed by: {}.",
                target,
                conflict_info.join(", ")
            ));
        }

        self.audit(
            "cynic_coord_claim_batch",
            &p.agent_id,
            &serde_json::json!({
                "targets": p.targets,
                "claimed": result.claimed.len(),
                "conflicts": result.conflicts.len(),
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(
        name = "cynic_coord_release",
        description = "Release claims on files/features. If no target specified, releases ALL claims for this agent. Call at session end or when done with a file."
    )]
    async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&Some(p.agent_id.clone()))?;
        if let Some(ref t) = p.target
            && (t.is_empty() || t.len() > 256)
        {
            return Err(McpError::invalid_params(
                "target must be 1-256 characters",
                None,
            ));
        }

        let desc = self
            .coord
            .release(&p.agent_id, p.target.as_deref())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP release failed");
                sanitize_error("Coordination")
            })?;

        self.audit(
            "cynic_coord_release",
            &p.agent_id,
            &serde_json::json!({
                "target": p.target,
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(desc)]))
    }

    #[tool(
        name = "cynic_coord_who",
        description = "Show active agents, their intents, and current file/feature claims. Use before starting work to avoid conflicts. Also expires stale sessions (>5 min no heartbeat)."
    )]
    async fn cynic_coord_who(
        &self,
        params: Parameters<WhoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let p = params.0;

        let snapshot = self.coord.who(p.agent_id.as_deref()).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP who query failed");
            sanitize_error("Coordination")
        })?;

        let summary = snapshot.into_summary();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".into()),
        )]))
    }

    // ── cynic_observe ────────────────────────────────────────

    #[tool(
        name = "cynic_observe",
        description = "Record a development workflow observation (tool use, file edit, command). Used by Claude Code hooks and agents to feed CYNIC's CCM crystal learning pipeline. Fire-and-forget: returns immediately, stores asynchronously."
    )]
    async fn cynic_observe(
        &self,
        params: Parameters<ObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let params = params.0;

        if params.tool.is_empty() || params.tool.len() > 64 {
            return Err(McpError::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                "tool must be 1-64 characters",
                None,
            ));
        }

        let tool_name = params.tool.clone();
        let agent_id = params.agent_id.clone().unwrap_or_else(|| "unknown".into());

        let obs = build_observation(
            params.tool,
            params.target,
            params.domain,
            params.status,
            params.context,
            params.project,
            params.agent_id,
            params.session_id,
        );

        let storage = Arc::clone(&self.storage);
        // Bounded spawn — mirrors REST observe.rs pattern. Semaphore prevents unbounded task accumulation.
        match self.bg_semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                tokio::spawn(async move {
                    let _permit = permit; // held until task completes
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        storage.store_observation(&obs),
                    )
                    .await
                    {
                        Ok(Err(e)) => tracing::warn!(error = %e, "cynic_observe: store failed"),
                        Err(_) => tracing::warn!("cynic_observe: store timed out (5s)"),
                        _ => {}
                    }
                });
            }
            Err(_) => {
                tracing::warn!("cynic_observe: bg_semaphore full, observation dropped");
            }
        }

        self.audit(
            "cynic_observe",
            &agent_id,
            &serde_json::json!({ "tool": tool_name }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            r#"{"status":"observed"}"#,
        )]))
    }

    // ── Helpers ────────────────────────────────────────────────

    /// RC1-1: Check session authentication. Returns Err if not authenticated.
    fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Not authenticated — call cynic_auth first",
                None,
            ));
        }
        Ok(())
    }

    /// Refresh heartbeat for any agent that identifies itself.
    /// Called by every tool via audit() — fixes stale-session problem.
    async fn touch(&self, agent_id: &str) {
        if !agent_id.is_empty()
            && agent_id != "unknown"
            && self.coord.heartbeat(agent_id).await.is_err()
        {
            // Already logged inside heartbeat impl
        }
    }

    /// Audit + heartbeat in one shot (best-effort, non-blocking).
    async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
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
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("cynic-kernel", env!("CYNIC_VERSION")))
        .with_instructions("CYNIC epistemic immune system — independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dogs::deterministic::DeterministicDog;
    use crate::domain::coord::NullCoord;
    use crate::domain::storage::{NullStorage, StorageError};
    use crate::domain::usage::DogUsageTracker;
    use crate::judge::Judge;

    /// Storage that fails on ping() — simulates SurrealDB being down.
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

    /// Build a CynicMcp with real DeterministicDog + null storage/coord.
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
        let infer = Arc::new(crate::domain::inference::NullInfer) as Arc<dyn InferPort>;
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
        );
        mcp.authenticated.store(true, Ordering::Relaxed);
        mcp
    }

    /// Extract text from the first Content element in a CallToolResult.
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
        // NullStorage.ping() returns Err → storage_ok = false → critical
        // (honest: NullStorage IS degraded mode — it should not claim "connected")
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
        // Must have a verdict and q_score
        assert!(v["verdict_id"].is_string());
        assert!(v["q_score"]["total"].as_f64().unwrap() > 0.0);
        assert!(v["q_score"]["total"].as_f64().unwrap() <= PHI_INV);
        assert_eq!(v["dog_count"], 1);
        // Verdict must be a known kind
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
        // NullStorage returns ConnectionFailed for list_verdicts
        let result = mcp.cynic_verdicts(params).await;
        assert!(
            result.is_err(),
            "NullStorage should propagate error for list_verdicts"
        );
    }

    #[tokio::test]
    async fn crystals_returns_error_with_null_storage() {
        let mcp = test_mcp();
        let params = Parameters(ListParams {
            limit: Some(5),
            agent_id: None,
        });
        // NullStorage returns Err (RC5 fix: no silent Ok(()) on unavailable storage)
        let result = mcp.cynic_crystals(params).await;
        assert!(
            result.is_err(),
            "NullStorage should propagate error for list_crystals"
        );
    }

    #[tokio::test]
    async fn coord_register_claim_release_cycle() {
        let mcp = test_mcp();

        // Register
        let reg = mcp
            .cynic_coord_register(Parameters(RegisterParams {
                agent_id: "test-1".into(),
                intent: "testing".into(),
                agent_type: Some("test".into()),
            }))
            .await
            .unwrap();
        assert!(text_of(&reg).contains("test-1"));

        // Claim
        let claim = mcp
            .cynic_coord_claim(Parameters(ClaimParams {
                agent_id: "test-1".into(),
                target: "mod.rs".into(),
                claim_type: Some("file".into()),
            }))
            .await
            .unwrap();
        assert!(text_of(&claim).contains("Claimed"));

        // Release
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

        // Verify usage was recorded
        let usage = mcp.usage.lock().await;
        let dogs = usage.merged_dogs();
        assert!(
            dogs.contains_key("deterministic-dog"),
            "Usage should be tracked for deterministic-dog"
        );
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
        let infer = Arc::new(crate::domain::inference::NullInfer) as Arc<dyn InferPort>;
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
        );
        mcp.authenticated.store(true, Ordering::Relaxed);

        let result = mcp.cynic_health().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(
            v["status"], "critical",
            "Storage down should force critical status"
        );
        assert_eq!(v["storage"], "down");
        assert_eq!(v["dog_count"], 1); // dog is healthy, but storage pulls status to critical
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
        assert!(params.agent_id.is_none());
    }

    #[test]
    fn observe_params_deserialize_full() {
        let json = r#"{"tool":"Edit","target":"src/main.rs","domain":"rust","agent_id":"claude-123","session_id":"s1"}"#;
        let params: ObserveParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tool, "Edit");
        assert_eq!(params.agent_id.as_deref(), Some("claude-123"));
    }
}

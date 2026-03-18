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

use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    model::*,
    tool, tool_router, tool_handler,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::domain::coord::{CoordPort, ClaimResult};
use crate::domain::dog::{Stimulus, PHI_INV};
use crate::judge::Judge;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::ccm;

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

// ── MCP Server ───────────────────────────────────────────────

/// CYNIC MCP Server — exposes kernel capabilities to AI agents.
#[derive(Clone)]
pub struct CynicMcp {
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    coord: Arc<dyn CoordPort>,
    usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CynicMcp {
    pub fn new(
        judge: Arc<Judge>,
        storage: Arc<dyn StoragePort>,
        coord: Arc<dyn CoordPort>,
        usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    ) -> Self {
        Self {
            judge,
            storage,
            coord,
            usage,
            tool_router: Self::tool_router(),
        }
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
        let p = params.0;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        // Input validation — same limits as REST to prevent token drain
        if p.content.trim().is_empty() {
            return Err(McpError::invalid_params("content must not be empty", None));
        }
        if p.content.len() > 4_000 {
            return Err(McpError::invalid_params("content exceeds 4000 chars", None));
        }
        if let Some(ref ctx) = p.context
            && ctx.len() > 2_000
        {
            return Err(McpError::invalid_params("context exceeds 2000 chars", None));
        }

        // CCM feedback: enrich context with crystallized wisdom
        let domain_hint = p.domain.as_deref().unwrap_or("general");
        let enriched_context = match self.storage.list_crystals(50).await {
            Ok(crystals) => {
                let crystal_ctx = crate::domain::ccm::format_crystal_context(&crystals, domain_hint, 800);
                match (p.context, crystal_ctx) {
                    (Some(ctx), Some(cc)) => Some(format!("{}\n\n{}", ctx, cc)),
                    (Some(ctx), None) => Some(ctx),
                    (None, Some(cc)) => Some(cc),
                    (None, None) => None,
                }
            }
            Err(_) => p.context,
        };

        let stimulus = Stimulus {
            content: p.content,
            context: enriched_context,
            domain: p.domain,
        };

        let dogs_filter = p.dogs.as_deref();
        let verdict = self.judge.evaluate(&stimulus, dogs_filter).await
            .map_err(|e| McpError::internal_error(format!("Judge error: {}", e), None))?;

        // Side-effect order matches REST: store → usage → audit
        // Store verdict first (best effort — don't fail the response)
        if let Err(e) = self.storage.store_verdict(&verdict).await {
            eprintln!("[MCP] Warning: failed to store verdict: {}", e);
        }

        // Track usage
        {
            let mut usage = self.usage.lock().await;
            for ds in &verdict.dog_scores {
                usage.record(&ds.dog_id, ds.prompt_tokens, ds.completion_tokens, ds.latency_ms);
            }
        }

        // Audit (best effort)
        let _ = self.audit("cynic_judge", &agent_id, &serde_json::json!({
            "stimulus": stimulus.content.chars().take(200).collect::<String>(),
            "dogs_used": verdict.dog_id,
            "verdict": format!("{:?}", verdict.kind),
            "q_score": verdict.q_score.total,
        })).await;

        // CCM: observe crystal atomically (no read-modify-write race)
        {
            let crystal_id = format!("{:x}", ccm::content_hash(&format!("{}:{}", stimulus.domain.as_deref().unwrap_or("general"), verdict.stimulus_summary)));
            let domain = stimulus.domain.clone().unwrap_or_else(|| "general".to_string());
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) = self.storage.observe_crystal(
                &crystal_id, &verdict.stimulus_summary, &domain, verdict.q_score.total, &now
            ).await {
                eprintln!("[MCP/CCM] Warning: failed to observe crystal: {}", e);
            }
        }

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

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&response).unwrap_or_default())
        ]))
    }

    // ── cynic_health ─────────────────────────────────────────

    #[tool(
        name = "cynic_health",
        description = "Get CYNIC kernel health: active Dogs, circuit breaker states, axioms, φ constants. Use this to verify the kernel is operational before submitting judgments."
    )]
    async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        let dog_health = self.judge.dog_health();
        let dogs: Vec<serde_json::Value> = dog_health.into_iter().map(|(id, circuit, failures)| {
            serde_json::json!({ "id": id, "circuit": circuit, "failures": failures })
        }).collect();

        let dog_count = dogs.len();
        let status = if dog_count == 0 { "critical" }
            else if dog_count == 1 { "degraded" }
            else { "sovereign" };

        let response = serde_json::json!({
            "status": status,
            "dogs": dogs,
            "dog_count": dog_count,
            "axioms": ["FIDELITY", "PHI", "VERIFY/FALSIFY", "CULTURE", "BURN", "SOVEREIGNTY"],
            "phi_max": PHI_INV,
        });

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&response).unwrap_or_default())
        ]))
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
        let limit = params.0.limit.unwrap_or(20).min(100);
        let verdicts = self.storage.list_verdicts(limit).await
            .map_err(|e| McpError::internal_error(format!("Storage error: {}", e), None))?;

        let items: Vec<serde_json::Value> = verdicts.iter().map(|v| {
            serde_json::json!({
                "id": v.id,
                "verdict": format!("{:?}", v.kind),
                "q_score": v.q_score.total,
                "stimulus": v.stimulus_summary.chars().take(100).collect::<String>(),
                "dogs_used": v.dog_id,
                "anomaly": v.anomaly_detected,
            })
        }).collect();

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&items).unwrap_or_default())
        ]))
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
        let limit = params.0.limit.unwrap_or(20).min(100);
        let crystals = self.storage.list_crystals(limit).await
            .map_err(|e| McpError::internal_error(format!("Storage error: {}", e), None))?;

        let items: Vec<serde_json::Value> = crystals.iter().map(|c| {
            serde_json::json!({
                "id": c.id,
                "content": c.content,
                "domain": c.domain,
                "confidence": c.confidence,
                "observations": c.observations,
                "state": format!("{:?}", c.state),
            })
        }).collect();

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&items).unwrap_or_default())
        ]))
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
        let p = params.0;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        let sovereign_url = std::env::var("CYNIC_SOVEREIGN_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let api_key = std::env::var("CYNIC_SOVEREIGN_KEY").unwrap_or_default();

        let mut messages = vec![
            serde_json::json!({"role": "user", "content": p.prompt}),
        ];
        if let Some(sys) = &p.system {
            messages.insert(0, serde_json::json!({"role": "system", "content": sys}));
        }

        let request = serde_json::json!({
            "model": "local",
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        let client = reqwest::Client::new();
        let mut req = client.post(format!("{}/v1/chat/completions", sovereign_url))
            .json(&request)
            .timeout(std::time::Duration::from_secs(120));

        if !api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req.send().await
            .map_err(|e| McpError::internal_error(format!("Inference error: {}", e), None))?;
        let data: serde_json::Value = resp.json().await
            .map_err(|e| McpError::internal_error(format!("Parse error: {}", e), None))?;

        let text = data["choices"][0]["message"]["content"].as_str().unwrap_or("(no response)");
        let usage = &data["usage"];

        let _ = self.audit("cynic_infer", &agent_id, &serde_json::json!({
            "prompt_len": p.prompt.len(),
            "prompt_tokens": usage["prompt_tokens"],
            "completion_tokens": usage["completion_tokens"],
        })).await;

        let response = serde_json::json!({
            "text": text,
            "model": data["model"].as_str().unwrap_or("local"),
            "prompt_tokens": usage["prompt_tokens"],
            "completion_tokens": usage["completion_tokens"],
            "sovereign": true,
        });

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&response).unwrap_or_default())
        ]))
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
        let p = params.0;
        let limit = p.limit.unwrap_or(20).min(100);

        match self.coord.query_audit(p.tool.as_deref(), p.agent_id.as_deref(), limit).await {
            Ok(results) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into()))
            ])),
            Err(e) => Ok(CallToolResult::success(vec![
                Content::text(format!("Audit query failed: {}", e))
            ])),
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
        let p = params.0;
        let agent_type = p.agent_type.unwrap_or_else(|| "unknown".into());

        self.coord.register_agent(&p.agent_id, &agent_type, &p.intent).await
            .map_err(|e| McpError::internal_error(format!("Register failed: {}", e), None))?;

        self.audit("cynic_coord_register", &p.agent_id, &serde_json::json!({
            "intent": p.intent, "agent_type": agent_type,
        })).await;

        Ok(CallToolResult::success(vec![
            Content::text(format!("Agent '{}' registered. Intent: {}. Heartbeat refreshed on every MCP call.", p.agent_id, p.intent))
        ]))
    }

    #[tool(
        name = "cynic_coord_claim",
        description = "Claim a file, feature, or zone before working on it. Prevents other agents from conflicting. Returns existing claims if conflict detected."
    )]
    async fn cynic_coord_claim(
        &self,
        params: Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let claim_type = p.claim_type.unwrap_or_else(|| "file".into());

        match self.coord.claim(&p.agent_id, &p.target, &claim_type).await
            .map_err(|e| McpError::internal_error(format!("Claim failed: {}", e), None))?
        {
            ClaimResult::Conflict(infos) => {
                let conflict_info: Vec<String> = infos.iter().map(|c| {
                    format!("{} (since {})", c.agent_id, c.claimed_at)
                }).collect();
                Ok(CallToolResult::success(vec![
                    Content::text(format!("CONFLICT: '{}' already claimed by: {}. Coordinate before proceeding.",
                        p.target, conflict_info.join(", ")))
                ]))
            }
            ClaimResult::Claimed => {
                self.audit("cynic_coord_claim", &p.agent_id, &serde_json::json!({
                    "target": p.target, "claim_type": claim_type,
                })).await;
                Ok(CallToolResult::success(vec![
                    Content::text(format!("Claimed '{}' ({}) for agent '{}'.", p.target, claim_type, p.agent_id))
                ]))
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
        let p = params.0;
        let claim_type = p.claim_type.unwrap_or_else(|| "file".into());

        if p.targets.is_empty() {
            return Err(McpError::invalid_params("targets must not be empty", None));
        }
        if p.targets.len() > 20 {
            return Err(McpError::invalid_params("batch claim limited to 20 targets", None));
        }

        let result = self.coord.claim_batch(&p.agent_id, &p.targets, &claim_type).await
            .map_err(|e| McpError::internal_error(format!("Batch claim failed: {}", e), None))?;

        let mut lines = Vec::new();

        for target in &result.claimed {
            lines.push(format!("CLAIMED: '{}' ({}) for agent '{}'.", target, claim_type, p.agent_id));
        }
        for (target, infos) in &result.conflicts {
            let conflict_info: Vec<String> = infos.iter().map(|c| {
                format!("{} (since {})", c.agent_id, c.claimed_at)
            }).collect();
            lines.push(format!("CONFLICT: '{}' already claimed by: {}.", target, conflict_info.join(", ")));
        }

        self.audit("cynic_coord_claim_batch", &p.agent_id, &serde_json::json!({
            "targets": p.targets,
            "claimed": result.claimed.len(),
            "conflicts": result.conflicts.len(),
        })).await;

        Ok(CallToolResult::success(vec![
            Content::text(lines.join("\n"))
        ]))
    }

    #[tool(
        name = "cynic_coord_release",
        description = "Release claims on files/features. If no target specified, releases ALL claims for this agent. Call at session end or when done with a file."
    )]
    async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        let desc = self.coord.release(&p.agent_id, p.target.as_deref()).await
            .map_err(|e| McpError::internal_error(format!("Release failed: {}", e), None))?;

        self.audit("cynic_coord_release", &p.agent_id, &serde_json::json!({
            "target": p.target,
        })).await;

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
        let p = params.0;

        let snapshot = self.coord.who(p.agent_id.as_deref()).await
            .map_err(|e| McpError::internal_error(format!("Who failed: {}", e), None))?;

        let response = serde_json::json!({
            "active_agents": snapshot.agents.len(),
            "active_claims": snapshot.claims.len(),
            "agents": snapshot.agents,
            "claims": snapshot.claims,
        });

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".into()))
        ]))
    }

    // ── Helpers ────────────────────────────────────────────────

    /// Refresh heartbeat for any agent that identifies itself.
    /// Called by every tool via audit() — fixes stale-session problem.
    async fn touch(&self, agent_id: &str) {
        if !agent_id.is_empty() && agent_id != "unknown" {
            let _ = self.coord.heartbeat(agent_id).await;
        }
    }

    /// Audit + heartbeat in one shot (best-effort, non-blocking).
    async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
        let _ = self.coord.store_audit(tool_name, agent_id, details).await;
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
    use crate::domain::storage::NullStorage;
    use crate::domain::coord::NullCoord;
    use crate::domain::usage::DogUsageTracker;
    use crate::dogs::deterministic::DeterministicDog;
    use crate::judge::Judge;

    /// Build a CynicMcp with real DeterministicDog + null storage/coord.
    fn test_mcp() -> CynicMcp {
        let judge = Arc::new(Judge::new(vec![Box::new(DeterministicDog)]));
        let storage = Arc::new(NullStorage) as Arc<dyn StoragePort>;
        let coord = Arc::new(NullCoord) as Arc<dyn CoordPort>;
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        CynicMcp::new(judge, storage, coord, usage)
    }

    /// Extract text from the first Content element in a CallToolResult.
    fn text_of(result: &CallToolResult) -> &str {
        &result.content[0].as_text().expect("Expected text content").text
    }

    #[tokio::test]
    async fn health_returns_degraded_with_one_dog() {
        let mcp = test_mcp();
        let result = mcp.cynic_health().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        // 1 dog = degraded (not sovereign, not critical)
        assert_eq!(v["status"], "degraded");
        assert_eq!(v["dog_count"], 1);
        assert_eq!(v["dogs"][0]["id"], "deterministic-dog");
        // φ constant present
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
        assert!(["Howl", "Wag", "Growl", "Bark"].iter().any(|k| verdict_str == *k),
            "unexpected verdict: {}", verdict_str);
    }

    #[tokio::test]
    async fn verdicts_returns_error_with_null_storage() {
        let mcp = test_mcp();
        let params = Parameters(ListParams { limit: Some(5), agent_id: None });
        // NullStorage returns ConnectionFailed for list_verdicts
        let result = mcp.cynic_verdicts(params).await;
        assert!(result.is_err(), "NullStorage should propagate error for list_verdicts");
    }

    #[tokio::test]
    async fn crystals_returns_empty_with_null_storage() {
        let mcp = test_mcp();
        let params = Parameters(ListParams { limit: Some(5), agent_id: None });
        // NullStorage returns Ok(vec![]) for list_crystals
        let result = mcp.cynic_crystals(params).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn coord_register_claim_release_cycle() {
        let mcp = test_mcp();

        // Register
        let reg = mcp.cynic_coord_register(Parameters(RegisterParams {
            agent_id: "test-1".into(),
            intent: "testing".into(),
            agent_type: Some("test".into()),
        })).await.unwrap();
        assert!(text_of(&reg).contains("test-1"));

        // Claim
        let claim = mcp.cynic_coord_claim(Parameters(ClaimParams {
            agent_id: "test-1".into(),
            target: "mod.rs".into(),
            claim_type: Some("file".into()),
        })).await.unwrap();
        assert!(text_of(&claim).contains("Claimed"));

        // Release
        let rel = mcp.cynic_coord_release(Parameters(ReleaseParams {
            agent_id: "test-1".into(),
            target: None,
        })).await.unwrap();
        assert!(!text_of(&rel).is_empty());
    }

    #[tokio::test]
    async fn who_returns_empty_snapshot() {
        let mcp = test_mcp();
        let result = mcp.cynic_coord_who(Parameters(WhoParams { agent_id: None })).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert_eq!(v["active_agents"], 0);
        assert_eq!(v["active_claims"], 0);
    }

    #[tokio::test]
    async fn audit_query_returns_empty() {
        let mcp = test_mcp();
        let result = mcp.cynic_audit_query(Parameters(AuditQueryParams {
            tool: None,
            agent_id: None,
            limit: Some(10),
        })).await.unwrap();
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
        });
        let _ = mcp.cynic_judge(params).await.unwrap();

        // Verify usage was recorded
        let usage = mcp.usage.lock().await;
        let dogs = usage.merged_dogs();
        assert!(dogs.contains_key("deterministic-dog"),
            "Usage should be tracked for deterministic-dog");
        assert!(dogs["deterministic-dog"].requests >= 1);
    }

    #[tokio::test]
    async fn server_info_is_correct() {
        let mcp = test_mcp();
        let info = mcp.get_info();
        assert_eq!(info.server_info.name, "cynic-kernel");
        assert!(info.instructions.is_some());
    }
}

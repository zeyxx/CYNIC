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

use crate::dog::{Stimulus, PHI_INV};
use crate::judge::Judge;
use crate::storage_port::StoragePort;
use crate::storage_http::SurrealHttpStorage;
use crate::rest::DogUsageTracker;

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

// ── MCP Server ───────────────────────────────────────────────

/// CYNIC MCP Server — exposes kernel capabilities to AI agents.
#[derive(Clone)]
pub struct CynicMcp {
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    raw_db: Option<Arc<SurrealHttpStorage>>,
    usage: Arc<std::sync::Mutex<DogUsageTracker>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CynicMcp {
    pub fn new(
        judge: Arc<Judge>,
        storage: Arc<dyn StoragePort>,
        raw_db: Option<Arc<SurrealHttpStorage>>,
        usage: Arc<std::sync::Mutex<DogUsageTracker>>,
    ) -> Self {
        Self {
            judge,
            storage,
            raw_db,
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

        let stimulus = Stimulus {
            content: p.content,
            context: p.context,
            domain: p.domain,
        };

        let dogs_filter = p.dogs.as_deref();
        let verdict = self.judge.evaluate(&stimulus, dogs_filter).await
            .map_err(|e| McpError::internal_error(format!("Judge error: {}", e), None))?;

        // Track usage
        {
            let mut usage = self.usage.lock().unwrap();
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

        // Store verdict (best effort)
        let _ = self.storage.store_verdict(&verdict).await;

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
        let mut conditions = Vec::new();

        if let Some(tool) = &p.tool {
            conditions.push(format!("tool = '{}'", tool.replace('\\', "\\\\").replace('\'', "\\'")));
        }
        if let Some(agent) = &p.agent_id {
            conditions.push(format!("agent_id = '{}'", agent.replace('\\', "\\\\").replace('\'', "\\'")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let query = format!(
            "SELECT * FROM mcp_audit{} ORDER BY ts DESC LIMIT {};",
            where_clause, limit
        );

        let Some(db) = &self.raw_db else {
            return Ok(CallToolResult::success(vec![
                Content::text("Audit unavailable (storage in DEGRADED mode)")
            ]));
        };

        match db.query_one(&query).await {
            Ok(results) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into()))
            ])),
            Err(e) => Ok(CallToolResult::success(vec![
                Content::text(format!("Audit query failed: {}", e))
            ])),
        }
    }

    // ── Audit helper (best-effort, non-blocking) ─────────────

    async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
        let Some(db) = &self.raw_db else { return };

        let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
        let safe_details = escape(&details.to_string());
        let query = format!(
            "CREATE mcp_audit SET ts = time::now(), tool = '{}', agent_id = '{}', details = '{}';",
            escape(tool_name), escape(agent_id), safe_details,
        );

        let _ = db.query_one(&query).await;
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
        .with_server_info(Implementation::new("cynic-kernel", env!("CARGO_PKG_VERSION")))
        .with_instructions("CYNIC epistemic immune system — independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.")
    }
}

//! MCP Judge tools — auth, judge, health, verdicts, crystals, infer, audit.

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};

use crate::domain::dog::{AXIOM_NAMES, PHI_INV};
use crate::domain::health_gate::count_healthy_dogs;

use super::{
    AuditQueryParams, AuthParams, CynicMcp, InferParams, JudgeParams, ListParams, sanitize_error,
    validate_agent_id,
};

#[tool_router(router = tool_router_judge, vis = "pub(super)")]
impl CynicMcp {
    #[tool(
        name = "cynic_auth",
        description = "Authenticate this MCP session. Required before calling sensitive tools (judge, observe, validate, git, coord). Pass the CYNIC_API_KEY. Call once per session."
    )]
    pub(crate) async fn cynic_auth(
        &self,
        params: Parameters<AuthParams>,
    ) -> Result<CallToolResult, McpError> {
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

        self.authenticated
            .store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(agent_id, "MCP session authenticated");

        Ok(CallToolResult::success(vec![Content::text(
            r#"{"authenticated": true}"#,
        )]))
    }

    #[tool(
        name = "cynic_judge",
        description = "Submit content for epistemic evaluation by CYNIC's independent AI validators (Dogs). Returns φ-bounded Q-Score, per-axiom breakdown (FIDELITY/PHI/VERIFY/CULTURE/BURN/SOVEREIGNTY), verdict (HOWL/WAG/GROWL/BARK), and multi-model reasoning. Max confidence: 61.8%."
    )]
    pub(crate) async fn cynic_judge(
        &self,
        params: Parameters<JudgeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

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

    #[tool(
        name = "cynic_health",
        description = "Get CYNIC kernel health: active Dogs, circuit breaker states, storage status, axioms, φ constants. Use this to verify the kernel is operational before submitting judgments."
    )]
    pub(crate) async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let judge = self.judge.load_full();
        let dog_health = judge.dog_health();
        let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);
        let dogs: Vec<serde_json::Value> = dog_health
            .into_iter()
            .map(|(id, circuit, failures)| {
                serde_json::json!({ "id": id, "circuit": circuit, "failures": failures })
            })
            .collect();

        let storage_ok = self.storage.ping().await.is_ok();

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

    #[tool(
        name = "cynic_verdicts",
        description = "List recent CYNIC verdicts. Use to review judgment history and track quality."
    )]
    pub(crate) async fn cynic_verdicts(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        validate_agent_id(&params.0.agent_id)?;
        let limit = params.0.limit.unwrap_or(20).min(100);
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

    #[tool(
        name = "cynic_crystals",
        description = "List crystallized truths from CYNIC's CCM. States: Forming → Crystallized → Canonical → Decaying → Dissolved."
    )]
    pub(crate) async fn cynic_crystals(
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

    #[tool(
        name = "cynic_infer",
        description = "Run sovereign local inference via llama-server (no cloud API, no quota). Use for tasks that should stay local: summarization, classification, extraction."
    )]
    pub(crate) async fn cynic_infer(
        &self,
        params: Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

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
                &serde_json::json!({
                    "prompt_len": p.prompt.len(),
                    "prompt_tokens": infer_resp.prompt_tokens,
                    "completion_tokens": infer_resp.completion_tokens,
                }),
            )
            .await;

        let response = serde_json::json!({
            "text": infer_resp.text,
            "prompt_tokens": infer_resp.prompt_tokens,
            "completion_tokens": infer_resp.completion_tokens,
            "sovereign": true,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_audit_query",
        description = "Query the audit trail of all MCP actions. Every tool call is logged. Use to review agent history, detect anomalies, or coordinate between agents."
    )]
    pub(crate) async fn cynic_audit_query(
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
}

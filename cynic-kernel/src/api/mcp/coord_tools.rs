//! MCP Coordination tools — agent registration, claims, release.

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};

use crate::domain::coord::ClaimResult;
use crate::domain::events::KernelEvent;

use super::{
    BatchClaimParams, ClaimParams, CynicMcp, RegisterParams, ReleaseParams, WhoParams,
    sanitize_error, validate_agent_id,
};

#[tool_router(router = tool_router_coord, vis = "pub(super)")]
impl CynicMcp {
    #[tool(
        name = "cynic_coord_register",
        description = "Register an agent session with CYNIC. Call at session start. Every subsequent MCP call refreshes the heartbeat. Sessions expire after 5 minutes of inactivity."
    )]
    pub(crate) async fn cynic_coord_register(
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
    pub(crate) async fn cynic_coord_claim(
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
    pub(crate) async fn cynic_coord_claim_batch(
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
    pub(crate) async fn cynic_coord_release(
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
    pub(crate) async fn cynic_coord_who(
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
}

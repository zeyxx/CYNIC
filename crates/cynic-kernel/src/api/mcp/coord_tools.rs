//! MCP Coordination tools — agent registration, claims, release.

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};

use crate::domain::coord::{ClaimResult, LeaseGuard};
use crate::domain::events::KernelEvent;

use super::{
    BatchClaimParams, ClaimParams, CynicMcp, FinalizeLeaseParams, RegisterParams, ReleaseParams,
    RenewLeaseParams, ScopeParams, WhoParams, sanitize_error, validate_agent_id,
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
        self.require_coord_principal(&p.agent_id)?;
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
        self.require_coord_principal(&p.agent_id)?;
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
            ClaimResult::Claimed(grant) => {
                self.audit(
                    "cynic_coord_claim",
                    &p.agent_id,
                    &serde_json::json!({
                        "target": p.target, "claim_type": claim_type, "grant": &grant,
                    }),
                )
                .await;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({
                        "status": "claimed",
                        "agent_id": p.agent_id,
                        "target": p.target,
                        "claim_type": claim_type,
                        "lease": grant,
                    })
                    .to_string(),
                )]))
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
        self.require_coord_principal(&p.agent_id)?;
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

        self.audit(
            "cynic_coord_claim_batch",
            &p.agent_id,
            &serde_json::json!({
                "targets": p.targets,
                "claimed": result.claimed_count(),
                "conflicts": result.conflict_count(),
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
        )]))
    }

    #[tool(
        name = "cynic_coord_renew",
        description = "Renew an active claim using the exact lease id and fencing token returned by claim. Stale or superseded guards are rejected."
    )]
    pub(crate) async fn cynic_coord_renew(
        &self,
        params: Parameters<RenewLeaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let guard = validated_lease_guard(p.agent_id, p.target, p.lease_id, p.fencing_token)?;
        self.require_coord_principal(&guard.agent_id)?;
        let result = self.coord.renew_lease(&guard).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP lease renew failed");
            sanitize_error("Coordination")
        })?;
        self.audit(
            "cynic_coord_renew",
            &guard.agent_id,
            &serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
        )]))
    }

    #[tool(
        name = "cynic_coord_release",
        description = "Release a claim by providing target, lease_id, and fencing_token for that lease."
    )]
    pub(crate) async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let guard = validated_lease_guard(p.agent_id, p.target, p.lease_id, p.fencing_token)?;
        self.require_coord_principal(&guard.agent_id)?;
        let result = self.coord.release(&guard).await.map_err(|e| {
            tracing::warn!(error = %e, "MCP release failed");
            sanitize_error("Coordination")
        })?;
        self.audit(
            "cynic_coord_release",
            &guard.agent_id,
            &serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
        )]))
    }

    #[tool(
        name = "cynic_coord_finalize",
        description = "Validate and atomically commit a runtime proof bundle while finalizing one active claim using its exact lease id and fencing token."
    )]
    pub(crate) async fn cynic_coord_finalize(
        &self,
        params: Parameters<FinalizeLeaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let proof_bundle: cynic_protocol::RuntimeProofBundle =
            serde_json::from_value(p.proof_bundle).map_err(|error| {
                McpError::invalid_params(format!("invalid runtime proof bundle: {error}"), None)
            })?;
        let guard = validated_lease_guard(p.agent_id, p.target, p.lease_id, p.fencing_token)?;
        self.require_coord_principal(&guard.agent_id)?;
        let now_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u128;
        let proof = crate::domain::lease_proof::ValidatedLeaseProof::validate(
            &proof_bundle,
            &guard.target,
            now_unix_ms,
        )
        .map_err(|message| McpError::invalid_params(message, None))?;
        let result = self
            .coord
            .finalize_lease(&guard, &proof)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP lease finalize failed");
                sanitize_error("Coordination")
            })?;
        self.audit(
            "cynic_coord_finalize",
            &guard.agent_id,
            &serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
        )]))
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

    #[tool(
        name = "cynic_coord_scope",
        description = "Update the task scope for your agent session. Call after the user's first message. Shows in /coord/who to help detect overlap between concurrent cortex."
    )]
    pub(crate) async fn cynic_coord_scope(
        &self,
        params: Parameters<ScopeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&Some(p.agent_id.clone()))?;
        self.require_coord_principal(&p.agent_id)?;
        if p.scope.is_empty() || p.scope.chars().count() > 500 {
            return Err(McpError::invalid_params(
                "scope must be 1-500 characters",
                None,
            ));
        }
        self.coord
            .scope_update(&p.agent_id, &p.scope)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "MCP scope update failed");
                sanitize_error("Coordination")
            })?;
        self.audit(
            "cynic_coord_scope",
            &p.agent_id,
            &serde_json::json!({ "scope": p.scope }),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Scope updated for '{}': {}",
            p.agent_id, p.scope
        ))]))
    }
}

fn validated_lease_guard(
    agent_id: String,
    target: String,
    lease_id: String,
    fencing_token: u64,
) -> Result<LeaseGuard, McpError> {
    validate_agent_id(&Some(agent_id.clone()))?;
    if target.is_empty() || target.len() > 256 || lease_id.is_empty() || lease_id.len() > 256 {
        return Err(McpError::invalid_params(
            "target and lease_id must be 1-256 characters",
            None,
        ));
    }
    if fencing_token == 0 {
        return Err(McpError::invalid_params(
            "fencing_token must be greater than zero",
            None,
        ));
    }
    Ok(LeaseGuard {
        agent_id,
        target,
        lease_id,
        fencing_token,
    })
}

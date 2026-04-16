//! MCP Observe + Build tools — observation recording, validation, git.

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};

use crate::domain::ccm::build_observation;

use super::{CynicMcp, GitParams, ObserveParams, ValidateParams};

#[tool_router(router = tool_router_observe, vis = "pub(super)")]
impl CynicMcp {
    #[tool(
        name = "cynic_observe",
        description = "Record a development workflow observation (tool use, file edit, command). Used by Claude Code hooks and agents to feed CYNIC's CCM crystal learning pipeline. Fire-and-forget: returns immediately, stores asynchronously."
    )]
    pub(crate) async fn cynic_observe(
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
            params.tags,
        );

        let storage = Arc::clone(&self.storage);
        match self.bg_semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                tokio::spawn(async move {
                    let _permit = permit;
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

    #[tool(
        name = "cynic_validate",
        description = "Run the full validation pipeline: cargo build --tests + cargo clippy + cargo test. Returns pass/fail with stdout/stderr. Requires authentication. Takes ~2-5 minutes."
    )]
    pub(crate) async fn cynic_validate(
        &self,
        params: Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let agent_id = params.0.agent_id.unwrap_or_else(|| "unknown".into());

        tracing::info!(agent_id, "cynic_validate started");
        let result = super::build_tools::run_validate(&self.project_root).await;

        self.audit(
            "cynic_validate",
            &agent_id,
            &serde_json::json!({
                "passed": result.passed,
                "build_ok": result.build_ok,
                "clippy_ok": result.clippy_ok,
                "test_ok": result.test_ok,
                "duration_ms": result.duration_ms,
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result)
                .unwrap_or_else(|_| r#"{"error":"serialize failed"}"#.into()),
        )]))
    }

    #[tool(
        name = "cynic_git",
        description = "Git operations: status, log, diff, commit. For commit: provide message + file list. No push (deploy = human decision). Requires authentication."
    )]
    pub(crate) async fn cynic_git(
        &self,
        params: Parameters<GitParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

        let result = super::build_tools::run_git(&self.project_root, &p.op).await;

        self.audit(
            "cynic_git",
            &agent_id,
            &serde_json::json!({
                "op": format!("{:?}", p.op),
                "success": result.success,
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result)
                .unwrap_or_else(|_| r#"{"error":"serialize failed"}"#.into()),
        )]))
    }
}

//! MCP Agent tools — task dispatch and queue polling.

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};
use uuid::Uuid;

use crate::domain::storage::AgentTask;

use super::{
    CynicMcp, DispatchAgentTaskParams, ListPendingAgentTasksParams, UpdateAgentTaskResultParams,
    sanitize_error, validate_agent_id,
};

#[tool_router(router = tool_router_agent, vis = "pub(super)")]
impl CynicMcp {
    #[tool(
        name = "cynic_dispatch_agent_task",
        description = "Submit a task to the agent queue. Hermes, Nightshift, or future agents poll pending tasks and execute. Returns task_id."
    )]
    pub(crate) async fn cynic_dispatch_agent_task(
        &self,
        params: Parameters<DispatchAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.kind.is_empty() || p.kind.len() > 64 {
            return Err(McpError::invalid_params(
                "kind must be 1-64 characters",
                None,
            ));
        }
        if p.domain.is_empty() || p.domain.len() > 64 {
            return Err(McpError::invalid_params(
                "domain must be 1-64 characters",
                None,
            ));
        }
        if p.content.is_empty() || p.content.chars().count() > 10_000 {
            return Err(McpError::invalid_params(
                "content must be 1-10000 characters",
                None,
            ));
        }

        let task_id = format!("agent-task:{}", Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        let task = AgentTask {
            id: task_id.clone(),
            kind: p.kind.clone(),
            domain: p.domain.clone(),
            content: p.content,
            status: "pending".to_string(),
            result: None,
            created_at: now,
            completed_at: None,
            agent_id: p.agent_id.clone(),
            error: None,
        };

        self.storage
            .store_agent_task(&task)
            .await
            .map_err(|_| sanitize_error("Agent task store"))?;

        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());
        self.audit(
            "cynic_dispatch_agent_task",
            &agent_id,
            &serde_json::json!({
                "kind": p.kind,
                "domain": p.domain,
                "task_id": task_id,
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "task_id": task_id,
                "status": "pending",
                "kind": p.kind,
                "domain": p.domain,
            })
            .to_string(),
        )]))
    }

    #[tool(
        name = "cynic_list_pending_agent_tasks",
        description = "Poll agent task queue for pending tasks of a specific kind. Used by agents (Hermes, Nightshift) to discover work."
    )]
    pub(crate) async fn cynic_list_pending_agent_tasks(
        &self,
        params: Parameters<ListPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.kind.is_empty() || p.kind.len() > 64 {
            return Err(McpError::invalid_params(
                "kind must be 1-64 characters",
                None,
            ));
        }

        let limit = p.limit.unwrap_or(10).min(100);
        let tasks = self
            .storage
            .list_pending_agent_tasks(&p.kind, limit)
            .await
            .map_err(|_| sanitize_error("Agent task list"))?;

        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());
        self.audit(
            "cynic_list_pending_agent_tasks",
            &agent_id,
            &serde_json::json!({
                "kind": p.kind,
                "count": tasks.len(),
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "tasks": tasks,
                "count": tasks.len(),
            })
            .to_string(),
        )]))
    }

    #[tool(
        name = "cynic_update_agent_task_result",
        description = "Mark an agent task as completed or failed with optional result/error. Called by agents after execution."
    )]
    pub(crate) async fn cynic_update_agent_task_result(
        &self,
        params: Parameters<UpdateAgentTaskResultParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.task_id.is_empty() || p.task_id.len() > 128 {
            return Err(McpError::invalid_params(
                "task_id must be 1-128 characters",
                None,
            ));
        }

        // Both result and error can't be present, but either can be absent
        if p.result.is_some() && p.error.is_some() {
            return Err(McpError::invalid_params(
                "result and error cannot both be present",
                None,
            ));
        }

        // Validate result/error bounds if present
        if let Some(ref r) = p.result {
            if r.chars().count() > 10_000 {
                return Err(McpError::invalid_params(
                    "result must be ≤10000 characters",
                    None,
                ));
            }
        }

        if let Some(ref e) = p.error {
            if e.chars().count() > 10_000 {
                return Err(McpError::invalid_params(
                    "error must be ≤10000 characters",
                    None,
                ));
            }
        }

        let has_result = p.result.is_some();
        let has_error = p.error.is_some();

        self.storage
            .update_agent_task_result(&p.task_id, p.result, p.error)
            .await
            .map_err(|_| sanitize_error("Agent task result update"))?;

        let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());
        self.audit(
            "cynic_update_agent_task_result",
            &agent_id,
            &serde_json::json!({
                "task_id": p.task_id,
                "has_result": has_result,
                "has_error": has_error,
            }),
        )
        .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "task_id": p.task_id,
                "status": if has_error { "failed" } else { "completed" },
            })
            .to_string(),
        )]))
    }
}

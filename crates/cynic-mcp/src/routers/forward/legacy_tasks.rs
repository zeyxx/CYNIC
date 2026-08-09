#[tool_router(router = tool_router_forward_legacy_tasks, vis = "pub(crate)")]
impl CynicMcpProxy {
    fn blocked_legacy_agent_task_projection(
        requested: serde_json::Value,
        canonical_replacement: &'static str,
        reason: &'static str,
    ) -> CallToolResult {
        CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "schema": "cynic.mcp.legacy_agent_task_projection.v1",
                "status": "blocked_deprecated_surface",
                "mutates": false,
                "legacy_surface": "agent-tasks",
                "requested": requested,
                "canonical_replacement": canonical_replacement,
                "reason": reason,
            })
            .to_string(),
        )])
    }

    #[tool(
        name = "cynic_dispatch_agent_task",
        description = "Legacy compatibility projection. Mutation is blocked; use cynic_validate and typed node-owned agent workloads."
    )]
    async fn cynic_dispatch_agent_task(
        &self,
        params: Parameters<DispatchAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({"kind": p.kind, "domain": p.domain}),
            "cynic_validate",
            "the legacy AgentTask queue is not mutation execution authority; use a typed agent workload with durable claim, grant, completion, and read-back evidence",
        ))
    }

    #[tool(
        name = "cynic_list_pending_agent_tasks",
        description = "List pending tasks for a given agent kind."
    )]
    async fn cynic_list_pending_agent_tasks(
        &self,
        params: Parameters<ListPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({"kind": p.kind, "domain": p.domain, "limit": p.limit}),
            "cynic_validate",
            "legacy task queue is a deprecated compatibility surface; use cynic_validate for planned workload admission",
        ))
    }

    #[tool(
        name = "cynic_watch_pending_agent_tasks",
        description = "Wait for pending tasks for a given agent kind through the kernel watch surface. Prefer this to repeated list polling when the consumer can block."
    )]
    async fn cynic_watch_pending_agent_tasks(
        &self,
        params: Parameters<WatchPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({
                "kind": p.kind,
                "domain": p.domain,
                "limit": p.limit,
                "timeout_secs": p.timeout_secs,
            }),
            "cynic_validate",
            "legacy task queue is a deprecated compatibility surface; use cynic_validate for planned workload admission",
        ))
    }

    #[tool(
        name = "cynic_list_completed_agent_tasks",
        description = "List completed tasks for a given agent kind. Use after dispatch/claim/result flows to prove whether work was consumed rather than lost."
    )]
    async fn cynic_list_completed_agent_tasks(
        &self,
        params: Parameters<ListPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({"kind": p.kind, "domain": p.domain, "limit": p.limit}),
            "cynic_coord_claim_batch",
            "legacy completed-task query is not part of the durable execution contract; use cynic_coord_claim/cynic_validate paths",
        ))
    }

    #[tool(
        name = "cynic_claim_agent_task",
        description = "Legacy compatibility projection. Mutation is blocked; claims belong to typed node-owned agent workloads."
    )]
    async fn cynic_claim_agent_task(
        &self,
        params: Parameters<ClaimAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({"task_id": p.task_id}),
            "cynic_validate",
            "legacy task claims are not execution authority; claim a typed workload through the node-owned scheduler",
        ))
    }

    #[tool(
        name = "cynic_update_agent_task_result",
        description = "Legacy compatibility projection. Mutation is blocked; publish typed completion evidence through the node-owned scheduler."
    )]
    async fn cynic_update_agent_task_result(
        &self,
        params: Parameters<UpdateAgentTaskResultParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        Ok(Self::blocked_legacy_agent_task_projection(
            serde_json::json!({"task_id": p.task_id}),
            "cynic_validate",
            "legacy task completion is not execution authority; publish typed completion evidence through the node-owned workload scheduler",
        ))
    }
}

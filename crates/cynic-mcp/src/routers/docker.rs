// ── Docker tools (local docker CLI) ─────────────────────────

#[tool_router(router = tool_router_docker, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "docker_container_list",
        description = "List Docker containers via the local Docker CLI."
    )]
    async fn docker_container_list(
        &self,
        params: Parameters<DockerContainerListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let all = params.0.all.unwrap_or(true);
        let result = crate::local_tools::run_docker_container_list(all).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "docker_container_logs",
        description = "Read Docker container logs via the local Docker CLI."
    )]
    async fn docker_container_logs(
        &self,
        params: Parameters<DockerContainerLogsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let result = crate::local_tools::run_docker_container_logs(
            &p.container,
            p.tail,
            p.timestamps.unwrap_or(true),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "docker_container_inspect",
        description = "Inspect a Docker container via the local Docker CLI."
    )]
    async fn docker_container_inspect(
        &self,
        params: Parameters<DockerContainerInspectParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = crate::local_tools::run_docker_container_inspect(&params.0.container).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}

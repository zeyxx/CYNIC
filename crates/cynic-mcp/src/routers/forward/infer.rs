#[tool_router(router = tool_router_forward_infer, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_infer",
        description = "Run sovereign local inference via llama-server (no cloud API, no quota). Forwarded to kernel's OpenAI-compatible endpoint. Use for tasks that should stay local: summarization, classification, extraction."
    )]
    async fn cynic_infer(&self, params: Parameters<InferParams>) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.prompt.trim().is_empty() {
            return Err(McpError::invalid_params("prompt must not be empty", None));
        }

        let mut messages = Vec::new();
        if let Some(sys) = &p.system {
            messages.push(serde_json::json!({"role": "system", "content": sys}));
        }
        messages.push(serde_json::json!({"role": "user", "content": p.prompt}));

        self.post(
            "/v1/chat/completions",
            &serde_json::json!({
                "messages": messages,
                "temperature": p.temperature.unwrap_or(0.7),
                "max_tokens": p.max_tokens.unwrap_or(512),
            }),
        )
        .await
    }
}

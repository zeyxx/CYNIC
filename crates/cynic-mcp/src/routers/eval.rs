// ── Agent evaluation tool (local OpenAI-compatible endpoint) ────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct AgentEvalParams {
    /// Dataset path relative to the MCP runtime project root; defaults to agent-eval-v0.
    dataset: Option<String>,
    /// OpenAI-compatible base URL including /v1.
    endpoint: String,
    /// Model identifier sent to the endpoint.
    model: String,
    /// Optional API-key file. Secret contents are never returned.
    api_key_file: Option<String>,
    /// Aggregate target percentage. Default 80.
    target_percent: Option<f64>,
    /// Repetitions per case. Default 3.
    repetitions: Option<u32>,
    /// Seed increment between repetitions. Default 1.
    seed_stride: Option<i64>,
}

#[tool_router(router = tool_router_eval, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_agent_eval",
        description = "Run the versioned CYNIC agent evaluation against an already-active OpenAI-compatible endpoint. Measures empirical variance, confidence intervals, deterministic oracles, tool-call selection, and controlled fixture tool execution. Does not start models or expose secrets."
    )]
    async fn cynic_agent_eval(
        &self,
        params: Parameters<AgentEvalParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let params = params.0;
        let dataset = params
            .dataset
            .unwrap_or_else(|| "crates/cynic-askesis/datasets/agent-eval-v0.jsonl".into());
        let dataset = PathBuf::from(dataset);
        let dataset = if dataset.is_absolute() {
            dataset
        } else {
            PathBuf::from(&self.project_root).join(dataset)
        };
        let api_key = resolve_agent_eval_api_key(params.api_key_file.as_deref())?;
        let dataset = cynic_askesis::eval::dataset::load_jsonl_dataset(&dataset)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        let client = cynic_askesis::eval::openai::OpenAiCompletionClient::new(
            params.endpoint,
            params.model,
            api_key,
        )
        .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        let report = cynic_askesis::eval::runner::run_agent_eval(
            &dataset,
            &client,
            params.target_percent.unwrap_or(80.0),
            params.repetitions.unwrap_or(3),
            params.seed_stride.unwrap_or(1),
        )
        .await
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()),
        )]))
    }
}

fn resolve_agent_eval_api_key(path: Option<&str>) -> Result<Option<String>, McpError> {
    if let Ok(value) = std::env::var("CYNIC_CHAT_API_KEY")
        && !value.trim().is_empty()
    {
        return Ok(Some(value.trim().to_string()));
    }
    let path = path.map(PathBuf::from).or_else(|| {
        std::env::var("CYNIC_CHAT_API_KEY_FILE")
            .ok()
            .map(PathBuf::from)
    });
    path.map(std::fs::read_to_string)
        .transpose()
        .map(|value| value.map(|secret| secret.trim().to_string()))
        .map_err(|error| McpError::internal_error(format!("read agent eval API key: {error}"), None))
}

#[tool_router(router = tool_router_agent_loop_audit, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_agent_loop_audit",
        description = "Audit the declared CYNIC MCP gateway for complete agent-execution and sovereign data-learning loops. Reports declared, missing, and unknown coverage without claiming runtime proof."
    )]
    async fn cynic_agent_loop_audit(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let root = PathBuf::from(&self.project_root);
        let exposure = read_agent_loop_json(root.join("infra/agent-exposure.json"))?;
        let ecosystem = read_agent_loop_json(root.join("infra/ecosystem-pins.json"))?;
        let report = cynic_askesis::agent_loop::audit_agent_loop(&exposure, &ecosystem);
        let json = serde_json::to_string_pretty(&report).map_err(|error| {
            McpError::internal_error(format!("serialize agent loop audit: {error}"), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

fn read_agent_loop_json(path: PathBuf) -> Result<serde_json::Value, McpError> {
    let bytes = std::fs::read(&path).map_err(|error| {
        McpError::internal_error(format!("read {}: {error}", path.display()), None)
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        McpError::internal_error(format!("parse {}: {error}", path.display()), None)
    })
}

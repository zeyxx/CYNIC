//! MCP-to-REST proxy — thin forwarder that replaces the full kernel in MCP mode.
//!
//! Instead of spawning Judge+Storage+Dogs+Metrics locally, forwards tool calls
//! to the running REST kernel via HTTP. Zero local state, zero SurrealDB connection.
//!
//! Local tools (validate, git) run in-process (filesystem access required).
//! All other tools → HTTP to REST kernel.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};

use super::{
    AuditQueryParams, AuthParams, BatchClaimParams, ClaimParams, DispatchAgentTaskParams,
    GitParams, JudgeParams, ListParams, ListPendingAgentTasksParams, McpRateLimit, ObserveParams,
    RegisterParams, ReleaseParams, UpdateAgentTaskResultParams, ValidateParams, WhoParams,
    validate_agent_id,
};

// ── Proxy struct ────────────────────────────────────────────

#[derive(Clone)]
pub struct CynicMcpProxy {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    authenticated: Arc<AtomicBool>,
    rate_limit: Arc<McpRateLimit>,
    project_root: String,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcpProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcpProxy")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl CynicMcpProxy {
    pub fn new(base_url: String, api_key: String, project_root: String) -> Self {
        // WHY: reqwest::Client::builder().build() only fails on TLS init error,
        // which is unrecoverable. Unwrap is safe here but clippy wants map_err.
        #[allow(clippy::expect_used)] // WHY: TLS init failure = unrecoverable
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client init failed");

        Self {
            client,
            base_url,
            api_key,
            authenticated: Arc::new(AtomicBool::new(
                std::env::var("CYNIC_ALLOW_OPEN_API")
                    .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                    .unwrap_or(false),
            )),
            rate_limit: Arc::new(McpRateLimit::new()),
            project_root,
            tool_router: Self::tool_router_forward()
                + Self::tool_router_coord()
                + Self::tool_router_local(),
        }
    }

    fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Not authenticated — call cynic_auth first",
                None,
            ));
        }
        Ok(())
    }

    // ── HTTP helpers ────────────────────────────────────────

    async fn get(&self, path: &str) -> Result<CallToolResult, McpError> {
        let resp = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    async fn get_query(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<CallToolResult, McpError> {
        let mut url = format!("{}{}", self.base_url, path);
        if !params.is_empty() {
            url.push('?');
            for (i, (k, v)) in params.iter().enumerate() {
                if i > 0 {
                    url.push('&');
                }
                url.push_str(k);
                url.push('=');
                url.push_str(v);
            }
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<CallToolResult, McpError> {
        let resp = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// ── Forwarded tools (REST kernel) ───────────────────────────

#[tool_router(router = tool_router_forward, vis = "pub(super)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_judge",
        description = "Submit content for epistemic evaluation by CYNIC's independent AI validators (Dogs). Returns φ-bounded Q-Score, per-axiom breakdown (FIDELITY/PHI/VERIFY/CULTURE/BURN/SOVEREIGNTY), verdict (HOWL/WAG/GROWL/BARK), and multi-model reasoning. Max confidence: 61.8%."
    )]
    async fn cynic_judge(
        &self,
        params: Parameters<JudgeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        self.post(
            "/judge",
            &serde_json::json!({
                "content": p.content,
                "context": p.context,
                "domain": p.domain,
                "dogs": p.dogs,
                "crystals": p.crystals,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_health",
        description = "Get CYNIC system health: Dog status, quality rates, crystal counts, version. No auth required."
    )]
    async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        self.get("/health").await
    }

    #[tool(
        name = "cynic_verdicts",
        description = "List recent verdicts (limit defaults to 10)."
    )]
    async fn cynic_verdicts(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(10).to_string();
        self.get_query("/verdicts", &[("limit", limit)]).await
    }

    #[tool(
        name = "cynic_crystals",
        description = "List crystals (coherence patterns learned from verdict history)."
    )]
    async fn cynic_crystals(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(20).to_string();
        self.get_query("/crystals", &[("limit", limit)]).await
    }

    #[tool(
        name = "cynic_audit_query",
        description = "Query MCP audit trail (tool calls, agent activity)."
    )]
    async fn cynic_audit_query(
        &self,
        params: Parameters<AuditQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(t) = p.tool {
            q.push(("tool", t));
        }
        if let Some(a) = p.agent_id {
            q.push(("agent_id", a));
        }
        let limit = p.limit.unwrap_or(20).to_string();
        q.push(("limit", limit));
        self.get_query("/audit", &q).await
    }

    #[tool(
        name = "cynic_observe",
        description = "Record a tool observation (Edit, Write, Bash, etc.) for the Crystal Coherence Machine. Fire-and-forget."
    )]
    async fn cynic_observe(
        &self,
        params: Parameters<ObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/observe",
            &serde_json::json!({
                "tool": p.tool,
                "target": p.target,
                "domain": p.domain,
                "status": p.status,
                "context": p.context,
                "project": p.project,
                "agent_id": p.agent_id,
                "session_id": p.session_id,
                "tags": p.tags,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_dispatch_agent_task",
        description = "Dispatch a task for an agent (hermes, nightshift, etc.)."
    )]
    async fn cynic_dispatch_agent_task(
        &self,
        params: Parameters<DispatchAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/agent-tasks",
            &serde_json::json!({
                "kind": p.kind,
                "domain": p.domain,
                "content": p.content,
                "agent_id": p.agent_id,
            }),
        )
        .await
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
        let limit = p.limit.unwrap_or(10).to_string();
        self.get_query("/agent-tasks", &[("kind", p.kind), ("limit", limit)])
            .await
    }

    #[tool(
        name = "cynic_update_agent_task_result",
        description = "Update an agent task with its result or error."
    )]
    async fn cynic_update_agent_task_result(
        &self,
        params: Parameters<UpdateAgentTaskResultParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            &format!("/agent-tasks/{}/result", p.task_id),
            &serde_json::json!({
                "result": p.result,
                "error": p.error,
                "agent_id": p.agent_id,
            }),
        )
        .await
    }
}

// ── Coord tools (REST kernel) ───────────────────────────────

#[tool_router(router = tool_router_coord, vis = "pub(super)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_coord_register",
        description = "Register this agent with the kernel coordination system."
    )]
    async fn cynic_coord_register(
        &self,
        params: Parameters<RegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/coord/register",
            &serde_json::json!({
                "agent_id": p.agent_id,
                "intent": p.intent,
                "agent_type": p.agent_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_claim",
        description = "Claim a file or resource for exclusive editing."
    )]
    async fn cynic_coord_claim(
        &self,
        params: Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/coord/claim",
            &serde_json::json!({
                "agent_id": p.agent_id,
                "target": p.target,
                "claim_type": p.claim_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_claim_batch",
        description = "Claim multiple files/resources at once."
    )]
    async fn cynic_coord_claim_batch(
        &self,
        params: Parameters<BatchClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/coord/claim-batch",
            &serde_json::json!({
                "agent_id": p.agent_id,
                "targets": p.targets,
                "claim_type": p.claim_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_release",
        description = "Release claimed files/resources."
    )]
    async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        self.post(
            "/coord/release",
            &serde_json::json!({
                "agent_id": p.agent_id,
                "target": p.target,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_who",
        description = "Query active agents and their claims."
    )]
    async fn cynic_coord_who(
        &self,
        params: Parameters<WhoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(a) = p.agent_id {
            q.push(("agent_id", a));
        }
        self.get_query("/coord/who", &q).await
    }
}

// ── Local tools (no REST forwarding) ────────────────────────

#[tool_router(router = tool_router_local, vis = "pub(super)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_auth",
        description = "Authenticate this MCP session. Required before calling sensitive tools. Pass the CYNIC_API_KEY. Call once per session."
    )]
    async fn cynic_auth(&self, params: Parameters<AuthParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        // Auth is local: compare against env var, don't forward to REST
        let expected = std::env::var("CYNIC_API_KEY").unwrap_or_default();
        if expected.is_empty() {
            return Err(McpError::internal_error(
                "Kernel has no CYNIC_API_KEY configured",
                None,
            ));
        }
        if p.api_key != expected {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Authentication failed — invalid API key",
                None,
            ));
        }
        self.authenticated.store(true, Ordering::Relaxed);
        Ok(CallToolResult::success(vec![Content::text(
            r#"{"authenticated": true}"#,
        )]))
    }

    #[tool(
        name = "cynic_validate",
        description = "Run cargo build + clippy + lint-rules on the kernel. Returns pass/fail with details."
    )]
    async fn cynic_validate(
        &self,
        _params: Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        // Local: runs filesystem commands
        let result = super::build_tools::run_validate(&self.project_root).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_git",
        description = "Run safe git operations: status, diff, log, branch, stash."
    )]
    async fn cynic_git(&self, params: Parameters<GitParams>) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        // Local: runs git commands
        let result = super::build_tools::run_git(&self.project_root, &params.0.op).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}

// ── MCP ServerHandler ───────────────────────────────────────

#[tool_handler]
impl ServerHandler for CynicMcpProxy {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "cynic-kernel-proxy",
                env!("CYNIC_VERSION"),
            ))
            .with_instructions("CYNIC MCP proxy — forwards to REST kernel. Zero local state.")
    }
}

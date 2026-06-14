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

use crate::types::{
    AuditQueryParams, AuthParams, BatchClaimParams, ClaimParams, ComplianceParams,
    DispatchAgentTaskParams, GitParams, InferParams, JudgeParams, ListParams,
    ListPendingAgentTasksParams, McpRateLimit, MetabolismParams, ObserveParams,
    ProxmoxActionParams, RegisterParams, ReleaseParams, UpdateAgentTaskResultParams,
    ValidateParams, WhoParams, validate_agent_id,
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
    // WHY: K12 — tool_router is used via trait dispatch, compiler misses direct usage
    #[allow(dead_code)]
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
    pub fn tool_router() -> ToolRouter<Self> {
        Self::tool_router_forward() + Self::tool_router_coord() + Self::tool_router_local()
    }

    pub fn new(base_url: String, api_key: String, project_root: String) -> Self {
        // WHY: reqwest::Client::builder().build() only fails on TLS init error,
        // which is unrecoverable. Unwrap is safe here but clippy wants map_err.
        #[allow(clippy::expect_used)] // WHY: TLS init failure = unrecoverable
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client init failed");

        let auto_auth = !api_key.is_empty()
            || std::env::var("CYNIC_ALLOW_OPEN_API")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false);

        Self {
            client,
            base_url,
            api_key,
            // MCP stdio is a local subprocess; if the wrapper injected the kernel token,
            // requiring the model to echo it back through cynic_auth adds exposure only.
            authenticated: Arc::new(AtomicBool::new(auto_auth)),
            rate_limit: Arc::new(McpRateLimit::new()),
            project_root,
            tool_router: Self::tool_router(),
        }
    }

    fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "MCP not authenticated: CYNIC_API_KEY was not provided to the MCP process",
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

#[tool_router(router = tool_router_forward, vis = "pub(crate)")]
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
                "priority": p.priority,
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
        name = "cynic_metabolism",
        description = "Get the organism's metabolic state: observation intake, digestion ratio, crystallization rate, backlog, drops, verdict production. Zero-I/O snapshot of data flow health. Use to check if the organism is digesting its observations or starving."
    )]
    async fn cynic_metabolism(
        &self,
        _params: Parameters<MetabolismParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        // Metabolism is embedded in /health — extract it server-side.
        // We call /health and extract the metabolism + crystals + alerts subset.
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());

        // Extract metabolism subset from full health response
        let extracted = match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(health) => {
                serde_json::json!({
                    "metabolism": health.get("metabolism"),
                    "crystals": health.get("crystals"),
                    "alerts": health.get("alerts"),
                    "dogs": health.get("dogs").and_then(|d| d.as_array()).map(|dogs| {
                        dogs.iter().map(|d| serde_json::json!({
                            "id": d.get("id"),
                            "kind": d.get("kind"),
                            "circuit": d.get("circuit"),
                        })).collect::<Vec<_>>()
                    }),
                    "status": health.get("status"),
                    "uptime_seconds": health.get("uptime_seconds"),
                })
            }
            Err(_) => serde_json::json!({"raw": body}),
        };
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&extracted).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_compliance",
        description = "Query session compliance scores. With agent_id: score a specific agent's session (read-before-edit %, branch discipline, rule violations). Without: return recent compliance trend (last N sessions). Use to check if the organism is following its own rules."
    )]
    async fn cynic_compliance(
        &self,
        params: Parameters<ComplianceParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        if let Some(agent_id) = p.agent_id {
            // Per-session compliance
            self.get(&format!("/session/{agent_id}/compliance")).await
        } else {
            // Trend mode
            let limit = p.limit.unwrap_or(20).to_string();
            self.get_query("/compliance", &[("limit", limit)]).await
        }
    }

    #[tool(
        name = "cynic_infer",
        description = "Run sovereign local inference via llama-server (no cloud API, no quota). Forwarded to kernel's OpenAI-compatible endpoint. Use for tasks that should stay local: summarization, classification, extraction."
    )]
    async fn cynic_infer(
        &self,
        params: Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.prompt.trim().is_empty() {
            return Err(McpError::invalid_params("prompt must not be empty", None));
        }

        // Build OpenAI-compatible chat completion request
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

#[tool_router(router = tool_router_coord, vis = "pub(crate)")]
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
        description = "Query active agents and their claims. No auth required (aligns with kernel)."
    )]
    async fn cynic_coord_who(
        &self,
        params: Parameters<WhoParams>,
    ) -> Result<CallToolResult, McpError> {
        // No auth required — matches kernel behavior (coord_tools.rs:264).
        // Coordination visibility helps all agents avoid conflicts.
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

#[tool_router(router = tool_router_local, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_auth",
        description = "Fallback manual authentication for MCP sessions missing CYNIC_API_KEY env forwarding. Prefer fixing the MCP wrapper instead of passing secrets through the model."
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
        let result = crate::local_tools::run_validate(&self.project_root).await;
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
        let result = crate::local_tools::run_git(&self.project_root, &params.0.op).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_proxmox_list",
        description = "List all VMs and LXC containers on the Proxmox hypervisor. Returns their status, memory, and ID."
    )]
    async fn cynic_proxmox_list(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let token_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let secret = std::env::var("CYNIC_PROXMOX_SECRET").unwrap_or_default();

        if proxmox_url.is_empty() || token_id.is_empty() || secret.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_header = format!("PVEAPIToken={}={}", token_id, secret);

        // 1. Get nodes
        let nodes_resp = client
            .get(format!("{proxmox_url}/api2/json/nodes"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Proxmox API failed (nodes): {e}"), None)
            })?;

        let nodes_json: serde_json::Value = nodes_resp.json().await.map_err(|e| {
            McpError::internal_error(format!("Invalid JSON from Proxmox: {e}"), None)
        })?;

        let mut output = String::new();

        if let Some(nodes) = nodes_json["data"].as_array() {
            for node in nodes {
                if let Some(node_name) = node["node"].as_str() {
                    output.push_str(&format!("=== Node: {} ===\n", node_name));

                    // 2. Get VMs
                    let qemu_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/qemu"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = qemu_resp {
                        if let Ok(qemu_json) = resp.json::<serde_json::Value>().await {
                            output.push_str("VMs:\n");
                            if let Some(vms) = qemu_json["data"].as_array() {
                                for vm in vms {
                                    let vmid = vm["vmid"].as_u64().unwrap_or(0);
                                    let name = vm["name"].as_str().unwrap_or("unknown");
                                    let status = vm["status"].as_str().unwrap_or("unknown");
                                    let mem = vm["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                    output.push_str(&format!("- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"));
                                }
                            }
                        }
                    }

                    // 3. Get LXCs
                    let lxc_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/lxc"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = lxc_resp {
                        if let Ok(lxc_json) = resp.json::<serde_json::Value>().await {
                            output.push_str("Containers (LXC):\n");
                            if let Some(lxcs) = lxc_json["data"].as_array() {
                                for lxc in lxcs {
                                    let vmid = lxc["vmid"].as_u64().unwrap_or(0);
                                    let name = lxc["name"].as_str().unwrap_or("unknown");
                                    let status = lxc["status"].as_str().unwrap_or("unknown");
                                    let mem = lxc["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                    output.push_str(&format!("- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "cynic_proxmox_action",
        description = "Perform actions (start, stop, reboot, suspend, resume) on Proxmox VMs and LXC containers."
    )]
    async fn cynic_proxmox_action(
        &self,
        params: Parameters<ProxmoxActionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let token_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let secret = std::env::var("CYNIC_PROXMOX_SECRET").unwrap_or_default();

        if proxmox_url.is_empty() || token_id.is_empty() || secret.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;
        let vmid = p.vmid;
        let node = p.node;
        let type_ = p.target_type; // "qemu" or "lxc"
        let action = p.action; // "start", "stop", "reboot", "suspend", "resume"

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_header = format!("PVEAPIToken={}={}", token_id, secret);

        let url = format!("{proxmox_url}/api2/json/nodes/{node}/{type_}/{vmid}/status/{action}");

        let resp = client
            .post(&url)
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// ── MCP ServerHandler ───────────────────────────────────────

#[tool_handler]
impl ServerHandler for CynicMcpProxy {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cynic-mcp", env!("CYNIC_VERSION")))
            .with_instructions("CYNIC MCP proxy — forwards to REST kernel. Zero local state.")
    }
}

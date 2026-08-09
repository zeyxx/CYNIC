#[tool_router(router = tool_router_forward_health, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_health",
        description = "Get CYNIC system health: Dog status, quality rates, crystal counts, version. No auth required."
    )]
    async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        self.get("/health").await
    }

    #[tool(
        name = "cynic_attention",
        description = "Cortex startup attention surface: compactly summarizes fleet liveness, degraded dogs, overdue tasks, metabolism pressure, and next actions from kernel health + mempool. Use at session start instead of manually inspecting full /health JSON."
    )]
    async fn cynic_attention(
        &self,
        params: Parameters<AttentionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        let task_kind = p.task_kind.unwrap_or_else(|| "cortex".to_string());
        let task_limit = p.task_limit.unwrap_or(10).clamp(1, 25).to_string();
        let mut task_query = vec![("task_kind", task_kind), ("task_limit", task_limit)];
        if let Some(domain) = p.task_domain {
            task_query.push(("task_domain", domain));
        }
        self.get_query("/attention", &task_query).await
    }

    #[tool(
        name = "cynic_verdicts",
        description = "List recent verdicts (limit defaults to 10)."
    )]
    async fn cynic_verdicts(&self, params: Parameters<ListParams>) -> Result<CallToolResult, McpError> {
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
        name = "cynic_library_query",
        description = "Query CYNIC's dynamic knowledge library from Kernel observations. Use for research, market signals, mechanism studies and source discoveries instead of repo-local Markdown notes."
    )]
    async fn cynic_library_query(
        &self,
        params: Parameters<LibraryQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(domain) = p.domain {
            q.push(("domain", domain));
        }
        if let Some(target) = p.target {
            q.push(("target", target));
        }
        if let Some(agent_id) = p.agent_id {
            q.push(("agent_id", agent_id));
        }
        if let Some(project) = p.project {
            q.push(("project", project));
        }
        if let Some(workstream) = p.workstream {
            q.push(("workstream", workstream));
        }
        if let Some(thread_id) = p.thread_id {
            q.push(("thread_id", thread_id));
        }
        let limit = p.limit.unwrap_or(20).min(100).to_string();
        q.push(("limit", limit));
        self.get_query("/observations", &q).await
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
        let resp = self
            .kernel
            .client()
            .get(self.kernel.url("/health"))
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());

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
            self.get(&format!("/sessions/{agent_id}/compliance")).await
        } else {
            let limit = p.limit.unwrap_or(20).to_string();
            self.get_query("/compliance", &[("limit", limit)]).await
        }
    }
}

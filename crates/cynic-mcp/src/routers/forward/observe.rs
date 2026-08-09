#[tool_router(router = tool_router_forward_observe, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_observe",
        description = "Record a tool observation (Edit, Write, Bash, etc.) and perform bounded storage read-back when available. The ingestion acceptance remains explicit; unresolved storage proof is returned as an unknown."
    )]
    async fn cynic_observe(
        &self,
        params: Parameters<ObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let body = observe_request_body(&p);
        let response = self.post_json("/observe", &body).await?;
        let violations = cynic_protocol::validate_observation_ingestion_read_back(&response);
        let target = p.target.clone().unwrap_or_default();
        let agent_id = p.agent_id.clone().unwrap_or_else(|| "unknown".to_string());
        let session_id = p.session_id.clone().unwrap_or_default();
        let context = p
            .context
            .clone()
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect::<String>();
        let status = p.status.clone().unwrap_or_else(|| "success".to_string());
        let mut read_back_verified = false;
        for attempt in 0..5 {
            let mut query = vec![("agent_id", agent_id.clone()), ("limit", "100".to_string())];
            if !target.is_empty() {
                query.push(("target", target.clone()));
            }
            if let Some(domain) = p.domain.clone() {
                query.push(("domain", domain));
            }
            if let Ok(rows) = self.get_query_json("/observations", &query).await {
                if cynic_protocol::observation_read_back_matches(
                    &rows,
                    &p.tool,
                    &target,
                    p.domain.as_deref(),
                    &agent_id,
                    &session_id,
                    &context,
                    &status,
                ) {
                    read_back_verified = true;
                    break;
                }
            }
            if attempt < 4 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        let mut result = response;
        if let Some(object) = result.as_object_mut() {
            object.insert(
                "read_back_violations".to_string(),
                serde_json::to_value(violations).unwrap_or_default(),
            );
            object.insert(
                "storage_read_back_verified".to_string(),
                serde_json::Value::Bool(read_back_verified),
            );
            object.insert(
                "read_back_unknowns".to_string(),
                serde_json::json!(if read_back_verified {
                    Vec::<String>::new()
                } else {
                    vec!["observation_storage_background_read_back_pending".to_string()]
                }),
            );
        }
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }
}

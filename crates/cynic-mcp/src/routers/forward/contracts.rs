#[tool_router(router = tool_router_forward_contracts, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_api_contracts",
        description = "Discover CYNIC's machine-readable API contracts through the kernel: manifest, OpenAPI, API topology, or layer alignment. No auth required."
    )]
    async fn cynic_api_contracts(
        &self,
        params: Parameters<ApiContractsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let contract = params.0.contract.unwrap_or_else(|| "manifest".to_string());
        match contract.as_str() {
            "manifest" => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "schema": "cynic.api_contracts_manifest.v1",
                    "contracts": {
                        "openapi": "/openapi.json",
                        "topology": "/api/topology",
                        "layer_alignment": "/api/layer-alignment"
                    },
                    "valid_contract_values": ["manifest", "openapi", "topology", "layer_alignment"]
                })
                .to_string(),
            )])),
            "openapi" => self.get("/openapi.json").await,
            "topology" => self.get("/api/topology").await,
            "layer_alignment" => self.get("/api/layer-alignment").await,
            other => Err(McpError::invalid_params(
                format!(
                    "unknown contract {other:?}; expected manifest, openapi, topology, or layer_alignment"
                ),
                None,
            )),
        }
    }

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
}

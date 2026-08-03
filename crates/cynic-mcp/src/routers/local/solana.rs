use cynic_observer::{plan_fanout_request, FanoutPlanRequest};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};

use crate::types::SolanaPlanParams;

#[tool_router(router = tool_router_local_solana, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_solana_plan",
        description = "Build a read-only Solana fan-out plan with Helius strategy and credit estimate. Does not contact a provider, read secrets, or launch ingestion."
    )]
    async fn cynic_solana_plan(
        &self,
        params: Parameters<SolanaPlanParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let plan = plan_fanout_request(FanoutPlanRequest {
            cluster: p.cluster,
            target: p.target,
            helius_plan: p.helius_plan.unwrap_or_else(|| "free".to_string()),
            signature_page_limit: p.signature_page_limit.unwrap_or(1_000),
            max_signature_pages: p.max_signature_pages.unwrap_or(10),
            max_transactions: p.max_transactions.unwrap_or(1_000),
            expected_transactions: p.expected_transactions.unwrap_or(1_000),
            commitment: p.commitment.unwrap_or_else(|| "finalized".to_string()),
        })
        .map_err(|error| {
            McpError::invalid_params(format!("invalid Solana plan: {error:?}"), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&plan)
                .map_err(|error| McpError::internal_error(error.to_string(), None))?,
        )]))
    }
}

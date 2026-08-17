use cynic_observer::{FanoutPlanRequest, plan_fanout_request};
use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, tool, tool_router,
};

use crate::types::SolanaPlanParams;
use crate::types_split::SolanaPayParams;

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

    #[tool(
        name = "cynic_solana_pay_generate",
        description = "Generate a Solana Pay URL + terminal QR code, guarded by the CYNIC threat firewall. Safety-first: the firewall evaluates the request before any URL/QR is materialized."
    )]
    async fn cynic_solana_pay_generate(
        &self,
        params: Parameters<SolanaPayParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let requested = params.0;
        let result = serde_json::json!({
            "schema": "cynic.solana_pay_generation.v1",
            "status": "blocked_unavailable_runtime",
            "mutates": false,
            "requested": {
                "recipient": requested.recipient,
                "amount": requested.amount,
                "spl_token": requested.spl_token,
                "reference": requested.reference,
                "label": requested.label,
                "message": requested.message,
                "memo": requested.memo,
                "allowance_present": requested.allowance.is_some(),
                "context_tokens": requested.context_tokens,
                "actions_budget": requested.actions_budget,
            },
            "unknowns": [
                "the typed Solana payment and allowance contracts are unavailable",
                "the ZeroClaw Solana adapter is not admitted in this MCP build"
            ],
            "message": "No Solana Pay URL or QR code was generated. Restore and validate the payment port, policy, and injected adapter before enabling this mutation-capable surface.",
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }
}

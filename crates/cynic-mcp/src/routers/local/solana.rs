use cynic_observer::{plan_fanout_request, FanoutPlanRequest};
use cynic_protocol::agent_recursion::AgentBudget;
use cynic_protocol::solana_ops::{AllowanceGrant, SolanaPayRequest};
use cynic_zeroclaw_solana::ZeroClawSolanaPlugin;
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
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
        let p = params.0;
        let mut pay_req = SolanaPayRequest::new(p.recipient)
            .with_amount(p.amount);
        if let Some(mint) = p.spl_token {
            pay_req = pay_req.with_spl_token(mint);
        }
        if let Some(label) = p.label {
            pay_req = pay_req.with_label(label);
        }
        if let Some(msg) = p.message {
            pay_req = pay_req.with_message(msg);
        }
        if let Some(memo) = p.memo {
            pay_req = pay_req.with_memo(memo);
        }
        if let Some(reference) = p.reference {
            pay_req = pay_req.with_reference(reference);
        }
        let allowance = p.allowance.and_then(|v| {
            serde_json::from_value::<AllowanceGrant>(v).ok()
        });
        let budget = AgentBudget {
            context_tokens: p.context_tokens.unwrap_or(4000),
            actions: p.actions_budget.unwrap_or(10),
            children: 2,
            wall_time_ms: 30000,
        };
        let plugin = ZeroClawSolanaPlugin::new();
        let output = plugin.generate_solana_pay(pay_req, allowance, budget).await;
        let json = serde_json::to_string_pretty(&output)
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

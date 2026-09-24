//! ZeroClaw Plugin implementation for Solana Native Operations & CYNIC Threat Firewall.

use cynic_intel::adapters::SovereignAgenticToolCallAdapter;
use cynic_intel::ports::AgenticToolCallPort;
use cynic_protocol::agent_recursion::AgentBudget;
use cynic_protocol::agentic_tool_call::{
    AgenticToolCallInput, AgenticToolCallResult, AgenticToolCallStatus,
};
use cynic_solana_pay::{AllowanceGrant, SolanaPayRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::qr::render_qr_ascii;

/// Struct representing a ZeroClaw plugin tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroClawToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Output of a ZeroClaw Solana Pay generation execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroClawSolanaPayOutput {
    pub url: String,
    pub qr_ascii: String,
    pub result: AgenticToolCallResult,
}

/// Main ZeroClaw Solana Sovereign Plugin engine.
#[derive(Debug, Default, Clone)]
pub struct ZeroClawSolanaPlugin {
    adapter: SovereignAgenticToolCallAdapter,
}

impl ZeroClawSolanaPlugin {
    pub fn new() -> Self {
        Self {
            adapter: SovereignAgenticToolCallAdapter::new(),
        }
    }

    /// List tool definitions exposed to ZeroClaw agents.
    pub fn tools(&self) -> Vec<ZeroClawToolDefinition> {
        vec![
            ZeroClawToolDefinition {
                name: "solana_pay_generate".to_string(),
                description: "Generates an official Solana Pay URL and terminal QR code with CYNIC safety evaluation.".to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "recipient": { "type": "string", "description": "Base58 Solana public key" },
                        "amount": { "type": "number", "description": "Payment amount in SOL or SPL token" },
                        "spl_token": { "type": "string", "description": "Optional SPL Token Mint address" },
                        "label": { "type": "string", "description": "Payment merchant label" },
                        "message": { "type": "string", "description": "Payment order message" }
                    },
                    "required": ["recipient", "amount"]
                }),
            },
            ZeroClawToolDefinition {
                name: "sop_threat_firewall".to_string(),
                description: "Evaluates an incoming Standard Operating Procedure (SOP) for prompt injections and malicious overrides.".to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "sop_text": { "type": "string", "description": "Raw SOP or message text to evaluate" },
                        "requested_action": { "type": "string", "description": "Intended action name" }
                    },
                    "required": ["sop_text", "requested_action"]
                }),
            },
        ]
    }

    /// Execute a Solana Pay URL generation request through ZeroClaw.
    /// Safety-first ordering: the threat firewall evaluates the request BEFORE
    /// any URL or QR code is materialized, so a rejected payload never emits
    /// a spendable Solana Pay link.
    pub async fn generate_solana_pay(
        &self,
        pay_req: SolanaPayRequest,
        allowance: Option<AllowanceGrant>,
        budget: AgentBudget,
    ) -> ZeroClawSolanaPayOutput {
        let params = json!(pay_req);

        let mut input = AgenticToolCallInput::new(
            format!("atc_zc_{}", pay_req.recipient),
            "zeroclaw_solana_plugin",
            "cynic_node",
            "solana_pay_generate",
            params,
            budget,
        );
        input.allowance = allowance;

        // Run the threat firewall FIRST — before URL/QR generation.
        let result = self.adapter.process_call(input).await;

        // Only materialize URL + QR when the request cleared the safety firewall.
        let (url, qr_ascii) = match result.status {
            AgenticToolCallStatus::Completed => {
                let url = pay_req
                    .to_url_string()
                    .unwrap_or_else(|e| format!("solana:invalid?error={e}"));
                let qr_ascii = render_qr_ascii(&url);
                (url, qr_ascii)
            }
            _ => (String::new(), String::new()),
        };

        ZeroClawSolanaPayOutput {
            url,
            qr_ascii,
            result,
        }
    }

    /// Evaluate an SOP input through the CYNIC threat firewall for ZeroClaw.
    pub async fn evaluate_sop(
        &self,
        sop_text: &str,
        requested_action: &str,
        budget: AgentBudget,
    ) -> AgenticToolCallResult {
        let input = AgenticToolCallInput::new(
            "atc_zc_sop_eval",
            "zeroclaw_solana_plugin",
            "cynic_node",
            requested_action,
            json!({ "instructions": sop_text }),
            budget,
        );

        self.adapter.process_call(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::agentic_tool_call::AgenticToolCallStatus;

    #[tokio::test]
    async fn plugin_generates_solana_pay_and_qr() {
        let plugin = ZeroClawSolanaPlugin::new();
        let pay_req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c")
            .with_amount(15.5)
            .with_label("Superteam Brasil Shop")
            .with_message("Bounty Invoice #101")
            .with_memo("INV-101");

        let budget = AgentBudget {
            context_tokens: 1000,
            actions: 5,
            children: 2,
            wall_time_ms: 10000,
            cpu_ms: None,
        };

        let output = plugin.generate_solana_pay(pay_req, None, budget).await;
        assert!(
            output
                .url
                .starts_with("solana:7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c?amount=15.5")
        );
        assert!(!output.qr_ascii.is_empty());
        assert_eq!(output.result.status, AgenticToolCallStatus::Completed);
    }

    #[tokio::test]
    async fn plugin_rejects_invalid_recipient_no_url_emitted() {
        let plugin = ZeroClawSolanaPlugin::new();
        let pay_req = SolanaPayRequest::new("INVALID_KEY").with_amount(25.0);
        let budget = AgentBudget {
            context_tokens: 1000,
            actions: 5,
            children: 2,
            wall_time_ms: 10000,
            cpu_ms: None,
        };
        let output = plugin.generate_solana_pay(pay_req, None, budget).await;
        assert_eq!(output.result.status, AgenticToolCallStatus::Rejected);
        assert!(output.url.is_empty());
        assert!(output.qr_ascii.is_empty());
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolanaSopParams {
    /// Optional Agentic Tool Call unique identifier (generated if omitted).
    pub call_id: Option<String>,
    /// Tool or action name to evaluate (e.g. solana_pay_generate, transfer_funds).
    pub tool_name: String,
    /// Parameters payload to sanitize and evaluate.
    pub parameters: serde_json::Value,
    /// Optional context token budget limit.
    pub context_tokens: Option<u64>,
    /// Optional max actions budget limit.
    pub actions_budget: Option<u32>,
}

/// Parameters for the `cynic_solana_pay_generate` tool.
/// Builds a Solana Pay transfer-request URL + ASCII QR, guarded by the
/// CYNIC safety dog and an optional AllowanceGrant spend cap.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolanaPayParams {
    /// Recipient wallet public key (base58).
    pub recipient: String,
    /// Transfer amount in SOL or SPL token units.
    pub amount: f64,
    /// Optional SPL Token mint address (base58). When omitted, request is in native SOL.
    pub spl_token: Option<String>,
    /// Optional reference public key for tracking on-chain via getSignaturesForAddress.
    pub reference: Option<String>,
    /// Human readable label for the transfer request.
    pub label: Option<String>,
    /// Human readable message describing the purpose.
    pub message: Option<String>,
    /// On-chain memo instruction payload.
    pub memo: Option<String>,
    /// Optional allowance grant JSON (serialized AllowanceGrant) for spend-cap enforcement.
    pub allowance: Option<serde_json::Value>,
    /// Optional context token budget limit.
    pub context_tokens: Option<u64>,
    /// Optional max actions budget limit.
    pub actions_budget: Option<u32>,
}

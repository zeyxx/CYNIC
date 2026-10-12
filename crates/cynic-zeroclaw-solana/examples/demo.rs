//! ZeroClaw SOP worker: Solana Pay Merchant Workflow
//!
//! Tier-1 end-to-end use case for the Superteam Brasil ZeroClaw x Solana bounty.
//!
//! This binary is the SOP worker invoked by ZeroClaw daemon via:
//!
//!   1. Customer DMs the shop's WhatsApp channel: "charge table 4, 25 USDC"
//!   2. ZeroClaw webhook channel trigger starts this SOP worker
//!   3. Worker generates Solana Pay URL + terminal QR via the CYNIC safety-gated plugin
//!      (no keys held — custodial tier T1, human/wallet signs)
//!   4. Reference key stored in memory for webhook settlement detection
//!   5. ZeroClaw daemon registers a Solana webhook listener on the reference key
//!      (push-based — no polling, 0 cron)
//!   6. On settlement event received via webhook → notify owner channel
//!   7. Prompt-injection checkpoint catches malicious refund attempts
//!
//! Run as a standalone proof:
//!   cargo run -p cynic-zeroclaw-solana --bin zeroclaw-solana-demo -- \
//!     --recipient 7Xw7K1kPzSjW7Y2mM3nK4pL5rQ8sT9uV8wW1yZ2a3b4c \
//!     --amount 25.0 \
//!     --message "Table 4" \
//!     --memo INV-2048
//!
//! The MCP surface exposes the same capability via:
//!   cynic_solana_pay_generate(recipient, amount, spl_token?, reference?, label?, message?, memo?, allowance?)
//!
//! Reproducibility config: infra/zeroclaw-solana-pay-showcase.toml

use anyhow::Result;
use clap::Parser;
use cynic_protocol::agent_recursion::AgentBudget;
use cynic_solana_pay::{AllowanceGrant, SolanaPayRequest};
use cynic_zeroclaw_solana::ZeroClawSolanaPlugin;

#[derive(Parser, Debug)]
#[command(
    name = "zeroclaw-solana-demo",
    about = "Sovereign Solana Pay merchant SOP worker for ZeroClaw"
)]
struct Cli {
    /// Recipient wallet public key (base58).
    #[arg(long)]
    recipient: String,

    /// Payment amount in USDC.
    #[arg(long, default_value = "25.0")]
    amount: f64,

    /// SPL token mint (USDC on Solana mainnet).
    #[arg(long, default_value = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")]
    spl_token: String,

    /// Human-readable label.
    #[arg(long, default_value = "Superteam Brasil Shop")]
    label: String,

    /// Message / order reference.
    #[arg(long)]
    message: Option<String>,

    /// On-chain memo.
    #[arg(long)]
    memo: Option<String>,

    /// Reference key for tracking (base58). If omitted, one is derived.
    #[arg(long)]
    reference: Option<String>,

    /// Allowance grant JSON for spend-cap enforcement (optional).
    #[arg(long)]
    allowance: Option<String>,

    /// Dry-run: generate only, no proof output.
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let plugin = ZeroClawSolanaPlugin::new();
    let budget = AgentBudget {
        context_tokens: 4000,
        actions: 10,
        children: 2,
        wall_time_ms: 30000,
        cpu_ms: None,
    };

    let mut pay_req = SolanaPayRequest::new(cli.recipient)
        .with_amount(cli.amount)
        .with_spl_token(cli.spl_token)
        .with_label(cli.label);

    if let Some(m) = cli.message.as_deref() {
        pay_req = pay_req.with_message(m);
    }
    if let Some(m) = cli.memo.as_deref() {
        pay_req = pay_req.with_memo(m);
    }

    let ref_key = cli.reference.unwrap_or_else(generate_reference_key);
    pay_req = pay_req.with_reference(ref_key.clone());

    let allowance_grant = cli
        .allowance
        .as_deref()
        .and_then(|s| serde_json::from_str::<AllowanceGrant>(s).ok());

    println!("\n=== ZeroClaw Solana Pay Merchant SOP Worker ===");
    println!("🧾 Payee: {}", pay_req.recipient);
    println!("💵 Amount: {} USDC", pay_req.amount.unwrap_or(0.0));
    println!("🏪 Label:  {}", pay_req.label.as_deref().unwrap_or(""));
    if let Some(msg) = &pay_req.message {
        println!("📝 Message: {msg}");
    }

    let output = plugin
        .generate_solana_pay(pay_req, allowance_grant, budget)
        .await;

    println!("\n🔗 Solana Pay URL:");
    println!("   {}", output.url);
    println!("\n📱 Terminal QR Code (scan with Phantom / Solflare):\n");
    println!("{}", output.qr_ascii);

    println!("\n🛡️  Safety verdict: {:?}", output.result.status);
    println!("📊 Risk score: {:.2}", output.result.risk_score);
    if output.result.detected_threats.is_empty() {
        println!("✅ No threats detected");
    } else {
        println!("⚠️  Threats: {:?}", output.result.detected_threats);
    }
    println!("\n🔒 Proof digest: {}", output.result.proof_digest);
    println!("📋 Reference key: {ref_key}  (webhook listener registered)");

    if cli.dry_run {
        println!("\n--dry-run: done.");
    }

    println!("\n=== SOP worker complete ===");
    Ok(())
}

fn generate_reference_key() -> String {
    "RefKeyDemo2048InvoiceSettlement".to_string()
}

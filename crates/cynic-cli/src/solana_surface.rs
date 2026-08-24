use std::process::ExitCode;

use clap::Subcommand;
use cynic_observer::solana_archive::{
    FanoutExecutionPlan, FanoutPlanRequest, ProbeTarget, ProviderRegistry, build_probe_plan,
    plan_fanout_request,
};

use crate::cli_json::emit_pretty_json;

#[derive(Subcommand)]
pub(crate) enum SolanaCommand {
    /// Build a read-only Solana fan-out plan; never contacts a provider.
    Plan {
        #[arg(long)]
        cluster: String,
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "free")]
        helius_plan: String,
        #[arg(long, default_value_t = 1000)]
        signature_page_limit: u16,
        #[arg(long, default_value_t = 10)]
        max_signature_pages: u32,
        #[arg(long, default_value_t = 1000)]
        max_transactions: u64,
        #[arg(long, default_value_t = 1000)]
        expected_transactions: u64,
        #[arg(long, default_value = "finalized")]
        commitment: String,
        #[arg(long)]
        json: bool,
    },
    /// Build a bounded multi-provider probe plan; never resolves secrets or contacts providers.
    ProbePlan {
        #[arg(long)]
        registry: String,
        #[arg(long)]
        cluster: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        corpus_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_solana(command: SolanaCommand) -> ExitCode {
    match command {
        SolanaCommand::Plan {
            cluster,
            target,
            helius_plan,
            signature_page_limit,
            max_signature_pages,
            max_transactions,
            expected_transactions,
            commitment,
            json,
        } => {
            if !json {
                eprintln!("solana plan requires --json");
                return ExitCode::FAILURE;
            }
            let result = build_plan(
                cluster,
                target,
                helius_plan,
                signature_page_limit,
                max_signature_pages,
                max_transactions,
                expected_transactions,
                commitment,
            );
            match result {
                Ok(plan) => emit_pretty_json(&plan),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        SolanaCommand::ProbePlan {
            registry,
            cluster,
            target,
            corpus_id,
            json,
        } => {
            if !json {
                eprintln!("solana probe-plan requires --json");
                return ExitCode::FAILURE;
            }
            let result = std::fs::read(&registry)
                .map_err(|error| format!("read registry: {error}"))
                .and_then(|bytes| {
                    serde_json::from_slice::<ProviderRegistry>(&bytes)
                        .map_err(|error| format!("parse registry: {error}"))
                })
                .and_then(|provider_registry| {
                    build_probe_plan(
                        &provider_registry,
                        ProbeTarget {
                            cluster,
                            target,
                            corpus_id,
                        },
                    )
                    .map_err(|error| format!("build probe plan: {error:?}"))
                });
            match result {
                Ok(plan) => emit_pretty_json(&plan),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn build_plan(
    cluster: String,
    target: String,
    helius_plan: String,
    signature_page_limit: u16,
    max_signature_pages: u32,
    max_transactions: u64,
    expected_transactions: u64,
    commitment: String,
) -> Result<FanoutExecutionPlan, String> {
    plan_fanout_request(FanoutPlanRequest {
        cluster,
        target,
        helius_plan,
        signature_page_limit,
        max_signature_pages,
        max_transactions,
        expected_transactions,
        commitment,
    })
    .map_err(|error| format!("invalid Solana plan: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::build_plan;
    use cynic_observer::HeliusPlan;

    #[test]
    fn plan_parser_keeps_provider_and_commitment_explicit() {
        let plan = build_plan(
            "mainnet".to_string(),
            "mint".to_string(),
            "developer".to_string(),
            1000,
            2,
            100,
            100,
            "finalized".to_string(),
        )
        .unwrap();
        assert!(!plan.mutates_chain);
        assert_eq!(plan.helius_plan, HeliusPlan::Developer);
    }
}

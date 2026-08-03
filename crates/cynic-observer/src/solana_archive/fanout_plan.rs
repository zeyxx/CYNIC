use super::{
    next_fanout_action, select_backfill_strategy, BackfillStrategy, FanoutAction, FanoutCheckpoint,
    FanoutSpec, HeliusCreditCatalog, HeliusPlan,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutExecutionPlan {
    pub cluster: String,
    pub target: String,
    pub helius_plan: HeliusPlan,
    pub strategy: BackfillStrategy,
    pub expected_transactions: u64,
    pub estimated_credits: Option<u64>,
    pub credit_catalog_version: Option<String>,
    pub initial_action: FanoutAction,
    pub mutates_chain: bool,
    pub requires_network: bool,
    pub requires_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutPlanRequest {
    pub cluster: String,
    pub target: String,
    pub helius_plan: String,
    pub signature_page_limit: u16,
    pub max_signature_pages: u32,
    pub max_transactions: u64,
    pub expected_transactions: u64,
    pub commitment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanoutPlanError {
    InvalidSpec,
    MissingTarget,
    MissingCluster,
    UnsupportedCommitment(String),
    UnsupportedHeliusPlan(String),
}

pub fn plan_fanout_request(
    request: FanoutPlanRequest,
) -> Result<FanoutExecutionPlan, FanoutPlanError> {
    let commitment = match request.commitment.trim().to_ascii_lowercase().as_str() {
        "confirmed" => super::Commitment::Confirmed,
        "finalized" => super::Commitment::Finalized,
        other => return Err(FanoutPlanError::UnsupportedCommitment(other.to_string())),
    };
    let helius_plan = match request.helius_plan.trim().to_ascii_lowercase().as_str() {
        "free" => HeliusPlan::Free,
        "developer" => HeliusPlan::Developer,
        "business" => HeliusPlan::Business,
        "professional" => HeliusPlan::Professional,
        "enterprise" => HeliusPlan::Enterprise,
        "unknown" => HeliusPlan::Unknown,
        other => return Err(FanoutPlanError::UnsupportedHeliusPlan(other.to_string())),
    };
    plan_fanout(
        &FanoutSpec {
            cluster: request.cluster,
            target: request.target,
            signature_page_limit: request.signature_page_limit,
            max_signature_pages: request.max_signature_pages,
            max_transactions: request.max_transactions,
            commitment,
        },
        helius_plan,
        request.expected_transactions,
    )
}

pub fn plan_fanout(
    spec: &FanoutSpec,
    helius_plan: HeliusPlan,
    expected_transactions: u64,
) -> Result<FanoutExecutionPlan, FanoutPlanError> {
    if spec.cluster.trim().is_empty() {
        return Err(FanoutPlanError::MissingCluster);
    }
    if spec.target.trim().is_empty() {
        return Err(FanoutPlanError::MissingTarget);
    }
    let initial_action = next_fanout_action(spec, &FanoutCheckpoint::default());
    if matches!(initial_action, FanoutAction::Stop { .. }) {
        return Err(FanoutPlanError::InvalidSpec);
    }
    let strategy = select_backfill_strategy(&helius_plan);
    let estimate = HeliusCreditCatalog::documentation_snapshot_2026_08_03()
        .estimate_backfill(&strategy, expected_transactions);
    Ok(FanoutExecutionPlan {
        cluster: spec.cluster.clone(),
        target: spec.target.clone(),
        helius_plan: helius_plan.clone(),
        strategy,
        expected_transactions,
        estimated_credits: estimate.credits,
        credit_catalog_version: estimate.catalog_version,
        initial_action,
        mutates_chain: false,
        requires_network: true,
        requires_api_key: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::Commitment;

    fn spec() -> FanoutSpec {
        FanoutSpec {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            signature_page_limit: 1000,
            max_signature_pages: 2,
            max_transactions: 1_500,
            commitment: Commitment::Finalized,
        }
    }

    #[test]
    fn free_plan_is_planned_as_standard_rpc_without_chain_mutation() {
        let plan = plan_fanout(&spec(), HeliusPlan::Free, 1_500).unwrap();
        assert_eq!(plan.strategy, BackfillStrategy::StandardRpcFanout);
        assert_eq!(plan.estimated_credits, Some(1_502));
        assert!(!plan.mutates_chain);
        assert!(plan.requires_network);
        assert!(plan.requires_api_key);
    }

    #[test]
    fn invalid_spec_is_rejected_before_network_execution() {
        let mut invalid = spec();
        invalid.signature_page_limit = 0;
        assert_eq!(
            plan_fanout(&invalid, HeliusPlan::Free, 1),
            Err(FanoutPlanError::InvalidSpec)
        );
    }

    #[test]
    fn request_parser_is_canonical_for_surface_projections() {
        let request = FanoutPlanRequest {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            helius_plan: "FREE".to_string(),
            signature_page_limit: 1000,
            max_signature_pages: 2,
            max_transactions: 100,
            expected_transactions: 100,
            commitment: "FINALIZED".to_string(),
        };
        let plan = plan_fanout_request(request).unwrap();
        let value = serde_json::to_value(plan).unwrap();
        assert_eq!(value["strategy"], "standard_rpc_fanout");
        assert_eq!(value["mutates_chain"], false);
        assert_eq!(value["requires_network"], true);
    }
}

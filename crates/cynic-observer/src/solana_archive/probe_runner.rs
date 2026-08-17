use super::{
    summarize_provider_probe, ProbeAction, ProbeActionKind, ProbePlan, ProbeSample,
    ProviderCapabilityProfile, ProviderKind, ProviderProbeSpec,
};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

pub type BoxProbeFuture = Pin<Box<dyn Future<Output = Result<Vec<ProbeSample>, String>> + Send>>;

pub trait ProbeExecutor: Send + Sync {
    fn execute(&self, action: ProbeAction, target: &super::ProbeTarget) -> BoxProbeFuture;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProbeRunReport {
    pub schema: String,
    pub corpus_id: String,
    pub actions_attempted: u64,
    pub profiles: Vec<ProviderCapabilityProfile>,
    pub errors: Vec<String>,
    pub mutates_chain: bool,
}

pub async fn run_probe_plan<E: ProbeExecutor>(plan: &ProbePlan, executor: &E) -> ProbeRunReport {
    let mut samples_by_provider: BTreeMap<ProviderKind, Vec<ProbeSample>> = BTreeMap::new();
    let mut specs_by_provider: BTreeMap<ProviderKind, ProviderProbeSpec> = BTreeMap::new();
    let mut errors = Vec::new();
    for action in &plan.actions {
        let action_result = executor.execute(action.clone(), &plan.target).await;
        match action_result {
            Ok(samples) => {
                samples_by_provider
                    .entry(action.provider.clone())
                    .or_default()
                    .extend(samples);
                specs_by_provider
                    .entry(action.provider.clone())
                    .or_insert_with(|| spec_for_action(action));
            }
            Err(error) => errors.push(format!("{:?}/{}: {error}", action.provider, action.method)),
        }
    }
    let profiles = samples_by_provider
        .into_iter()
        .filter_map(|(provider, samples)| {
            specs_by_provider
                .remove(&provider)
                .map(|spec| summarize_provider_probe(&spec, &samples))
        })
        .collect();
    ProbeRunReport {
        schema: "cynic.solana.provider_probe_report.v1".to_string(),
        corpus_id: plan.target.corpus_id.clone(),
        actions_attempted: plan.actions.len() as u64,
        profiles,
        errors,
        mutates_chain: false,
    }
}

fn spec_for_action(action: &ProbeAction) -> ProviderProbeSpec {
    let (max_signature_pages, max_transactions, max_blocks) = match action.kind {
        ProbeActionKind::Signatures => (action.max_requests as u32, 0, 0),
        ProbeActionKind::Transactions => (0, action.max_requests, 0),
        ProbeActionKind::Blocks => (0, 0, action.max_requests as u32),
    };
    ProviderProbeSpec {
        provider: action.provider.clone(),
        endpoint_class: action.endpoint_class.clone(),
        methods: vec![action.method.clone()],
        max_signature_pages,
        max_transactions,
        max_blocks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        build_probe_plan, CreditConfidence, CreditMeasurement, ProbeOutcome, ProbeTarget,
        ProviderRegistry, ProviderRegistryEntry, RequestMetrics,
    };

    struct FixtureExecutor;

    impl ProbeExecutor for FixtureExecutor {
        fn execute(&self, action: ProbeAction, _target: &ProbeTarget) -> BoxProbeFuture {
            Box::pin(async move {
                Ok(vec![ProbeSample {
                    metrics: RequestMetrics {
                        method: action.method,
                        provider: action.provider,
                        started_at_unix_ms: 1,
                        latency_ms: 12,
                        request_bytes: 20,
                        response_bytes: 40,
                        http_status: 200,
                        records_returned: 2,
                        credits: CreditMeasurement {
                            credits: Some(1),
                            confidence: CreditConfidence::CatalogEstimate,
                            catalog_version: Some("fixture".to_string()),
                        },
                    },
                    outcome: ProbeOutcome::Success,
                }])
            })
        }
    }

    #[tokio::test]
    async fn runner_aggregates_plan_actions_without_chain_mutation() {
        let entry = ProviderRegistryEntry {
            provider: ProviderKind::Helius,
            endpoint_ref: "env:RPC".to_string(),
            auth_ref: Some("env:KEY".to_string()),
            enabled: true,
            probe: ProviderProbeSpec {
                provider: ProviderKind::Helius,
                endpoint_class: "rpc".to_string(),
                methods: vec!["getTransaction".to_string()],
                max_signature_pages: 0,
                max_transactions: 2,
                max_blocks: 0,
            },
        };
        let plan = build_probe_plan(
            &ProviderRegistry {
                schema: "v1".to_string(),
                entries: vec![entry],
            },
            ProbeTarget {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                corpus_id: "fixture".to_string(),
            },
        )
        .unwrap();
        let report = run_probe_plan(&plan, &FixtureExecutor).await;
        assert_eq!(report.actions_attempted, 1);
        assert_eq!(report.profiles[0].total_records, 2);
        assert!(!report.mutates_chain);
    }
}

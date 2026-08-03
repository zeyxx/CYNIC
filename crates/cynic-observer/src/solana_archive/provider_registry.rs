use super::{ProviderKind, ProviderProbeSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRegistryEntry {
    pub provider: ProviderKind,
    pub endpoint_ref: String,
    pub auth_ref: Option<String>,
    pub enabled: bool,
    pub probe: ProviderProbeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProviderRegistry {
    pub schema: String,
    pub entries: Vec<ProviderRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeTarget {
    pub cluster: String,
    pub target: String,
    pub corpus_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeActionKind {
    Signatures,
    Transactions,
    Blocks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeAction {
    pub provider: ProviderKind,
    pub endpoint_ref: String,
    pub auth_ref: Option<String>,
    pub kind: ProbeActionKind,
    pub method: String,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbePlan {
    pub schema: String,
    pub target: ProbeTarget,
    pub actions: Vec<ProbeAction>,
    pub total_request_budget: u64,
    pub mutates_chain: bool,
    pub requires_network: bool,
    pub requires_secret_injection: bool,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbePlanError {
    EmptyCluster,
    EmptyTarget,
    EmptyCorpus,
    EmptyRegistry,
    InvalidEntry(String),
}

pub fn build_probe_plan(
    registry: &ProviderRegistry,
    target: ProbeTarget,
) -> Result<ProbePlan, ProbePlanError> {
    if target.cluster.trim().is_empty() {
        return Err(ProbePlanError::EmptyCluster);
    }
    if target.target.trim().is_empty() {
        return Err(ProbePlanError::EmptyTarget);
    }
    if target.corpus_id.trim().is_empty() {
        return Err(ProbePlanError::EmptyCorpus);
    }
    if registry.entries.is_empty() {
        return Err(ProbePlanError::EmptyRegistry);
    }

    let mut actions = Vec::new();
    let mut residual_unknowns = Vec::new();
    for entry in registry.entries.iter().filter(|entry| entry.enabled) {
        if entry.endpoint_ref.trim().is_empty() {
            return Err(ProbePlanError::InvalidEntry(format!(
                "{} has no endpoint_ref",
                entry.provider_name()
            )));
        }
        let spec = &entry.probe;
        if spec.methods.is_empty() {
            residual_unknowns.push(format!("{} has no declared methods", entry.provider_name()));
        }
        for method in &spec.methods {
            let (kind, max_requests) = match method.as_str() {
                "getSignaturesForAddress" => (
                    ProbeActionKind::Signatures,
                    u64::from(spec.max_signature_pages),
                ),
                "getTransaction" => (ProbeActionKind::Transactions, spec.max_transactions),
                "getBlock" => (ProbeActionKind::Blocks, u64::from(spec.max_blocks)),
                other => {
                    residual_unknowns.push(format!(
                        "{} declares unsupported probe method {other}",
                        entry.provider_name()
                    ));
                    continue;
                }
            };
            if max_requests == 0 {
                residual_unknowns.push(format!(
                    "{} has zero budget for {method}",
                    entry.provider_name()
                ));
                continue;
            }
            actions.push(ProbeAction {
                provider: entry.provider.clone(),
                endpoint_ref: entry.endpoint_ref.clone(),
                auth_ref: entry.auth_ref.clone(),
                kind,
                method: method.clone(),
                max_requests,
            });
        }
    }
    if actions.is_empty() {
        return Err(ProbePlanError::EmptyRegistry);
    }
    let total_request_budget = actions.iter().map(|action| action.max_requests).sum();
    Ok(ProbePlan {
        schema: "cynic.solana.provider_probe_plan.v1".to_string(),
        target,
        actions,
        total_request_budget,
        mutates_chain: false,
        requires_network: true,
        requires_secret_injection: true,
        residual_unknowns,
    })
}

impl ProviderRegistryEntry {
    fn provider_name(&self) -> String {
        format!("{:?}", self.provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(provider: ProviderKind, endpoint: &str) -> ProviderRegistryEntry {
        ProviderRegistryEntry {
            provider: provider.clone(),
            endpoint_ref: endpoint.to_string(),
            auth_ref: Some(format!("secret/{provider:?}")),
            enabled: true,
            probe: ProviderProbeSpec {
                provider,
                endpoint_class: "rpc".to_string(),
                methods: vec![
                    "getSignaturesForAddress".to_string(),
                    "getTransaction".to_string(),
                ],
                max_signature_pages: 10,
                max_transactions: 50,
                max_blocks: 0,
            },
        }
    }

    #[test]
    fn plan_is_bounded_and_contains_no_secret_values() {
        let plan = build_probe_plan(
            &ProviderRegistry {
                schema: "cynic.solana.provider_registry.v1".to_string(),
                entries: vec![entry(ProviderKind::Helius, "endpoint/helius")],
            },
            ProbeTarget {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                corpus_id: "asdfasdfa-v0".to_string(),
            },
        )
        .unwrap();
        assert_eq!(plan.total_request_budget, 60);
        assert!(!plan.mutates_chain);
        assert_eq!(plan.actions[0].auth_ref.as_deref(), Some("secret/Helius"));
    }

    #[test]
    fn disabled_provider_is_not_scheduled() {
        let mut disabled = entry(ProviderKind::Alchemy, "endpoint/alchemy");
        disabled.enabled = false;
        assert_eq!(
            build_probe_plan(
                &ProviderRegistry {
                    schema: "v1".to_string(),
                    entries: vec![disabled],
                },
                ProbeTarget {
                    cluster: "mainnet".to_string(),
                    target: "mint".to_string(),
                    corpus_id: "corpus".to_string(),
                }
            ),
            Err(ProbePlanError::EmptyRegistry)
        );
    }
}

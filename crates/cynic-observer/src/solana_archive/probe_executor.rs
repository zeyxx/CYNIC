use super::{
    BoxProbeFuture, Commitment, JsonRpcAdapter, ProbeAction, ProbeActionKind, ProbeExecutor,
    ProbeOutcome, ProbeSample, ProbeTarget, ProviderKind, SignatureQuery, SignatureSource,
    TransactionQuery, TransactionSource,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct JsonRpcProbeExecutor {
    adapters: BTreeMap<ProviderKind, JsonRpcAdapter>,
    corpus_signatures: BTreeMap<String, Vec<String>>,
}

impl JsonRpcProbeExecutor {
    pub fn new(
        adapters: BTreeMap<ProviderKind, JsonRpcAdapter>,
        corpus_signatures: BTreeMap<String, Vec<String>>,
    ) -> Self {
        Self {
            adapters,
            corpus_signatures,
        }
    }
}

impl ProbeExecutor for JsonRpcProbeExecutor {
    fn execute(&self, action: ProbeAction, target: &ProbeTarget) -> BoxProbeFuture {
        let Some(adapter) = self.adapters.get(&action.provider).cloned() else {
            return Box::pin(async move {
                Err(format!(
                    "adapter unavailable for provider {:?}",
                    action.provider
                ))
            });
        };
        let target = target.clone();
        let signatures = self
            .corpus_signatures
            .get(&target.corpus_id)
            .cloned()
            .unwrap_or_default();
        Box::pin(async move {
            match action.kind {
                ProbeActionKind::Signatures => {
                    probe_signatures(adapter, target, action.max_requests as u32).await
                }
                ProbeActionKind::Transactions => {
                    if signatures.is_empty() {
                        return Err(
                            "transaction probe requires signatures in the declared corpus"
                                .to_string(),
                        );
                    }
                    probe_transactions(adapter, target, signatures, action.max_requests).await
                }
                ProbeActionKind::Blocks => {
                    Err("block probe is not yet implemented by JsonRpcProbeExecutor".to_string())
                }
            }
        })
    }
}

async fn probe_signatures(
    adapter: JsonRpcAdapter,
    target: ProbeTarget,
    max_pages: u32,
) -> Result<Vec<ProbeSample>, String> {
    let mut before = None;
    let mut samples = Vec::new();
    for _ in 0..max_pages {
        let page = adapter
            .fetch_signatures(SignatureQuery {
                cluster: target.cluster.clone(),
                target: target.target.clone(),
                before,
                until: None,
                limit: 1_000,
                commitment: Commitment::Finalized,
            })
            .await
            .map_err(|error| format_source_error(&error))?;
        before = page.next_before.clone();
        samples.push(ProbeSample {
            metrics: page.metrics,
            outcome: ProbeOutcome::Success,
        });
        if before.is_none() {
            break;
        }
    }
    Ok(samples)
}

async fn probe_transactions(
    adapter: JsonRpcAdapter,
    target: ProbeTarget,
    signatures: Vec<String>,
    max_transactions: u64,
) -> Result<Vec<ProbeSample>, String> {
    let mut samples = Vec::new();
    for signature in signatures.into_iter().take(max_transactions as usize) {
        let transaction = adapter
            .fetch_transaction(TransactionQuery {
                cluster: target.cluster.clone(),
                signature,
                commitment: Commitment::Finalized,
            })
            .await
            .map_err(|error| format_source_error(&error))?;
        samples.push(ProbeSample {
            metrics: transaction.metrics,
            outcome: ProbeOutcome::Success,
        });
    }
    Ok(samples)
}

fn format_source_error(error: &super::SourceError) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_corpus_is_explicit_for_transaction_probe() {
        let mut adapters = BTreeMap::new();
        adapters.insert(
            ProviderKind::PublicRpc,
            JsonRpcAdapter::public_rpc("https://api.mainnet.solana.com"),
        );
        let executor = JsonRpcProbeExecutor::new(adapters, BTreeMap::new());
        let action = ProbeAction {
            provider: ProviderKind::PublicRpc,
            endpoint_ref: "https://api.mainnet.solana.com".to_string(),
            auth_ref: None,
            endpoint_class: "public".to_string(),
            kind: ProbeActionKind::Transactions,
            method: "getTransaction".to_string(),
            max_requests: 1,
        };
        let future = executor.execute(
            action,
            &ProbeTarget {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                corpus_id: "empty".to_string(),
            },
        );
        assert!(future.await.unwrap_err().contains("requires signatures"));
    }
}

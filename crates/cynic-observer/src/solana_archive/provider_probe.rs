use super::{ProviderKind, RawTransactionObservation, RequestMetrics, SignaturePage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveDepth {
    Unknown,
    RecentOnly,
    Extended,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    Success,
    HttpError,
    RpcError,
    RateLimited,
    Timeout,
    InvalidResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeSample {
    pub metrics: RequestMetrics,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProbeSpec {
    pub provider: ProviderKind,
    pub endpoint_class: String,
    pub methods: Vec<String>,
    pub max_signature_pages: u32,
    pub max_transactions: u64,
    pub max_blocks: u32,
}

impl ProviderProbeSpec {
    pub fn as_budget(&self) -> ProbeBudget {
        ProbeBudget {
            signature_pages: self.max_signature_pages,
            transactions: self.max_transactions,
            blocks: self.max_blocks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeBudget {
    pub signature_pages: u32,
    pub transactions: u64,
    pub blocks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilityProfile {
    pub schema: String,
    pub provider: ProviderKind,
    pub endpoint_class: String,
    pub methods_supported: Vec<String>,
    pub archive_depth: ArchiveDepth,
    pub max_page_size_observed: Option<u16>,
    pub rate_limit_observed: Option<u64>,
    pub latency_p50_ms: Option<u64>,
    pub latency_p95_ms: Option<u64>,
    pub success_rate_micros: Option<u64>,
    pub error_rate_micros: Option<u64>,
    pub total_samples: u64,
    pub total_records: u64,
    pub estimated_credits: u64,
    pub residual_unknowns: Vec<String>,
}

pub fn summarize_provider_probe(
    spec: &ProviderProbeSpec,
    samples: &[ProbeSample],
) -> ProviderCapabilityProfile {
    let mut methods = BTreeSet::new();
    let mut latencies = Vec::new();
    let mut success_count = 0_u64;
    let mut rate_limited_count = 0_u64;
    let mut records = 0_u64;
    let mut estimated_credits = 0_u64;
    let mut max_page_size: Option<u16> = None;
    let mut residual_unknowns = Vec::new();

    for sample in samples {
        methods.insert(sample.metrics.method.clone());
        latencies.push(sample.metrics.latency_ms);
        records += sample.metrics.records_returned;
        estimated_credits += sample.metrics.credits.credits.unwrap_or_default();
        if matches!(sample.outcome, ProbeOutcome::Success) {
            success_count += 1;
        }
        if matches!(sample.outcome, ProbeOutcome::RateLimited) {
            rate_limited_count += 1;
        }
        max_page_size = Some(
            max_page_size
                .unwrap_or_default()
                .max(sample.metrics.records_returned.min(u64::from(u16::MAX)) as u16),
        );
    }
    latencies.sort_unstable();
    if samples.is_empty() {
        residual_unknowns.push("no provider samples observed".to_string());
    }
    if rate_limited_count > 0 {
        residual_unknowns.push("rate limit observed during probe".to_string());
    }
    for method in &spec.methods {
        if !methods.contains(method) {
            residual_unknowns.push(format!("method not probed: {method}"));
        }
    }

    let total = samples.len() as u64;
    let success_rate = ratio_micros(success_count, total);
    ProviderCapabilityProfile {
        schema: "cynic.solana.provider_capability_profile.v1".to_string(),
        provider: spec.provider.clone(),
        endpoint_class: spec.endpoint_class.clone(),
        methods_supported: methods.into_iter().collect(),
        archive_depth: ArchiveDepth::Unknown,
        max_page_size_observed: max_page_size,
        rate_limit_observed: (rate_limited_count > 0).then_some(rate_limited_count),
        latency_p50_ms: percentile(&latencies, 50),
        latency_p95_ms: percentile(&latencies, 95),
        success_rate_micros: success_rate,
        error_rate_micros: success_rate.map(|value| 1_000_000 - value),
        total_samples: total,
        total_records: records,
        estimated_credits,
        residual_unknowns,
    }
}

pub fn samples_from_signature_pages(pages: &[SignaturePage]) -> Vec<ProbeSample> {
    pages
        .iter()
        .map(|page| ProbeSample {
            metrics: page.metrics.clone(),
            outcome: ProbeOutcome::Success,
        })
        .collect()
}

pub fn samples_from_transactions(transactions: &[RawTransactionObservation]) -> Vec<ProbeSample> {
    transactions
        .iter()
        .map(|transaction| ProbeSample {
            metrics: transaction.metrics.clone(),
            outcome: ProbeOutcome::Success,
        })
        .collect()
}

fn percentile(values: &[u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let rank = (values.len() * percentile).div_ceil(100).max(1);
    let index = (rank - 1).min(values.len() - 1);
    values.get(index).copied()
}

fn ratio_micros(numerator: u64, denominator: u64) -> Option<u64> {
    (denominator > 0).then(|| numerator.saturating_mul(1_000_000) / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{CreditConfidence, CreditMeasurement};

    fn sample(method: &str, latency_ms: u64, outcome: ProbeOutcome) -> ProbeSample {
        ProbeSample {
            metrics: RequestMetrics {
                method: method.to_string(),
                provider: ProviderKind::Helius,
                started_at_unix_ms: 1,
                latency_ms,
                request_bytes: 10,
                response_bytes: 20,
                http_status: 200,
                records_returned: 2,
                credits: CreditMeasurement {
                    credits: Some(1),
                    confidence: CreditConfidence::CatalogEstimate,
                    catalog_version: Some("fixture".to_string()),
                },
            },
            outcome,
        }
    }

    fn spec() -> ProviderProbeSpec {
        ProviderProbeSpec {
            provider: ProviderKind::Helius,
            endpoint_class: "helius_rpc".to_string(),
            methods: vec!["getTransaction".to_string(), "getBlock".to_string()],
            max_signature_pages: 100,
            max_transactions: 500,
            max_blocks: 10,
        }
    }

    #[test]
    fn probe_profile_preserves_unknown_methods_and_latency_distribution() {
        let profile = summarize_provider_probe(
            &spec(),
            &[
                sample("getTransaction", 10, ProbeOutcome::Success),
                sample("getTransaction", 30, ProbeOutcome::RateLimited),
            ],
        );
        assert_eq!(profile.latency_p50_ms, Some(10));
        assert_eq!(profile.latency_p95_ms, Some(10));
        assert_eq!(profile.success_rate_micros, Some(500_000));
        assert_eq!(profile.rate_limit_observed, Some(1));
        assert!(
            profile
                .residual_unknowns
                .iter()
                .any(|value| value.contains("getBlock"))
        );
    }

    #[test]
    fn empty_probe_is_unknown_not_healthy() {
        let profile = summarize_provider_probe(&spec(), &[]);
        assert_eq!(profile.success_rate_micros, None);
        assert_eq!(profile.archive_depth, ArchiveDepth::Unknown);
        assert!(!profile.residual_unknowns.is_empty());
    }
}

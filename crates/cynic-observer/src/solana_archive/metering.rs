use super::ProviderKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeliusPlan {
    Free,
    Developer,
    Business,
    Professional,
    Enterprise,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackfillStrategy {
    StandardRpcFanout,
    IndexedHistory,
}

pub fn select_backfill_strategy(plan: &HeliusPlan) -> BackfillStrategy {
    if HeliusCreditCatalog::supports_indexed_history(plan) {
        BackfillStrategy::IndexedHistory
    } else {
        BackfillStrategy::StandardRpcFanout
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditConfidence {
    CatalogEstimate,
    ProjectUsageDeltaIsolated,
    ProjectUsageDeltaShared,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectUsageSnapshot {
    pub captured_at_unix_ms: u128,
    pub credits_used: u64,
}

pub fn reconcile_project_usage(
    before: &ProjectUsageSnapshot,
    after: &ProjectUsageSnapshot,
    requests_in_window: u64,
) -> CreditMeasurement {
    if after.captured_at_unix_ms < before.captured_at_unix_ms
        || after.credits_used < before.credits_used
    {
        return CreditMeasurement {
            credits: None,
            confidence: CreditConfidence::Unknown,
            catalog_version: None,
        };
    }
    CreditMeasurement {
        credits: Some(after.credits_used - before.credits_used),
        confidence: if requests_in_window == 1 {
            CreditConfidence::ProjectUsageDeltaIsolated
        } else {
            CreditConfidence::ProjectUsageDeltaShared
        },
        catalog_version: None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreditMeasurement {
    pub credits: Option<u64>,
    pub confidence: CreditConfidence,
    pub catalog_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestMetrics {
    pub method: String,
    pub provider: ProviderKind,
    pub started_at_unix_ms: u128,
    pub latency_ms: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub http_status: u16,
    pub records_returned: u64,
    pub credits: CreditMeasurement,
}

impl RequestMetrics {
    pub fn unmetered(provider: ProviderKind, method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            provider,
            started_at_unix_ms: 0,
            latency_ms: 0,
            request_bytes: 0,
            response_bytes: 0,
            http_status: 0,
            records_returned: 0,
            credits: CreditMeasurement {
                credits: None,
                confidence: CreditConfidence::Unknown,
                catalog_version: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeliusCreditCatalog {
    pub version: String,
    pub standard_rpc_credits: u64,
    pub indexed_history_credits: u64,
    pub das_credits: u64,
    pub enhanced_transaction_credits: u64,
}

impl HeliusCreditCatalog {
    /// Documentation snapshot. Runtime reports retain this version so a later
    /// price change does not rewrite historical estimates.
    pub fn documentation_snapshot_2026_08_03() -> Self {
        Self {
            version: "helius-credits-docs-2026-08-03".to_string(),
            standard_rpc_credits: 1,
            indexed_history_credits: 50,
            das_credits: 10,
            enhanced_transaction_credits: 100,
        }
    }

    pub fn estimate_rpc_method(&self, method: &str) -> CreditMeasurement {
        let credits = match method {
            "getTransactionsForAddress" => self.indexed_history_credits,
            _ => self.standard_rpc_credits,
        };
        CreditMeasurement {
            credits: Some(credits),
            confidence: CreditConfidence::CatalogEstimate,
            catalog_version: Some(self.version.clone()),
        }
    }

    pub fn supports_indexed_history(plan: &HeliusPlan) -> bool {
        matches!(
            plan,
            HeliusPlan::Developer
                | HeliusPlan::Business
                | HeliusPlan::Professional
                | HeliusPlan::Enterprise
        )
    }

    pub fn estimate_backfill(
        &self,
        strategy: &BackfillStrategy,
        expected_records: u64,
    ) -> CreditMeasurement {
        let credits = match strategy {
            BackfillStrategy::IndexedHistory => {
                ceil_div(expected_records, 100) * self.indexed_history_credits
            }
            BackfillStrategy::StandardRpcFanout => {
                ceil_div(expected_records, 1_000) * self.standard_rpc_credits
                    + expected_records * self.standard_rpc_credits
            }
        };
        CreditMeasurement {
            credits: Some(credits),
            confidence: CreditConfidence::CatalogEstimate,
            catalog_version: Some(self.version.clone()),
        }
    }
}

fn ceil_div(value: u64, divisor: u64) -> u64 {
    if value == 0 {
        0
    } else {
        1 + (value - 1) / divisor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MetricsAggregate {
    pub requests: u64,
    pub records: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub latency_ms: u64,
    pub estimated_credits: u64,
    pub observed_credit_delta: Option<u64>,
}

impl MetricsAggregate {
    pub fn record(&mut self, metrics: &RequestMetrics) {
        self.requests += 1;
        self.records += metrics.records_returned;
        self.request_bytes += metrics.request_bytes;
        self.response_bytes += metrics.response_bytes;
        self.latency_ms += metrics.latency_ms;
        self.estimated_credits += metrics.credits.credits.unwrap_or_default();
    }

    pub fn average_latency_ms(&self) -> Option<u64> {
        (self.requests > 0).then(|| self.latency_ms / self.requests)
    }

    pub fn credits_per_record_micros(&self) -> Option<u64> {
        (self.records > 0).then(|| self.estimated_credits * 1_000_000 / self.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_plan_does_not_claim_indexed_history_entitlement() {
        assert!(!HeliusCreditCatalog::supports_indexed_history(
            &HeliusPlan::Free
        ));
        assert!(HeliusCreditCatalog::supports_indexed_history(
            &HeliusPlan::Developer
        ));
    }

    #[test]
    fn catalog_distinguishes_standard_rpc_and_indexed_history() {
        let catalog = HeliusCreditCatalog::documentation_snapshot_2026_08_03();
        assert_eq!(
            catalog.estimate_rpc_method("getTransaction").credits,
            Some(1)
        );
        assert_eq!(
            catalog
                .estimate_rpc_method("getTransactionsForAddress")
                .credits,
            Some(50)
        );
    }

    #[test]
    fn free_plan_uses_standard_rpc_cost_model() {
        let catalog = HeliusCreditCatalog::documentation_snapshot_2026_08_03();
        let strategy = select_backfill_strategy(&HeliusPlan::Free);
        assert_eq!(strategy, BackfillStrategy::StandardRpcFanout);
        assert_eq!(
            catalog.estimate_backfill(&strategy, 1_500).credits,
            Some(1_502)
        );
    }
}

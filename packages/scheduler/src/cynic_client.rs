//! CYNIC API client for reputation lookups

use crate::{ReputationScore, Result, SchedulerError, Verdict, PHI_INV};
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Cached reputation entry
struct CacheEntry {
    score: ReputationScore,
    cached_at: Instant,
}

/// CYNIC API client with caching
pub struct CynicClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    cache_ttl: Duration,
}

/// CYNIC judgment request
#[derive(Debug, Serialize)]
struct JudgeRequest {
    item: String,
    context: String,
}

/// CYNIC judgment response
#[derive(Debug, Deserialize)]
struct JudgeResponse {
    #[serde(rename = "qScore")]
    q_score: f64,
    verdict: Verdict,
    confidence: f64,
    #[serde(rename = "kScore")]
    k_score: Option<f64>,
    #[serde(rename = "eScore")]
    e_score: Option<f64>,
}

impl CynicClient {
    /// Create a new CYNIC client
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        cache_ttl: Duration,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| SchedulerError::cynic_api(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: base_url.into(),
            api_key,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
        })
    }

    /// Get reputation score for a wallet address
    pub async fn get_wallet_reputation(&self, address: &str) -> Result<ReputationScore> {
        // Check cache first
        if let Some(cached) = self.get_cached(address) {
            debug!(address = %address, "Cache hit for wallet reputation");
            return Ok(cached);
        }

        // Query CYNIC API
        let score = self.query_reputation(address, "wallet").await?;

        // Cache result
        self.cache_score(address, score.clone());

        Ok(score)
    }

    /// Get reputation score for a token mint
    pub async fn get_token_reputation(&self, mint: &str) -> Result<ReputationScore> {
        let cache_key = format!("token:{}", mint);

        // Check cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            debug!(mint = %mint, "Cache hit for token reputation");
            return Ok(cached);
        }

        // Query CYNIC API
        let score = self.query_reputation(mint, "token").await?;

        // Cache result
        self.cache_score(&cache_key, score.clone());

        Ok(score)
    }

    /// Query CYNIC API for reputation
    async fn query_reputation(&self, item: &str, context: &str) -> Result<ReputationScore> {
        let url = format!("{}/api/judge", self.base_url);

        let mut request = self.client
            .post(&url)
            .json(&JudgeRequest {
                item: item.to_string(),
                context: context.to_string(),
            });

        if let Some(ref key) = self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "CYNIC API request failed");
                SchedulerError::cynic_api(format!("Request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "CYNIC API error response");

            // Return default score on API error (don't block transactions)
            return Ok(ReputationScore::default());
        }

        let judge_response: JudgeResponse = response
            .json()
            .await
            .map_err(|e| SchedulerError::cynic_api(format!("Failed to parse response: {}", e)))?;

        Ok(ReputationScore {
            q_score: judge_response.q_score,
            verdict: judge_response.verdict,
            k_score: judge_response.k_score,
            e_score: judge_response.e_score,
            confidence: judge_response.confidence.min(PHI_INV * 100.0), // Cap at 61.8%
        })
    }

    /// Get cached score if still valid
    fn get_cached(&self, key: &str) -> Option<ReputationScore> {
        let cache = self.cache.read();
        if let Some(entry) = cache.get(key) {
            if entry.cached_at.elapsed() < self.cache_ttl {
                return Some(entry.score.clone());
            }
        }
        None
    }

    /// Cache a reputation score
    fn cache_score(&self, key: &str, score: ReputationScore) {
        let mut cache = self.cache.write();
        cache.insert(
            key.to_string(),
            CacheEntry {
                score,
                cached_at: Instant::now(),
            },
        );

        // Prune old entries if cache is too large
        if cache.len() > 10_000 {
            let now = Instant::now();
            cache.retain(|_, entry| now.duration_since(entry.cached_at) < self.cache_ttl);
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read();
        let total = cache.len();
        let valid = cache
            .values()
            .filter(|e| e.cached_at.elapsed() < self.cache_ttl)
            .count();
        (valid, total)
    }
}

/// Batch reputation lookup for multiple addresses
impl CynicClient {
    /// Get reputation scores for multiple wallets (parallel)
    pub async fn get_batch_wallet_reputation(
        &self,
        addresses: &[&str],
    ) -> HashMap<String, ReputationScore> {
        let mut results = HashMap::new();

        // First, collect cached results
        let mut uncached = Vec::new();
        for addr in addresses {
            if let Some(cached) = self.get_cached(addr) {
                results.insert(addr.to_string(), cached);
            } else {
                uncached.push(*addr);
            }
        }

        // Query uncached in parallel (with limit)
        let futures: Vec<_> = uncached
            .iter()
            .take(100) // Limit concurrent requests
            .map(|addr| async move {
                let score = self.get_wallet_reputation(addr).await.unwrap_or_default();
                (addr.to_string(), score)
            })
            .collect();

        let fetched: Vec<_> = futures::future::join_all(futures).await;

        for (addr, score) in fetched {
            results.insert(addr, score);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_reputation() {
        let score = ReputationScore::default();
        assert!((score.q_score - 50.0).abs() < 1e-10);
        assert_eq!(score.verdict, Verdict::Howl);
        assert!((score.confidence - 61.8).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = CynicClient::new(
            "https://example.com",
            None,
            Duration::from_secs(60),
            Duration::from_millis(100),
        );
        assert!(client.is_ok());
    }
}

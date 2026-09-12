//! Dynamic routing calculator — computes Dog-to-domain routing based on observed latencies.
//!
//! Consumes dog_performance observations (dog_id, domain, latency_ms, success).
//! Produces live routing table: domain → Dogs suitable for latency SLA.
//!
//! Data-centric: routing adapts as Dogs degrade, new Dogs arrive, or patterns shift.
//! No static config → always reflects current reality.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::ports::routing_event_port::RoutingEventPort;

/// Live routing calculator — selects suitable Dogs based on recent performance.
#[derive(Clone)]
pub struct RoutingCalculator {
    /// Domain -> Vec<DogPerformance> (Dogs sorted by latency)
    /// Refreshed every N observations or on-demand
    cache: Arc<RwLock<HashMap<String, Vec<DogPerformance>>>>,
    /// Optional event sink port for observations
    event_port: Option<Arc<dyn RoutingEventPort>>,
    dog_nodes: HashMap<String, String>,
}

impl std::fmt::Debug for RoutingCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingCalculator")
            .field("cache", &self.cache)
            .field(
                "event_port",
                &self
                    .event_port
                    .as_ref()
                    .map(|_| "Arc<dyn RoutingEventPort>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct DogPerformance {
    pub dog_id: String,
    pub avg_latency_ms: u32,
    pub success_rate: f64, // 0.0-1.0
    pub sample_count: usize,
}

impl RoutingCalculator {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            event_port: None,
            dog_nodes: HashMap::new(),
        }
    }

    pub fn with_event_port(event_port: Arc<dyn RoutingEventPort>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            event_port: Some(event_port),
            dog_nodes: HashMap::new(),
        }
    }

    pub fn set_dog_nodes(&mut self, dog_nodes: HashMap<String, String>) {
        self.dog_nodes = dog_nodes;
    }

    /// Record a performance observation for a Dog in a domain.
    /// Updates the in-memory routing table immediately.
    pub fn record_observation(&self, domain: &str, dog_id: &str, latency_ms: u32, success: bool) {
        // Update in-memory cache
        {
            let mut cache = self.cache.write().unwrap();
            let perfs = cache.entry(domain.to_string()).or_default();

            if let Some(perf) = perfs.iter_mut().find(|p| p.dog_id == dog_id) {
                // Moving average: new_avg = (old_avg * count + new_val) / (count + 1)
                let count = perf.sample_count as u64;
                perf.avg_latency_ms =
                    ((perf.avg_latency_ms as u64 * count + latency_ms as u64) / (count + 1)) as u32;
                let successes =
                    (perf.success_rate * count as f64) + if success { 1.0 } else { 0.0 };
                perf.sample_count += 1;
                perf.success_rate = successes / perf.sample_count as f64;
            } else {
                perfs.push(DogPerformance {
                    dog_id: dog_id.to_string(),
                    avg_latency_ms: latency_ms,
                    success_rate: if success { 1.0 } else { 0.0 },
                    sample_count: 1,
                });
            }

            // Sort by success rate (descending) then latency (ascending)
            perfs.sort_by(|a, b| {
                b.success_rate
                    .partial_cmp(&a.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.avg_latency_ms.cmp(&b.avg_latency_ms))
            });
        }

        // Persist to event port if configured
        if let Some(port) = &self.event_port {
            let port = Arc::clone(port);
            let domain = domain.to_string();
            let dog_id = dog_id.to_string();
            tokio::spawn(async move {
                let _ = port
                    .record_routing_observation(&domain, &dog_id, latency_ms, success)
                    .await;
            });
        }
    }

    /// Bulk update routing table for a domain (called by DogPerformanceCollector).
    pub fn update_domain_routing(&self, domain: &str, mut perfs: Vec<DogPerformance>) {
        let mut cache = self.cache.write().unwrap();
        perfs.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.avg_latency_ms.cmp(&b.avg_latency_ms))
        });
        cache.insert(domain.to_string(), perfs);
    }
    pub fn select_dogs(
        &self,
        domain: &str,
        max_dogs: usize,
        max_latency_ms: Option<u32>,
    ) -> Vec<String> {
        let cache = self.cache.read().unwrap();

        if let Some(perfs) = cache.get(domain) {
            perfs
                .iter()
                .filter(|p| {
                    // Filter by latency SLA if specified
                    if let Some(max_lat) = max_latency_ms {
                        p.avg_latency_ms <= max_lat
                    } else {
                        true
                    }
                })
                .filter(|p| p.success_rate >= 0.5) // Exclude fundamentally broken Dogs
                .take(max_dogs)
                .map(|p| p.dog_id.clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Clear cached performance metrics for a specific dog.
    pub fn reset_dog(&self, dog_id: &str) {
        let mut cache = self.cache.write().unwrap();
        for perfs in cache.values_mut() {
            perfs.retain(|p| p.dog_id != dog_id);
        }
        tracing::info!(dog_id = %dog_id, "RoutingCalculator reset — benefit of doubt restored");
    }
}

impl Default for RoutingCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_fastest_dog_with_good_success_rate() {
        let calc = RoutingCalculator::new();
        calc.record_observation("chess", "fast-dog", 100, true);
        calc.record_observation("chess", "slow-dog", 500, true);

        let selected = calc.select_dogs("chess", 1, None);
        assert_eq!(selected, vec!["fast-dog"]);
    }

    #[test]
    fn filters_out_broken_dogs() {
        let calc = RoutingCalculator::new();
        calc.record_observation("code", "broken-dog", 50, false);
        calc.record_observation("code", "broken-dog", 50, false);
        calc.record_observation("code", "good-dog", 200, true);

        let selected = calc.select_dogs("code", 5, None);
        assert_eq!(selected, vec!["good-dog"]);
    }
}

//! Dog performance observation collector and aggregator.
//!
//! Consumes on_dog callbacks from the pipeline and aggregates performance metrics
//! (latency, success rate) per (domain, dog_id) pair. Periodically flushes aggregated
//! metrics to RoutingCalculator for dynamic Dog routing.
//!
//! K15 seam 3: pipeline.on_dog → DogPerformanceCollector → routing_calc

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Per-Dog sample: latency and success flag.
#[derive(Debug, Clone)]
struct DogSample {
    latency_ms: u64,
    success: bool,
}

/// Accumulator for a single Dog in a domain: samples, running avg latency, success rate.
#[derive(Debug)]
struct DogAggregator {
    samples: Vec<DogSample>,
    max_samples: usize, // Windowing: keep last N samples, discard old
}

impl DogAggregator {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::new(),
            max_samples,
        }
    }

    /// Record a Dog evaluation result.
    fn observe(&mut self, latency_ms: u64, success: bool) {
        self.samples.push(DogSample {
            latency_ms,
            success,
        });
        if self.samples.len() > self.max_samples {
            self.samples.remove(0); // FIFO: discard oldest
        }
    }

    /// Compute avg_latency_ms (mean of all samples).
    fn avg_latency_ms(&self) -> u32 {
        if self.samples.is_empty() {
            return 0;
        }
        let sum: u64 = self.samples.iter().map(|s| s.latency_ms).sum();
        (sum / self.samples.len() as u64) as u32
    }

    /// Compute success_rate (% of successful evaluations).
    fn success_rate(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let successes = self.samples.iter().filter(|s| s.success).count();
        successes as f64 / self.samples.len() as f64
    }

    /// Snapshot for export to RoutingCalculator.
    fn snapshot(&self, dog_id: String) -> crate::infra::routing_calc::DogPerformance {
        crate::infra::routing_calc::DogPerformance {
            dog_id,
            avg_latency_ms: self.avg_latency_ms(),
            success_rate: self.success_rate(),
            sample_count: self.samples.len(),
        }
    }
}

/// Aggregates Dog performance observations per (domain, dog_id).
///
/// WHY: Performance metrics must be aggregated before flowing to routing_calc.
/// Raw observations are noisy (network jitter, one-off failures). Averaging over
/// a window of samples produces stable routing decisions.
///
/// WINDOW_SIZE=100: Each Dog retains last 100 evaluations. After 100 more judgments,
/// older samples decay out. This gives ~5-10min of history at typical QPS.
#[derive(Debug)]
pub struct DogPerformanceCollector {
    // domain -> (dog_id -> aggregator)
    aggregators: Arc<RwLock<HashMap<String, HashMap<String, DogAggregator>>>>,
    window_size: usize,
}

impl DogPerformanceCollector {
    /// Create new collector with default window size (100 samples per Dog).
    pub fn new() -> Self {
        Self::with_window(100)
    }

    /// Create new collector with custom window size.
    pub fn with_window(window_size: usize) -> Self {
        Self {
            aggregators: Arc::new(RwLock::new(HashMap::new())),
            window_size,
        }
    }

    /// Record a single Dog evaluation result.
    pub fn observe(&self, domain: &str, dog_id: &str, latency_ms: u64, success: bool) {
        if let Ok(mut agg) = self.aggregators.write() {
            let dogs = agg.entry(domain.to_string()).or_insert_with(HashMap::new);
            let aggregator = dogs
                .entry(dog_id.to_string())
                .or_insert_with(|| DogAggregator::new(self.window_size));
            aggregator.observe(latency_ms, success);
        }
    }

    /// Export aggregated metrics for a domain to RoutingCalculator.
    pub fn flush_domain(
        &self,
        domain: &str,
        routing_calc: &Arc<crate::infra::routing_calc::RoutingCalculator>,
    ) {
        if let Ok(agg) = self.aggregators.read() {
            if let Some(dogs) = agg.get(domain) {
                let snapshots: Vec<_> = dogs
                    .iter()
                    .map(|(dog_id, agg)| agg.snapshot(dog_id.clone()))
                    .collect();

                if !snapshots.is_empty() {
                    routing_calc.update_domain_routing(domain, snapshots);
                }
            }
        }
    }

    /// Export all domain metrics.
    pub fn flush_all(&self, routing_calc: &Arc<crate::infra::routing_calc::RoutingCalculator>) {
        if let Ok(agg) = self.aggregators.read() {
            for (domain, dogs) in agg.iter() {
                let snapshots: Vec<_> = dogs
                    .iter()
                    .map(|(dog_id, agg)| agg.snapshot(dog_id.clone()))
                    .collect();

                if !snapshots.is_empty() {
                    routing_calc.update_domain_routing(domain, snapshots);
                }
            }
        }
    }
}

impl Default for DogPerformanceCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregator_computes_averages() {
        let mut agg = DogAggregator::new(10);
        agg.observe(100, true);
        agg.observe(150, true);
        agg.observe(50, false);

        assert_eq!(agg.avg_latency_ms(), 100); // (100 + 150 + 50) / 3 = 100
        assert!((agg.success_rate() - 2.0 / 3.0).abs() < 0.01); // 2/3 ≈ 0.667
        assert_eq!(agg.samples.len(), 3);
    }

    #[test]
    fn collector_per_dog_isolation() {
        let collector = DogPerformanceCollector::new();
        collector.observe("chess", "deterministic-dog", 10, true);
        collector.observe("chess", "qwen-7b-hf", 250, true);

        // Verify they're tracked separately
        if let Ok(agg) = collector.aggregators.read() {
            if let Some(dogs) = agg.get("chess") {
                assert_eq!(dogs.len(), 2);
                assert!(dogs.contains_key("deterministic-dog"));
                assert!(dogs.contains_key("qwen-7b-hf"));
            }
        }
    }

    #[test]
    fn window_overflow_fifo() {
        let collector = DogPerformanceCollector::with_window(3);
        collector.observe("chess", "dog1", 10, true);
        collector.observe("chess", "dog1", 20, true);
        collector.observe("chess", "dog1", 30, true);
        collector.observe("chess", "dog1", 100, true); // Should evict first sample (10)

        if let Ok(agg) = collector.aggregators.read() {
            if let Some(dogs) = agg.get("chess") {
                if let Some(dog1) = dogs.get("dog1") {
                    // Now has [20, 30, 100]. Avg should be (20+30+100)/3 ≈ 50
                    assert_eq!(dog1.avg_latency_ms(), 50);
                    assert_eq!(dog1.samples.len(), 3);
                }
            }
        }
    }
}

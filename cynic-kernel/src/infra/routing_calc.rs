//! Dynamic routing calculator — computes Dog-to-domain routing based on observed latencies.
//!
//! Consumes dog_performance observations (dog_id, domain, latency_ms, success).
//! Produces live routing table: domain → Dogs suitable for latency SLA.
//!
//! Data-centric: routing adapts as Dogs degrade, new Dogs arrive, or patterns shift.
//! No static config → always reflects current reality.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::domain::storage::{Event, StoragePort};

/// Live routing calculator — selects suitable Dogs based on recent performance.
#[derive(Clone)]
pub struct RoutingCalculator {
    /// Domain -> Vec<DogPerformance> (Dogs sorted by latency)
    /// Refreshed every N observations or on-demand
    cache: Arc<RwLock<HashMap<String, Vec<DogPerformance>>>>,
    /// Persistent storage for observations
    storage: Option<Arc<dyn StoragePort>>,
}

impl std::fmt::Debug for RoutingCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingCalculator")
            .field("cache", &self.cache)
            .field(
                "storage",
                &self.storage.as_ref().map(|_| "Arc<dyn StoragePort>"),
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
            storage: None,
        }
    }

    pub fn with_storage(storage: Arc<dyn StoragePort>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            storage: Some(storage),
        }
    }

    /// Record a dog performance observation.
    /// Called after each dog evaluation in the judge.
    pub fn record_observation(&self, dog_id: &str, domain: &str, latency_ms: u32, success: bool) {
        // Persist to storage as an infrastructure event.
        // Consumed by fleet_stats query to compute live routing.
        if let Some(storage) = &self.storage {
            let event = Event {
                tool: "dog_evaluation".to_string(),
                node: crate::api::rest::inference_router::dog_to_node(dog_id),
                elapsed_ms: latency_ms as u64,
                output_bytes: 0,
                success,
                failure_reason: if success { "none" } else { "unknown" }.to_string(),
                agent_id: "kernel-routing".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                metadata: format!("dog={dog_id} domain={domain}"),
            };

            let storage = Arc::clone(storage);
            tokio::spawn(async move {
                let _ = storage.store_event(&event).await;
            });
        }
    }

    /// Get suitable Dogs for a domain, filtered by latency SLA.
    /// Returns Dogs meeting the SLA, sorted by latency (fastest first).
    /// Returns empty vec if no suitable Dogs found.
    pub fn dogs_for_domain(&self, domain: &str, latency_sla_ms: u32) -> Vec<String> {
        self.cache
            .read()
            .ok() // Lock poisoning → empty Vec (cache miss, will recompute on next refresh)
            .and_then(|cache| {
                cache.get(domain).map(|dogs| {
                    let mut suitable: Vec<_> = dogs
                        .iter()
                        .filter(|perf| {
                            perf.avg_latency_ms <= latency_sla_ms && perf.success_rate >= 0.95
                        })
                        .collect();
                    // Sort by latency, fastest first
                    suitable.sort_by_key(|perf| perf.avg_latency_ms);
                    suitable
                        .into_iter()
                        .map(|perf| perf.dog_id.clone())
                        .collect()
                })
            })
            .unwrap_or_default()
    }

    /// Dogs suitable for a domain: proven reliable (>=10 samples, >=0.95 success)
    /// OR too new to judge (<10 samples — benefit of doubt, K22 principle).
    /// Returns None if no performance data exists (domain never seen).
    pub fn reliable_dogs(&self, domain: &str) -> Option<Vec<String>> {
        self.cache.read().ok().and_then(|cache| {
            cache.get(domain).map(|dogs| {
                dogs.iter()
                    .filter(|p| {
                        // K22: unknown ≠ unreliable. Dogs with < 10 samples get benefit
                        // of doubt — exclude only Dogs with enough data to prove unreliable.
                        p.sample_count < 10 || p.success_rate >= 0.95
                    })
                    .map(|p| p.dog_id.clone())
                    .collect()
            })
        })
    }

    /// Update routing table for a domain (called by observation consumer).
    pub fn update_domain_routing(&self, domain: &str, dogs: Vec<DogPerformance>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(domain.to_string(), dogs);
        }
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
    fn test_dogs_for_domain_filters_by_sla() {
        let calc = RoutingCalculator::new();
        let dogs = vec![
            DogPerformance {
                dog_id: "fast-dog".to_string(),
                avg_latency_ms: 1000,
                success_rate: 0.98,
                sample_count: 50,
            },
            DogPerformance {
                dog_id: "slow-dog".to_string(),
                avg_latency_ms: 6000,
                success_rate: 0.99,
                sample_count: 50,
            },
        ];
        calc.update_domain_routing("token", dogs);

        // SLA 5000ms: only fast-dog qualifies
        let suitable = calc.dogs_for_domain("token", 5000);
        assert_eq!(suitable, vec!["fast-dog".to_string()]);

        // SLA 10000ms: both qualify (sorted by latency)
        let suitable = calc.dogs_for_domain("token", 10000);
        assert_eq!(
            suitable,
            vec!["fast-dog".to_string(), "slow-dog".to_string()]
        );
    }

    #[test]
    fn test_filters_by_success_rate() {
        let calc = RoutingCalculator::new();
        let dogs = vec![
            DogPerformance {
                dog_id: "reliable".to_string(),
                avg_latency_ms: 2000,
                success_rate: 0.98,
                sample_count: 50,
            },
            DogPerformance {
                dog_id: "flaky".to_string(),
                avg_latency_ms: 2000,
                success_rate: 0.80, // Below 95% threshold
                sample_count: 50,
            },
        ];
        calc.update_domain_routing("chess", dogs);

        let suitable = calc.dogs_for_domain("chess", 5000);
        assert_eq!(suitable, vec!["reliable".to_string()]);
    }

    #[test]
    fn test_no_dogs_for_unconfigured_domain() {
        let calc = RoutingCalculator::new();
        let suitable = calc.dogs_for_domain("unknown", 5000);
        assert!(suitable.is_empty());
    }

    #[test]
    fn test_reliable_dogs_includes_new_dogs() {
        // K22: Dogs with < 10 samples get benefit of doubt (not excluded)
        let calc = RoutingCalculator::new();
        let dogs = vec![
            DogPerformance {
                dog_id: "proven".to_string(),
                avg_latency_ms: 2000,
                success_rate: 0.98,
                sample_count: 50,
            },
            DogPerformance {
                dog_id: "new-dog".to_string(),
                avg_latency_ms: 3000,
                success_rate: 0.0, // 0 successes out of 3 attempts — but < 10 samples
                sample_count: 3,
            },
            DogPerformance {
                dog_id: "proven-flaky".to_string(),
                avg_latency_ms: 2000,
                success_rate: 0.80, // Below 95% WITH enough data to prove it
                sample_count: 50,
            },
        ];
        calc.update_domain_routing("token", dogs);

        let reliable = calc.reliable_dogs("token").unwrap();
        assert!(
            reliable.contains(&"proven".to_string()),
            "proven Dog should be reliable"
        );
        assert!(
            reliable.contains(&"new-dog".to_string()),
            "new Dog should get benefit of doubt"
        );
        assert!(
            !reliable.contains(&"proven-flaky".to_string()),
            "proven-flaky should be excluded"
        );
    }

    #[test]
    fn test_reliable_dogs_none_for_unknown_domain() {
        let calc = RoutingCalculator::new();
        assert!(calc.reliable_dogs("never-seen").is_none());
    }

    #[test]
    fn test_sorts_by_latency() {
        let calc = RoutingCalculator::new();
        let dogs = vec![
            DogPerformance {
                dog_id: "slow".to_string(),
                avg_latency_ms: 4000,
                success_rate: 0.98,
                sample_count: 50,
            },
            DogPerformance {
                dog_id: "fast".to_string(),
                avg_latency_ms: 1000,
                success_rate: 0.98,
                sample_count: 50,
            },
        ];
        calc.update_domain_routing("token", dogs);

        let suitable = calc.dogs_for_domain("token", 5000);
        assert_eq!(
            suitable,
            vec!["fast".to_string(), "slow".to_string()],
            "Dogs should be sorted by latency, fastest first"
        );
    }
}

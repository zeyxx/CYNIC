//! Exchange rate calculator — pure computation from observed samples.
//!
//! Takes observed samples of resource consumption and token flows,
//! computes exchange rates with confidence scores.
//!
//! No I/O — samples are collected by adapters and passed in.

use crate::exchange::{ExchangeRate, ExchangeRateTable};
use crate::types::ResourceUnit;
use std::collections::HashMap;

/// A single observed sample: resource consumed and tokens charged.
#[derive(Debug, Clone, PartialEq)]
pub struct RateSample {
    /// The resource unit consumed.
    pub resource: ResourceUnit,
    /// The amount of resource consumed.
    pub resource_amount: f64,
    /// The tokens charged for this consumption.
    pub token_amount: f64,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
}

impl RateSample {
    pub fn new(resource: ResourceUnit, resource_amount: f64, token_amount: f64, timestamp: u64) -> Self {
        Self {
            resource,
            resource_amount,
            token_amount,
            timestamp,
        }
    }
}

/// Computes exchange rates from observed samples.
///
/// Groups samples by resource type, computes the median rate (tokens per resource unit),
/// and assigns a confidence score based on sample count and variance.
pub fn compute_exchange_rates(samples: &[RateSample]) -> ExchangeRateTable {
    let mut table = ExchangeRateTable::new(
        samples.last().map(|s| s.timestamp).unwrap_or(0),
    );

    // Group samples by resource type
    let mut groups: HashMap<crate::types::ResourceType, Vec<&RateSample>> = HashMap::new();
    for sample in samples {
        groups
            .entry(sample.resource.resource_type())
            .or_default()
            .push(sample);
    }

    for (_resource_type, group) in groups {
        if group.is_empty() {
            continue;
        }

        // Compute rates for each sample
        let mut rates: Vec<f64> = group
            .iter()
            .filter(|s| s.resource_amount > 0.0)
            .map(|s| s.token_amount / s.resource_amount)
            .collect();

        if rates.is_empty() {
            continue;
        }

        // Sort for median
        rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Median rate
        let median = if rates.len() % 2 == 0 {
            let mid = rates.len() / 2;
            (rates[mid - 1] + rates[mid]) / 2.0
        } else {
            rates[rates.len() / 2]
        };

        // Confidence based on sample count and variance
        let variance = if rates.len() > 1 {
            let mean = rates.iter().sum::<f64>() / rates.len() as f64;
            rates.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rates.len() as f64
        } else {
            0.0
        };

        let std_dev = variance.sqrt();
        let cv = if median > 0.0 { std_dev / median } else { 1.0 };

        // Confidence: more samples + lower variance = higher confidence
        let sample_confidence = (rates.len() as f64 / 100.0).min(1.0);
        let variance_confidence = (1.0 - cv.min(1.0));
        let confidence = (sample_confidence * 0.5 + variance_confidence * 0.5).clamp(0.0, 1.0);

        // Create a representative resource unit for this type
        let representative = group[0].resource.clone();

        let rate = ExchangeRate::new(
            representative,
            ResourceUnit::TokenCyd(1),
            median,
            group.last().unwrap().timestamp,
            rates.len() as u64,
            confidence,
        );

        table.add_rate(rate);
    }

    table
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_exchange_rates_empty_returns_empty_table() {
        let table = compute_exchange_rates(&[]);
        assert!(table.is_empty());
    }

    #[test]
    fn compute_exchange_rates_single_sample() {
        let samples = vec![RateSample::new(
            ResourceUnit::ComputeCpuMs(1000),
            1000.0,
            10.0,
            1000,
        )];
        let table = compute_exchange_rates(&samples);
        assert_eq!(table.len(), 1);
        let rate = &table.rates[0];
        assert!((rate.rate - 0.01).abs() < 0.0001);
        assert_eq!(rate.sample_count, 1);
    }

    #[test]
    fn compute_exchange_rates_multiple_samples_same_type() {
        let samples = vec![
            RateSample::new(ResourceUnit::ComputeCpuMs(1000), 1000.0, 10.0, 1000),
            RateSample::new(ResourceUnit::ComputeCpuMs(2000), 2000.0, 20.0, 2000),
            RateSample::new(ResourceUnit::ComputeCpuMs(3000), 3000.0, 30.0, 3000),
        ];
        let table = compute_exchange_rates(&samples);
        assert_eq!(table.len(), 1);
        let rate = &table.rates[0];
        assert!((rate.rate - 0.01).abs() < 0.0001);
        assert_eq!(rate.sample_count, 3);
        assert!(rate.confidence > 0.0);
    }

    #[test]
    fn compute_exchange_rates_different_resource_types() {
        let samples = vec![
            RateSample::new(ResourceUnit::ComputeCpuMs(1000), 1000.0, 10.0, 1000),
            RateSample::new(ResourceUnit::StorageBytes(1000), 1000.0, 5.0, 1000),
        ];
        let table = compute_exchange_rates(&samples);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn compute_exchange_rates_skips_zero_resource_amount() {
        let samples = vec![
            RateSample::new(ResourceUnit::ComputeCpuMs(0), 0.0, 10.0, 1000),
            RateSample::new(ResourceUnit::ComputeCpuMs(1000), 1000.0, 10.0, 2000),
        ];
        let table = compute_exchange_rates(&samples);
        assert_eq!(table.len(), 1);
        assert_eq!(table.rates[0].sample_count, 1);
    }

    #[test]
    fn compute_exchange_rates_confidence_increases_with_samples() {
        let mut samples = vec![];
        for i in 0..50 {
            samples.push(RateSample::new(
                ResourceUnit::ComputeCpuMs(1000),
                1000.0,
                10.0,
                i as u64,
            ));
        }
        let table = compute_exchange_rates(&samples);
        assert_eq!(table.len(), 1);
        assert!(table.rates[0].confidence > 0.4);
    }
}

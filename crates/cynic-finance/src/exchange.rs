//! Exchange rate table — measured rates, never hardcoded.
//!
//! Rates emerge from observed usage patterns. Each rate has a confidence
//! score based on sample count and variance.

pub use crate::types::EXCHANGE_RATE_TABLE_SCHEMA;
use crate::types::{EXCHANGE_RATE_SCHEMA, ResourceUnit};
use serde::{Deserialize, Serialize};

/// A measured exchange rate between two resource units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub schema: String,
    /// Source resource unit type.
    pub from: ResourceUnit,
    /// Target resource unit type.
    pub to: ResourceUnit,
    /// Rate: 1 from = rate * to.
    pub rate: f64,
    /// Unix timestamp in milliseconds when measured.
    pub measured_at: u64,
    /// Number of samples used to compute this rate.
    pub sample_count: u64,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

impl ExchangeRate {
    /// Creates a new exchange rate.
    pub fn new(
        from: ResourceUnit,
        to: ResourceUnit,
        rate: f64,
        measured_at: u64,
        sample_count: u64,
        confidence: f64,
    ) -> Self {
        Self {
            schema: EXCHANGE_RATE_SCHEMA.to_string(),
            from,
            to,
            rate,
            measured_at,
            sample_count,
            confidence,
        }
    }

    /// Returns true if this rate has sufficient confidence.
    pub fn is_reliable(&self) -> bool {
        self.confidence >= 0.8 && self.sample_count >= 10
    }

    /// Converts an amount from the source unit to the target unit.
    pub fn convert(&self, amount: f64) -> f64 {
        amount * self.rate
    }
}

/// A table of exchange rates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateTable {
    pub schema: String,
    pub rates: Vec<ExchangeRate>,
    /// Unix timestamp in milliseconds when the table was last updated.
    pub last_updated: u64,
}

impl ExchangeRateTable {
    /// Creates a new empty exchange rate table.
    pub fn new(last_updated: u64) -> Self {
        Self {
            schema: EXCHANGE_RATE_TABLE_SCHEMA.to_string(),
            rates: Vec::new(),
            last_updated,
        }
    }

    /// Adds a rate to the table.
    pub fn add_rate(&mut self, rate: ExchangeRate) {
        self.rates.push(rate);
    }

    /// Finds a rate from one resource type to another.
    pub fn find_rate(&self, from: &ResourceUnit, to: &ResourceUnit) -> Option<&ExchangeRate> {
        self.rates.iter().find(|r| {
            std::mem::discriminant(&r.from) == std::mem::discriminant(from)
                && std::mem::discriminant(&r.to) == std::mem::discriminant(to)
        })
    }

    /// Returns all rates with sufficient confidence.
    pub fn reliable_rates(&self) -> Vec<&ExchangeRate> {
        self.rates.iter().filter(|r| r.is_reliable()).collect()
    }

    /// Returns the number of rates in the table.
    pub fn len(&self) -> usize {
        self.rates.len()
    }

    /// Returns true if the table has no rates.
    pub fn is_empty(&self) -> bool {
        self.rates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ResourceUnit;

    #[test]
    fn exchange_rate_convert() {
        let rate = ExchangeRate::new(
            ResourceUnit::ComputeCpuMs(1000),
            ResourceUnit::TokenCyd(1),
            0.01,
            1000,
            100,
            0.95,
        );
        assert!(rate.is_reliable());
        assert_eq!(rate.convert(1000.0), 10.0);
    }

    #[test]
    fn exchange_rate_not_reliable_with_few_samples() {
        let rate = ExchangeRate::new(
            ResourceUnit::ComputeCpuMs(1000),
            ResourceUnit::TokenCyd(1),
            0.01,
            1000,
            5,
            0.5,
        );
        assert!(!rate.is_reliable());
    }

    #[test]
    fn exchange_rate_table_find_rate() {
        let mut table = ExchangeRateTable::new(1000);
        let rate = ExchangeRate::new(
            ResourceUnit::ComputeCpuMs(1000),
            ResourceUnit::TokenCyd(1),
            0.01,
            1000,
            100,
            0.95,
        );
        table.add_rate(rate);

        let found = table.find_rate(&ResourceUnit::ComputeCpuMs(0), &ResourceUnit::TokenCyd(0));
        assert!(found.is_some());
    }
}

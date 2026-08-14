//! SystemContract — the kernel's self-model.
//!
//! Declares what the system SHOULD be: expected Dogs, storage requirement,
//! embedding expectation. Loaded from backends.toml at boot, compared against
//! live state by health_gate. The contract is the bridge between "what I am"
//! and "what I should be" — proprioception for the organism.
//!
//! Pure domain: no infra types, no I/O. Config layer constructs it.

use serde::Serialize;

/// The kernel's self-model — what it expects to have at full capacity.
#[derive(Debug, Clone, Serialize)]
pub struct SystemContract {
    /// Dog IDs the kernel expects (from backends.toml + deterministic-dog).
    expected_dogs: Vec<String>,
    /// Whether storage is required (not optional/degradable).
    pub storage_required: bool,
}

impl SystemContract {
    /// Build a contract from expected Dog IDs.
    /// `deterministic-dog` is always included if not already present.
    pub fn new(mut expected_dogs: Vec<String>, storage_required: bool) -> Self {
        let det = "deterministic-dog".to_string();
        if !expected_dogs.contains(&det) {
            expected_dogs.push(det);
        }
        expected_dogs.sort();
        Self {
            expected_dogs,
            storage_required,
        }
    }

    /// All expected Dog IDs, sorted.
    pub fn expected_dogs(&self) -> &[String] {
        &self.expected_dogs
    }

    /// How many Dogs the contract expects.
    pub fn expected_count(&self) -> usize {
        self.expected_dogs.len()
    }

    /// Prune a Dog from the expected list.
    /// Used when a Dog is confirmed to be permanently or long-term offline.
    /// Returns true if the Dog was removed.
    pub fn prune(&mut self, dog_id: &str) -> bool {
        let before = self.expected_dogs.len();
        self.expected_dogs.retain(|id| id != dog_id);
        self.expected_dogs.len() < before
    }

    /// Compare contract against live roster. Returns the delta.
    pub fn assess(&self, live_dog_ids: &[String]) -> ContractDelta {
        let missing: Vec<String> = self
            .expected_dogs
            .iter()
            .filter(|id| !live_dog_ids.contains(id))
            .cloned()
            .collect();

        let unexpected: Vec<String> = live_dog_ids
            .iter()
            .filter(|id| !self.expected_dogs.contains(id))
            .cloned()
            .collect();

        let fulfilled = missing.is_empty();

        ContractDelta {
            expected: self.expected_dogs.len(),
            actual: live_dog_ids.len(),
            missing,
            unexpected,
            fulfilled,
        }
    }
}

/// Delta between expected and actual system state.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContractDelta {
    /// Number of expected Dogs (from contract).
    pub expected: usize,
    /// Number of actual Dogs in live roster.
    pub actual: usize,
    /// Dogs in contract but not in roster.
    pub missing: Vec<String>,
    /// Dogs in roster but not in contract (dynamically registered).
    pub unexpected: Vec<String>,
    /// True when all expected Dogs are present.
    pub fulfilled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_always_includes_deterministic_dog() {
        let contract = SystemContract::new(vec!["qwen".into()], true);
        assert!(
            contract
                .expected_dogs()
                .contains(&"deterministic-dog".to_string())
        );
        assert_eq!(contract.expected_count(), 2);
    }

    #[test]
    fn contract_no_duplicate_deterministic_dog() {
        let contract = SystemContract::new(vec!["deterministic-dog".into(), "qwen".into()], true);
        assert_eq!(contract.expected_count(), 2);
    }

    #[test]
    fn assess_fulfilled_when_all_present() {
        let contract = SystemContract::new(vec!["qwen".into(), "gemma".into()], true);
        let live = vec!["deterministic-dog".into(), "gemma".into(), "qwen".into()];
        let delta = contract.assess(&live);
        assert!(delta.fulfilled);
        assert!(delta.missing.is_empty());
        assert!(delta.unexpected.is_empty());
        assert_eq!(delta.expected, 3);
        assert_eq!(delta.actual, 3);
    }

    #[test]
    fn assess_missing_dogs() {
        let contract = SystemContract::new(vec!["qwen".into(), "gemma".into(), "hf".into()], true);
        let live = vec!["deterministic-dog".into(), "qwen".into(), "hf".into()];
        let delta = contract.assess(&live);
        assert!(!delta.fulfilled);
        assert_eq!(delta.missing, vec!["gemma".to_string()]);
        assert_eq!(delta.expected, 4);
        assert_eq!(delta.actual, 3);
    }

    #[test]
    fn assess_unexpected_dogs() {
        let contract = SystemContract::new(vec!["qwen".into()], true);
        let live = vec!["deterministic-dog".into(), "qwen".into(), "surprise".into()];
        let delta = contract.assess(&live);
        assert!(delta.fulfilled); // all expected are present
        assert_eq!(delta.unexpected, vec!["surprise".to_string()]);
    }

    #[test]
    fn assess_empty_roster() {
        let contract = SystemContract::new(vec!["qwen".into()], true);
        let live: Vec<String> = vec![];
        let delta = contract.assess(&live);
        assert!(!delta.fulfilled);
        assert_eq!(delta.missing.len(), 2); // deterministic-dog + qwen
    }
}

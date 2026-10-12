//! Token entropy adapter for inference backends.
//!
//! This adapter exposes the inference-domain token entropy calculation behind
//! the `TokenEntropyPort`. It does not talk to any backend; it consumes logits
//! that an inference adapter has already obtained.

use cynic_protocol::information::EntropyError;

use crate::domain::token_entropy::{
    TokenEntropySnapshot, TokenUncertaintyAssessment, TokenUncertaintyParams,
    assess_token_uncertainty, token_entropy_from_logits,
};
use crate::ports::token_entropy_port::TokenEntropyPort;

#[derive(Debug, Default, Clone, Copy)]
pub struct TokenEntropyAdapter;

impl TokenEntropyPort for TokenEntropyAdapter {
    fn token_entropy(
        &self,
        logits: &[f64],
        token_index: usize,
    ) -> Result<TokenEntropySnapshot, EntropyError> {
        token_entropy_from_logits(logits, token_index)
    }

    fn assess_sequence(
        &self,
        snapshots: &[TokenEntropySnapshot],
        params: TokenUncertaintyParams,
    ) -> TokenUncertaintyAssessment {
        assess_token_uncertainty(snapshots, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_delegates_to_domain_token_entropy() {
        let adapter = TokenEntropyAdapter;
        let snapshot = adapter.token_entropy(&[0.0, 0.0], 0).unwrap();
        assert_eq!(snapshot.entropy_bits, 1.0);
    }

    #[test]
    fn adapter_classifies_uniform_logits_as_exploration() {
        let adapter = TokenEntropyAdapter;
        let snapshot = adapter.token_entropy(&[0.0, 0.0], 0).unwrap();
        let assessment = adapter
            .assess_sequence(&[snapshot], TokenUncertaintyParams::default());
        assert_eq!(
            assessment.level,
            crate::domain::token_entropy::TokenUncertaintyLevel::Exploration
        );
    }
}

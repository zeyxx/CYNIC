//! Port for token-level inference uncertainty used by judge calibration.

use cynic_protocol::information::EntropyError;

use crate::domain::token_entropy::{
    TokenEntropySnapshot, TokenUncertaintyAssessment, TokenUncertaintyParams,
};

/// Produces token-level uncertainty measurements for inference backends.
///
/// The trait is intentionally small: adapters provide the raw logits or
/// probabilities they already expose, and the calculation stays in the
/// inference domain rather than leaking into a judge surface.
pub trait TokenEntropyPort: Send + Sync {
    fn token_entropy(
        &self,
        logits: &[f64],
        token_index: usize,
    ) -> Result<TokenEntropySnapshot, EntropyError>;

    fn assess_sequence(
        &self,
        snapshots: &[TokenEntropySnapshot],
        params: TokenUncertaintyParams,
    ) -> TokenUncertaintyAssessment;
}

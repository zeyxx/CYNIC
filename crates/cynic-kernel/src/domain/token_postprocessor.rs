//! Compatibility adapter for token-analysis post-processing.
//!
//! Token intelligence heuristics live in `cynic-intel`; the kernel adapts its
//! current dog scoring structs until scoring ownership is extracted.

use crate::domain::dog::{AxiomScores, QScore, Stimulus};
pub use cynic_intel::token_postprocessor::{TokenClass, TokenMetadata};

pub fn postprocess_token_verdict(
    stimulus: &Stimulus,
    scores: &AxiomScores,
) -> (AxiomScores, QScore) {
    let metadata = cynic_intel::token_postprocessor::infer_token_metadata_from_context(
        stimulus.context.as_deref(),
    );
    let reasoning = scores.reasoning.clone();
    let reasoning_trace = scores.reasoning_trace.clone();
    let prompt_tokens = scores.prompt_tokens;
    let completion_tokens = scores.completion_tokens;
    let thinking_tokens = scores.thinking_tokens;
    let abstentions = scores.abstentions.clone();

    let (processed, q_score) = cynic_intel::token_postprocessor::postprocess_token_scores(
        &metadata,
        cynic_intel::token_postprocessor::AxiomScoreVector {
            fidelity: scores.fidelity,
            phi: scores.phi,
            verify: scores.verify,
            culture: scores.culture,
            burn: scores.burn,
            sovereignty: scores.sovereignty,
        },
    );

    (
        AxiomScores {
            fidelity: processed.fidelity,
            phi: processed.phi,
            verify: processed.verify,
            culture: processed.culture,
            burn: processed.burn,
            sovereignty: processed.sovereignty,
            reasoning,
            reasoning_trace,
            prompt_tokens,
            completion_tokens,
            thinking_tokens,
            abstentions,
        },
        QScore {
            total: q_score.total,
            fidelity: q_score.fidelity,
            phi: q_score.phi,
            verify: q_score.verify,
            culture: q_score.culture,
            burn: q_score.burn,
            sovereignty: q_score.sovereignty,
        },
    )
}

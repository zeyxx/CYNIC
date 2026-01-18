// CYNIC Score Range Proof
//
// Proves that a Q-Score is within valid range [0, 100]
// without revealing the exact score.
//
// "φ distrusts φ" - κυνικός

/// Prove that score is within valid range [0, 100]
///
/// Private inputs:
/// - score: The actual Q-Score (0-100)
///
/// Public inputs:
/// - commitment: Hash commitment of the score
/// - min_score: Minimum threshold to prove (e.g., 61 for WAG)
///
/// Returns: true if score >= min_score AND score <= 100
fn main(
    score: Field,           // Private: actual score
    commitment: pub Field,  // Public: H(score || blinding)
    min_score: pub Field,   // Public: minimum threshold
    blinding: Field         // Private: blinding factor
) {
    // Verify commitment matches
    let computed_commitment = std::hash::pedersen_hash([score, blinding]);
    assert(commitment == computed_commitment, "Commitment mismatch");

    // Score must be >= 0 (Field is always >= 0 in Noir)
    // Score must be <= 100
    assert(score as u8 <= 100, "Score out of range");

    // Score must be >= min_score (the threshold being proven)
    let score_u8 = score as u8;
    let min_u8 = min_score as u8;
    assert(score_u8 >= min_u8, "Score below minimum");
}

/// Compute Pedersen commitment for a score
/// Used to generate commitments off-chain
#[export]
fn compute_commitment(score: Field, blinding: Field) -> Field {
    std::hash::pedersen_hash([score, blinding])
}

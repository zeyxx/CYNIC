// CYNIC Verdict Validity Proof
//
// Proves that a verdict matches the correct threshold for a given score
// without revealing the exact score.
//
// Verdicts (φ-aligned thresholds):
// - HOWL: score >= 76 (excellent)
// - WAG:  score >= 61 (φ⁻¹ * 100, good)
// - GROWL: score >= 38 (φ⁻² * 100, warning)
// - BARK: score < 38 (danger)
//
// "φ distrusts φ" - κυνικός

// Verdict constants (as Field values for circuit compatibility)
global VERDICT_HOWL: Field = 0;
global VERDICT_WAG: Field = 1;
global VERDICT_GROWL: Field = 2;
global VERDICT_BARK: Field = 3;

// Thresholds
global THRESHOLD_HOWL: u8 = 76;
global THRESHOLD_WAG: u8 = 61;   // φ⁻¹ * 100
global THRESHOLD_GROWL: u8 = 38; // φ⁻² * 100

/// Prove that verdict matches score correctly
///
/// Private inputs:
/// - score: The actual Q-Score (0-100)
/// - blinding: Blinding factor for commitment
///
/// Public inputs:
/// - commitment: Hash commitment of the score
/// - verdict: The claimed verdict (0=HOWL, 1=WAG, 2=GROWL, 3=BARK)
fn main(
    score: Field,           // Private: actual score
    blinding: Field,        // Private: blinding factor
    commitment: pub Field,  // Public: H(score || blinding)
    verdict: pub Field      // Public: claimed verdict
) {
    // Verify commitment matches
    let computed_commitment = std::hash::pedersen_hash([score, blinding]);
    assert(commitment == computed_commitment, "Commitment mismatch");

    // Convert to u8 for comparisons
    let score_u8 = score as u8;

    // Verify score is in valid range
    assert(score_u8 <= 100, "Score out of range");

    // Verify verdict matches score according to thresholds
    if verdict == VERDICT_HOWL {
        // HOWL: score >= 76
        assert(score_u8 >= THRESHOLD_HOWL, "Invalid HOWL: score too low");
    } else if verdict == VERDICT_WAG {
        // WAG: 61 <= score < 76
        assert(score_u8 >= THRESHOLD_WAG, "Invalid WAG: score too low");
        assert(score_u8 < THRESHOLD_HOWL, "Invalid WAG: score too high");
    } else if verdict == VERDICT_GROWL {
        // GROWL: 38 <= score < 61
        assert(score_u8 >= THRESHOLD_GROWL, "Invalid GROWL: score too low");
        assert(score_u8 < THRESHOLD_WAG, "Invalid GROWL: score too high");
    } else if verdict == VERDICT_BARK {
        // BARK: score < 38
        assert(score_u8 < THRESHOLD_GROWL, "Invalid BARK: score too high");
    } else {
        assert(false, "Invalid verdict value");
    }
}

/// Determine correct verdict for a score (helper for testing)
#[export]
fn get_verdict(score: u8) -> Field {
    if score >= THRESHOLD_HOWL {
        VERDICT_HOWL
    } else if score >= THRESHOLD_WAG {
        VERDICT_WAG
    } else if score >= THRESHOLD_GROWL {
        VERDICT_GROWL
    } else {
        VERDICT_BARK
    }
}

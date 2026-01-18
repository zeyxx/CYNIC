// CYNIC Burn Threshold Proof
//
// Proves that a burn amount is >= minimum threshold without
// revealing the exact amount burned.
//
// Default minimum: φ⁻¹ SOL = 0.618... SOL = 618,033,988 lamports
//
// "Don't extract, burn" - κυνικός

// φ⁻¹ in lamports (scaled by 1e9)
// 618033988 lamports ≈ 0.618 SOL
global PHI_INV_LAMPORTS: u64 = 618033988;

/// Prove that burn amount >= minimum threshold
///
/// Private inputs:
/// - amount: Actual burn amount in lamports
/// - blinding: Blinding factor for commitment
///
/// Public inputs:
/// - commitment: Hash commitment of the amount
/// - min_amount: Minimum amount to prove (default: PHI_INV_LAMPORTS)
/// - tx_hash: Transaction hash for binding to specific tx
fn main(
    amount: Field,          // Private: actual amount in lamports
    blinding: Field,        // Private: blinding factor
    commitment: pub Field,  // Public: H(amount || blinding)
    min_amount: pub Field,  // Public: minimum threshold
    tx_hash: pub Field      // Public: tx signature hash (for binding)
) {
    // Verify commitment matches amount
    let computed_commitment = std::hash::pedersen_hash([amount, blinding, tx_hash]);
    assert(commitment == computed_commitment, "Commitment mismatch");

    // Convert to u64 for comparison (lamports fit in u64)
    let amount_u64 = amount as u64;
    let min_u64 = min_amount as u64;

    // Verify amount >= minimum
    assert(amount_u64 >= min_u64, "Burn amount below minimum");
}

/// Compute commitment for burn amount
#[export]
fn compute_burn_commitment(amount: Field, blinding: Field, tx_hash: Field) -> Field {
    std::hash::pedersen_hash([amount, blinding, tx_hash])
}

/// Check if amount meets φ threshold
#[export]
fn meets_phi_threshold(amount: u64) -> bool {
    amount >= PHI_INV_LAMPORTS
}

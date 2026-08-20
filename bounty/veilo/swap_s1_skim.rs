//! Deterministic proof of Swap §1: `transact_swap` lets the relayer skim the ENTIRE
//! swap spread because `swap_data_hash` is NOT proof-bound (swap.rs L54-57), so the
//! relayer controls the DEX route and `relayer_fee` is only floored (L860), never capped.
//!
//! This mirrors the real program logic at swap.rs L851-868:
//!   relayer_fee  = ext_data.fee                 (relayer-chosen, >= min_required_fee)
//!   vault_amount = swapped_amount - relayer_fee (L862)
//!   require vault_amount >= dest_amount         (L868)
//! => relayer_fee <= swapped_amount - dest_amount, and the relayer CHOOSES it.
//!
//! Reproduce the accounting to show the relayer pockets (swapped_amount - dest_amount)
//! with a still-valid proof (the proof only binds min_amount_out, not the route).

/// Replicates the program's fee accounting in `transact_swap` (swap.rs:851-868).
/// `swapped_amount` = what the DEX actually returned (relayer-controlled via swap_data).
/// `dest_amount`    = what the user committed in their proof (what the note will hold).
/// `ext_data_fee`   = relayer-chosen fee.
fn transact_swap_fee_accounting(
    swapped_amount: u64,
    dest_amount: u64,
    ext_data_fee: u64,
    min_required_fee: u64,
) -> Result<(u64 /*relayer_take*/, u64 /*vault_credit*/), String> {
    // L860: relayer_fee is enforced as a MINIMUM, not a cap
    if ext_data_fee < min_required_fee {
        return Err("fee below minimum".into());
    }
    // L852: relayer_fee < swapped_amount
    if ext_data_fee >= swapped_amount {
        return Err("fee >= swapped".into());
    }
    // L862
    let vault_amount = swapped_amount.checked_sub(ext_data_fee).ok_or("underflow")?;
    // L868: vault_amount >= dest_amount
    if vault_amount < dest_amount {
        return Err("vault credit below dest_amount".into());
    }
    Ok((ext_data_fee, vault_amount))
}

#[test]
fn s1_relayer_skims_full_spread() {
    // User commits dest_amount = 1000 (what their private note will hold).
    // Relayer routes the swap to realize swapped_amount = 1500 (real DEX output).
    let dest_amount: u64 = 1_000;
    let swapped_amount: u64 = 1_500;
    let min_required_fee: u64 = 1; // protocol floor

    // Relayer sets fee = swapped - dest (the MAX allowed by L868), pocketing the spread.
    let ext_data_fee = swapped_amount - dest_amount; // = 500

    let (relayer_take, vault_credit) =
        transact_swap_fee_accounting(swapped_amount, dest_amount, ext_data_fee, min_required_fee)
            .expect("program accepts this — proof still valid, route unbound");

    assert_eq!(relayer_take, 500, "relayer extracts the ENTIRE 500 spread");
    assert_eq!(vault_credit, 1_000, "vault only credits dest_amount; user gets no price improvement");

    // The proof (Groth16) only binds min_amount_out <= swapped_amount (L849) and
    // dest_amount (via swap_params_hash). It does NOT bind swap_data / the route,
    // so this exact call verifies on-chain. The relayer's 500 is pure extraction.
    let spread = swapped_amount - dest_amount;
    assert_eq!(relayer_take, spread, "relayer takes 100% of the spread, repeatably");
}

#[test]
fn s1_no_upper_bound_on_fee() {
    // Show the protocol FLOORS the fee (L860) but never caps it against the spread.
    let dest_amount: u64 = 1_000;
    let swapped_amount: u64 = 10_000; // big price improvement captured by relayer
    let min_required_fee: u64 = 1;

    // Relayer can take up to swapped - dest = 9000, every time, proof-valid.
    for skim in [1_000u64, 5_000, 9_000] {
        let r = transact_swap_fee_accounting(swapped_amount, dest_amount, skim, min_required_fee);
        assert!(r.is_ok(), "any fee in [min, swapped-dest] is accepted — no cap");
    }
    // The MAX the relayer can take equals the whole spread:
    let max_take = swapped_amount - dest_amount;
    assert_eq!(max_take, 9_000);
}

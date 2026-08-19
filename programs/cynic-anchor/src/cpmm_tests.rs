//! Unit tests for CPMM (Constant Product Market Maker) futarchy invariant.
//!
//! From the AMM <-> CYNIC mapping:
//!   The pool follows x * y = k where:
//!     x = no_pool_units (liquidity backing "no" outcome)
//!     y = yes_pool_units (liquidity backing "yes" outcome)
//!
//!   A bet transforms the reserves along the curve:
//!     dy = (y * dx) / (x + dx)
//!
//!   The invariant k = x * y is preserved (minus rounding).

/// Compute the output amount for a given input (CPMM formula).
/// dy = (y * dx) / (x + dx)
pub fn cpmm_swap(x: u128, y: u128, dx: u128) -> u128 {
    if dx == 0 || x == 0 || y == 0 {
        return 0;
    }
    (y * dx) / (x + dx)
}

/// Compute the invariant k = x * y.
pub fn cpmm_invariant(x: u128, y: u128) -> u128 {
    x * y
}

/// Price of "yes" in terms of "no" = no_pool / total_pool.
/// Expressed in basis points (0-10000).
pub fn cpmm_price_bps(yes_pool: u64, no_pool: u64) -> u32 {
    let total = yes_pool as u128 + no_pool as u128;
    if total == 0 {
        return 5000; // 50/50
    }
    ((no_pool as u128 * 10_000) / total as u128) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpmm_swap_basic() {
        // Pool: 1000 no, 1000 yes (k = 1,000,000)
        // Bet 100 on yes → dx=100 goes to no_pool, dy comes from yes_pool
        let x = 1000u128; // no_pool
        let y = 1000u128; // yes_pool
        let dx = 100u128;

        let dy = cpmm_swap(x, y, dx);
        // dy = (1000 * 100) / (1000 + 100) = 100000 / 1100 ≈ 90
        assert_eq!(dy, 90);

        // After swap: no_pool += dx, yes_pool -= dy
        let new_x = x + dx; // 1100
        let new_y = y - dy; // 910
        let new_k = cpmm_invariant(new_x, new_y);
        // k should be preserved (within rounding)
        let original_k = cpmm_invariant(x, y);
        assert!(new_k <= original_k, "invariant must not increase");
        assert!(new_k >= original_k - 1000, "invariant preserved within tolerance");
    }

    #[test]
    fn cpmm_invariant_preserved() {
        // Initial pool: 500 yes, 2000 no
        let mut yes_pool = 500u128;
        let mut no_pool = 2000u128;
        let original_k = cpmm_invariant(yes_pool, no_pool);

        // Series of bets
        for dx in [10, 20, 50, 100, 200] {
            let dy = cpmm_swap(no_pool, yes_pool, dx);
            no_pool += dx;
            yes_pool -= dy;
        }

        let new_k = cpmm_invariant(yes_pool, no_pool);
        // k should be monotonically decreasing (due to rounding) but preserved
        assert!(new_k <= original_k);
        let ratio = new_k as f64 / original_k as f64;
        assert!(ratio > 0.99, "invariant preserved >99%");
    }

    #[test]
    fn cpmm_price_converges_to_truth() {
        // As betting pushes toward one outcome, price should reflect conviction
        let mut yes_pool = 1000u64;
        let mut no_pool = 1000u64;

        // Heavy yes betting
        for _ in 0..10 {
            let dx = 100u128;
            let dy = cpmm_swap(no_pool as u128, yes_pool as u128, dx);
            no_pool += dx as u64;
            yes_pool -= dy as u64;
        }

        let price = cpmm_price_bps(yes_pool, no_pool);
        // After heavy yes betting, no_pool grows → yes price should rise
        assert!(price > 5000, "yes price should exceed 50% after heavy yes betting");
    }

    #[test]
    fn cpmm_price_symmetry() {
        // Equal pools = 50/50
        assert_eq!(cpmm_price_bps(1000, 1000), 5000);
        // Double yes = 66.67% yes price
        assert_eq!(cpmm_price_bps(2000, 1000), 3333);
        // Double no = 33.33% yes price
        assert_eq!(cpmm_price_bps(1000, 2000), 6666);
    }

    #[test]
    fn cpmm_empty_pool_returns_zero() {
        assert_eq!(cpmm_swap(0, 100, 10), 0);
        assert_eq!(cpmm_swap(100, 0, 10), 0);
        assert_eq!(cpmm_swap(100, 100, 0), 0);
    }

    #[test]
    fn cpmm_swap_preserves_direction() {
        // Betting on yes increases no_pool, decreases yes_pool
        let yes = 1000u128;
        let no = 1000u128;
        let dx = 500u128;

        let dy = cpmm_swap(no, yes, dx);
        assert!(dy < dx, "you always get less than you put in (slippage)");
        assert!(dy > 0, "you get something");
    }

    #[test]
    fn cpmm_conviction_threshold() {
        // Market with 6000 bps (60%) threshold
        let threshold_bps = 6000;

        // Scenario 1: Strong yes (no_pool >> yes_pool means high conviction in yes)
        let yes_pool = 100u64;
        let no_pool = 900u64;
        let yes_price = cpmm_price_bps(yes_pool, no_pool);
        // no_pool/total = 900/1000 = 90% → yes_price = 9000 bps
        assert_eq!(yes_price, 9000);
        assert!(yes_price >= threshold_bps, "conviction should pass threshold");

        // Scenario 2: Weak conviction
        let yes_pool = 400u64;
        let no_pool = 600u64;
        let yes_price = cpmm_price_bps(yes_pool, no_pool);
        assert_eq!(yes_price, 6000);
        assert!(yes_price >= threshold_bps, "conviction at threshold");
    }

    #[test]
    fn cpmm_liquidity_sensitivity() {
        // Large pools have less slippage
        let small_yes = 100u128;
        let small_no = 100u128;
        let large_yes = 100_000u128;
        let large_no = 100_000u128;
        let dx = 10u128;

        let small_dy = cpmm_swap(small_no, small_yes, dx);
        let large_dy = cpmm_swap(large_no, large_yes, dx);

        // Large pool gives proportionally more output (less slippage)
        let small_ratio = small_dy as f64 / dx as f64;
        let large_ratio = large_dy as f64 / dx as f64;
        assert!(large_ratio > small_ratio, "larger pools have less slippage");
    }

/// Validate that the Anchor program's CPMM formula matches the pure spec.
#[test]
fn anchor_cpmm_matches_spec() {
    // Simulate the Anchor program's place_futarchy_bet logic
    let mut yes_pool = 5000u64;
    let mut no_pool = 5000u64;

    // Bet 100 on yes (is_yes = true)
    let is_yes = true;
    let amount_units = 100u64;

    let (in_pool, out_pool) = if is_yes {
        (&mut no_pool, &mut yes_pool)
    } else {
        (&mut yes_pool, &mut no_pool)
    };

    let dx = amount_units as u128;
    let x = *in_pool as u128;
    let y = *out_pool as u128;

    let dy = (y * dx) / (x + dx);
    let shares_bought = dy as u64;

    *in_pool += amount_units;
    *out_pool = out_pool.saturating_sub(shares_bought);

    // Compare with pure spec
    let pure_dy = cpmm_swap(5000, 5000, 100);
    assert_eq!(dy, pure_dy, "Anchor formula must match pure spec");
    assert_eq!(shares_bought, pure_dy as u64);

    // Verify resulting state
    assert_eq!(yes_pool, 5000 - pure_dy as u64);
    assert_eq!(no_pool, 5000 + 100);
}

/// Test resolve_futarchy_market yes_price_bps calculation.
#[test]
fn resolve_futarchy_price_bps() {
    let yes_pool = 3000u64;
    let no_pool = 7000u64;
    let total = yes_pool + no_pool;

    // From Anchor: yes_price_bps = ((no_pool * 10000) / total) as u32
    let yes_price_bps = ((no_pool as u128 * 10_000) / total as u128) as u32;
    assert_eq!(yes_price_bps, 7000);

    // Compare with pure spec
    assert_eq!(cpmm_price_bps(yes_pool, no_pool), yes_price_bps);
}
}

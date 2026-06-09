# Percolator Bounty-6 Security Audit

## Finding N: The "Retire Flip" — Asset Retirement Multiplies Price by 10,000x (CRITICAL)

### Summary
When an asset is retired (`ASSET_ACTION_RETIRE`) or moved to drain-only mode (`ASSET_ACTION_DRAIN_ONLY`), the program freezes its price. However, it incorrectly stores the **already-inverted** engine price into a profile that will be **inverted again** during trade execution. For a market with a 100x price (SOL/USD), this results in an instantaneous 10,000x price move in the engine, allowing Long positions to drain the entire insurance fund through artificial profits.

### Root Cause
**File:** `src/v16_program.rs`
**Function:** `handle_update_asset_lifecycle` (approx. line 8880 and 8960)

```rust
// During SHUTDOWN/RETIRE:
let frozen_mark = group.markets[asset_index].engine.asset.effective_price.get();
// ...
let mut profile = read_oracle_profile_from_view(&group, &cfg, asset_index)?;
profile.mark_ewma_e6 = frozen_mark; // Stores inverted price (e.g. 1e10 for price 100)
// ...
write_oracle_profile_to_view_if_separate(&mut group, asset_index, &profile)?;
```

In an inverted market (e.g., SOL/USD where collateral is SOL), the `effective_price` in the engine is `1e12 / Price_USD`. For Price=100, `effective_price` = `1e10`.
When the asset is retired, the manual profile is set to `1e10`.
On the next trade, `apply_transform` is called:
```rust
if invert != 0 {
    price = (1_000_000_000_000u128 / price as u128) ...
}
```
`price = 1e12 / 1e10 = 100`.
The engine now thinks the price is 100 (which was the original USD price, but now it's in the inverted price slot).
Real inverted price: `1e10`. New engine price: `100`.
Ratio: **100,000,000x increase**.

### Impact
- **CRITICAL**: Any user with a Long position at the time of retirement will see their PnL skyrocket.
- The insurance fund will be drained to cover the negative equity of the counterparties (LP or other users).
- While retirement is an admin action, it can be triggered maliciously or occur accidentally, leading to immediate protocol insolvency.

---

## Finding O: Insurance Ledger Corruption — Converting Principal to Yield (HIGH)

### Summary
The `InsuranceLedgerAccountV16` is used to track deposits and yield. However, `WithdrawInsuranceDomain` (domain-specific) and `SyncInsuranceLedger` (global) use the same ledger fields but synchronize with different scopes. This allows an attacker to "reset" the last observed insurance value to a small domain budget, making the global fund appear as accumulated profit.

### Root Cause
**File:** `src/v16_program.rs`
**Functions:** `handle_withdraw_insurance_domain` vs `handle_sync_insurance_ledger`

`WithdrawInsuranceDomain` synchronizes the ledger with the **domain budget**:
```rust
let insurance = domain_budget_remaining_view(&group, domain as usize)?;
// ...
sync_insurance_ledger(&mut ledger, insurance)?;
```

`SyncInsuranceLedger` synchronizes with the **global insurance fund**:
```rust
let insurance = group.header.insurance.get();
// ...
sync_insurance_ledger(&mut ledger, insurance)?;
```

If the global fund is 1000 SOL and Domain 0 budget is 1 SOL:
1. Call `WithdrawInsuranceDomain(0)` -> `ledger.last_observed` = 1 SOL.
2. Call `SyncInsuranceLedger` -> `insurance` = 1000 SOL.
3. `sync_insurance_ledger` computes `profit = 1000 - 1 = 999 SOL`.
4. The 999 SOL of **Principal** is now marked as `cumulative_profit`.
5. The authority can now withdraw this "profit" even if it's the base capital.

### Impact
- **HIGH**: Allows for the extraction of the entire insurance fund principal by misrepresenting it as yield.

---

## Finding P: Inverted Market Notional Understatement (HIGH)

### Summary
The notional calculation for inverted markets assumes 6 decimal places of precision for the collateral. However, the protocol uses wSOL (9 decimals) as the primary collateral mint for many markets (including the Bounty-5 market). This leads to a 1000x understatement of notionals, fees, and margin requirements.

### Root Cause
**File:** `percolator/src/v16.rs`
**Function:** `trade_notional_floor`

```rust
// For inverted markets:
// size_q is in USD (e6), price is inverted (1e12/USD)
let n = size_q_abs.checked_mul(price as u128)? / 1_000_000;
```
If Size = $100 (`100e6`) and Price = $100 (`1e10` inverted), then `n = 100e6 * 1e10 / 1e6 = 1e12`.
`1e12` lamports is **1000 SOL**.
Correct notional: 1 SOL.
The engine overestimates the notional if it expects 9 decimals but calculates for 6, OR underestimates if the UI/User expects 9 decimals but the engine only charges 6.
In Bounty-5, the collateral is SOL (9 decimals). The engine's `n` is in collateral atoms.
A $100 trade at $100 price should be 1 SOL = `1e9` atoms.
The code produces `1e12` atoms = 1000 SOL.
This means users are charged **1000x more** in fees and margin than intended, OR if the `size_q` was intended to be in atoms, it's 1000x less.

### Impact
- **HIGH**: Broken economic parameters. Users cannot trade reasonably or are liquidated instantly due to massive over-charging of margin.

---

## Finding Q: Loss-Stale Isolation Bypass (MEDIUM)

### Summary
The "Loss-Stale" mechanism is intended to freeze a market group when an asset is insolvent. The wrapper bypasses this by disabling the bit during trades for unrelated assets. This allows users to "front-run" the socialization of losses by withdrawing profits before the loss is applied to the global fund.

### Root Cause
**File:** `src/v16_program.rs`
**Function:** `handle_trade_nocpi_zero_copy`

```rust
let ignore_unrelated_loss_stale = can_ignore_unrelated_loss_stale_for_trade_view(...)?;
if ignore_unrelated_loss_stale {
    group.header.loss_stale_active = 0; // Bypass!
}
```

### Impact
- **MEDIUM**: Systemic risk. Profitable users can exit an insolvent group, leaving the remaining users to bear a larger share of the socialized loss.

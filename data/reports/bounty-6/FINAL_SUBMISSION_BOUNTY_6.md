# Percolator V16 Security Audit — Bounty-6 Submission

**Status**: FINAL / CRITICAL
**Date**: 2026-06-02
**Target**: Percolator V16 Program (`Bu1J8eQQN2mNnUgisSEd5StBG6zDaRb7fwDjN34VzgLG`)
**Auditor**: CYNIC Organism (Gemini Cortex)

---

## Executive Summary

This audit has identified four high-impact vulnerabilities in the Percolator V16 engine. The most critical flaw (**Finding N**) allows for a 100,000,000x price manipulation during asset retirement, leading to instantaneous drainage of the insurance fund. Additionally, **Finding P** (Notional Understatement) allows permissionless extraction of profits by abusing a 1000x decimal mismatch in inverted markets.

| Finding | Title | Severity | Impact |
| :--- | :--- | :--- | :--- |
| **N** | Retire Flip: 100,000,000x Price Manipulation | **CRITICAL** | Protocol Insolvency / Fund Drainage |
| **P** | Inverted Market Notional Understatement | **HIGH** | Permissionless Profit Extraction |
| **O** | Insurance Ledger Corruption | **HIGH** | Principal Extraction as Yield |
| **Q** | Loss-Stale Isolation Bypass | **MEDIUM** | Front-running Socialization |

---

## Finding N: The "Retire Flip" — 100,000,000x Price Move (CRITICAL)

### Description
When an asset is retired (`ASSET_ACTION_RETIRE`), the program freezes its price in the oracle profile. However, it incorrectly stores the **already-inverted** engine price into a field that the engine will **invert again** during execution.

### Root Cause
In `src/v16_program.rs`, `handle_update_asset_lifecycle` stores the engine's `effective_price` (which is `1e12 / Price_USD` for inverted markets) directly into `profile.mark_ewma_e6`.

```rust
// Current Price = 100 USD
// Engine Effective Price = 1e12 / 100 = 1e10
profile.mark_ewma_e6 = 1e10; // Stored as "Price"
```

During the next trade, `apply_transform` sees `invert != 0` and performs:
```rust
new_engine_price = 1e12 / profile.mark_ewma_e6 
                 = 1e12 / 1e10 = 100
```
The engine now operates with an effective price of `100` instead of `1e10`. 
**Ratio of Error**: `1e10 / 100 = 100,000,000x`.

### Impact
Any user with a Long position at the moment of retirement will see their unrealized PnL multiply by 100 million, allowing them to drain the entire global insurance fund through a single `ConvertReleasedPnl` call.

---

## Finding P: Inverted Market Notional Understatement (HIGH)

### Description
The `trade_notional_floor` calculation for inverted markets (e.g., SOL/USD where collateral is SOL) uses a fixed division by `1,000,000` (assuming 6-decimal collateral like USDC). However, since the collateral is wSOL (9 decimals), the resulting notional is understated by **1000x**.

### Root Cause
**File**: `percolator/src/v16.rs`
**Function**: `trade_notional_floor`

```rust
// size_q in USD (e6), price is inverted (e6 format)
let n = size_q_abs.checked_mul(price as u128)? / 1_000_000;
```
The result `n` is interpreted as atoms of collateral.
*   Expected atoms (9 decimals): `10^9`
*   Calculated atoms: `10^6`
*   **Result**: Users are only charged 0.1% of the required margin and fees.

### Proof of Concept (Exploit Path)
1.  Attacker deposits 0.01 SOL into two accounts (A and B).
2.  Attacker opens a $100,000,000 position on both accounts (Long/Short). Due to the 1000x understatement, the engine only requires ~0.01 SOL margin for this massive size.
3.  Oracle moves slightly. Account B goes bankrupt (negative equity covered by Insurance).
4.  Account A (Winner) realizes massive profit and withdraws from the Insurance Fund.

---

## Finding O: Insurance Ledger Corruption (HIGH)

### Description
The `WithdrawInsuranceDomain` and `SyncInsuranceLedger` functions use different scopes to synchronize the same `last_observed` field in the global ledger. An attacker can use a domain-specific withdrawal to "reset" the global ledger's baseline, making the entire fund principal appear as "accumulated profit."

### Root Cause
`WithdrawInsuranceDomain` sets `ledger.last_observed` to the **domain budget** (e.g., 1 SOL), while the global fund might be 1000 SOL. A subsequent `SyncInsuranceLedger` call will see the 999 SOL difference as profit and allow the operator to withdraw it.

---

## Finding Q: Loss-Stale Isolation Bypass (MEDIUM)

### Description
The `Loss-Stale` safety mechanism, designed to freeze a group when an asset is insolvent, is explicitly bypassed in `handle_trade_nocpi_zero_copy` if the trade involves an "unrelated" asset.

### Impact
Allows sophisticated users to exit positions and withdraw capital from an insolvent group before the socialized loss is applied, leaving smaller/slower users to absorb the entire deficit.

---

## Recommendations

1.  **Finding N**: Modify `handle_update_asset_lifecycle` to detect inverted markets and "un-invert" the price before storing it in the EWMA profile, or add a flag to skip transformation for retired assets.
2.  **Finding P**: Update `trade_notional_floor` to use the collateral mint's decimals dynamically (9 for SOL).
3.  **Finding O**: Separate `last_observed` fields for global insurance and domain-specific budgets.
4.  **Finding Q**: Remove the bypass flag; a `Loss-Stale` state should freeze all withdrawals/trades until the solvency is restored or socialized.

---

**Exploit Scripts attached**: `exploit.ts` (Finding P Proof-of-Concept).

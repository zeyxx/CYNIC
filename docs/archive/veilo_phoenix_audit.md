# Veilo privacy-pool — `phoenix.rs` Security Audit (fund-loss focus)

**Scope:** `/tmp/veilo-official/programs/privacy-pool/src/phoenix.rs` (handlers `phoenix_*`).
**Excluded (already known):** V-01 — overcredit of `pending.amount` in `phoenix_ember_unwrap` (lines ~1662-1670). Not reported here.
**Method:** manual review of every Phoenix handler + their `Accounts` structs in `lib.rs`, tracing fund flows, access control, arithmetic, and CPI account validation.

---

## TL;DR

Beyond V-01, **no clean cross-user fund-loss (theft) bug was found in this file.** The anti-theft primitives are actually enforced:

- `phoenix_queue_withdraw` *does* enforce `withdrawn <= deposited` (cap is on a validated PDA; bounded by the live Phoenix balance).
- `phoenix_reissue_notes` *does* strictly require the `claimant` to be a **Signer**, seeds `pending_reissue` / `phoenix_slot` by `claimant.key()`, and bounds `pending.amount`. **No bug** in priority #2.

Two **defense-in-depth / access-control gaps** were identified that *could* become real fund loss depending on the (unverified) behavior of the external Phoenix program and the relayer threat model:

1. `phoenix_transfer_collateral` (+ `phoenix_place_order` inline transfer) — `dstTraderAccount` is never validated to belong to the executor.
2. `phoenix_queue_withdraw` — `max_slot_amount` lets the caller inflate the per-slot cap (`slot.amount`) to any value, defeating the documented "cannot withdraw more than deposited" guarantee (still bounded by Phoenix balance).

---

## Priority 1 — `phoenix_queue_withdraw` (line 1098)

### Invariant `withdrawn <= amount` — actually enforced
The cap is checked against a **deserialized, validated PDA** (`phoenix_slot`, seeds `b"phoenix_slot_v1", mint, claimant, withdrawal_id`), not a caller-supplied plain value:

```rust
let mut slot = PhoenixSlot::try_deserialize(&mut &slot_data[..])?;
require_keys_eq!(slot.claimant_pubkey, claimant, InvalidClaimant);   // 1137
if let Some(max) = max_slot_amount { if max > slot.amount { slot.amount = max; } } // 1140-1144
let new_withdrawn = slot.withdrawn.checked_add(amount)...;            // 1146
require!(new_withdrawn <= slot.amount, SlotOverdraft);                // 1149
// writes back withdrawn = new_withdrawn                                // 1151
```

- `phoenix_slot` is a program-owned PDA (lib.rs:2128-2139) → cannot be forged by the caller.
- `slot.claimant_pubkey == claimant` is enforced, and `claimant_signer` must equal `claimant` (lib.rs:2142-2143) → a victim's slot cannot be touched without the victim's signature.
- The destination PhUSD ATA (`remaining[9]`) is pinned to `ATA(executor, PHUSD)` (1126-1127).

**Result:** within a single `withdrawal_id`, `slot.withdrawn` monotonically accumulates and `pending.amount` in `phoenix_ember_unwrap` is bounded by `slot.withdrawn` (line 1532). The Phoenix `withdraw_funds` balance check is the final hard ceiling. **You cannot withdraw more than the live Phoenix trader balance**, and in a non-profitable position that balance equals the deposited amount. So **over-withdrawal beyond deposited+PnL is not possible.**

### Weakening: `max_slot_amount` defeats the per-slot cap
`slot.amount` — the very value the cap compares against — is **overwritable upward by the transaction caller**:

```rust
if let Some(max) = max_slot_amount { if max > slot.amount { slot.amount = max; } }
```

The comment claims this is "for trading PnL," but it makes the deposit-amount cap **caller-controllable**. Practical impact:
- In a non-profitable position the Phoenix balance still caps the actual outflow, so no theft.
- It removes the documented guarantee "prevents any single deposit slot from withdrawing more than it deposited" and lets a relayer+claimant launder a larger withdrawal through one slot's bookkeeping than the slot was funded for.

**Severity:** Medium (defense-in-depth bypass; not directly extractible). Recommend: drop `max_slot_amount`, or clamp it so `slot.amount` can only be *lowered*, or bind it to a verified realized-PnL oracle value.

---

## Priority 2 — `phoenix_reissue_notes` (line 1690)

### Claimant is STRICTLY verified — NO bug
Contrary to a naive reading of the `require_keys_eq!` checks (which only compare pubkey *values*), the `claimant` account **is a mandatory Signer**:

```rust
// lib.rs:2445
pub claimant: Signer<'info>,
```

And the escrow/slot accounts are **seeded by `claimant.key()`**, so they can only be the caller's own:

```rust
// lib.rs:2417-2428  pending_reissue:  seeds = [b"phoenix_pending_v1", mint, claimant.key(), withdrawal_id]
// lib.rs:2432-2441  phoenix_slot:     seeds = [b"phoenix_slot_v1",    mint, claimant.key(), withdrawal_id]
```

Handler checks (1762-1781) therefore bind the caller to both `pending.claimant_pubkey` and `phoenix_slot.claimant_pubkey`, and because `claimant` is a Signer, **only the holder of the claim key can reissue.** A relayer cannot substitute a victim's `claimant_pubkey` and mint their exit notes — doing so would require the victim's signature on a `claimant` key they don't control.

### `pending.amount` has a correct bound — NO bug
```rust
require!(pending.amount >= amount, InsufficientFundsForWithdrawal);   // 1767
pending.amount = pending.amount.checked_sub(amount)...;               // 1768-1770
```
`pending.amount` can only decrease here; it is credited only in `phoenix_ember_unwrap` (relayer-driven, but bounded by `slot.withdrawn`, line 1532) and consumed 1:1 in reissue. No over-mint path exists without V-01.

**Conclusion for P2:** the signer check and the `pending.amount` bound are both correct. The earlier "claimant not strictly verified" hypothesis does **not** hold.

---

## Other handlers — access-control / arithmetic / CPI

### Finding A (Medium, conditional) — `phoenix_transfer_collateral` & `phoenix_place_order` inline transfer path: `dstTraderAccount` unvalidated

`phoenix_transfer_collateral` (898) and the `transfer_amount.is_some()` branch of `phoenix_place_order` (614-652) pass `remaining[4]` straight into Phoenix's `transferCollateral` CPI as `dstTraderAccount`:

```rust
AccountMeta::new(remaining[4].key(), false), // dstTraderAccount  (938 / 645)
```

There is **no check** that `dstTraderAccount` is a sub-account of the executor PDA (`get_program_address(["trader", executor_key, ...], PHOENIX_PROGRAM_ID)`). The docstring (lib.rs:1996) asserts it moves "between two Phoenix trader sub-accounts owned by the same executor," but **the code does not enforce this** — it trusts an external assumption.

- If Phoenix Eternal's `transferCollateral` requires `src` and `dst` to share the same trader wallet (executor), the impact is nil.
- If Phoenix permits cross-wallet collateral transfer, a relayer + claimant-signed tx can sweep the pool's entire Phoenix margin from the executor's trader account into an **attacker-controlled trader account** (wallet = attacker), from which the attacker withdraws via Phoenix directly. This is a classic missing-input-validation → fund-loss.

The handler is gated only by `is_relayer` + `claimant_signer == claimant` (lib.rs:2006-2015). In a relayer-mediated UX where the relayer builds `remaining_accounts` and the user signs, the user cannot easily audit that `remaining[4]` is their own sub-account.

**Recommendation:** pin `remaining[4]` to `get_program_address(["trader", executor_key, sub_idx, ...], PHOENIX_PROGRAM_ID)` (or at minimum assert it is a program-owned PDA and reject arbitrary accounts), independent of Phoenix's behavior.

**PoC (conceptual, Rust pseudo):**
```rust
// Attacker pre-registers their own Phoenix trader account (wallet = attacker_pk)
let attacker_trader = register_trader(wallet = attacker_pk);
// Relayer submits transfer_collateral with remaining[4] = attacker_trader,
// claimant_signer = victim (social-engineered / malicious relayer):
phoenix_transfer_collateral(ctx, mint, victim_claimant, amount = ALL_MARGIN);
// Phoenix debits the pool's executor-trader, credits attacker_trader.
// Attacker then calls Phoenix withdraw_funds on attacker_trader -> drains pool margin.
```

### Finding B (Low) — `phoenix_ember_wrap` standalone is 1:1 and internal
`phoenix_ember_wrap` (1337) wraps executor USDC→PhUSD in-place; no external outflow, no loss. Only validation gap is the same "trust Phoenix for traderAccount" pattern, which is harmless here (no dst).

### Finding C (informational) — `phoenix_ember_unwrap` relies entirely on V-01-free EMPER accounting
The slot-cap check at line 1532 (`new_pending <= slot.withdrawn`) and the subsequent `pending.amount += amount` (1668-1670) trust that `amount` equals the PhUSD actually burned by EMBER. The overcredit bug V-01 lives exactly there; once fixed, the invariant holds. No *additional* independent bug found in that handler.

### What was verified sound
- `phoenix_deposit_from_pool`: vault→executor transfer, EMBER wrap, Phoenix depositFunds all use the same `deposit_amount`; TVL decremented by `total_outflow`; fee capped and paid from vault; slot initialized with `amount = deposit_amount`, `withdrawn = 0`, `claimant_pubkey = ext_data.claimant` (bound by ZK proof). No loss.
- `phoenix_close_position` / `phoenix_place_order` / `phoenix_cancel_*`: all CPIs sign with `executor_seeds` as `traderWallet`; Phoenix enforces traderWallet↔traderAccount ownership, so a relayer cannot redirect orders to a non-pool trader. Order placement is a trading-risk/trust concern, not a code-level fund-loss bug.
- ATA pinning: `executor_ph_usd_ata`, `executor_token_account`, `vault_token_account` are all validated as canonical ATAs in the wrap/unwrap/queue handlers.

---

## Severity summary

| # | Location | Issue | Severity | Extractible theft? |
|---|----------|-------|----------|-------------------|
| A | `phoenix_transfer_collateral` / `place_order` transfer (938/645) | `dstTraderAccount` not validated to executor | Medium→High* | Yes *if* Phoenix allows cross-wallet transfer |
| B | `phoenix_queue_withdraw` (1140-1144) | `max_slot_amount` inflates per-slot cap | Medium | No (capped by Phoenix balance) |
| — | `phoenix_reissue_notes` | claimant Signer + `pending.amount` bound | — | **None — verified safe** |
| — | `phoenix_queue_withdraw` cap | `withdrawn<=amount` enforced on PDA | — | **None — verified safe** |

\* depends on external Phoenix `transferCollateral` wallet check (unverified in this audit).

---

## Recommended fixes
1. **Transfer collateral:** validate `remaining[4]` (and `remaining[3]`) derive from `executor_key` via Phoenix's `["trader", ...]` seeds; reject any account not owned by the program / not an executor sub-account.
2. **Queue withdraw cap:** remove `max_slot_amount`, or only allow lowering `slot.amount`, or source the allowance from a verified realized-PnL value rather than a caller arg.
3. **Defense-in-depth:** even where Phoenix is trusted to enforce traderWallet ownership, explicitly pin `traderAccount` / `traderConditionalOrders` PDAs in every Phoenix CPI to avoid silent reliance on external program invariants.

*Report generated by focused subagent review. V-01 intentionally omitted per scope.*

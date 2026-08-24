# Veilo privacy-pool — Solana Security Audit (perps.rs / predictions.rs)

Scope: `programs/privacy-pool/src/perps.rs`, `programs/privacy-pool/src/predictions.rs`
Focus: fund-loss vulnerabilities in `jperp_reissue_notes` / `prediction_reissue`
Auditor stance: Solana pentester.

## TL;DR

**No over-mint / pool-draining vulnerability found.** The executor/ATA balance check that the
code comment relies on IS correctly and strictly implemented, so private notes can never be minted
for more USDC than is actually moved into the vault. Signer checks are strict. No arithmetic
overflow. No PDA seed collision.

One real **fund-handling defect** (Medium, conditional) and one **code/circuit contradiction**
(Low/Medium, needs circuit confirmation) were found. Details below.

---

## Pattern (1) — over-mint of private notes vs real USDC  →  DEFENSE HOLDS

### jperp_reissue_notes (perps.rs:1054-1332)

* SPL / USDC pools (else-branch, perps.rs:1215-1282):
  - `executor_ata_data.amount >= gross_outflow` (perps.rs:1230-1233) where
    `gross_outflow = reissue_amount + ext_data.fee` (perps.rs:1130-1132).
  - ATA account is pinned: `executor_token_account.key() == ATA(executor_key, mint)` (perps.rs:1216-1220).
  - Transfer moves exactly `reissue_amount` to vault (perps.rs:1255-1266); fee to relayer.
  - Transfer is bounded by the real balance; can never exceed it.
* Native-SOL pools (if-branch, perps.rs:1187-1214):
  - `executor_wsol_data.amount >= reissue_amount` (perps.rs:1201-1204), WSOL ATA pinned (perps.rs:1193-1197).
  - `close_account` sweeps the **entire** ATA balance into the vault (perps.rs:1206-1214).

Verdict: `reissue_amount` is always bounded by the genuine source-ATA token balance, which is
pinned to the correct authority/mint. There is **no way** to pass a substituted ATA with an inflated
balance (the address is recomputed and compared), and no path where notes minted exceed the real
USDC transferred. **Over-mint is not possible.**

### prediction_reissue (predictions.rs:317-558)

* `ephemeral_ata_data.amount >= gross_outflow` (predictions.rs:458-461), ATA pinned to
  `ATA(claimant, mint)` (predictions.rs:444-448).
* Transfer: `reissue_amount` to vault (predictions.rs:484-494), fee to relayer. Bounded by real balance.

Verdict: same as above — **no over-mint.**

### The "reissued is NOT a ceiling" design is sound
`slot.reissued` is a cumulative audit counter only (perps.rs:1099-1104, predictions.rs:437-439);
the real ceiling is the source-ATA balance, which is enforced. The comment's claim is accurate.

---

## Pattern (2) — strict signer check on claimant  →  HOLDS

* `jperp_reissue_notes`: `claimant: Signer<'info>` (lib.rs:2866). Handler requires
  `claimant.key() == jperp_slot.claimant_pubkey` (perps.rs:1093-1097). The signing `claimant`
  is also embedded in the `jperp_executor` PDA seed (lib.rs:2873; perps.rs:1178-1184), so the PDA
  that authorizes the USDC transfer can only be derived from the authentic signer. Strict.
* `prediction_reissue`: `claimant: Signer<'info>` (lib.rs:3183). Handler requires
  `claimant.key() == slot.claimant_pubkey` (predictions.rs:432-436) and the ephemeral ATA is
  `ATA(claimant, mint)`; claimant is the ATA authority that signs the transfer
  (predictions.rs:484-494). Strict — a relayer cannot move another user's settled proceeds
  without the ephemeral private key.

Verdict: signer gating is correct and cannot be bypassed.

---

## Pattern (3) — arithmetic overflow  →  HOLDS

All sensitive arithmetic uses checked ops:
* `gross_outflow = reissue_amount.checked_add(ext_data.fee)` (perps.rs:1130, predictions.rs:379).
* `slot.reissued = slot.reissued.checked_add(reissue_amount)` (perps.rs:1102, predictions.rs:437,
  perps.rs:1476 recover_native).
* `cfg.total_tvl = total_tvl.checked_add(reissue_amount)` (perps.rs:1304, predictions.rs:523) and
  `checked_sub` (predictions.rs:227).
* `max_fee` computed in `u128` with `checked_mul` (perps.rs:1121, predictions.rs:370).

No unchecked `wrapping_add`/`saturating` on fund amounts. Verdict: no overflow vector.

---

## Pattern (4) — PDA seed collision (JupiterPerpSlot / PhoenixSlot / PredictionSlot)  →  NO COLLISION

Seed prefixes are distinct (lib.rs):
* `JupiterPerpSlot`  = `[b"jperp_slot_v1",      mint, withdrawal_id]`            (lib.rs:351, 2614)
* `PredictionSlot`   = `[b"prediction_slot_v1", mint, withdrawal_id]`            (lib.rs:379, 3191)
* `PhoenixSlot`      = `[b"phoenix_slot_v1",    mint, claimant, withdrawal_id]`  (lib.rs:283, 2136)

Different leading seed strings make a collision mathematically impossible even with identical
`mint`/`withdrawal_id`. (JupiterPerpSlot and PredictionSlot deliberately omit `claimant` from the
seed; that is fine because the handler re-binds `claimant` to `slot.claimant_pubkey` and the slot is
created once via `init`/`init_if_needed` keyed by `withdrawal_id`.) Verdict: no collision.

---

## Finding A (MEDIUM, conditional) — SOL-pool reissue under-credits TVL / strands excess

Location: `jperp_reissue_notes`, native-SOL branch, perps.rs:1187-1214 + 1304-1306.

Mechanism:
1. Balance check only requires `executor_wsol_data.amount >= reissue_amount` (perps.rs:1201-1204).
2. The branch then `close_account`s the **entire** WSOL balance into the vault
   (perps.rs:1206-1214) — i.e. it moves `executor_wsol_data.amount`, which may be **larger** than
   `reissue_amount`.
3. TVL is bumped by only `reissue_amount` (perps.rs:1304-1306).

If a caller supplies `reissue_amount < executor_wsol_data.amount` (e.g. a winning position where the
keeper returned more than the chosen reissue figure), the delta
(`amount - reissue_amount`) is added to the vault's native balance but no private note is minted to
claim it. That excess is permanently unclaimable (the executor ATA is closed; only the program can
move vault funds, and no note redeems the surplus). Net effect: the position owner silently loses the
excess (their profit/principal above `reissue_amount`); the pool carries an unbacked TVL undercount.

This is the *opposite* of over-mint (vault ends **over**-collateralized), but it is a genuine
fund-handling defect and a real loss of value for the rightful owner. It is conditional on the
client passing `reissue_amount` below the true WSOL balance.

Conceptual PoC:
1. Open a SOL long; keeper settles and returns 2000 WSOL to the executor WSOL ATA (deposited 1000).
2. Call `jperp_reissue_notes` with `reissue_amount = 1000`.
3. Check passes (2000 >= 1000). `close_account` moves 2000 WSOL to vault; `total_tvl += 1000`.
4. 1000 WSOL sits in the vault with no redeeming note — destroyed from the owner's perspective.

Fix: assert `reissue_amount == executor_wsol_data.amount` (or credit TVL with the full swept amount),
and/or reject `reissue_amount` that does not equal the ATA balance in the SOL branch.

Note: the SPL-ATA branch (perps.rs:1215-1282) does NOT have this bug — it transfers exactly
`reissue_amount` and leaves any remainder in the ATA. The bug is specific to the `close_account`
sweep in the native-SOL branch.

---

## Finding B (LOW/MEDIUM, confirm against circuit) — input-nullifier contradiction

Location: handlers require non-zero input nullifiers (perps.rs:1151-1154, predictions.rs:400-403)
while the `prediction_reissue` doc-comment claims the deposit circuit uses
"dummy zero nullifiers for inputs" (predictions.rs:315).

If the reissue ZK circuit truly emits fixed dummy (constant non-zero) input nullifiers, then every
`jperp_reissue_notes` / `prediction_reissue` marks the *same* nullifier spent, and a second/partial
reissue of the same position fails with `NullifierAlreadyUsed` — blocking retrieval of remaining
settlement proceeds (fund stranding / availability loss). If instead the circuit consumes genuine
input notes, the comment is merely inaccurate and there is no issue.

This cannot be confirmed from the on-chain code alone (the Groth16 circuit / proof generator is
out of scope of these two files). Flagged for verification: confirm the reissue circuit's
input-nullifier semantics; if dummy, sequential/multiple reissues of one position are unsafe.

---

## Summary table

| # | Pattern checked                              | Result        | Note |
|---|----------------------------------------------|---------------|------|
| 1 | Over-mint via weak executor-ATA balance check| SAFE          | Check is strict; ATA pinned; bounded by real balance |
| 2 | Strict claimant signer gate                  | SAFE          | `claimant: Signer`, bound to `slot.claimant_pubkey` + PDA seed |
| 3 | Arithmetic overflow on reissued/amount       | SAFE          | All `checked_*` |
| 4 | PDA seed collision (perp/phoenix/prediction) | SAFE          | Distinct seed prefixes |
| A | SOL reissue TVL under-credit / stranding     | MEDIUM (bug)  | `close_account` sweeps full balance, TVL+=reissue_amount |
| B | dummy-nullifier vs ZeroNullifier contradiction| LOW/MED (TBD)| needs circuit confirmation |

**Bottom line:** the feared over-mint / pool-drain is NOT present. The only fund-affecting defect is
Finding A (SOL-pool reissue silently destroys any excess above `reissue_amount`); Finding B is an
open question pending circuit review.

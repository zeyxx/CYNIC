# Veilo privacy-pool — `swap.rs` Pentest Audit (fund-loss focus)

**File:** `programs/privacy-pool/src/swap.rs` (1026 LOC) + `lib.rs` `ExtData`/`SwapParams` hashing
**Scope:** direct, extractable loss of funds in `transact_swap` / `fund_native_source`.
**Verdict:** No *trivial* infinite-mint or double-spend primitive was found, BUT there is one
HIGH-severity design gap (swap route not proof-bound → uncapped relayer fee extraction) plus a
MEDIUM latent gap (claimant/recipient bound only in the (unverifiable-here) circuit, never on-chain).

---

## 1. HIGH — `swap_data` is NOT proof-bound ⇒ relayer controls routing & skims the full spread

### Mechanism
`SwapParams::hash()` (lib.rs ~L101-127) folds only `min_amount_out`, `deadline`, `dest_amount`
into `swap_params_hash`. The `swap_data_hash` field is **explicitly excluded** (confirmed by the
comment at swap.rs L54-57: *"swap_data_hash is not proof-bound until the circuit and verifying
key are upgraded"*).

In `transact_swap`:
- L310 `swap_params_hash` → proof public input (Groth16-verified at L324).
- `swap_params.swap_data_hash` is **not** part of that hash, so it is NOT proven.
- L827-828 is the only check: `sha256(swap_data) == swap_params.swap_data_hash`. Both values are
  supplied by the **relayer** and are mutually consistent by construction — it proves nothing about
  the user's intent.
- The relayer therefore fully controls the DEX instruction executed (route, pools, exact
  `amount_in`, exact `amount_out`) as long as the loose on-chain guards pass.

The only on-chain ceilings a relayer faces:
- CPMM/AMM: `dex_amount_in == swap_amount` (L553/L646) and `dex_min_out >= min_amount_out` (L557/L650).
- Jupiter: **NO `amount_in == swap_amount` check at all** (only mints + `swap_data_hash`); the
  leftover-revert at L954 forces full consumption but does NOT bind the route.
- Final: `swapped_amount >= min_amount_out` (L849) and `relayer_fee < swapped_amount` (L852).

### Extractable loss
The victim's proof commits `dest_amount` = the value of the destination note (what they can later
withdraw). After the swap:
```
relayer_fee  = ext_data.fee                       (L851, relayer-chosen, only >= min_required_fee)
vault_amount = swapped_amount - relayer_fee       (L862)
require vault_amount >= dest_amount               (L868)
```
So `relayer_fee <= swapped_amount - dest_amount`. Because the relayer also controls `swap_data`,
they route to realize `swapped_amount` as high as the DEX allows (or exactly `min_amount_out` to
minimize the victim's take) and then set `relayer_fee = swapped_amount - dest_amount`, **pocketing
the entire swap surplus / price improvement**. Worse, `min_required_fee` is enforced as a **minimum**
(L860 `require!(relayer_fee >= min_required_fee)`), not a cap, so the protocol actively forces the
relayer to take *at least* the percentage fee — there is no upper bound except the user's own
`dest_amount`. A user who commits a low `dest_amount` (or is forced to via relayed submission) loses
`swapped_amount - dest_amount` to the relayer with no on-chain recourse, since the proof cannot pin
the route.

This is the realistic, repeatable, **extractable** loss: the relayer grabs the full spread on every
swap they relay, unbounded by anything the user can prove.

### PoC (conceptual Rust)
```rust
// Relayer, holding a valid victim proof (swap_amount=S, dest_amount=D, min_amount_out=M):
// 1) Build swap_data that routes S source-tokens through a pool yielding SWAPPED > D.
//    For CPMM/AMM set data[8..16] = S, data[16..24] = M (meets dex_min_out >= M).
//    For Jupiter: any route; no amount_in binding exists.
// 2) ext_data.fee = SWAPPED - D;  // max allowed by L868
// 3) swap_params.swap_data_hash = sha256(swap_data);  // satisfies L827
// On-chain result:
//   vault_amount = SWAPPED - (SWAPPED - D) = D   -> dest pool stays solvent, note D is "backed"
//   relayer_token_account receives (SWAPPED - D)  -> PURE EXTRACTION, proof still valid.
// Repeat per swap. Loss per tx = SWAPPED - D (the whole spread).
```

---

## 2. MEDIUM — claimant/secret binding is circuit-only; inert on-chain in `swap.rs`

### Mechanism
`ext_data.recipient`, `ext_data.claimant`, `ext_data.relayer` are folded into `ext_data_hash`
(lib.rs L527-537, `ExtData::hash`) and `ext_data_hash` is a Groth16 public input (L324). So *if* the
circuit hashes them identically, substitution breaks the proof — **but the on-chain handler never
reads them**: in `transact_swap`, `ext_data` is consumed only for `ext_data.hash()` (L311) and
`ext_data.fee` (L851). There is **no** `require_keys_eq!(ext_data.recipient, …)`, no
`require_keys_eq!(ext_data.claimant, …)`, no `ext_data.relayer == ctx.accounts.relayer` check
(contrast `predictions.rs` L104-105 / `perps.rs` L301 which DO enforce it).

Consequence: the swap routes funds to the **destination pool vault** (L925/L893), never to
`ext_data.recipient`. So within `swap.rs` there is no direct theft. But the binding that "the proof
belongs to a specific claimant" has **zero defense-in-depth on-chain**, and depends entirely on the
(unverifiable-in-this-audit) circuit. Given the file itself documents that `swap_data_hash` was
shipped *not* proof-bound (L54-57), the same class of gap for `recipient`/`claimant` cannot be
ruled out by reading Rust alone. If the circuit ever lags the hash function, a relayer could
substitute recipient/claimant without the on-chain layer catching it.

**Direct fund loss in `swap.rs`: none today (assuming circuit matches). Latent risk: HIGH if circuit
binding is incomplete.**

---

## 3. Findings against the 6 requested focus points

| # | Focus | Result |
|---|-------|--------|
| 1 | Claimant/recipient binding in `ext_data_hash` strict? Can relayer substitute? | Proof-bound *if* circuit matches; **never enforced on-chain** in `swap.rs`. Substitution blocked by proof, not by handler. See §2. |
| 2 | Fee / refund arithmetic — overflow / excess extraction | No overflow (u128 math, L855-858). **But `relayer_fee` is a MINIMUM (L860), not capped** → uncapped skim bounded only by user `dest_amount`. See §1. No `refund` used in swap. |
| 3 | `public_amount` i64 sign → fraudulent mint | **N/A** — `swap.rs` uses `u64 swap_amount` (L240) and has no deposit/public_amount path. This class of bug lives in `transact`, not `swap`. |
| 4 | Merkle root forgery (`root_history`) | **Correctly mitigated**: `MerkleTree::is_known_root` (L327) gates the proof's `source_root`. Forged root ⇒ proof fails. OK. |
| 5 | Nullifier double-spend via swap | **Correctly mitigated**: both markers checked `!is_spent` (L340-341) then `mark_nullifier_spent` (L343-358). Reuse in any instruction reverts. OK. |
| 6 | `min_amount_out` constraint bypassable to drain | `min_amount_out` is proof-bound, and `swapped_amount >= min_amount_out` (L849) + `dex_min_out >= min_amount_out` (L557/L650) hold. **BUT** because `swap_data` is relayer-controlled (§1), the relayer can always make `swapped_amount == min_amount_out` and keep the spread as fee — effectively bypassing the user's price protection. |

---

## 4. Additional notes (not direct loss)

- **Native non-prefunded float path (L399-429, L983-994):** relayer floats `swap_amount`, vault is
  re-debited, relayer reimbursed — arithmetic is balanced; no double-count found.
- **Prefunded path (L391-398):** `swap_amount` cross-checked against executor (L392); TVL debited
  once in `transact_swap`; `fund_native_source` debits vault once. Balanced.
- **Jupiter has no `amount_in == swap_amount` assertion** (only CPMM/AMM, L553/L646). Not directly
  exploitable because L954 reverts on leftover, but it is an inconsistency and removes a guard the
  other two paths have.
- `reclaim_stale_executor` is referenced (SwapExecutor doc) but **not defined in `swap.rs`** — out of
  scope / possibly dead or in another module; verify it cannot re-close an already-settled executor
  for a second rent skim.

---

## 5. Recommendation (bounty-grade fixes)

1. **Bind the swap route in the proof.** Include `swap_data_hash` (or at minimum `dex_amount_in` and
   `dex_min_amount_out`) in `swap_params_hash` so the circuit, not the relayer, fixes the DEX call.
2. **Cap `relayer_fee`** with a MAX (e.g. `relayer_fee <= swapped_amount - dest_amount` is the only
   current ceiling, and it is user-committed) — e.g. enforce
   `relayer_fee <= swapped_amount * max_fee_bps / 10000`. The current *minimum*-fee logic is inverted
   for user protection.
3. **Add on-chain `require_keys_eq!`** for `ext_data.relayer == ctx.accounts.relayer` (and, where a
   recipient concept exists, `ext_data.recipient`) so the proof's binding is backed by handler checks,
   matching `predictions.rs`/`perps.rs`.
4. Add the Jupiter `dex_amount_in == swap_amount` parity check for consistency with CPMM/AMM.

**Bottom line:** No open double-spend or mint primitive, but **§1 is a real, extractable,
repeatable loss** (relayer skims the full swap surplus on every relayed swap) and is the strongest
finding for the bounty.

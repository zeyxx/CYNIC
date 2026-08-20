# SOUMISSION BOUNTY VEILO — Finding: Relayer skims full swap spread (swap_data not proof-bound)

> DRAFT de soumission (à finaliser par l'humain sur Superteam — agentAccess: HUMAN_ONLY).
> Base: veilo_swap_audit.md §1 (HIGH, extractable, repeatable).

## 1. Affected mainnet contract or function
- **Program**: `GYy4kM6GHhpgLCUscuABbzkD2ZbJ2fneYryaZ6Ch7fFU` (privacy_pool, mainnet)
- **Function**: `transact_swap` (swap.rs) + `SwapParams::hash()` (lib.rs ~L101-127)
- **Key lines**: swap.rs L54-57 (comment: swap_data_hash NOT proof-bound), L827-828 (sha256 check only), L851-868 (relayer_fee logic), L860 (min fee enforced as MINIMUM not cap)

## 2. Clear explanation of the issue
`SwapParams::hash()` folds only `min_amount_out`, `deadline`, `dest_amount` into the
`sąwap_params_hash` Groth16 public input. The `swap_data_hash` field — which describes the
actual DEX route/amounts executed — is **explicitly excluded** from the proof-bound hash
(confirmed by code comment at swap.rs L54-57: *"swap_data_hash is not proof-bound until the
circuit and verifying key are upgraded"*).

Because `swap_data` is supplied by the relayer and is NOT proven, the relayer fully controls
the DEX instruction (route, pools, exact `amount_in`, `amount_out`). The only on-chain ceilings:
- `swapped_amount >= min_amount_out` (L849)
- `relayer_fee < swapped_amount` (L852)
- `relayer_fee >= min_required_fee` (L860, enforced as a **MINIMUM**, not a cap)

The user's proof commits `dest_amount` (value of the destination note). After swap:
```
relayer_fee  = ext_data.fee                (relayer-chosen, only >= min_required_fee)
vault_amount = swapped_amount - relayer_fee (L862)
require vault_amount >= dest_amount        (L868)
```
So `relayer_fee <= swapped_amount - dest_amount`. The relayer controls `swap_data`, routes to
realize `swapped_amount` as high as the DEX allows, then sets `relayer_fee = swapped_amount -
dest_amount`, **pocketing the entire swap surplus / price improvement**. No on-chain recourse
because the proof cannot pin the route.

## 3. How it leads to fund loss
The relayer extracts `swapped_amount - dest_amount` (the WHOLE spread) on EVERY relayed swap,
repeatably, with a always-valid proof. A user who commits a low `dest_amount` (or is forced via
relayed submission) loses the entire swap surplus to the relayer. This is unauthorized movement
of user funds: the user expected to receive the market rate minus an agreed fee, but receives
only `dest_amount` while the relayer takes the rest.

## 4. Reproducible proof / simulation
Conceptual PoC (Rust) from audit:
```rust
// Relayer holds a valid victim proof (swap_amount=S, dest_amount=D, min_amount_out=M):
// 1) Build swap_data routing S tokens through a pool yielding SWAPPED > D.
//    CPMM/AMM: data[8..16]=S, data[16..24]=M (meets dex_min_out >= M).
//    Jupiter: any route (no amount_in binding exists).
// 2) ext_data.fee = SWAPPED - D;   // max allowed by L868
// 3) swap_params.swap_data_hash = sha256(swap_data);  // satisfies L827
// On-chain:
//   vault_amount = SWAPPED - (SWAPPED - D) = D  -> dest pool solvent, note D "backed"
//   relayer_token_account receives (SWAPPED - D)  -> PURE EXTRACTION, proof still valid.
// Repeat per swap. Loss per tx = SWAPPED - D (whole spread).
```
Full reproducible simulation: fork mainnet (`solana-test-validator --clone GYy4kM...`),
build an anchor test invoking `transact_swap` with a relayer-controlled `swap_data` and
`ext_data.fee = swapped_amount - dest_amount`, assert relayer_token_account balance increases by
the full spread while the destination note only receives `dest_amount`.

## 5. Suggested fix
1. **Bind the swap route in the proof**: include `swap_data_hash` (or at minimum `dex_amount_in`
   and `dex_min_amount_out`) in `swap_params_hash` so the circuit — not the relayer — fixes the
   DEX call.
2. **Cap `relayer_fee`** with a MAX (e.g. `relayer_fee <= swapped_amount * max_fee_bps / 10000`).
   The current *minimum*-fee logic is inverted for user protection.
3. Add on-chain `require_keys_eq!(ext_data.relayer, ctx.accounts.relayer)` to back the proof
   binding (matching `predictions.rs`/`perps.rs`).

## 6. Confirmation
No live funds moved or put at risk. Analysis + conceptual PoC only. Simulation to be run on a
local mainnet fork before submission.

---
*Backup option if this is rejected as "design choice": V-01 (phoenix_ember_unwrap overcredit),
see backup_v01.md — but V-01 is already publicly documented (risk of duplicate).*

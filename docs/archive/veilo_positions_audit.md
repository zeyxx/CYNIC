# Veilo `privacy-pool` — positions.rs & merkle_tree.rs Audit

**Scope:** `/tmp/veilo-official/programs/privacy-pool/src/positions.rs` (2447 lines) and `.../merkle_tree.rs` (308 lines).
**Objective:** find directly *extractible* direct fund-loss bugs — double-spend of notes, or draining of the position/shared vaults.
**Verdict: NO critical, extractible double-spend or vault-drain vulnerability found in these two files.** The core invariants (nullifier uniqueness, proof-bound amounts, physical measurement of received funds, checked arithmetic) are correctly enforced. Detailed reasoning + the few defense-in-depth gaps are below.

---

## 1. Double-spend protection — SOUND

### Nullifier markers use `init` (not `init_if_needed`)
- `open_position` (positions.rs 531-546): source notes marked spent via `source_nullifier_marker_0/1` — Account contexts use `init` with seeds `[b"nullifier_v3", source_mint, nullifier]` (lib.rs 1274-1290). Replays fail at PDA creation ("already in use").
- `close_position` / `close_position_to_sol` (positions.rs 1072-1165, 1474-1563): position notes marked spent via `position_nullifier_marker_0/1` — `init` with seeds `[b"position_nullifier_v1", source_mint, nullifier]` (lib.rs 1429-1445).
- `merge_positions` (positions.rs 2211-2285): same `position_nullifier_v1` namespace, `init`.

All three spend-paths share the **same** `position_nullifier_v1` seed namespace, so a note closed via `close_position` cannot also be consumed by `close_position_to_sol` or `merge_positions`. `mark_nullifier_spent` (lib.rs 4901) additionally sets `is_spent=true` and bumps the `NullifierSet.count`. The `init` constraint is the hard guarantee; the `is_spent` check is belt-and-suspenders. **No double-spend path exists.**

### PositionPDA is balance-bounded
- On close, `require!(swap_amount == pos_pda.balance, ...)` (positions.rs 1179, 1497) and `pos_pda` is `close = relayer` (lib.rs 1421-1427), so it can only be closed once.
- `pos_pda.balance` is set on open to `swap_params.dest_amount` (positions.rs 973) and on merge to `pda_0.balance + pda_1.balance` (positions.rs 2320). Both are proven values (see §2).
- `claimant` must sign (`require_keys_eq!` positions.rs 1076, 1478) — prevents a whitelisted relayer from closing a victim's PDA with a stolen proof.

## 2. Amount proof-binding — SOUND (eliminates minting/inflation)

- `swap_params.hash` (swap.rs 102-127) commits to `min_amount_out`, `deadline`, `dest_amount`, `source_mint`, `dest_mint`. `swap_amount` is a separate `SwapPublicInputs` field (swap.rs 133, positions.rs 503-512). The Groth16 proof therefore binds **every** monetary value: the input routed to the swap, the output note value, and the vault deposit target.
- Consequence: an attacker cannot inflate `pos_pda.balance`, `swap_amount`, `dest_amount`, or `min_amount_out`. Vault debits are always bounded by a proof-committed, real deposited value.

## 3. Vault accounting — SOUND (no over-withdrawal)

`close_position` / `close_position_to_sol` (positions.rs 1202-1247, 1593-1701):
- `require!(vault_balance >= swap_amount)` checks the **physical** ATA balance before transfer.
- `swap_amount == pos_pda.balance` (proven). So the debit can never exceed the position's proven balance, and (inductively) never exceeds the vault's physical holdings.
- `vault_record.total_balance` uses `checked_sub`/`checked_add` (positions.rs 962-967, 1370-1373, 1775-1777) — no under/overflow, and even if bookkeeping drifted, the **physical** ATA is the debit source of truth.

`open_position` (positions.rs 876-914): `vault_received` is measured as a **physical** delta (`after - before`) of the vault ATA, and `require!(vault_received >= dest_amount)` guarantees the deposited PDA value is backed. T22 fee-on-transfer is handled correctly because the code re-measures rather than trusting the transferred amount.

`validate_fee_to_vault` (positions.rs 349-368): computes `pct_fee` in `u128` (no overflow), enforces `relayer_fee >= min_fee`, and returns `received.saturating_sub(relayer_fee)` with `require!(vault_amount >= dest_amount)`. The relayer can only take a fee as large as the proven output; the vault always keeps ≥ the note value. The `saturating_sub` + subsequent `>= dest_amount` check means `relayer_fee > received` (which would zero the vault share) is rejected unless `dest_amount == 0`.

## 4. Merkle tree / root history — SOUND

`merkle_tree.rs`:
- `append` (65-114): only the program can insert; leaves are Poseidon-hashed up the tree; `next_index` is `checked_add`. Circular `root_history` is updated on every append.
- `is_known_root` (116-148): returns true **only** if the supplied root equals a value actually stored in `root_history` (a real computed root). The zero root is rejected. This blocks forged roots / fraudulent note "minting" — an attacker cannot present a root the tree never produced.
- No insertion/commitment manipulation surface: callers (`append_and_emit`, positions.rs 318-345) only pass caller-supplied commitments after the ZK proof has validated them; the tree math itself is correct.
- Layout tests (168-273) protect against account-corruption regressions.

## 5. Lower-severity observations (NOT direct fund loss)

These do not permit extraction of funds beyond what was deposited, but are worth noting:

| # | Location | Issue | Impact |
|---|----------|-------|--------|
| L1 | positions.rs 427, 1330-1337 | `position_pda_key` (`_position_pda_key`) is **never validated** against `output_commitment_1` / `ext_data` in the handler. PDA created at arbitrary key. | A user/relayer could mint a PositionPDA at a key not derivable from their note. Since `claimant` must sign on close and the balance is deposit-backed, this is a self-inflicted recoverability problem, **not** a theft vector. Low. |
| L2 | positions.rs 1814, 1906-1920, 1999-2001; swap.rs 53-58 | `swap_data_hash` (the actual Jupiter route / staged legs) is **not proof-bound** in the current circuit/VK (explicitly flagged in code comments). | A relayer (already whitelisted via `is_relayer`) chooses the DEX route. This is a trust assumption on the relayer, **not** extractible theft: `execute_dex_swap` pins the swap's `auth/src/dst` to the executor + its ATAs, and `received_amount` is physically re-measured from `executor_dest_token`. The route cannot redirect funds out of the executor ATAs. Medium (architecture/trust). |
| L3 | positions.rs 2194-2333 (`merge_positions`) | The merge proof's input/output **amounts** are private witness (public inputs are only `root, public_amount=0, ext_data_hash, mint, input_nullifiers, output_commitments`). `merged_amount` is a free caller param, checked only against `pda_0.balance + pda_1.balance`, not against the proof's `out0`. | Creates a PDA whose recorded balance may differ from the tree note's real value, but the vault stays **solvent**: `new_pda.balance = pda_0.balance + pda_1.balance` = sum of real deposits, and the vault physically holds exactly that. Closing later withdraws only what was deposited. No net theft. Low/consistency-only. |
| L4 | positions.rs 962-967 vs 876-914 | On open, `vault_record.total_balance += vault_received` (≥ `dest_amount`) while `pos_pda.balance = dest_amount`. The excess (`vault_received - dest_amount`) is stranded in the vault on close. | Donation/rounding inefficiency, not extractible theft. Informational. |

## 6. Conclusion

The two files implement their anti-double-spend (nullifier `init` + `is_spent`), proof-binding of all monetary amounts, physical measurement of swap outputs, and checked arithmetic correctly. **No directly extractible double-spend or vault-drain vulnerability was identified.** The residual findings (L1-L4) are trust-model / consistency issues that an adversary cannot convert into net fund extraction. No PoC for a critical bug is warranted because none exists in this code.

*Note: this assessment covers `positions.rs` and `merkle_tree.rs` only. The ZK circuit / VK (`verify_swap_transaction_groth16`, `verify_transaction_groth16`) and the `swap.rs` swap-execution paths were read only to confirm proof-binding; a circuit-level soundness review (e.g., whether `dest_amount` is truly enforced inside the constraint system, not just as a public input) is out of scope but recommended as the highest-value next step for the bounty.*

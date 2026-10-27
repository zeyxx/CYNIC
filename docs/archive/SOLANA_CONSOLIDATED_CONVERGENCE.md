# Solana Consolidated Convergence (Solana bridge convergence map, consolidated)

> Status: deliberation reference. Runtime authority remains `contracts/foundation.json`
> and `infra/runtime-authority.json`. This file consolidates scattered leads only;
> it introduces no program behavior and no authority claim.

## 1. Purpose

CYNIC has many Solana-adjacent leads: a ten-dimension convergence map,
a draft Solana anchor program, a real pump-origin ASDF mint, a gas-service
plan, a token quote-chain mechanism and deferred Anchor program compile
errors. They live in separate docs, branches and crates.

This file is the single index that points at each lead, records what is
observed (not asserted) and lists the concrete convergence options that
the operator can select between. It is the source of the option list used
in operator conversation.

## 2. Authoritative evidence (observed)

- Branch `agent/hermes/native-proof-full-suite` tip `4c7a1561`.
- `programs/cynic-anchor/src/lib.rs` — Anchor 0.30 program: PoH checkpoint,
  futarchy CPMM, `execute_ignis_burn`, state zero-copy. Pending on-chain
  deploy (program ID placeholder).
- `SOLANA_CONVERGENCE_MAP.md` — ten dimensions, all labelled
  informal / semi-formal / orthogonal, verification log dated Oct 12 2026.
- ASDF mint `9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump`, fixed
  supply 853 137 556, 6 decimals, pump origin (explorer read Oct 12 2026).
- `docs/design/GASdf` rationale: gas as a service, billed in USDC, fee
  payer SOL; strict waterfall; survival rule "profitable alone or dead".
- Skill `cynic-solana-token-quote-chains` carries depth-chain mechanics
  (rules 1-7, pitfalls).
- Pilot native skills for quote chains already installed (Oct 14 2026).

## 3. Pillars (pointer to dimension map)

<|message|> # | Dimension | Authority | Gap opened |
|---|-----------|-----------|------------|
| 1 | Time/consensus | Solana slots 250ms | Calendar only — no deployed anchor |
| 2 | Execution | Sealevel/CUs | Pricing CUs <-> workloads not defined |
| 3 | Accounts/PDAs | solana docs | Client derivation is a SHA-256 stub, not `find_program_address` |
| 4 | Assets | Token-2022/Legacy | ASDF is legacy Token; burn CPI was pinned wrong — fixed in `4c7a1561` |
| 5 | Data/reads | gRPC/RPC | No submission adapter, no commitment policy |
| 6 | Keys | Phantom/MWA | No signer bridge from Kyon vault |
| 7 | Payments | Solana Pay/x402 | No on-chain settlement proof yet |
| 8 | Distribution | Blinks/Actions | Static QR only |
| 9 | Markets | Pump curves, Meteora | Futarchy CPMM informel, single oracle |
| 10 | Proof/scale | compression/nonces | No anchored root, no portable consent |

Each row's detailed state lives in `SOLANA_CONVERGENCE_MAP.md`. Nothing
below redefines those rows.

## 4. The three market paths (pick one)

### Path A — "ASDF as native quote pivot" (chosen root)

Declared now: ASDF (depth 0) is the pivot quote for CYNIC ecosystem
services. `virtual_quote_reserves` auto-calibrate each service's launch to
the ~85 SOL raise-equivalent in the ASDF chain. Settlement via
`multi_hop_swap` SOL->ASDF->service->nested.

- Holding cost: depth account groups, 200k compute units budget, mayhem
  exclusion, migration completeness along the chain.
- Non-goal: selling yield; subsidizing GASdf.

### Path B — "Convergence by native proof, ASDF separate"

Each pillar (PDA, burn, RPC) becomes a real primitive in small probes;
ASDF stays an external pump token until GASdf proves its own economics.
Convergence emerges by verified intersections, not by a single pivot coin.

- Risk: slower, but lower correctness risk around transaction-size inflation and
  quote-path amnesia.

### Path C — "Opportunistic quotes"

Services choose SOL quote or ASDF quote per use case, no golden path.

- Risk: fragmentation, two settlement currencies forever, no derivable
  "chain-stable" identifier.

Recommendation (services layer): Path A for CYNIC-native services, Path B
for infrastructure, no C. Rationale: Path A gives quote-discoverable price
discovery for nothing beyond the burn dispatch already done; Path B keeps
the programically risky parts staged.

## 5. Hard blockers still open (ranked)

1. Anchor program compiles only on remaining padding errors
   (`E0119 x6`, `E0080`). Needs repr(C) layout redesign of
   `PohRecordAccount`, `FutarchyMarketAccount`, `IgnisVaultAccount` so the
   sum of field sizes is a multiple of the largest field alignment, or a
   move of zero-copy to standard `Account<T>`. Resolution required before
   any deployment.
2. No PDA rename to real `find_program_address` derivation (the SHA-256 stub
   in `crates/cynic-protocol/src/anchor_client.rs`). Blocks chain-anchored
   roots referenced in dimension 10.
3. No submission adapter for any transaction (dimension 5 gap).
4. Burn/vault custody per key class unresolved (decision #3).
5. Futarchy single-oracle resolution (decision #7).

## 6. Pending user decisions (unchanged list, 8 total)

From `SOLANA_CONVERGENCE_MAP.md` "Pending decisions":
1. Burn path: Token program against ASDF or new Token-2022 mint.
2. Mint economics: supply, authorities, renunciation, decimals.
3. Custody per key class.
4. RPC provider + commitment level.
5. Real PDA derivation replacing SHA-256 stub.
6. Anchor cadence / Merkle leaves / epoch / account close policy.
7. Market resolution: single oracle or staked resolution.
8. Obsolete prose: Livre IV slots (400 ms), foundation scope.

Only item 1 partially resolved (dispatch added to keep both classes
acceptable, awaiting a final mint-class decision).

## 7. Branches needed in the convergence lane

- `agent/hermes/single-binary-topology-writer` — topology writer + boundary
  tests + topology.json resync. Its 8 modifications match the pre-session
  working-tree diff; merge required to resync the audit surface.
- `agent/hermes/native-proof-fail-closed` — node activation fixes on
  the proof suite (recent cherry-picks). Adopt diagnostics at the very
  least; the `cargo check` permission bug fixed by removing
  `Program<Token>` pin.
- `agent/hermes/agent-isolation*` family — read but isolate.
- Everything else stays outside the Solana convergence scope unless used.

## 8. Working rule for this sub-system (from operator)

- Bottom-up only: every convergence claim carries an `observed` tag and the
  source path. Speculative bridges rejected.
- Zero `git add -A`.
- Commit identity `-c user.name=hermes -c user.email=hermes@agent.local` form.
- Push before repack; never trust local-only git.
- No cargo/rustc locally; builders only.

## 9. One-line summary

The substrate is real (ASDF mint + multi-time dimension + a draft program), the
program anchor is blocked on its own compile (padding class), and the
convergence strategy choice (ASDF pivot vs. decomposed primitives) is the
one remaining operator decision that unblocks planning for actions like
depth-2 quote launches. All else flows from that selection.

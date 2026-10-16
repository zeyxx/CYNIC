# GASdf Peer Plan — first external project at `observed` stage

> Status: deliberation draft. Non-authoritative narrative; authority remains
> `contracts/foundation.json`, `infra/external-pins.json` (pin via registry),
> `infra/external-project-registry.json` (adoption `referent`, status `observed`),
> and `docs/specs/EXTERNAL_PROJECT_GOVERNANCE_SPEC.md` (graduation §6b).
> Companion: `docs/specs/GASDF_SUBSCRIPTION_SPEC.md` (subscription, distinct concern).

## 0. Intent

CYNIC stays the sovereign OS; GASdf stays a sovereign project. The correct
posture is **no absorption and no runtime coupling**: CYNIC ingests evidence and
contracts, GASdf keeps its code, keys and deploy pipeline, and exposes
machine-readable surfaces (REST, WebSocket, Geyser/Yellowstone) that CYNIC
consumes as a peer. Pure vocabulary may live in `cynic-protocol`; adapters
live in GASdf. `cynic-node` hosts only generic OS capabilities after
graduation — never project-named adapters (see governance §7).

See `docs/specs/EXTERNAL_PROJECT_GOVERNANCE_SPEC.md`.

## 1. What we drop on ingestion (observed excess)

| Prototype piece | Decision | Reason |
|-----------------|----------|--------|
| Express/Node runtime | KEEP in GASdf repo | project stack stays light and iterable; 100% Rust rule binds CYNIC crates, not the project |
| Redis quote store + locks | DROP (Phase 0) | state ephemeral in-process or file; no stateful server |
| PostgreSQL | DROP (Phase 0) | SQLite canon or append-only JSONL ledger |
| HolDex K-score token gating | DEFER | anti-sybil dependency; not needed for Phase 0 gasless proof |
| Holder tiers / E-Score / yield | DROP | revenue overengineering; re-earn by measured demand |
| Multi fee-payer pool | DEFER | single payer until funded |
| Jupiter swap in request path (tx size) | DEFER to Phase 2 | Phase 0 proves one tx; keep route, no swap in MVP or server-side only |

What we keep is the **physical reality**: quote → co-sign → submit → burn proof.

## 2. Observed baseline (bottom-up first, no abstraction yet)

Verified toolchain (2026-10-16, local): `solana-cli 4.2.0` (Agave),
`anchor-cli 1.2.1`; GASdf prototype already on `@solana/kit ^8.5.0` +
`helius-sdk ^2.0.5`. Baselines reuse this stack, no downgrade.

Before any predicate, measure on Solana devnet first, captured and hashed:

1. Cost of a minimal sponsored transaction ( lamports in / out, 10 samples).
2. Cost of fee-payer funding + refill cadence.
3. Transaction size headroom (MTU 1232) for quote/submit path.
4. Helius Sender p50/p99 latency for a single raw tx.
5. Jupiter price dispersion for input-token → ASDF (when Phase 2).

Each measurement becomes a ledger entry (JSONL append-only, preregistered) — pattern
from `docs/superpowers/plans/2026-10-15-paper-week-*.md`. No abstraction lands
before these five baselines exist.

## 3. Core model (extracted, FCIS)

No workspace adapter crate (`cynic-<project>` is forbidden by governance §7):

- Domain (pure, in `cynic-protocol::gasdf`): types `GasQuote`, `GasSession`,
  `FeePayer`; predicates `decide_gas_quote`, `decide_submit`,
  `decide_fee_payer_health`, plus subscription-side `decide_subscription_quote`.
- Adapters (Helius Sender HTTP, Jupiter HTTP, signer, ledger) live in the
  GASdf repo, next to its secrets and fee-payer. CYNIC never holds them.

The existing `cynic-solana-pay` allowance logic is adjacent and reusable for the
fee-payment instruction validation.

## 4. On-chain asset

The prototype's Pinocchio program stays in the GASdf repo and deploys from
its own pipeline. CYNIC records Program ID, seeds, and bump policy in
`infra/registry.json` only as observed surface, never as build input.

## 5. Phases

- **Phase 0 — one real tx (devnet first)**: quote + submit + co-sign via the
  GASdf repo stack (Kit sender path, Helius Sender) + proof recorded.
  DONE 2026-10-16: 2 devnet txs finalized, evidence
  `data_system/vault/captures/gasdf-phase0-20261016/`. No onchain program
  yet — deferred until funds allow.
- **MVP production**: quote TTL, Jupiter API pricing + swap (free tier),
  Redis/DB allowed, strict per-tx cost ledger (lamports + CU onchain,
  CYNIC compute offchain), circuit breaker armed. See GASdf
  `docs/MVP_SCOPE.md` (operator-decided scope).
- **Phase 1 — token registry and slippage**: data-driven accepted tokens via
  `crates/cynic-protocol/tests/fixtures/gasdf/*.json`; K-score gating deferred or stubbed to a registry of
  allowed mints.
- **Phase 2 — swap-to-ASDF + burn worker**: Jupiter route, burn worker,
  MTU/ALT mitigation if needed, all gated by measured baseline from §2.5.

## 6. Agents as engine (graduation protocol)

Each phase advances the graduation stages (governance §6b): scope claimed →
agent produces candidate → verification (cargo check/test + ledger read-back)
→ stage promotion with proof reference. No unverified mutation. The
non-overengineering rule is enforced by the constitutional court on the diff
(no new Redis/DB/yield/holder-tier modules in Phase 0). CYNIC acts as the
frictionless scientific engine under the project — proofs, measures, runs —
while GASdf stays iterable in its own repo.

## 7. Pending decisions

1. Program custody: RESOLVED — program stays in the GASdf repo; CYNIC keeps
   an external pin and records surfaces only. Revisit at `standard` stage.
2. Fee-payer custody class (`restricted`, no export) and secret locator.
3. Network target for Phase 0 (devnet vs mainnet) and who funds the payer.
4. Subscription and gasless verticals share `cynic-protocol::gasdf` domain
   modules only (subscription vs gasless predicates); adapters stay in GASdf.

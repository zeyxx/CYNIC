# GASdf Ingestion Plan — CYNIC first citizen app

> Status: deliberation draft. Non-authoritative narrative; authority remains
> `contracts/foundation.json` and `infra/external-pins.json` (pin: gasdf @ 86d917f).
> Companion: `docs/specs/GASDF_SUBSCRIPTION_SPEC.md` (subscription, distinct concern).

## 0. Intent

CYNIC stays the sovereign OS; GASdf stays a sovereign project. The correct
posture is **no absorption and no runtime coupling**: CYNIC ingests evidence and
contracts, GASdf keeps its code, keys and deploy pipeline, and exposes
machine-readable surfaces (REST, WebSocket, Geyser/Yellowstone) that CYNIC
consumes as a peer. Pure vocabulary may live in `cynic-protocol`; adapters
live in GASdf or `cynic-node` behind ports.

See `docs/design/EXTERNAL_PROJECT_BOUNDARY.md`.

## 1. What we drop on ingestion (observed excess)

| Prototype piece | Decision | Reason |
|-----------------|----------|--------|
| Express/Node runtime | DROP | 100% Rust supply chain |
| Redis quote store + locks | DROP (Phase 0) | state ephemeral in-process or file; no stateful server |
| PostgreSQL | DROP (Phase 0) | SQLite canon or append-only JSONL ledger |
| HolDex K-score token gating | DEFER | anti-sybil dependency; not needed for Phase 0 gasless proof |
| Holder tiers / E-Score / yield | DROP | revenue overengineering; re-earn by measured demand |
| Multi fee-payer pool | DEFER | single payer until funded |
| Jupiter swap in request path (tx size) | DEFER to Phase 2 | Phase 0 proves one tx; keep route, no swap in MVP or server-side only |

What we keep is the **physical reality**: quote → co-sign → submit → burn proof.

## 2. Observed baseline (bottom-up first, no abstraction yet)

Before any predicate, measure on Solana mainnet/devnet, captured and hashed:

1. Cost of a minimal sponsored transaction ( lamports in / out, 10 samples).
2. Cost of fee-payer funding + refill cadence.
3. Transaction size headroom (MTU 1232) for quote/submit path.
4. Helius Sender p50/p99 latency for a single raw tx.
5. Jupiter price dispersion for input-token → ASDF (when Phase 2).

Each measurement becomes a ledger entry (JSONL append-only, preregistered) — pattern
from `docs/superpowers/plans/2026-10-15-paper-week-*.md`. No abstraction lands
before these five baselines exist.

## 3. Core model (extracted, FCIS)

New crate (future) `cynic-gasdf`:

- Domain (pure, in `cynic-protocol` first): types `GasQuote`, `GasSession`,
  `FeePayer`, `BurnDecision`; predicates `decide_quote`, `decide_submit`,
  `decide_burn`, `decide_fee_payer_health`.
- Ports: `JupiterPort`, `HeliusSenderPort`, `FeePayerSignerPort`,
  `LedgerPort`, `TokenRegistryPort`.
- Adapters: Helius HTTP, Jupiter HTTP, in-process signer (from env key via secret
  reference, never value), SQLite/JSONL ledger, token registry JSON.

The existing `cynic-solana-pay` allowance logic is adjacent and reusable for the
fee-payment instruction validation.

## 4. On-chain asset

The prototype's Pinocchio program (`programs/gasdf`) stays as the Solana program
artifact. It is ingested as a **versioned build input** under `programs/gasdf`
in CYNIC and goes through `make release.deploy` (builder 10.10.10.119), not by
hand. Program ID, seeds, and bump policy must be declared in
`infra/registry.json` before deploy.

## 5. Phases

- **Phase 0 — one real tx**: quote + submit + co-sign + Helius Sender + burn proof
  recorded. No Redis, no DB, no yield. Gate: on-chain signature + explorer link
  (orbmarkets.io per prototype rule) in the ledger.
- **Phase 1 — token registry and slippage**: data-driven accepted tokens via
  `infra/gasdf-*.json`; K-score gating deferred or stubbed to a registry of
  allowed mints.
- **Phase 2 — swap-to-ASDF + burn worker**: Jupiter route, burn worker,
  MTU/ALT mitigation if needed, all gated by measured baseline from §2.5.

## 6. Agents as factory (citizen protocol)

Each phase ships as a citizen-claim lifecycle: scope claimed → agent produces
candidate → verification (cargo check/test + ledger read-back) → release with proof
reference. No unverified mutation. The non-overengineering rule is enforced by the
constitutional court on the diff (no new Redis/DB/yield/holder-tier modules in
Phase 0).

## 7. Pending decisions

1. Program ingestion: move `programs/gasdf` into this repo now or keep external pin
   until Phase 0 green? (Recommend: external pin now, ingest on Phase 0 green.)
2. Fee-payer custody class (`restricted`, no export) and secret locator.
3. Network target for Phase 0 (devnet vs mainnet) and who funds the payer.
4. Whether the subscription vertical and the gasless vertical share a crate or
   stay separate. Recommend: separate crates, shared `cynic-protocol` domain.

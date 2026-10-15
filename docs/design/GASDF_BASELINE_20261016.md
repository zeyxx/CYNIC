# GASdf Baseline — devnet, 2026-10-16

> Raw evidence: `data_system/vault/captures/gasdf-baseline-20261016/ledger.jsonl` (9 rows).
> Network: Solana devnet, public RPC `https://api.devnet.solana.com`, commitment `confirmed`.
> Signer: `66gjdKepF5N2iXeJRdoDsyqRG5ecgc64ExQgFsZz6kTy` (local devnet keypair, not committed).
> ASDF mainnet mint unaffected; no mainnet spend.

## Measured

| Metric | n | min | max | mean |
|--------|---|-----|-----|------|
| Transaction fee (lamports) | 9 | 4000 | 4000 | 4000 |
| Command round-trip (ms) | 9 | 1306 | 11386 | 5423 |
| Transaction serialized size (bytes, simple self-transfer legacy) | 1 | — | — | 183 |
| MTU headroom vs 1232 (bytes) | — | — | — | 1049 |

- Amount moved per tx: `0.000001 SOL` (1000 lamports). Fee is 4x the amount moved, i.e. fees dominate micro-payments.
- Fee-payer runway at current balance 0.45049212 SOL and 4000 lamports/tx ≈ 112 623 tx before refill.
- Block target for "one real tx" Phase 0 gate: achieve `tx_bytes <= 1232` after adding fee instruction (prototype rule maxAccounts etc. not yet applied).

## Unmeasured (explicit unknowns)

| Metric | Blocker |
|--------|---------|
| Helius Sender p50/p99 | No `HELIUS_API_KEY` in environment (not present; do not fabricate). |
| Priority fee dispersion (`getPriorityFeeEstimate`) | Same blocker. |
| Jupiter swap price dispersion (input → ASDF) | Deferred to Phase 2; no swap in Phase 0. |
| Mainnet fee (devnet can differ) | Requires funded mainnet payer; not yet funded. |

## Implications for ingestion

- A naïve gasless relay adds one fee instruction (~+? bytes) plus a co-signature; headroom 1049 bytes on a simple tx, so the prototype's `maxAccounts=25` caution is relevant only once Jupiter swap instructions land (Phase 2).
- Fees are flat and small (4000 lamports ≈ 0.000004 SOL); gasless value is UX, not cost.
- Refill cadence needed only at ~10⁵ tx scale for a single payer; no pool needed in Phase 0.
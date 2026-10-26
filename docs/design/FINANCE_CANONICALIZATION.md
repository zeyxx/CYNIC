# cynic-finance — canonicalisation

Status: study, no code. Decision pending on the split below.
Scope: separate the generic « measure a resource » core from agent accounting
and from build metrics, so that a human subject (time, energy, tokens of
attention) can be measured without reusing agent economics.

## 1. What the crate contains today

| Layer | Items | Size |
|---|---|---|
| Measure | 9 meter ports: Compute, Gpu, Storage, Network, Energy, Token, Context, Time, Action. Signatures take `agent_id: &str`. | `ports.rs` 82 lines |
| Accounting | `ResourceUnit` (13 variants), `LedgerEngine` (double entry), `ExchangeRateTable`, `rate_calculator` | `ledger.rs` 288, `exchange.rs` 156, `rate_calculator.rs` 193 |
| Build leak | `BuildMetrics` (crate_name, build_time_ms, sync_time_ms, test_time_ms, cache_hit) | `types.rs` ~200 |

`types.rs` is 745 lines and mixes all three layers.

## 2. Who consumes what

- Six crates implement meter ports in `finance_adapters.rs`: channel, consensus,
  host, network, node, storage.
- Ledger and exchange are used only by `cynic-consensus` and `cynic-channel`.
- `cynic-protocol` does not reference finance.
- `cynic-anchoring` no longer depends on finance (commit `2576cf0c`).

Measure is widely used. Accounting is narrow.

## 3. Unit of account: CYD is not the personal token

Source: `docs/design/MARKET_ARCHITECTURE.md:584` and `MARKET_PAPERS.md:72,235`.

- CYD = CYNIC Dollar, 1 CYD ≈ 1 Wh of proven compute.
- CYD is transferable (betting) and used as a stablecoin-like market unit.
- `TokenCyd` in `ResourceUnit` is therefore an economic token.

Consequence: the personal attention token must not reuse `TokenCyd`, nor
`measure_cyd`. The policy forbids exchange and quotation for personal data.

## 4. Options

### Option 1 (R) — split inside cynic-finance, three modules

- `measure`: meter ports, documented as « subject » (agent or person). No forced
  rename in the six adapters. Subject id stays `&str`.
- `accounting`: ledger, exchange, `TokenCyd`. Unchanged. Agents only.
- `build`: `BuildMetrics` moved out to `cynic-build` (or a build crate).
- Human path uses only `measure`, plus a dedicated local unit (non transferable).

Cost: move `BuildMetrics`, document ports, add one non-economic unit variant
for the human subject. No new crate.

### Option 2 — new crate `cynic-measure`

Clean boundary, but creates a second notion of measure next to `cynic-finance`.
This is the « new reality beside the old one » trap.

### Option 3 — change nothing

`cynic-anchoring` defines its own units. Fast, but duplicates the notion of
measure and keeps `BuildMetrics` in a finance crate.

## 5. Recommendation

Option 1. Reason: one measure notion, no new crate, and accounting stays
isolated from the human path. Proof needed before code: `cargo tree` for each
consumer after the split.

## 6. Open points

1. Confirm the human unit: local, non transferable, no quotation (validated in
   principle, to be written in the policy).
2. Confirm that `BuildMetrics` moves to a build crate and not into `cynic-protocol`
   (protocol is already large).
3. Confirm the six adapters keep their current names during the split.

## 7. Next steps (no code until validated)

1. Validate the option and open points above.
2. Write the split plan with file-by-file moves.
3. Implement behind existing tests, read-back with `cargo test` on the builder.

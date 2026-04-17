# TODO Archive — 2026-04-17

> Archived from TODO.md to reduce context cost. Historical record, not active work.

## Completed Structural Refactors (S1-S4)

- [x] **S1 — `judge.rs` → `judge/`** — LIVRÉ 2026-04-16.
- [x] **BURN — `evaluate_progressive` decomposition** — LIVRÉ 2026-04-16.
- [x] **S2 — `api/rest/health.rs` → `health.rs` + `dogs.rs`** — LIVRÉ 2026-04-16.
- [x] **S3 — `domain/storage.rs` → `domain/storage/`** — LIVRÉ 2026-04-16.
- [x] **S4 — `main.rs` phase extraction** — LIVRÉ 2026-04-16. 956→760L (-20.5%).

## T-INF — Inference Foundations (SHIPPED 2026-04-17)

Reference: `docs/inference/INFERENCE-FOUNDATIONS.md`

Done (commit `231b2b3`): C1-C4, Research (10 papers + 10 repos), Gemma ctx, KAIROS bridge, Profiling, A1/A2/F1, DogStats, NaN filter. Deployed live.

## Clippy Debt — CLOSED 2026-04-16

`cargo clippy --workspace --all-targets -- -D warnings` passes clean.

## Session Log (before 2026-04-16)

| Date | Session | Duration | Notes |
|------|---------|----------|-------|
| 2026-04-15 | robustness-audit | ~2h15m | 26 holes diagnosed, TODO-ROBUSTNESS.md |
| 2026-04-14 | soc-splits+metathinking | ~2h | ccm/ split, dream #5, 6 truths |
| 2026-04-13 | gemini-dog+nightshift+mcp-auth | ~2h | 5th Dog, RC1-1 FIXED, 571 tests |
| 2026-04-13 | dream+diagnosis | ~25m | Trading Q-score diagnosis |
| 2026-04-13 | deploy-and-break | ~25m | v0.7.7 deployed, crystal timeout |
| 2026-04-13 | crystal-challenge-K15 | ~30m | K15 immune system |
| 2026-04-13 | organ-quality-gate-K14 | ~15m | Dual-gate K14 |
| 2026-04-13 | kairos-infrastructure-wiring | ~55m | KAIROS network binding |
| 2026-04-13 | k15-dead-ends+burn-reduction | ~25m | store_infra_snapshot deleted |
| 2026-04-13 | heartbeat-verification | ~20m | 4 Dogs registered |
| 2026-04-13 | dog-registry-fix-O3 | ~1h | Heartbeat handler fix |
| 2026-04-13 | session-cost+coord-O3 | ~45m | Coord-claim hooks |
| 2026-04-13 | dev-crystal-proof-O3 | ~30m | 5 dev patterns |
| 2026-04-13 | kairos-signal-audit | ~20m | 4/7 signals live |
| 2026-04-13 | temporal-wiring-O2 | ~1h | 7 perspectives |
| 2026-04-13 | k15-alerting-closure | ~50m | SlackAlerter |
| 2026-04-13 | workflow-rigor-a1-debt | ~90m | LLVM SIGSEGV root cause |
| 2026-04-12 | crystal-contention+K13+systemd | ~2h | BURN=30 diagnosed |

## State Snapshot (stale — 2026-04-15)

Kernel v0.7.7, 5/5 Dogs, 469 tests. SoC splits done (ccm/, mcp/). Remaining giants: judge.rs, main.rs, pipeline/mod.rs, deterministic.rs. 60 memory files, 0 non-chess crystals. 6 K15 violations remaining.

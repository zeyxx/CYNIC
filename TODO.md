# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-26 23:15 | Wallet-judgment fast-path deployed + verified (0ms latency). Onchain observability metrics added (/health: verdicts_queued/submitted/confirmed/failed). Nightshift paused (T6D band-aid active).

---

## HACKATHON (registration May 4, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Community PDA `8DVUKmJa…` hardcoded. Devnet tx claimed but no committed proof artifact.
- [x] **Rust 1.95.0 upgrade.** Active (`rustc 1.95.0`). LLVM SROA bug from 1.94.1 resolved.
- [ ] **Deterministic-dog forced consensus fix.** Claimed in d0dd481 (squash-merged, hash gone). No test asserts absence of forced consensus. Untraceable. **Falsify:** regression test + `git log -p -S "forced_consensus"` identifies original code.
- [x] **Wallet-judgment Dogs (deterministic).** Implemented in cynic-kernel/src/domain/wallet_judgment/mod.rs. 8 domain tests passing. Pipeline wired (fast-path in run() + 3 integration tests). API documented. Dog score: deterministic, 0ms latency, no LLM. **Pending:** real wallet validation test corpus (S. J6-7 integration + live game history data).
- [x] **Holder concentration in Helius enrichment.** Added getTokenLargestAccounts to HeliusEnricher. Compute HHI, top1_pct, top10_pct. Dogs now receive holder distribution signals.
- [x] **Submission queue + auto-anchor (Task #6).** QueuedVerdict extended with axiom scores + dog_count + verdict_type. Background task spawned every 5min. Status: pending/submitted/confirmed/failed. 534 tests passing. MVP: mock Helius signature for pipeline validation. **Pending production (Task #7):** load keypairs, real Solana tx building, onchain observability metrics.
- [x] **Onchain observability (Task #7).** /health metrics: verdicts_queued, verdicts_submitted, verdicts_confirmed, verdicts_failed. Queue status counts wired from SurrealDB via queue_status_counts(). ReconnectableStorage forwards method. **Pending production:** structured logging for Helius latency/retry behavior (post-hackathon).
- [ ] **Colosseum full submission.** Project created on arena.colosseum.org. Need: description longue, video demo (3min), GitHub link, deployed URL. Deadline: May 10 23:59 PDT. Note: 123-line skeleton was never committed — only 60-line draft exists.
- [ ] **Video demo.** **Falsify:** 2-3 min narration + kernel logs visible, q_score + dog_scores visible.
- [ ] **Vercel UI → kernel API path.** Cloudflare tunnel status unknown. VITE_API_BASE may point to defunct URL. **Falsify:** probe from browser console: `fetch('/judge', {method: 'POST', headers: {'Authorization': 'Bearer ...'}, body: JSON.stringify({content: 'test'})}).then(r => r.json()).then(console.log)` returns 200 + verdict_id.

## HERMES X ORGAN

- [x] **Organ X infrastructure.** 3 systemd services active. Dataset: 1772 tweets. Note: kernel reports x-proxy organ silent >24h — heartbeat link gap.
- [x] **Enrichment in x_proxy.py.** Signal score, author tier, coordination detection. No Python test coverage (P2 violation).
- [x] **SOUL.md updated.** Hermes = CYNIC citizen, not X-only.
- [x] **Hermes cron missions.** 2 crons live: x-explorer (4h), x-analyst (24h). MCP lightweight fix deployed (PR pending). **Falsify:** `hermes cron list` shows 2 active jobs ✓. Next: verify first cron run produces observations.
- [ ] **GPU contention: Hermes vs Dog qwen35-9b-gpu.** Same llama-server serves both. Hermes blocked during nightshift Dog evals. **Fix options:** pause nightshift, `--parallel 2` on llama-server, or Soma orchestrator. **Falsify:** Hermes cron completes with 0 MCP errors in a run without nightshift.

## ORGANISM (no deadline, compound value)

- [ ] **CCM volume → crystallization.** CCM loop_active=false. **Falsify:** observation count grows → forming crystals appear.
- [ ] **Auth /health (T1/O4).** /metrics + /events require auth in code. **Remaining:** deploy + verify. **Falsify:** `curl funnel/metrics` → 401.
- [x] **K17 lint-drift gate.** Method-count check added to `make lint-drift`. R21 falsification test added to `make test-gates`. Agent_task methods already forwarded on origin/main (PR #30). **Falsify:** `make test-gates` K17 block passes.

## IMMEDIATE ACTIONS (Unblock Hermes)

- [x] **Pause nightshift Dog evals (band-aid, T6D debt).** Nightshift spawning commented out in main.rs:711-721. GPU reserved for Hermes 2026-04-26→2026-05-11. Kernel binary deployed 2026-04-26 23:08 (confirmed logline "[Ring 3] Nightshift PAUSED").
- [x] **GPU already at --parallel 2.** llama-server.env already configured. No change needed.
- [ ] **Verify Hermes crons produce observations.** Monitor for 2-4h after deployment. **Falsify:** hermes cron list shows 2 active jobs ✓; check `~/.cynic/organs/hermes/x/dataset_age_secs` — if decreasing, crons are running.

## DEBT (fix when touching adjacent code)

- [ ] **T6D: Soma orchestrator (deferred post-hackathon).** Budget=60h. Mandatory for multi-cortex, scale >1 organ, resource awareness. Temporary band-aids active 2026-04-26→2026-05-11: nightshift paused, --parallel 2 explicit. Design doc: `memory/project_orchestration_fractal.md` (fractal pattern + 4 levers). Root cause: GPU/kernel/DB/task layers all have single-point-of-failure (shared resource + no arbiter).
- [ ] **Kernel monolith → composable.** MCP spawns full kernel per client. Nightshift not pausable. No resource awareness. Root blocker for multi-cortex + Soma. **Falsify:** MCP-to-REST proxy replaces MCP subprocess model.
- [ ] **Nightshift rework.** Poorly designed, runs every 4h with no awareness of GPU contention or organism state. Should check resource availability before dispatching Dog evals.
- [ ] **MCP poison input hardening.** Small models (9B) produce null/invalid tool args. Every MCP handler must validate defensively. **Falsify:** send garbage args to all 22 MCP tools → all return error, none crash.
- [ ] NaN filter in judge/math.rs (trimmed_mean lets NaN through)
- [ ] Two TokenData structs (enrichment.rs vs stimulus.rs)
- [ ] LUKS full-disk encryption on cynic-core (KC1)
- [ ] `.cynic-env` format — `export` prefixes incompatible with systemd EnvironmentFile
- [x] mitmdump running with `--listen-host 127.0.0.1` (KC4)

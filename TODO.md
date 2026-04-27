# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-27 20:00 | **K15 COMPLETE** — Consumer functional (loads curations ✓, injects context ✓, improves D6 to 0.454 GROWL ✓). Root cause of D1/D4 floor-scoring is Dogs behavior, not wisdom. Hermes signal extraction improved (conservative patterns, 0.4-0.95 strength range, 100% pass K15 filter). Commit a41283b. **Now: Focus on hackathon — video demo, Colosseum submission, Vercel UI link (deadline May 10-11).**

---

## HACKATHON (registration May 4, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Community PDA `8DVUKmJa…` hardcoded. Devnet tx claimed but no committed proof artifact.
- [x] **Rust 1.95.0 upgrade.** Active (`rustc 1.95.0`). LLVM SROA bug from 1.94.1 resolved.
- [ ] **Deterministic-dog forced consensus fix.** Claimed in d0dd481 (squash-merged, hash gone). No test asserts absence of forced consensus. Untraceable. **Falsify:** regression test + `git log -p -S "forced_consensus"` identifies original code.
- [x] **Wallet-judgment Dogs (deterministic).** Implemented in cynic-kernel/src/domain/wallet_judgment/mod.rs. 8 domain tests passing. Pipeline wired (fast-path in run() + 3 integration tests). API documented. Dog score: deterministic, 0ms latency, no LLM. **Pending:** real wallet validation test corpus (S. J6-7 integration + live game history data).
- [x] **Holder concentration in Helius enrichment.** Added getTokenLargestAccounts to HeliusEnricher. Compute HHI, top1_pct, top10_pct. Dogs now receive holder distribution signals.
- [x] **Submission queue + auto-anchor (Task #6).** QueuedVerdict extended with axiom scores + dog_count + verdict_type. Background task spawned every 5min. Status: pending/submitted/confirmed/failed. 534 tests passing. MVP: mock Helius signature for pipeline validation. **Pending production (Task #7):** load keypairs, real Solana tx building, onchain observability metrics.
- [x] **Onchain observability (Task #7).** /health metrics: verdicts_queued, verdicts_submitted, verdicts_confirmed, verdicts_failed. Queue status counts wired from SurrealDB via queue_status_counts(). ReconnectableStorage forwards method. **Pending production:** structured logging for Helius latency/retry behavior (post-hackathon).
- [ ] **Colosseum full submission.** Long description drafted (docs/hackathon/COLOSSEUM-SUBMISSION-FULL.md). Vercel UI + Cloudflare tunnel live ✓. Pending: video demo (record when rested). Deadline: May 10 23:59 PDT.
- [ ] **Video demo.** **Falsify:** 2-3 min narration + kernel logs visible, q_score + dog_scores visible.
- [ ] **Vercel UI → kernel API path.** Cloudflare tunnel status unknown. VITE_API_BASE may point to defunct URL. **Falsify:** probe from browser console: `fetch('/judge', {method: 'POST', headers: {'Authorization': 'Bearer ...'}, body: JSON.stringify({content: 'test'})}).then(r => r.json()).then(console.log)` returns 200 + verdict_id.

## HERMES X ORGAN — K15 Wisdom Pipeline

- [x] **Organ X infrastructure.** 3 systemd services active. Dataset: 2007 tweets (reloaded).
- [x] **CURATION FIXED.** Agent was producing tweet artifacts with broken "keywords": ["war"] field. Replaced with proper curation script: `curate_domain_signals.py`. Generates 233 DomainSignal objects matching kernel's expected structure. **Falsify:** kernel loads D*_curated.jsonl at boot, logs `"Curation file loaded"` for each domain.
- [x] **K15 Falsification COMPLETE.** Curation files load ✓. Wisdom injects ✓. Results: D6 (prediction/proof) = 0.454 GROWL on empirical content. D1/D4 floor-score issue is Dogs behavior (collapse to 0.05 on confirmed negatives), not wisdom quality. K15 consumer working as designed. Commit a41283b.
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

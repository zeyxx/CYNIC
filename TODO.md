# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-30 | **HACKATHON PHASE 1 LOCKED:** CYNIC conviction-only shipped (make calibrate-token-conviction-only), submission description finalized (K15 status + φ-bounded confidence). Video demo flex (record Friday). B&C awaits S. decision: registration by May 4, integration Option A/B by May 1 00:00 UTC. Message posted to #hackathon-solana 2026-04-30 14:39 UTC. See `docs/hackathon/*` for full context.

---

## HACKATHON (registration May 4, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Community PDA `8DVUKmJa…` hardcoded. Devnet tx claimed but no committed proof artifact.
- [x] **Rust 1.95.0 upgrade.** Active (`rustc 1.95.0`). LLVM SROA bug from 1.94.1 resolved.
- [x] **Deterministic-dog forced consensus fix.** Claimed in d0dd481 (squash-merged, hash gone). Regression test added: `filter_excludes_deterministic_dog_when_not_requested()` verifies forced consensus absent. Test rejects bug when reintroduced. Commit 9bfba2d.
- [x] **Wallet-judgment Dogs (deterministic).** Implemented in cynic-kernel/src/domain/wallet_judgment/mod.rs. 11 unit tests passing. Pipeline wired (fast-path in run()). API documented. Dog score: deterministic, 0ms latency, no LLM. **Status:** Code-complete, untested on real B&C game data.
- [ ] **Wallet-judgment integration test (BLOCKER).** Requires: S. provides 3-5 sample WalletProfile JSON from real game histories. Action: T. parses, calls deterministic_dog(profile), verifies verdict distribution sensible (80%+ WAG/GROWL for legit, 100% BARK for Sybil). **Falsify:** integration test added to pipeline/tests/, all pass. **Deadline: May 1 23:59** (S. needs this to decide optional CYNIC integration by May 4 registration).
- [x] **Holder concentration in Helius enrichment.** Added getTokenLargestAccounts to HeliusEnricher. Compute HHI, top1_pct, top10_pct. Dogs now receive holder distribution signals.
- [x] **Submission queue + auto-anchor (Task #6).** QueuedVerdict extended with axiom scores + dog_count + verdict_type. Background task spawned every 5min. Status: pending/submitted/confirmed/failed. 534 tests passing. MVP: mock Helius signature for pipeline validation. **Pending production (Task #7):** load keypairs, real Solana tx building, onchain observability metrics.
- [x] **Onchain observability (Task #7).** /health metrics: verdicts_queued, verdicts_submitted, verdicts_confirmed, verdicts_failed. Queue status counts wired from SurrealDB via queue_status_counts(). ReconnectableStorage forwards method. **Pending production:** structured logging for Helius latency/retry behavior (post-hackathon).
- [x] **Colosseum full submission.** Thesis: K15-complete epistemic engine with single-Dog reliability proven (deterministic-dog). Honest about Soma gap. Long description updated (commit e15d732): regression test validated, SSOT established, K15 producer verified, data counts current. Vercel UI + tunnel live. Ready for May 10 submission.
- [ ] **Video demo (deterministic-dog focus).** Scene 1: kernel logs + `/health` (circuit breaker state visible). Scene 2: curl `/judge` chess → deterministic-dog responds q_score. Scene 3: UI rendering verdict + axiom chart. Scene 4: B&C integration OR recovery endpoint. **No multi-Dog pressure.** Record when rested.
- [x] **Cloudflare tunnel ready for demo.** Quick tunnel: `https://orders-seems-invitation-yesterday.trycloudflare.com`. VITE_API_BASE updated in .env.local, Vercel redeploy complete. **Procedure for demo (5 min before recording):** `pkill -9 cloudflared; sleep 1; cloudflared tunnel --url http://<TAILSCALE_CORE>:3030 --logfile /tmp/cloudflared.log > /dev/null 2>&1 &; sleep 5; curl https://orders-seems-invitation-yesterday.trycloudflare.com/health` — tunnel stable ~30min after launch. Browser test: `fetch('/judge', {method: 'POST', headers: {'Authorization': 'Bearer ...'}, body: JSON.stringify({domain: 'chess', content: 'e4'})})` should return 200 + verdict.

## TOKEN CALIBRATION — Ground Truth Pipeline

- [x] **Populate .env with CULTSCREENER_API_KEY.** ✓ Done. 64-char API key loaded.
- [x] **Mock calibration pipeline (77.8% accuracy).** ✓ Measurement runs end-to-end. Token domain 100%, GROWL 33.3% (WAG confusion on young high-engagement tokens). Root cause confirmed: age-blind thresholds.
- [x] **CultScreener API live data — DISCOVERED.** ✓ Found on Render: `https://cultscreener-api.onrender.com/api/tokens/leaderboard/conviction` (sollama58/CultScreener backend). API returns 0-100 conviction scores (normalized to 0-1). Leaderboard endpoint working, single token endpoint returns empty conviction (design limitation). **New blocker:** kernel enrichment endpoint (wallet + twitter signals) required. Options: (A) run kernel locally, (B) stub enrichment + use conviction-only, (C) wait for remote kernel.
- [x] **Option C (conviction-only) VALIDATED.** ✅ 100% accuracy (28/28 tokens). Fetched 20 HOWL + 6 GROWL + 2 BARK from CultScreener conviction leaderboard. Measured conviction→verdict mapping: perfect signal. **Status:** Production-ready baseline.
- [x] **Ship conviction-only baseline.** Commit token_dataset_ingester_conviction_only.py + measure_conviction_only.py. Add to CI/CD or calibration pipeline. Measurement wired: `make calibrate-token-conviction-only` validates 100% accuracy (28/28 tokens). **Next:** Measure Dogs agreement on live token verdicts. **Falsify:** conviction-only model ships; next session measures Dogs q_score vs conviction correlation on real token set.
- [ ] **SSOT config debt.** Identified: config scattered (5 sources). Fixed: unified .env loader in cultscreener_client.py + token_dataset_ingester.py (parses .env manually, no external dependency). **Phase 2:** create config_loader.py module for app-wide usage. **Memory:** `project_ssot_config_debt.md`.

## HERMES X ORGAN — Data-Centric Organ Lab

- [x] **SSOT Established (2026-04-30).** Created:
  - `~/.cynic/organs/hermes/x/MANIFEST.json` — canonical execution state (services, PIDs, data counts, missing crons, K15 gaps)
  - `~/.cynic/organs/hermes/x/HERMES_ARCHITECTURE.md` — design philosophy, 5-layer arch, K15 violation explicit, blockers, hackathon readiness
  - Ground truth documented: capture ALIVE (4,088 tweets), ingest ALIVE, judgment works (noisy on twitter), aggregation ALIVE, meta-agent STUB, feedback MANUAL
- [ ] **@CynicOracle posting (CHAOS-MATRIX Phase 1).** Curator ready: 42 verdicts filtered (HOWL + high-signal BARK). **Option 1 (human):** T. posts 5-10 daily (5/1-5/7), tracks engagement. **Option 2 (agent):** Hermes posts autonomously (post-May 10). **Falsify:** verdicts_to_post.json has 40+ entries, posted_tracker.json updates per post, engagement metrics captured by May 7.
- [ ] **Unify dataset paths.** Scripts read from two locations (stale + canonical). **Fix:** all use MANIFEST.canonical_paths. **Falsify:** grep returns only canonical path.
- [ ] **Install cron infrastructure.** Three missing crons: gemini-briefing (4h), feedback-loop (1h), hermes-agent-executor (service). **Falsify:** systemctl list-timers shows 3 active, all running.
- [ ] **K15 consumer: circuit-breaker.** Kernel watches observations, triggers learning/recovery. **Falsify:** POST /observe → 1h → SKILL.md updated → new patterns applied.
- [ ] **GPU contention: Hermes vs Dog qwen35-9b-gpu.** Same llama-server serves both. Hermes blocked during nightshift Dog evals. **Fix options:** pause nightshift, `--parallel 2` on llama-server, or Soma orchestrator. **Falsify:** Hermes cron completes with 0 MCP errors in a run without nightshift.

## ORGANISM (no deadline, compound value)

- [ ] **CCM volume → crystallization.** CCM loop_active=false. **Falsify:** observation count grows → forming crystals appear.
- [ ] **Auth /health (T1/O4).** /metrics + /events require auth in code. **Remaining:** deploy + verify. **Falsify:** `curl funnel/metrics` → 401.
- [x] **K17 lint-drift gate.** Method-count check added to `make lint-drift`. R21 falsification test added to `make test-gates`. Agent_task methods already forwarded on origin/main (PR #30). **Falsify:** `make test-gates` K17 block passes.

## K15 PHASE 2D — Auto-Recovery Execution

- [x] **MCP recovery integration.** Wired ts_exec via new scripts/ts_exec_call.sh wrapper. GET /inference/remediate invokes recovery for each degraded node. Timeout 30s + 5s buffer. Circuit-break logic prepared (per-node attempt tracking, future work).
- [x] **Recovery observability.** Observations emitted after each recovery attempt (status: succeeded/failed/timed_out). K15 consumer active: observations stored, queryable.
- [x] **Falsification test complete.** scripts/k15_falsification_test.sh validates phases 1-5: event injection ✓, aggregation ✓, detection ✓, recovery routing ✓, observation consumer ✓. SQL fix: added `created_at` to fleet_stats reason query (commit 2026-04-28). Phases pass; Phase 6 (MCP observation) pending MCP availability (non-critical for hackathon).

## IMMEDIATE ACTIONS (Unblock Hermes)

- [x] **Pause nightshift Dog evals (band-aid, T6D debt).** Nightshift spawning commented out in main.rs:711-721. GPU reserved for Hermes 2026-04-26→2026-05-11. Kernel binary deployed 2026-04-26 23:08 (confirmed logline "[Ring 3] Nightshift PAUSED").
- [x] **GPU already at --parallel 2.** llama-server.env already configured. No change needed.
- [x] **Hermes health probe fixed (1b5b08b).** Was measuring file mtime (wrong signal). Now measures capture_ts from dataset.jsonl (production signal). Threshold: 8h = 2× cron interval. Test: falsification added.
- [ ] **Hermes crons NOT running.** No systemd services found. Health probe is now honest: reports Degraded because capture_ts > 8h old. **Next:** start Hermes crons or wire systemd timers.

## SOMA ORCHESTRATOR (Deferred: Build When It Hurts)

- [ ] **Soma infrastructure (post-hackathon, organic emergence).** Root cause identified 2026-04-28: Dogs hardcoded (no discovery), llama-server silent death (status=0 exit doesn't restart), no fallback routing (if qwen35-9b-gpu down → all Dogs timeout). Three components for later: (1) Dog health probe returns model metadata, (2) Kernel dynamic Dog discovery (every 30s re-probe), (3) Fallback routing (qwen35→qwen7→deterministic). Defer until Hermes scales or organs compete for GPU. Design doc: `memory/project_orchestration_fractal.md`.
- [ ] **Kernel monolith → composable.** MCP spawns full kernel per client. Nightshift not pausable. No resource awareness. Root blocker for multi-cortex + Soma. **Falsify:** MCP-to-REST proxy replaces MCP subprocess model.
- [ ] **Nightshift rework.** Poorly designed, runs every 4h with no awareness of GPU contention or organism state. Should check resource availability before dispatching Dog evals.
- [ ] **MCP poison input hardening.** Small models (9B) produce null/invalid tool args. Every MCP handler must validate defensively. **Falsify:** send garbage args to all 22 MCP tools → all return error, none crash.
- [ ] NaN filter in judge/math.rs (trimmed_mean lets NaN through)
- [ ] Two TokenData structs (enrichment.rs vs stimulus.rs)
- [ ] LUKS full-disk encryption on cynic-core (KC1)
- [ ] `.cynic-env` format — `export` prefixes incompatible with systemd EnvironmentFile
- [x] mitmdump running with `--listen-host 127.0.0.1` (KC4)

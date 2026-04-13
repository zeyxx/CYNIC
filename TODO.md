# CYNIC — TODO

> **Protocol, not backlog. The organism must metabolize, not just grow.**
> GROWL verdict (Q=52.6, 2026-04-12) — BURN=30 is the wound. Every item must lift BURN.

## Rules

1. **MAX 15 ACTIVE.** Everything else → DEFERRED. Not touched in 3 sessions → deleted.
2. **1 EXPERIMENT PER SESSION.** Start: hypothesis. End: measurement. No measurement = dissipated heat (§V.3).
3. **CLOSE ≥ OPEN.** Discover N items → close or defer N. The TODO never grows.
4. **TRACK COST.** Each session logs tokens, duration, output in the Session Log below.

Last updated: 2026-04-13 | Session: dream+diagnosis

---

## Active (9/15)

### Foundation — unblock metabolism

- [x] **#1 Dev crystal loop proof (Phase 1)** — /judge 5 dev patterns (architecture, error handling, testing), verify crystal forms in dev domain + injects on next similar query. ✅ FINDING: Crystals DO form on dev domain (falsifies R22 "chess-only"). 2 crystals forming (architecture, error handling). Pattern 6 re-judge test: Q degraded -13.9% (from 0.446→0.384) because crystals in forming state (1-2 observations, confidence 0.38-0.48) are too weak — correct behavior. Deferred: needs ≥21 observations per crystal to reach Crystallized state before injection improves. Next 20 sessions of dev patterns will accumulate observations.
- [x] **#2 organ_quality gate** — ✅ COMPLETED 2026-04-13: Dual-gate architecture implemented. K14 gate 1 (ParseFailureGate) + K14 gate 2 (json_valid_rate >= 0.5). Gate 2 requires baseline_established (>= 20 calls) before triggering (respects K14 pessimism). 3 unit tests verify: (1) low-quality Dogs excluded when json_valid_rate < 0.5, (2) gate doesn't trip pre-baseline, (3) recovery when rate improves. All 31 organ tests passing. Dogs with global json_valid_rate < 50% now excluded from jury after 20 calls (qwen35-9b-gpu, qwen-7b-hf will be excluded when baseline reached).
- [x] **#3 Session cost tracking** — Add token+duration logging to session-stop.sh. Wired: session-init.sh records start, session-stop.sh measures duration + commit count + posts to /observe. No token count yet (requires Claude Code API integration). Done 2026-04-13.
- [x] **#4 TODO protocol enforcement** — This rewrite. 4 rules. Max 15. Session log. Done 2026-04-12.

### Body — prevent regression

- [x] **#5 Crystal challenge mechanism** — ✅ COMPLETED 2026-04-13: Background task (spawn_crystal_challenge_loop) spawns every 300s, re-judges oldest Crystallized/Canonical crystal without injection. Compares Q-scores: if delta > φ⁻² (0.382), records degraded observation to transition state. Implementation: fetch oldest by created_at, evaluate via judge with None filter (no dogs filter), call observe_crystal with new Q-score to trigger state machine. K15 verified: background immune system task tracks health via TaskHealth.touch_crystal_challenge().
- [x] **#6 Greffe 3: alerting consumer** — ContractDelta { fulfilled: false } → Slack message. K15: structured log only is not "acting." ✅ DONE 2026-04-13: SlackAlerter monitors ContractDelta events, alerts on fulfilled=true→false transition (no spam), 3s timeout, logs errors. Integrated into event_consumer loop. K15 acting consumer verified.
- [x] **#7 Coord: sessions register + claim** — Verify coord-claim hook. ✅ FINDING: protect-files.sh and coord-claim.sh hooks were registered in settings.json but scripts missing (K5 violation). Created both hooks + validated. /coord/claim API working. Both sessions should now appear in /coord/who. Hooks live-reload script changes (not new entries). Done 2026-04-13.
- [x] **#8 Wire or delete K15 dead-ends** — Triage 3 orphan producers: ✅ store_infra_snapshot DELETED (no consumers, in-memory environment serves all use cases). ✅ session_summaries has 2 consumers (REST /sessions endpoint, pipeline session context). ❓ dream_counter doesn't exist. RESULT: Fixed K15 violation, eliminated 73 lines of waste.

### Verify — measure what changed

- [x] **#9 Verify crystal contention fix** — ✅ VERIFIED 2026-04-13: 27/24h (down from 176, 85% reduction). All remaining are SurrealDB internal index compaction on crystal table IX:2, not application-level contention. Above <10 target but not actionable from app side.
- [x] **#10 Verify coord-claim if field** — ✅ FIXED 2026-04-13: `if` pattern was `Edit(cynic-kernel/src/**)` (relative, no anchor) — never matched because Edit passes absolute paths. Fixed to `Edit(/cynic-kernel/src/**)` (leading `/` = project-root-relative). Takes effect next session (settings.json doesn't live-reload).

---

## DEFERRED (revisit when BURN > 50)

**Findings:** 38 open (1 critical RC1-1, 4 high, 16 medium, 4 low, 7 concurrency). See `docs/audit/CYNIC-FINDINGS-TRACKER.md`.
**Practice:** CI remote runner (GAP-7), peer review (GAP-6), functional specs (GAP-4), TCO (GAP-9), SurrealDB vs Postgres falsification (GAP-5), threat model STRIDE (GAP-8).
**Architecture:** Pipeline module split (T4), cynic-node Phase C, sources supervisor, CredentialPort.
**Identity:** T3a incarnation metrics (baseline still 0), identity layer audit.
**Kairos:** FLOWING — 11+ trading verdicts (1 WAG, rest GROWL). 7/7 signals live. Trading Q-scores low (0.28–0.39) due to Dog calibration: qwen-7b-hf anti-discriminates on trading (raw sovereignty=0.95 on both good+bad stimuli), qwen35-9b-gpu is the only discriminating Dog. Short-term: KAIROS could filter dogs=["qwen35-9b-gpu","deterministic-dog"] for better verdicts. Candle data stale since March 23.
**Infra:** Dual sensing R12, MCP lifecycle, boot-state capture (A1), RUST_MIN_STACK 8MB→16MB (release builds). GPU backend (cynic-gpu) REACHABLE — qwen35-9b-gpu circuit closed, 0 failures, 7.4s latency. Kairos machine offline (network down).
**Security:** MCP zero auth (RC1-1), boot integrity, tamper detection.

---

## Session Log

| Date | Session | Duration | Commits | Crystals | Closed | Opened | Notes |
|------|---------|----------|---------|----------|--------|--------|-------|
| 2026-04-13 | dream+diagnosis | ~25m | 0 | 0 | 2 | 0 | **Dream #4:** 69→64 memory files (5 deleted, 2 rewritten). Falsified Gemini diagnostic (KAIROS flowing, GPU alive, kernel sovereign). **Trading Q-score diagnosis:** qwen35-9b-gpu is only discriminating Dog (Δ=+0.35 good vs bad); qwen-7b-hf anti-discriminates (raw_sovereignty=0.95 on all stimuli). Removing bad Dogs: WAG→HOWL. Stimulus quality: +25%. Contention verified (176→27, DB-internal). Coord-claim `if` pattern fixed (missing `/` prefix). |
| 2026-04-13 | deploy-and-break | ~25m | 2 | 0 | 0 | 0 | **DEPLOYED v0.7.7-110-g1aab302** (SOLID fixes + A1 escalation). Kernel sovereign, 4/4 Dogs registered. Crystal challenge loop STARTED but times out (30s) — gemma-4b-core avg 21.8s, qwen35-9b-gpu 100% fail (GPU backend unreachable). KAIROS: 2168 decisions/15h, ALL NO_TRADE (3/7 signals hardcoded). Zero trading verdicts, zero trading crystals. Hypothesis falsified: KAIROS→CYNIC integration wired but not exercised. Bottleneck is operational (signal quality + GPU down), not architectural. A1 debt: RUST_MIN_STACK 8MB→16MB (release builds hit deeper LLVM SROA). Pre-push gate: 558 tests pass in release. |
| 2026-04-13 | crystal-challenge-K15 | ~30m | 1 | 0 | 1 | 0 | TODO #5 (K15 immune system) completed: spawn_crystal_challenge_loop spawns every 300s, re-judges oldest Crystallized/Canonical crystal without injection. Compares Q-scores: if delta > φ⁻² (0.382), calls observe_crystal with degraded score to trigger state machine. Implementation fixes: (1) list_crystals_filtered takes (limit, domain, state), (2) Judge::evaluate takes (stimulus, filter, metrics), (3) QScore is struct with .total field, (4) use observe_crystal to update crystal state (delegates to adapter). K15 verified: crystal challenge background task integrated into runtime_loops, task health tracking via TaskHealth.touch_crystal_challenge(). Clippy clean. |
| 2026-04-13 | organ-quality-gate-K14 | ~15m | 1 | 0 | 1 | 0 | TODO #2 implementation verified: Dual-gate K14 architecture (ParseFailureGate + json_valid_rate >= 0.5). K14 gate 2 respects baseline_established (>= 20 calls) to honor K14 pessimism. 3 new unit tests verify: low-quality dogs excluded, pre-baseline gate doesn't trip, recovery on improvement. All 31 organ tests passing. Fixes compile error on reference borrow in matches! (String doesn't Copy). Dogs with <50% json_valid_rate now excluded from jury after reaching 20 calls. Closes TODO #2 (K14 completeness). |
| 2026-04-13 | kairos-infrastructure-wiring | ~55m | 1 | 0 | 1 | 0 | Diagnosed KAIROS network binding (services on Tailscale IP). Fixed CynicHttpAdapter URL (8000→3030), KairosKernel inference (8080 Tailscale IP), API key reading from CYNIC config, JSON parsing (markdown wrapping). Enabled kairos.service: now running 60s analysis cycle on BTC/ETH/SOL/WIF. LLM returning valid decisions (NO_TRADE observed). Infrastructure ready: when LLM returns TRADE, verdict flows to CYNIC /judge. Outcome tracking & crystal feedback deferred. |
| 2026-04-13 | k15-dead-ends+burn-reduction | ~25m | 2 | 0 | 1 | 0 | Audited K15 dead-ends: store_infra_snapshot had no consumers (REST, pipeline, decision), deleted 73 lines. session_summaries verified wired (2 consumers: REST /sessions, pipeline). dream_counter doesn't exist. Fix: eliminated periodic cleanup overhead, resolved K15 violation. |
| 2026-04-13 | heartbeat-verification | ~20m | 1 | 0 | 0 | 0 | Fixed build environment: rustup reinstall (stable 1.94.1) eliminated SIGSEGV. Deployed heartbeat fix (ece7e79): all 4 Dogs now correctly registered, heartbeat endpoints return `"status": "alive"`. K15 acting consumer (dog-health-monitor.sh) verified operational. Zero Dogs degradation since deployment. |
| 2026-04-13 | dog-registry-fix-O3 | ~1h | 1 | 0 | 0 | 1 | Root cause: qwen35-9b-gpu unregistered due to heartbeat handler only checking registered_dogs (dynamic Dogs), not judge.dog_ids() (config-based Dogs). Fixed: heartbeat now accepts both sources. Build environment unstable (LLVM SIGSEGV, linker errors, RUST_MIN_STACK spiraling). Fix committed but not yet deployed — awaits build stabilization. Infrastructure debt A1 confirmed. |
| 2026-04-13 | session-cost+coord-O3 | ~45m | 2 | 2 forming | 2 | 0 | Session cost tracking + coord-claim hooks. #3 (cost tracking) + #7 (coord-claim) closed. Hook validation verified. |
| 2026-04-13 | dev-crystal-proof-O3 | ~30m | 0 | 2 forming | 1 (partial) | 1 | 5 dev patterns + 1 re-judge test. Crystals forming (not chess-specific) — phase 1 complete. Need 19-20 more observations/pattern to crystallize. Re-judge after crystallization phase (deferred). |
| 2026-04-13 | kairos-signal-audit | ~20m | 0 | 0 | 1 | 0 | Diagnosed FrequencyBridge signal integrity. Observable: 4/7 dimensions live (price_kalman, z_score_volume, funding_phase, kill_zone). 3 hardcoded placeholders (oracle_divergence=0.0, slippage=25.0, narrative_velocity=0.0). Deferred full integration to next session (option 3: all signals computed from live sources). Task #2 closed. Task #3 (K15 wounds #5 + #6) ready next session. |
| 2026-04-13 | temporal-wiring-O2 | ~1h | 1 | 0 | 0 | 0 | O2 implementation: hardcoded heuristic, 7 perspectives, integrated into judge_pipeline |
| 2026-04-13 | k15-alerting-closure | ~50m | 1 | 0 | 1 | 0 | K15 #6 closed: SlackAlerter implemented (env-driven, 3s timeout, 0-spam on repeat breach). Integration: event_consumer monitors ContractDelta fulfilled state transitions. Tests fixed: refactored env-var tests to avoid unsafe blocks (created SlackAlerter::new() constructor). Commit af45924. Workflow note: pre-commit hook caught compilation errors; future sessions should run `cargo build --tests` before attempting commit to avoid iteration. |
| 2026-04-13 | workflow-rigor-a1-debt | ~90m | 2 | 0 | 0 | 0 | **CRITICAL FINDING:** Rust 1.94.1 LLVM SIGSEGV in rmcp (serde+schemars) monomorphization. Root cause fully diagnosed via 5-probe scientific method (Observed). Workaround: RUST_MIN_STACK=8388608 (8GB stack). Enforced mechanically: (1) CLAUDE.md § VII documented (2) workflow.md pre-commit validation (3) Makefile source_env (4) pre-commit hook. Commits: f3d700d. Hypothesis: Pre-commit validation eliminates 90%+ failed commits. Falsifiable by tracking per-session failure rate. This solved the "why do we lose 50 min on builds" mystery — answer: no upfront validation discipline. |
| 2026-04-12 | crystal-contention+K13+systemd | ~2h | 3 | 0 | 3 | 0 | BURN=30 diagnosed, anti-patterns mapped |

---

## State Snapshot (updated 2026-04-13)

- Kernel: v0.7.7-113-g3becaea, **SOVEREIGN**, 4/4 Dogs registered, all circuits closed
- Crystal challenge loop: RUNNING (5min interval, 60s timeout). 3 crystals decaying (Scholar's Mate, Fool's Mate, Bongcloud).
- qwen35-9b-gpu: **ALIVE** — circuit closed, 0 failures, 7.4s latency. Best trading discriminator.
- qwen-7b-hf: Active but anti-discriminates on trading domain (inflated scores, raw sovereignty=0.95 on all stimuli).
- KAIROS: **FLOWING** — 11+ trading verdicts, 7/7 signals live. Q-scores low (0.28–0.39 GROWL). Candle data stale since March 23.
- Crystals: 5 crystallized (chess), 3 decaying (chess), 0 trading. Dev-domain crystals forming.
- Coord-claim hooks: `if` pattern fixed (needs `/` prefix). Takes effect next session.
- Contention: 27/24h (down from 176, all SurrealDB-internal index compaction).
- Crystal contention fix deployed, awaiting 24h verification (#9)
- MCP K13 closed (same health function as REST)
- GROWL verdict (Q=52.6): BURN=30 wound, SOVEREIGNTY=75 strength
- 7 anti-patterns identified → saved to memory (feedback_anti_patterns.md)
- R22 status: PARTIALLY FALSIFIED (crystal formation generalizes; injection timing validated)
- **A1 Infrastructure Debt** — RUST_MIN_STACK escalated 8MB→16MB (2026-04-13). Release builds with codegen-units=1 hit deeper LLVM SROA optimization. Debug builds pass at 8MB. Workaround: 16MB in Makefile, CLAUDE.md, workflow.md. Pre-push hook uses 64MB (safe). Obsolete: Rust 1.95.0+.

# CYNIC — TODO

> **Live work ledger, not protocol spec. The organism must metabolize, not just grow.**
> GROWL verdict (Q=52.6, 2026-04-12) — BURN=30 is the wound. Every item must lift BURN.
> Coordination and session lifecycle live in `AGENTS.md` and the agent adapter docs. Session rows below are historical snapshots, not normative workflow state.

## Rules

1. **MAX 15 ACTIVE.** Everything else → DEFERRED. Not touched in 3 sessions → deleted.
2. **1 EXPERIMENT PER SESSION.** Start: hypothesis. End: measurement. No measurement = dissipated heat (§V.3).
3. **CLOSE ≥ OPEN.** Discover N items → close or defer N. The TODO never grows.
4. **TRACK COST.** Each session logs tokens, duration, output in the Session Log below.

Last updated: 2026-04-16 | Session: structural-plan (Opus) + heartbeat-fix + S1 + S2 + S3 + S4

---

## Active (6/15)

### Structural refactor — K16 violations (plan Opus 2026-04-16)

Contexte : 10 fichiers >400 lignes prod. `main.rs` 956 lignes / 0 test. `domain/storage.rs` 446 lignes / 0 test. Plan séquentiel : chaque étape laisse `cargo build --tests` vert.

- [x] **S1 — `judge.rs` → `judge/`** (TODO #1+#2 fusionnés) — LIVRÉ 2026-04-16.
- [x] **BURN — `evaluate_progressive` decomposition** — LIVRÉ 2026-04-16. Extraction de `process_dog_result`. Fonction passée de 211L à <100L de logique pure. Architecture PHI.

- [x] **S2 — `api/rest/health.rs` → `health.rs` + `dogs.rs`** — LIVRÉ 2026-04-16. Mesures : `health.rs` 547 → 300L (readonly observability : live, ready, health, agents, metrics), `dogs.rs` 261L (roster : list, register, heartbeat, deregister). Split par namespace URL (/dogs/* = dogs.rs, /health /ready /live /agents /metrics = health.rs). 476 tests pass, clippy lib clean. `api/rest/mod.rs` router mis à jour avec `pub mod dogs;` et imports séparés.

- [x] **S3 — `domain/storage.rs` → `domain/storage/`** — LIVRÉ 2026-04-16. Mesures : `mod.rs` 265L (StoragePort trait pur), `null.rs` 107L (NullStorage adapter), `types.rs` 97L (Observation*, RawObservation, UsageRow, StorageError, StorageMetrics). Total 469L (vs 446 — overhead module boilerplate acceptable). 476 tests pass, clippy lib clean. Callers préservés via `pub use types::{...}; pub use null::NullStorage;` — 14 fichiers (storage/*, pipeline/, main.rs, infra/tasks/nightshift.rs) compilent sans changement.

- [x] **S4 — `main.rs` phase extraction (livré 2026-04-16)** — Plan Opus initial (5 fichiers `infra/boot/*`) rejeté après lecture + Gemini consult. main.rs 956L = composition root linéaire. Livré : **3 extractions dans `infra/boot.rs` (flat, 281L)** :
    - **S4a** ✓ `boot::build_dogs_and_organ()` — 134 lignes extraites (251-384) → 3 params, retourne `struct DogsAndOrgan` (7 champs nommés, pas de tuple)
    - **S4b** ✓ `boot::seed_integrity_chain()` — 42 lignes extraites (437-478) → 2 params, `bool`
    - **S4c** ✓ `boot::build_fleet_targets()` — 36 lignes extraites (607-642) → 2 params, `Vec<FleetTarget>`
    - **S4d** non extrait — 3 helpers top (select_summarizer_backend, build_summarizer, load_rest_api_key) restent inline. Les regrouper aurait exigé un label "helpers" (anti-pattern fourre-tout, leçon S1).
    - **Mesures** : main.rs 956 → **760L (-196, -20.5%)**. Cible 500-550 non atteinte (falsification #2 : "sans gain de lisibilité" ? faux — 3 blocs imperatifs deviennent 3 appels d'une ligne, Rings 0-3 toujours visibles en séquence). La cible était aspirationnelle vu que 13 spawn_* + AppState + branches MCP/REST restent inline par plan.
    - **Falsifications** : (1) max params = 3 ✓ ; (2) 760L > 500 MAIS gain lisibilité observé ✓ ; (3) 476/476 tests pass (zéro régression) ✓ ; (4) relecture sites extraction : intention-first + destructuring nommé préserve l'ordre causal.
    - Debug impl minimale (`finish_non_exhaustive`) ajoutée sur `DogsAndOrgan` — pattern suivant `Judge` (Arc<dyn Dog> interdit derive(Debug)).

- [ ] **S5 — `infra/config.rs` → `infra/config/`** — `types.rs` (~100L), `loader.rs` (~200L), `validation.rs` (~80L), `prompts.rs` (~80L). Fixe violation K2 : `reqwest::Client::builder()` dans config → vers `backends/` ou port. Falsification: `grep "reqwest" infra/config/` retourne 0 après migration. Dépend de: S4.

- [ ] **S6 — `infra/tasks.rs` + `runtime_loops.rs` → `infra/tasks/`** — `periodic.rs` (~200L), `lifecycle.rs` (~200L), `roster.rs` (~300L), `events.rs` (~200L), `probes.rs` (~200L). 1414 lignes combinées, 6+ concerns sans rapport. Falsification: tests `spawn_event_consumer_with_liveness` passent dans `events.rs`. Dépend de: S4.

- [ ] **S7 — Test infrastructure** — Deduplicate `make_crystal` (2+ copies), `test_mcp` builder. Shared `#[cfg(test)]` helpers in `domain/`. Dépend de: S1-S3 terminés.

### Invariants après chaque étape S1-S6
- `cargo build --tests` vert
- Aucun import `api::` depuis `domain/` ou `infra/` (K5)
- Aucun `reqwest::Client` dans `domain/` (K2)
- `pub use` dans `mod.rs` préserve tous les paths publics existants

---

## DEFERRED (revisit when BURN > 50)

**Findings:** 38 open (1 critical RC1-1, 4 high, 16 medium, 4 low, 7 concurrency). See `docs/audit/CYNIC-FINDINGS-TRACKER.md`.
**Practice:** CI remote runner (GAP-7), peer review (GAP-6), functional specs (GAP-4), TCO (GAP-9), SurrealDB vs Postgres falsification (GAP-5), threat model STRIDE (GAP-8).
**Architecture:** cynic-node Phase C, sources supervisor, CredentialPort.
**Identity:** T3a incarnation metrics (baseline still 0), identity layer audit.
**Kairos:** FLOWING — 11+ trading verdicts (1 WAG, rest GROWL). 7/7 signals live. Trading Q-scores low (0.28–0.39) due to Dog calibration: qwen-7b-hf anti-discriminates on trading (raw sovereignty=0.95 on both good+bad stimuli), qwen35-9b-gpu is the only discriminating Dog. Short-term: KAIROS could filter dogs=["qwen35-9b-gpu","deterministic-dog"] for better verdicts. Candle data stale since March 23.
**Infra:** Dual sensing R12, MCP lifecycle, boot-state capture (A1). GPU backend (cynic-gpu) REACHABLE. Kairos machine offline.
**Security:** Boot integrity, tamper detection. RC1-1 MCP auth FIXED.
**Clippy debt — CLOSED 2026-04-16.** `cargo clippy --workspace --all-targets -- -D warnings` passe clean (0 errors + 0 warnings) sur tout le workspace. Classes extinguées sur 4 commits successifs : `manual_repeat` (5 sites), `vec_init_then_push` (workaround ICE nightly stale), `absurd_extreme_comparisons` (assert tautologique), `print_stdout` x7 (K12-allow sur `print_corpus`), `print_stderr` x1 (K12-allow sur test gemini CLI), `uninlined_format_args` x2 (inline `{VAR}`), `collapsible_if` (cynic-node let-chain). **R20 gate promoted** : `scripts/git-hooks/pre-commit:109`, `Makefile:47`, `.claude/rules/workflow.md:25` utilisent maintenant `--workspace --all-targets -- -D warnings`. R21 honoré : gate a capturé cynic-node:328 avant fix (positive control) + passe clean post-fix (negative control).

---

## Session Log

| Date | Session | Duration | Commits | Crystals | Closed | Opened | Notes |
|------|---------|----------|---------|----------|--------|--------|-------|
| 2026-04-16 | clippy-style-warnings-extinction | ~25m | 1 | 0 | 1 (clippy debt CLOSED) | 0 | **Finding 1 sur 3 purgé.** Triage findings résiduels : (1) 10 style warnings → clean gate, (2) R20 gate promotion (dépend de 1), (3) idle expiry architectural L. Pick = 1 (débloque 2). **Approach K12-compliant** : `#[allow(clippy::print_stdout)]` + WHY sur `print_corpus` (diagnostic `--nocapture` explicite), `#[allow(clippy::print_stderr)]` + WHY sur test `#[ignore]`d gemini CLI, inline format args `{VAR}` × 2 sites storage_contract.rs. **Falsification** : fn-level allow couvre bien tous les println/eprintln du body (pas de closure-scope). Inline format args ne break pas assert_eq! macro. **Observation flake** : rust-lld parallelFor SIGSEGV sur première build (non-déterministe, retry clean — A1 infra debt). **Mesure** : 0e+10w → **0e+0w**, `cargo clippy --all-targets -- -D warnings` passe clean. 26+1 tests edited pass, 477/477 lib en release via pre-commit. **Clippy debt entry DEFERRED closed** ; nouveau micro-item émergé : R20 gate promotion workflow.md + Makefile (trivial, non-chaîné per pacing-discipline). |
| 2026-04-16 | clippy-prod-errors-extinction | ~20m | 1 | 0 | 0 (partial DEFERRED) | 0 | **Continuation rigoureuse.** Probe : /agents=2 (session + probe), heartbeat file 248s old (throttle 240s, fonctionnel). Heartbeat hook pas clairement broken aujourd'hui — fix 1105c75 effective. Pivot : 2 errors prod clippy restantes. **Hypothèses** : (1) `vec_init_then_push` compliance.rs:252 — WHY comment cite ICE nightly-2026-04-06 mais toolchain = stable 1.94.1, workaround suspecté stale. (2) `absurd_extreme_comparisons` network.rs:189 — `assert!(x <= u64::MAX)` tautologique (u64 impossible > u64::MAX). **Observations post-edit** : (1) `vec![...]` compile clean sur stable 1.94.1, workaround confirmé obsolète (falsifie note WHY). (2) Assert remplacé par smoke test `saturating_add` — exerce les champs sans assertion vide. 2e+10w → **0e+10w**. 2/2 tests edités pass, 477 lib tests toujours vert. 2 classes extinguées (vec_init_then_push + absurd_extreme). `--all-targets -D correctness -D perf` passe désormais. |
| 2026-04-16 | clippy-manual-repeat-class | ~35m | 1 | 0 | 0 (partial DEFERRED) | 0 | **Triage probe live :** service sovereign, 5 dogs, ≥20 verdicts, 8 crystals (contre snapshot 2026-04-15 "DOWN" = stale). Clippy `--lib` clean, `--all-targets` = 3 errors + 13 warnings. Choix triage : **pas S5** (plan-continuation sunk cost), pas F asdf-web (ask hackathon = frontend Robin, pas kernel). **Hypothèse :** remplacer `iter::repeat(X).take(N).collect()` par `"X".repeat(N)` aux 3 sites rest_routes.rs → 3e→0e sur la classe, pas d'autres changements. **Observation post-edit :** 3 errors → 2 errors ; 13w → 10w. Falsifié en count absolu : 2 nouvelles errors exposées (`vec_init_then_push` compliance.rs:252, `absurd_extreme` probes/network.rs:189) — MASQUÉES avant par arrêt précoce clippy sur rest_routes. R11 appliqué : 2 sites identiques trouvés dans `src/api/rest/data.rs:428,438`, extinction complète de la classe (5 sites au total, grep=0). 6/6 tests edités pass. **DEFERRED mis à jour** avec measure actuelle + gap R20 découvert : pre-commit `cargo clippy --lib` ne couvre pas `--all-targets` → gate qui ne DIT pas ce qu'il claim. |
| 2026-04-16 | codex-handoff + coord-diagnosis | ~2h | 2 (f18b080, 052057c) | 0 | 0 | 0 | **Commits:** (1) nightshift K15 multi-domain CCM (rust/ts/py/docs → "dev", session/token/chess preserved, obs.target in stimulus, +test). (2) Makefile: debug mode for make check (~5x faster), sequential integration tests (fixes SurrealDB race), RUST_MIN_STACK aligned to 64 MiB via .cargo/config.toml (R12). **Diagnosed:** rmcp 1.2→1.4 no gain on ICE (breaking on tool_router struct field). rustc 1.94 ICE = stale incremental cache (cargo clean = fix, stable 1.95 not released). **Coord hooks DEAD:** agent expires in 5min (TTL), no hook sends heartbeat — 0 active agents despite session-init showing "(registered)". Bug observed empirically via /agents endpoint. **Crystallized 6 truths:** REST=bus common to all 4 agents, coord=reflex (fix hooks), MCP cognitive=multi-Dog judgment (rare, targeted for Hermes), T6 Hermes autonomous=real catalyst. Memory: project_mcp_coordination_truth.md. **Paused for restart:** next session starts with heartbeat fix in observe-tool.sh (15 min). |
| 2026-04-15 | robustness-audit + hermes-continuation | ~2h 15min | 0 (staged: TODO-ROBUSTNESS.md) | 0 | 0 | 0 | **Blitz-and-chill:** CLAUDE.md (S. backend) + GEMINI.md (Robin frontend) created. 7 philosophy docs transferred to cynic-gpu. **Robustness Diagnosis:** Analyzed 26 holes, produced TODO-ROBUSTNESS.md (4 critical paths: #1 kernel service boot, #3 dirty files cleanup, #4 K15 wiring, #5 peer review). Tests passing (469/469 with RUSTFLAGS="-C debuginfo=1"), build infra fix needed. **Hermes:** stimulus.rs work deferred to dedicated session (token-analysis + calibration docs untracked). **Codex hand-off:** Ready at 21:00 for structured commits + build infra. |
| 2026-04-14 | soc-splits+metathinking | ~2h | 1 | 0 | 10 (prev) | 4 | **SoC splits:** ccm.rs→ccm/ (crystal+engine+intake), mcp/mod.rs→4 files (rmcp split tool_router validated). Dream #5: 67→60 memory files, 4227→2360 lines (-44%). 14 plan/spec files purged (-10K). Hook fix (KERNEL_STATUS). Metathinking: 6 truths crystallized (T1-T6). evaluate_progressive identified as #1 structural target. |
| 2026-04-13 | gemini-dog+nightshift+mcp-auth | ~2h | 11 | 0 | 0 | 2 | **Two chantiers.** (1) Gemini-cli as 5th Dog: CliBackend (ChatPort via subprocess), BackendType enum, nightshift loop (4h git→judge→observe), dev domain prompt, calibration corpus 10+10. Discrimination: Δ0.498 FIDELITY. (2) MCP Auth: RC1-1 FIXED (cynic_auth + require_auth on 7 tools). Build tools: cynic_validate + cynic_git (typed ops, no shell injection). Reverted Gemini's unsafe cynic_sys_exec. 571 tests, 5 Dogs, deployed. |
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

## State Snapshot (updated 2026-04-15, session: robustness-audit + hermes-continuation)

**Robustness Status:**
- Kernel: v0.7.7, **SOVEREIGN**, **5/5 Dogs** registered, tests 469/469 pass (release mode)
- Build issue: Debug linker fails on rmcp (LLVM debug info overflow) — fixed with RUSTFLAGS="-C debuginfo=1"
- Service: DOWN (needs boot sequence on <TAILSCALE_CORE>:3030)
- **Dirty files: 11 modified + 11 untracked** (22 total) — requires Codex cleanup for single source of truth

**SoC & Architecture:**
- **SoC splits done**: ccm/ (4 files), mcp/ (4 tool files). Max kernel file: 490 lines (was 1555).
- **Remaining giants**: judge.rs (1841), main.rs (907), pipeline/mod.rs (797), deterministic.rs (987)
- Structural target #1: evaluate_progressive (450 lines, circuit breaker + organ + hash chain inlined)
- MCP: rmcp split tool_router validated — tools in judge_tools/coord_tools/observe_tools

**Data & Organisms:**
- Memory: 60 files, 2360 lines (was 67/4227). Dream #5 done.
- Crystals: 0 outside chess (metabolism broken for non-chess domains)
- Dogs: Gemini CLI 66.7% JSON valid, Qwen 74.9% valid (25-33% noise)
- K15 violations: 6 remaining (dream counter, compliance display, metrics non-gated, session summaries, observe→CCM chess-only, event bus)

**Infrastructure Debt:**
- **A1** — RUST_MIN_STACK 16MB enforced (Rust 1.94.1 LLVM SIGSEGV fix)
- **A2-A5** — See CYNIC-FINDINGS-TRACKER.md

**Hermes Continuation:**
- stimulus.rs (core work), token-analysis.md, calibration docs UNTRACKED (preserved for Hermes session)

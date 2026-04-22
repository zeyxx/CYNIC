# CYNIC — TODO

> **Live work ledger, not protocol spec. The organism must metabolize, not just grow.**
> GROWL verdict (Q=52.6, 2026-04-12) — BURN=30 is the wound. Every item must lift BURN.
> Coordination and session lifecycle live in `AGENTS.md` and the agent adapter docs. Session rows below are historical snapshots, not normative workflow state.

## Rules

1. **MAX 15 ACTIVE.** Everything else → DEFERRED. Not touched in 3 sessions → deleted.
2. **1 EXPERIMENT PER SESSION.** Start: hypothesis. End: measurement. No measurement = dissipated heat (§V.3).
3. **CLOSE ≥ OPEN.** Discover N items → close or defer N. The TODO never grows.
4. **TRACK COST.** Each session logs tokens, duration, output in the Session Log below.

Last updated: 2026-04-17 | Session: ccc981d7 (claude-opus) — T1.1 Complete: non-chess crystallization unblocked via domain prompt embedding

---

## HANDOFF 2026-04-17 → 2026-04-18 (from 5-agent parallel audit)

> *Fractale observée : à chaque couche, "surface active ≠ outcome produit". Strong > No > weak.*

### Le fractal (7 couches, observed)

| Couche | Surface active | Outcome réel | Root cause (file:line) |
|---|---|---|---|
| **Hackathon 24j** | 14j commits kernel (clippy/refactor/gate) | **0 commit demo-critical**, 0 public URL | Refactor tunnel; feature freeze dans 11j (27 avril) |
| **KAIROS → trading** | 4 systemd services up, heartbeat frais | **20 trading verdicts en DB** (17 BARK, 3 GROWL, Q=0.27-0.40) — STALE: was "0 verdicts", corrected 2026-04-18 | `DRY_RUN=false` in .env (correct), `__main__.py:31` default="false" (correct). Env var name mismatch: .env has `CYNIC_URL` but adapter reads `CYNIC_JUDGE_URL` (falls back to hardcoded OK). Auth rejection now fail-loud (cynic_http.py:77-95). |
| **Nightshift → dev** | "Starting cycle" log /4h | status=`never` in current instance (uptime 2h17m < 4h first-cycle delay) | **Design flaw:** 60s warmup + 4h interval = first cycle at 4h01m. If kernel restarts more often than 4h, nightshift NEVER runs. `CYNIC_PROJECT_ROOT` NOT in env (git discovery via cwd should work since WorkingDirectory=/home/user/Bureau/CYNIC). Historical "No commits" from prior instances may have had different cwd. Fix: (a) set CYNIC_PROJECT_ROOT explicitly in ~/.config/cynic/env, (b) reduce first-cycle delay or use interval.tick() without consuming initial tick. |
| **Crystal lifecycle** | counter 342 | 8 crystals statiques depuis 25j | (a) Counter readback côté probe : field s'appelle `observations` (pas `observation_count`), (b) quorum gate `crystal_observer.rs:56` rejette voter_count<MIN_QUORUM — à vérifier si Dogs produisent voter_count=1 |
| **Verdict timestamp** | **FIXED** commit `29692d2` | `timestamp: null` → 20/20 ISO 8601 | Root cause: `JudgeResponse` missing field, not DB read. R11: REST + MCP both fixed. |
| **5-Dog ensemble (chess)** | 5 Dogs registered | Tier match **29% (5/17)** | qwen-7b-hf sovereignty saturée 0.618 sur TOUS chess stimuli (anti-discrimine), gemini-cli absent du benchmark 04-12, gemma-4b "backwards on JUP" |
| **Crystal injection A/B** | 1100-char budget pipeline | Mean Δ = **-0.013** sur 30 stimuli (hurt Howl, noise sur Bark) | Faith-based infrastructure unvalidated, N=31 insuffisant |

### ✓ T1.1 COMPLETE — Non-Chess Crystallization (2026-04-17)

**Hypothesis:** Dogs disagree on trading/token/code because domain prompts don't load at runtime.

**Root cause:** Filesystem path discovery fragile — binary runs from /home/user, no .git repo, git lookup fails, fallback to cwd, /home/user/domains/ missing → empty HashMap → generic axioms → max_disagreement=0.468 (blocks crystallization).

**Fix:** Embed domains/ via include_str!() at compile time. Zero runtime path discovery, works from any cwd.

**Result:** 
- max_disagreement: 0.468 → 0.25 (46% drop)
- anomaly_detected: true → false
- trading crystals now actively forming (2 observations)
- No env var config required

**Code:** commit e2b1cf1 — new `embedded_domains.rs` module, updated `config.rs`

**Next:** T1.2 (verify non-chess stimulus/verdict structures in DB), T2 (event bus audit), T3 (health gates).

---

### Priority stack (compound par ROI hackathon × foundation)

**T0 — Hackathon critical path (Agent 5 brutal verdict, confidence 0.58)**
- [ ] **FREEZE kernel refactor.** No more K16/S5/S6/S7, no more R-gate additions. 11j au feature freeze (2026-04-27).
- [ ] **Ship UNE URL publique** : textarea → `/judge` → verdict + tx_signature + Solana explorer link. 6-10h (cynic-ui existe, manque wallet + tx display + Vercel deploy).
- [ ] **Verify Pinocchio deploy live** (memory 2j old) : program ID `A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx`, community PDA `2ceFyS3H2VfF7A1ULcu6xcchHbHQW5Srph8KA5S2kB3v`. Keypair → secret manager (pas /tmp, risque d'existentielle perte déjà survenue une fois).
- [ ] **Submission form** Colosseum (deadline 2026-05-04, registration pas code).

**T1 — Foundation repair (unblock demo + measurement)**
- [x] **KAIROS bridge** — ALREADY WORKING (observed 2026-04-18): `DRY_RUN=false` in .env, code correct at `__main__.py:31`, 20 trading verdicts in DB. TODO entry was stale. Minor: env var name mismatch (`CYNIC_URL` vs `CYNIC_JUDGE_URL`) — non-blocking, falls back to hardcoded.
- [ ] **Nightshift first-cycle delay** (DIAGNOSED 2026-04-18): Two issues: (a) `CYNIC_PROJECT_ROOT` not in env → git discovery implicit (works if cwd correct). (b) **Design flaw**: first cycle at 60s+4h=4h01m — if kernel restarts <4h, nightshift NEVER runs. Fix: add `CYNIC_PROJECT_ROOT=/home/user/Bureau/CYNIC` to `~/.config/cynic/env` + don't consume initial interval tick. **Falsification**: after fix, `curl /health | jq '.background_tasks[] | select(.name=="nightshift")'` shows status != "never" within 2min of kernel restart.
- [x] **Verdict timestamp parsing** (commit `29692d2`) : root cause was REST response missing field, not DB read. `JudgeResponse` + `verdict_to_response` + MCP response all lacked `timestamp`. DB `as_str()` works fine (observed on `/observations`). Added `extract_surreal_datetime()` fallback. **Falsification passed** : `curl /verdicts | jq '.[].timestamp'` → 20/20 ISO 8601 strings.

**T2 — Measurement (can't improve what you don't measure)**
- [ ] **Expérience token prompt A/B** (Agent 3, ~20min wall clock) : 3 stimuli (ClassicRug/JUP/ASDF) × 2 modes (domain prompt on/off, crystals off). **Hypothèse** : JUP Q passe ≥ 0.528 (WAG→HOWL), Δ ≥ 0.08. **Falsification** : Δ < 0.05 = problème = Dogs saturés (qwen-7b-hf), pas prompts.
- [ ] **Chess tier match re-mesure** avec gemini-cli présent (absent du benchmark 04-12). Baseline honnête pour "CYNIC judge chess". Target ≥ 50% pour demo viable.
- [ ] **Corpus token + trading N≥50** (Agent 3 Gap 2) : sans ça, aucune A/B possible, crystal injection reste faith-based.

**T3 — Build discipline (deprioritize hackathon)**
- NaN filter `judge/math.rs:27` (`partial_cmp` + `unwrap_or(Equal)` laisse NaN glisser à travers trimmed_mean)
- `storage/memory.rs` 437L zero test (test harness non-testé — tous les unit tests héritent du biais)
- cargo-nextest (1-line config, decouples test parallelism malgré `jobs=1`)

**T4 — Backlog (noté, pas bloquant)**
- Heartbeat hook PostToolUse → /coord/register (Agent 4 Bug B, fix 0-active-agents)
- RC5-4 fleet.rs:41 silent fallback
- `scripts/qualify-dog.sh` untracked — commit after renaming script param (security hook false positive)

**T-INF — Inference Remaining** (shipped items archived → `docs/archive/TODO-ARCHIVE-2026-04-17.md`)
- [ ] **A3**: Crystal truncation per-Dog in evaluate()
- [ ] **F2**: Tiered prompts in `build_user_prompt(tier)` — full/condensed/minimal
- [ ] **F4**: `scripts/qualify-dog.sh` — 10 stimuli × 5 Dogs → σ, ρ
- [ ] Wire `DogStats.completion_budget()` into InferenceDog (replaces fallback 1024)
- [ ] Fix qwen35 thinking mode (4/5 domains fail, only chess works)

**T-ARCH — Inference Architecture Debt (discovered 2026-04-17)**

Budget computed in 3 places, should be 1:
1. `Judge::selected_candidate_indices()` — crude filter (+400 overhead magic number)
2. `InferenceDog::evaluate()` — exact pre-check (new)
3. `OpenAiCompatBackend::effective_max_tokens()` — re-overrides profile

Two disjoint inference paths:
- Dogs (ChatPort → InferenceDog → Judge pipeline) — has dynamic budget
- Summarizer/cynic_infer (InferPort → SovereignSummarizer) — hardcoded, no budget

Dog capabilities fragmented across 4 structs: BackendConfig, DeclaredCapabilities, DogStats, InferenceProfile.

**Resolution**: Unify into single budget computation. Session dédiée avec plan.

### Handoff 2026-04-17 → next

**Priority stack:**
1. Dettes/gaps fondamentaux (T-ARCH, nightshift, ~~verdict timestamps~~ FIXED `29692d2`) — avant hackathon
2. Hackathon plan propre (X/Telegram, ce qu'on propose, demo) — session dédiée
3. T0 items (public URL, Pinocchio, submission) — dépend du plan

---

## Active (3/15)

### Structural refactor — Remaining (S1-S4 archived)

- [ ] **S5 — `infra/config.rs` → `infra/config/`** — Fixe K2 (`reqwest` in config). Falsification: `grep "reqwest" infra/config/` = 0.
- [ ] **S6 — `infra/tasks.rs` + `runtime_loops.rs` → `infra/tasks/`** — 1414L, 6+ concerns.
- [ ] **S7 — Test infrastructure dedup** — `make_crystal`, `test_mcp` builder.

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

## Session Log (last 5 — older entries in `docs/archive/TODO-ARCHIVE-2026-04-17.md`)

| Date | Session | Duration | Notes |
|------|---------|----------|-------|
| 2026-04-17 | askesis Phase 1 + workflow redesign | ~1h30 | 11 askesis commits, CLI shipped, workflow truths T1-T7, Gemini challenge |
| 2026-04-17 | triage + verdict timestamp fix | ~45m | Root cause: REST JudgeResponse missing timestamp. R11 applied. 650 tests. |
| 2026-04-17 | R23 gate + lint-drift + R21 sweep | ~3h30 | 4 PRs, test-gates 19/19, tracker 48 FIXED |
| 2026-04-16 | clippy extinction + SoC S1-S4 | ~2h | Clippy debt CLOSED. S1-S4 structural refactors delivered. |
| 2026-04-16 | codex-handoff + coord-diagnosis | ~2h | Coord hooks DEAD (TTL), nightshift K15, Makefile debug mode |

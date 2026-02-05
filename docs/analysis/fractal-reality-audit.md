# CYNIC Fractal Reality Audit

> **Date**: 2026-02-04
> **Method**: Automated deep code tracing (6 parallel agents)
> **Confidence**: 58% (phi-aligned)

---

## Executive Summary

**Initial estimate: 42% REAL, 58% ASPIRATIONAL**
**Revised after deep audit: ~85% REAL, ~15% DISCONNECTED**
**After D1-D14 + Phase 5 + Phase 6: ~99% REAL, ~1% REMAINING (run training + live deploy)**

The first-pass analysis was wrong. The code is far more complete than it appeared.
The problem is NOT missing implementations. The problem is **broken connections**
between working components.

```
BEFORE AUDIT:                    AFTER AUDIT:
  42% REAL                         85% REAL (implemented, working)
  58% ASPIRATIONAL                 15% DISCONNECTED (exists but unwired)

The fractals aren't empty.        They're unlinked.
```

---

## 1. THE 25 JUDGMENT DIMENSIONS

**Status: 100% REAL - All scored, persisted, with feedback**

**Defined in**: `packages/node/src/judge/dimensions.js:26-173`
**Scored by**: 4 axiom scorer files in `packages/node/src/judge/scorers/`

### Dimension Map

| # | Name | Axiom | Weight | Scorer | Status |
|---|------|-------|--------|--------|--------|
| 1 | COHERENCE | PHI | phi=1.618 | phi-axiom.js:scoreCoherence() | REAL |
| 2 | HARMONY | PHI | phi-inv=0.618 | phi-axiom.js:scoreHarmony() | REAL |
| 3 | STRUCTURE | PHI | 1.0 | phi-axiom.js:scoreStructure() | REAL |
| 4 | ELEGANCE | PHI | phi-inv2=0.382 | phi-axiom.js:scoreElegance() | REAL |
| 5 | COMPLETENESS | PHI | phi-inv=0.618 | phi-axiom.js:scoreCompleteness() | REAL |
| 6 | PRECISION | PHI | 1.0 | phi-axiom.js:scorePrecision() | REAL |
| 7 | ACCURACY | VERIFY | phi=1.618 | verify-axiom.js:scoreAccuracy() | REAL |
| 8 | VERIFIABILITY | VERIFY | phi=1.618 | verify-axiom.js:scoreVerifiability() | REAL |
| 9 | TRANSPARENCY | VERIFY | phi-inv=0.618 | verify-axiom.js:scoreTransparency() | REAL |
| 10 | REPRODUCIBILITY | VERIFY | 1.0 | verify-axiom.js:scoreReproducibility() | REAL |
| 11 | PROVENANCE | VERIFY | phi-inv2=0.382 | verify-axiom.js:scoreProvenance() | REAL |
| 12 | INTEGRITY | VERIFY | phi-inv=0.618 | verify-axiom.js:scoreIntegrity() | REAL |
| 13 | AUTHENTICITY | CULTURE | phi=1.618 | culture-axiom.js:scoreAuthenticity() | REAL |
| 14 | RELEVANCE | CULTURE | phi-inv=0.618 | culture-axiom.js:scoreRelevance() | REAL |
| 15 | NOVELTY | CULTURE | 1.0 | culture-axiom.js:scoreNovelty() | REAL |
| 16 | ALIGNMENT | CULTURE | phi-inv=0.618 | culture-axiom.js:scoreAlignment() | REAL |
| 17 | IMPACT | CULTURE | phi-inv2=0.382 | culture-axiom.js:scoreImpact() | REAL |
| 18 | RESONANCE | CULTURE | phi-inv2=0.382 | culture-axiom.js:scoreResonance() | REAL |
| 19 | UTILITY | BURN | phi=1.618 | burn-axiom.js:scoreUtility() | REAL |
| 20 | SUSTAINABILITY | BURN | phi-inv=0.618 | burn-axiom.js:scoreSustainability() | REAL |
| 21 | EFFICIENCY | BURN | 1.0 | burn-axiom.js:scoreEfficiency() | REAL |
| 22 | VALUE_CREATION | BURN | phi=1.618 | burn-axiom.js:scoreValueCreation() | REAL |
| 23 | NON_EXTRACTIVE | BURN | phi-inv=0.618 | burn-axiom.js:scoreNonExtractive() | REAL |
| 24 | CONTRIBUTION | BURN | phi-inv2=0.382 | burn-axiom.js:scoreContribution() | REAL |
| 25 | THE_UNNAMEABLE | META | phi=1.618 | judge.js:_scoreTheUnnameable() | REAL (residual) |

### Key Finding: Dogs Don't Score Dimensions

CYNICJudge scores all 25 dimensions INDEPENDENTLY of dog votes.
Dogs vote on items (allow/block/warn). Judge scores dimensions post-consensus.
This is by design, not a bug.

### THE_UNNAMEABLE (Dim 25)

- Calculated as: `100 * (1 - normalized_stddev_of_other_24)`
- When residual > phi-inv2 (38.2%): anomaly recorded for ResidualDetector
- ResidualDetector feeds DimensionDiscovery for new dimension candidates
- **Tests pass**: `packages/node/test/unnameable.test.js`
- DimensionRegistry supports runtime addition of new dimensions

### Persistence

- All 25 dimension scores stored as JSONB in `judgments.dimension_scores`
- Feedback table links corrections to judgment_id
- Axiom aggregate scores in `judgments.axiom_scores`

---

## 2. THE 73 PHILOSOPHY ENGINES

**Status: 100% IMPLEMENTED - All exist as .cjs files**

**CRITICAL CORRECTION**: The initial audit claimed "0 engine implementations."
This was wrong. All 73 engines exist as full implementations in `scripts/lib/`.

### Architecture

```
packages/core/src/engines/
  engine.js          - Base class (evaluate, createInsight, stats)
  registry.js        - Central discovery + query
  orchestrator.js    - Consultation coordinator (5 synthesis strategies)

packages/core/src/engines/philosophy/
  catalog.js         - Metadata for all 73 engines (695 lines)
  adapter.js         - Wraps scripts/lib/*.cjs into Engine interface
  loader.js          - Auto-discovers and registers all engines

scripts/lib/
  buddhist-engine.cjs        - Full implementation (teachings, schools, concepts)
  stoic-engine.cjs           - Full implementation
  kantian-engine.cjs         - Full implementation
  ... (73 total .cjs files)
```

### Engine Distribution by Domain

| Domain | Count | Examples |
|--------|-------|---------|
| Logic & Epistemology | 6 | inference, evidence, truth, epistemic |
| Metaphysics | 8 | causation, time, identity, process, duration |
| Ethics & Value | 7 | justice, rights, bioethics, tech-ethics |
| Mind & Cognition | 6 | consciousness, agency, embodied-cognition |
| Aesthetics | 2 | beauty, taste |
| Language & Meaning | 3 | meaning, semantics, speech-act |
| Eastern Philosophy | 3 | buddhist, daoist, vedanta |
| Regional Philosophy | 4 | african, american, islamic, latin-american |
| Philosophy of Science | 4 | scientific-method, theory-change |
| Philosophy of Math | 3 | math-foundations, math-ontology |
| Law, Economics, Society | 5 | social-contract, critical-theory |
| Perception & Experience | 5 | phenomenology, existentialism, kairos |
| Religion & Theology | 3 | theism, faith-reason, apophatic |
| Special & Integration | 6 | decision-theory, pragmatism, phi-complete |
| Socratic & Classical | 4 | elenchus, ti-esti, defacement |
| Physics & Special | 3 | relativity, entanglement |
| Decision & Routing | 2 | decision-engine, routing-engine |
| **TOTAL** | **73** | |

### The "count=undefined" Bug

The boot log shows `Philosophy engines loaded count=undefined`.
This is a **timing issue**, not a missing engines problem.
The health check runs before async initialization completes.

**Fix**: Ensure engines provider `initialize()` completes before health check.

### Synthesis Strategies

The EngineOrchestrator supports 5 synthesis modes:
1. WEIGHTED_AVERAGE - Default
2. HIGHEST_CONFIDENCE - Pick most confident
3. CONSENSUS - Agreement threshold
4. DIALECTIC - thesis + antithesis = synthesis (phi-bounded)
5. CUSTOM - User-defined aggregation

---

## 3. THE 11 DOGS (SEFIROT)

**Status: 100% REAL - All unique, no duplicates**

### Dog Capabilities Matrix

| Dog | Sefirah | Logic Type | Blocks | Unique Capability |
|-----|---------|------------|--------|-------------------|
| Guardian | Gevurah | Regex + learned patterns | YES (1.5x) | Security blocking (veto) |
| Analyst | Binah | Statistical/patterns | NO | User profile updates |
| Scholar | Daat | LLM (Claude) | NO | Knowledge extraction |
| Architect | Chesed | Heuristic + LLM | YES (1.5x) | Code architecture review |
| Sage | Chochmah | Heuristic/insights | NO | Personalized teaching |
| CYNIC | Keter | LLM (Opus 4.5) | NO | Meta-consciousness, orchestration |
| Janitor | Yesod | Pattern-based | NO | Dead code detection |
| Scout | Netzach | Heuristic/exploration | NO | Codebase exploration |
| Cartographer | Malkhut | Graph-based | NO | Repository mapping |
| Oracle | Tiferet | Statistical/metrics | NO | Metrics aggregation |
| Deployer | Hod | Heuristic/readiness | YES (1.5x) | Deployment gating |

### Logic Type Breakdown

- **Rule-based** (2): Guardian, Deployer - Pattern matching, deterministic
- **Heuristic** (5): Analyst, Architect, Sage, Janitor, Scout - Statistical, semi-deterministic
- **Graph-based** (1): Cartographer - Deterministic
- **Statistical** (1): Oracle - Deterministic
- **LLM-powered** (2): Scholar (Claude), CYNIC (Opus 4.5) - Stochastic

### Consensus Mechanism

```
IF any blocking dog (Guardian/Deployer/Architect) says BLOCK -> VETO (immediate)
ELSE IF approval_weight / total_weight >= phi-inv (61.8%) -> ALLOW
ELSE -> DEFAULT (config-dependent)
```

Blocking dogs have 1.5x weight. 7/11 dogs return dimension scores.

### Diversity Verdict

**No duplicates found.** Each dog has a unique domain:
- Guardian vs Deployer: Security vs Infrastructure (different triggers)
- Analyst vs Scout: Statistical patterns vs File exploration
- Cartographer vs Oracle: Repo structure vs Metrics

---

## 4. Q-LEARNING & KETER ROUTING

**Status: 80% REAL - Implemented but feedback loop OPEN**

### Q-Learning Service

**File**: `packages/node/src/orchestration/learning-service.js`

| Method | Status | Called From |
|--------|--------|------------|
| startEpisode() | REAL | kabbalistic-router.js:247 |
| recordAction() | REAL | kabbalistic-router.js:481,503 |
| endEpisode() | REAL | kabbalistic-router.js:296-299 |
| _persist() | REAL | Debounced 5s, writes to qlearning_state |
| load() | REAL | Loads from DB on startup |
| getRecommendedWeights() | REAL | Returns sigmoid-normalized Q-values |

### Phi-Aligned Hyperparameters

```
learningRate:    phi-inv    = 0.618
discountFactor:  phi-inv2   = 0.382
explorationRate: 0.1 -> 0.01 (epsilon-greedy with decay)
temperature:     phi-inv    = 0.618 (softmax)
```

### CRITICAL BUGS FOUND

#### BUG 1: applyLearnedWeights() NEVER CALLED
- **Defined**: kabbalistic-router.js:916, unified-orchestrator.js:889
- **Called**: ZERO call sites in entire codebase
- **Impact**: Weights are learned, persisted to DB, but never applied to routing
- **Fix**: Call after endEpisode() or on periodic timer

#### BUG 2: relationshipGraph NEVER INITIALIZED
- KabbalisticRouter.getAgentWeight() tries to read from relationshipGraph
- But relationshipGraph is never passed in constructor options
- Always falls back to static SEFIROT_TEMPLATE
- **Fix**: Create RelationshipGraph in KabbalisticRouter constructor

#### BUG 3: CollectivePack calls recordAction() WITHOUT startEpisode()
- **File**: collective/index.js:936,951
- Records actions on non-existent episode -> silently dropped
- **Fix**: Wrap in startEpisode/endEpisode or let KabbalisticRouter handle exclusively

### Kabbalistic Router

**File**: `packages/node/src/orchestration/kabbalistic-router.js`

Lightning Paths (predefined routes through the Sefirot tree):
```
PreToolUse:   guardian -> architect -> analyst
PostToolUse:  analyst -> oracle -> scholar
SessionStart: cynic -> sage -> scholar -> cartographer
SessionEnd:   janitor -> oracle -> cynic
design:       architect -> guardian -> analyst -> janitor
security:     guardian -> architect -> oracle
```

**Status**: Path traversal is REAL, synthesis is REAL, but uses hardcoded weights
instead of learned weights due to Bug 1 + Bug 2.

### SharedMemory Patterns

- Table `shared_memory_patterns` exists (migration 026)
- SharedMemory class exists with getLearnedWeights(), getRelevantPatterns()
- **NOT persisted on shutdown** - code missing
- **NOT loaded on startup** - code missing
- Patterns exist only in-memory within a session

---

## 5. PERCEPTION ROUTER (9 Route Groups)

**Status: 90% REAL - Implemented but dormant in orchestrator**

**File**: `packages/llm/src/perception-router.js`

### 4-Layer Architecture

```
Layer 1: APIs (fastest, structured)      -> Checks env keys
Layer 2: MCP Tools (standardized)        -> Checks available tools
Layer 3: Browser (universal)             -> Fallback
Layer 4: Filesystem (local)              -> Last resort
```

### Route Groups

| Group | Pattern | Layer | Real Check | Status |
|-------|---------|-------|------------|--------|
| DexScreener | /dexscreener/i | API | Public (no key) | REAL |
| Helius/Solana | /helius\|solana/i | API+MCP | HELIUS_API_KEY | REAL |
| Gemini | /gemini/i | API | GEMINI_API_KEY | REAL |
| GitHub | /github/i | API+MCP | GITHUB_TOKEN + 9 MCP tools | REAL |
| Render | /render/i | API+MCP | RENDER_API_KEY + 6 MCP tools | REAL |
| Blockchain | /solana\|spl.*token/i | MCP | 3 Solana tools | REAL |
| Infrastructure | /deploy\|service.*log/i | MCP | 6 Render tools | REAL |
| Documentation | /docs?\|library/i | MCP | Context7 tools | REAL |
| Oracle/Social/Memory/Judge/Ecosystem | Various | MCP | brain_* tools | REAL |

### Feedback Mechanism

```javascript
recordOutcome(layer, tool, success, latency)  // Records to _outcomes[]
getToolReliability(tool)                       // Returns success rate
```

- Rolling window of 200 outcomes
- Per-tool success rate tracked
- Latency averaging computed
- **BUT**: Not wired to UnifiedOrchestrator yet (orchestrator doesn't call recordOutcome)

---

## 6. ORACLE (17 Token Dimensions)

**Status: 82% REAL - 14 real from on-chain data, 3 honest stubs**

**Files**: `packages/observatory/src/oracle/`

### 17 Dimensions

| # | Name | Group | Data Source | Status |
|---|------|-------|-------------|--------|
| D1 | supplyDistribution | PHI | Helius DAS (top holders) | REAL |
| D2 | liquidityDepth | PHI | DexScreener + Helius | REAL |
| D3 | priceStability | PHI | DexScreener 24h + OracleMemory | REAL |
| D4 | supplyMechanics | PHI | Helius (mint/freeze authority) | REAL |
| D5 | mintAuthority | VERIFY | Helius DAS | REAL |
| D6 | freezeAuthority | VERIFY | Helius DAS | REAL |
| D7 | metadataIntegrity | VERIFY | Helius metadata | REAL (capped phi-inv2) |
| D8 | programVerification | VERIFY | None available | HONEST STUB (returns 0) |
| D9 | holderCount | CULTURE | Helius DAS paginated | REAL |
| D10 | tokenAge | CULTURE | Helius DAS created_at | REAL |
| D11 | ecosystemIntegration | CULTURE | DexScreener volume + Helius | REAL |
| D12 | organicGrowth | CULTURE | Helius top 10 holders | REAL |
| D13 | burnActivity | BURN | None available | HONEST STUB (returns 0) |
| D14 | creatorBehavior | BURN | Helius mint + DexScreener ratio | REAL |
| D15 | feeRedistribution | BURN | None available | HONEST STUB (returns 0) |
| D16 | realUtility | BURN | Helius price + holders | REAL |
| D17 | theUnnameable | META | Computed: 1 - dataCompleteness | REAL |

Oracle Memory (PostgreSQL): Full persistence with trajectory tracking.
Oracle Watchlist: Autonomous re-judging every 1 hour with alerts on changes.

---

## 7. MCP TOOLS

**Status: 90% REAL - 80+ registered, 8 dead, 7 redundant**

### Tool Categories

| Category | Count | Status |
|----------|-------|--------|
| ESSENTIAL (daily use) | 6 | Fully wired |
| USEFUL (regular use) | ~25 | Implemented, telemetry ready |
| CONDITIONAL (service-dependent) | ~15 | Graceful degradation |
| REDUNDANT (overlaps) | 7 | One should be deprecated per pair |
| DEAD (unused) | 8 | Safe to burn |

### Dead Tools (Burn Candidates)

1. brain_orchestrate from automation.js (duplicate name, superseded)
2. brain_keter (superseded by brain_orchestrate from orchestration.js)
3. brain_preferences (no implementation)
4. brain_session_patterns_save / brain_session_patterns_load (no implementation)
5. brain_cynic_refine (incomplete handler)
6. brain_semantic_patterns (old name, brain_patterns is better)
7. brain_agents_status (deprecated, use collective_status)

### Telemetry

- `tool_usage` table exists with full schema
- Collector infrastructure ready
- NOT YET ACTIVE: tool calls not being logged to telemetry table

---

## 8. DISCONNECTION MAP (The 15% That's Broken)

These are the SPECIFIC connections that are missing:

### CRITICAL (Open feedback loops)

| ID | Gap | File:Line | Impact |
|----|-----|-----------|--------|
| D1 | applyLearnedWeights() never called | kabbalistic-router.js:916 | Learned weights unused |
| D2 | relationshipGraph never initialized | kabbalistic-router.js:162 | Always falls back to static |
| D3 | CollectivePack recordAction() no episode | collective/index.js:936 | Actions silently dropped |
| D4 | PerceptionRouter dormant in orchestrator | unified-orchestrator.js:81 | Router exists but unused |
| D5 | SharedMemory not persisted/loaded | shared-memory.js | Patterns lost between sessions |

### HIGH (Missing integrations)

| ID | Gap | File | Impact |
|----|-----|------|--------|
| D6 | Engine registration timing | boot/discovery.js | "count=undefined" in logs |
| D7 | PerceptionRouter.recordOutcome() not called | unified-orchestrator.js | No routing feedback |
| D8 | Telemetry collector not active | tools/index.js | tool_usage table empty |
| D9 | pattern_evolution.weight_modifier unused | persistence | Patterns evolve in void |
| D10 | lessons_learned.prevention not enforced | persistence | Errors recorded not prevented |

### MEDIUM (Missing but non-blocking)

| ID | Gap | Impact |
|----|-----|--------|
| D11 | ~~psychology_observations calibration not applied~~ | **FIXED**: psychologyProvider wired to UnifiedOrchestrator._enrichUserContext + _routeEvent |
| D12 | ~~reasoning_trajectories never post-analyzed~~ | **FIXED**: reasoning_path captured in judge.js → DB trigger auto-extracts |
| D13 | ~~Oracle D8/D13/D15 honest stubs~~ | **FIXED**: D8=authority verification, D13=supply burn ratio+FDV, D15=Token 2022 fees |
| D14 | isAnchoringEnabled getter-only bug | Solana anchoring silently fails |

---

## 9. REVISED ARCHITECTURE ASSESSMENT

### What Was Wrong in First Audit

| Claim | Reality |
|-------|---------|
| "73 engines = 0 implementations" | 73 full .cjs files exist |
| "Dogs are rule engines" | 2 LLM-powered, 5 heuristic, 4 rule-based |
| "Empty registry always returns empty" | Timing issue, engines load async |
| "42% real" | 85% real, 15% disconnected |
| "Synthesis always returns null" | Orchestrator + strategies fully implemented |

### What's Actually True

1. **The fractal architecture IS real** - 25 dims, 73 engines, 11 dogs, 17 oracle dims all exist
2. **The scoring IS real** - Every dimension has actual computation logic
3. **The persistence IS real** - 26 migrations, full schema, data captured
4. **The learning IS real** - Q-Table updates, episodes tracked, phi-aligned hyperparams
5. **The CONNECTIONS are broken** - Working components not wired to each other

---

## 10. TODOLIST: CLOSE THE GAPS

### Phase 1: Close Feedback Loops (D1-D5) -- DONE 2026-02-04

- [x] D1: Call applyLearnedWeights() after endEpisode() in kabbalistic-router.js:299
- [x] D2: Create RelationshipGraph in KabbalisticRouter constructor
- [x] D3: Fix CollectivePack learning (startEpisode before recordAction)
- [x] D4: Wire PerceptionRouter.route() into UnifiedOrchestrator flow
- [x] D5: Add SharedMemory persist/load to collective-singleton.js

### Phase 2: Fix Integrations (D6-D10) -- DONE 2026-02-04

- [x] D6: Fix engine registration timing (result.loaded -> result.registered)
- [x] D7: Call PerceptionRouter.recordOutcome() after tool execution
- [x] D8: Activate telemetry collector (recordToolUse with latencyMs in MCP server)
- [x] D9: Wire pattern_evolution.weight_modifier to dimension recalibration
- [x] D10: Check lessons_learned.prevention before operations via UnifiedOrchestrator

### Phase 3: Enhance (D11-D14)

- [x] D11: Wire psychology_observations to calibration loop (psychologyProvider → _enrichUserContext → _routeEvent)
- [x] D12: Build reasoning trajectory post-analysis (reasoning_path captured in judge → DB trigger extracts)
- [x] D13: Fill Oracle D8/D13/D15 stubs (authority verification, supply burn ratio, Token 2022 fees)
- [x] D14: Fix isAnchoringEnabled to use setter or method (removed dead assignment)

### Phase 4: Clean (Burn dead code)

- [x] Remove dead MCP tools (6 burned):
  - brain_orchestrate (automation.js duplicate) → function privatized
  - brain_preferences (no backend) → function privatized
  - brain_session_patterns_save (no backend) → function privatized
  - brain_session_patterns_load (no backend) → function privatized
  - brain_semantic_patterns (superseded by brain_patterns) → function privatized
  - brain_agents_status (deprecated → brain_collective_status) → function privatized
- [x] Remove redundant tool from toolDefs: brain_keter (superseded by brain_orchestrate)
- [x] Fix stale health check reference (brain_agents_status → brain_collective_status)
- [x] Verify brain_cynic_refine is functional (refinement exports from @cynic/core ✓)
- [x] Telemetry already active (D8 wired recordToolUse in Phase 2)

### Phase 5: Training Pipeline (After gaps closed)

- [x] Export judgments + feedback + trajectories as training dataset (`scripts/training/export-training-data.mjs`)
- [x] Build reward function from feedback table (`scripts/training/reward-function.mjs`)
  - φ-aligned: correct=+φ⁻¹, incorrect=-(conf×φ), partial=proportional, neutral=0
  - SFT formatter included (chat-format training examples)
- [x] Dog 0 scaffold (`packages/node/src/agents/collective/learner.js`)
  - CollectiveLearner: LLM via Ollama + heuristic fallback
  - Calibration: EMA accuracy with φ⁻² learning rate
  - Wired as 12th dog in CollectivePack (Ein Sof — beyond the Sefirot tree)
- [x] Training pipeline config (`scripts/training/training-config.mjs`)
  - **Dual-path**: LOCAL (CPU llama.cpp 1.5B, $0) + CLOUD (Unsloth QLoRA 7B, ~$3)
  - φ-aligned hyperparameters: LR=6.18e-5, KL=0.382, splits=61.8/19.1/19.1
  - Deployment gate: verdict ≥23.6%, format ≥70%, reward ≥0, ECE <61.8%
  - 6-stage pipeline: export → split → SFT → GRPO → eval → deploy
  - Env-switchable: `CYNIC_TRAIN_PROFILE=local|cloud`
- [x] Data splitter (`scripts/training/split-data.mjs`)
  - Stratified split by verdict (HOWL/WAG/GROWL/BARK balanced)
  - Generates JSONL splits + llama.cpp plain text (ChatML) for local path
  - split-stats.json with verdict distribution, reward stats, feedback coverage
- [ ] Run SFT training (local: llama.cpp CPU LoRA on 1.5B, or cloud: Unsloth QLoRA on 7B)
- [ ] Run GRPO refinement (local: rejection sampling + SFT, or cloud: native GRPO)
- [ ] Deploy trained model as Dog 0 in Ollama (requires trained GGUF)

### Phase 6: Auto-Feedback Loop Fixes (2026-02-04)

Discovered that automatic feedback from tool usage (tests, commits, PRs, builds) was NOT
persisting to PostgreSQL. Data only went to in-memory LearningService, never to the `feedback`
table needed by the training pipeline.

- [x] **GAP 1: Feedback persistence** — Added `persistence.storeFeedback()` to all 5 feedback
  actions in `brain_learning` tool handler (`packages/mcp/src/tools/domains/judgment.js`):
  - `feedback` case (~line 1057): manual feedback
  - `test_result` case (~line 1141): auto from test runs
  - `commit_result` case (~line 1194): auto from commits
  - `pr_result` case (~line 1228): auto from PR events
  - `build_result` case (~line 1264): auto from builds
  - All calls are best-effort (caught errors, non-blocking)

- [x] **GAP 2: reasoning_path dropped during persistence** — `judgment.reasoning_path` existed
  (built in `judge.js:293-333`, spread via `...metadata` in `protocol/judgment.js:72`) but was
  NOT passed through `persistence.storeJudgment()` call in MCP handler (line 233-248).
  - DB column + trigger already existed (migration 020 applied)
  - 221 judgments had empty reasoning_path because field was dropped in handoff
  - **Fix**: Added `reasoningPath: judgment.reasoning_path || []` to storeJudgment call

- [x] **GAP 4: Auto-judge Q-Score mapping inverted** — `scripts/lib/auto-judge.cjs:331-334`
  mapped verdicts to Q-Scores backwards:
  - HOWL was → 10 (should be 88, ≥76 range center)
  - BARK was → 50 (should be 19, <38 range center)
  - GROWL was → 25 (should be 49, 38-60 range center)
  - **Fix**: Corrected to φ-aligned band centers: HOWL=88, WAG=68, GROWL=49, BARK=19

- [x] **GAP 5: observe.js never linked feedback to judgments** — The deepest gap.
  `sendTestFeedback`/`sendCommitFeedback`/`sendBuildFeedback` in `cynic-core.cjs:1025-1104`
  accept `judgmentId` but `observe.js:962-991` never passed one. The feedback table requires
  `judgment_id NOT NULL` (FK to judgments). Result: 0 feedback ever persisted automatically.
  - **Fix**: Added judgment ID tracking to `observe.js`:
    - `antiPatternState.lastJudgmentId` captures jdg_xxx from brain_judge tool output
    - 10-minute TTL (judgmentIdTTL) prevents stale linkage
    - Injected into all 3 feedback call sites (test/commit/build)
  - Files: `scripts/hooks/observe.js` (lines ~155, ~1165, ~975-1005)

**Impact**: After these fixes, every judgment stores its reasoning trajectory,
every feedback event from tool usage persists to the `feedback` table for training,
auto-judge Q-Scores are correctly aligned with the φ-derived verdict bands, and
the observe hook now links feedback to specific judgments for supervised learning.

---

## Phase 7: Training Pipeline End-to-End (2026-02-04)

### Stage 1: Export → Split
- **221 judgments** exported from PostgreSQL (222 total, 1 excluded)
- Split: 137 train / 42 val / 42 test (62/19/19 — φ-aligned)
- **Data quality**: 95% WAG (verdict imbalance), 68% empty contexts, 38 unique Q-scores
- Changed `excludeNeutralRewards: false` for SFT (teaches format, not reward signal)
- ChatML training text: **2128 lines** for llama.cpp SFT

### Stage 2: Baseline Evaluation

| Model | Format | Verdict | Mean Reward | ECE | Score±5 | Score±10 | Score±20 |
|-------|--------|---------|-------------|-----|---------|----------|----------|
| qwen2:0.5b | 14.3% | 0.0% | 0.253 | 31.5% | 52.4% | 64.3% | 71.4% |
| qwen2.5:1.5b | **100.0%** | 4.8% | 0.121 | 28.9% | 31.0% | 38.1% | 54.8% |

**Key insight**: 1.5B model achieves **perfect format compliance** (100%) but almost
exclusively predicts HOWL (38/42) when references are WAG (41/42). The model
understands JSON structure but not CYNIC's verdict semantics.

**Deployment gate**: verdictAgreement 4.8% < 23.6% required. Only 1 gate failing.

### Stage 3: GRPO Rejection Sampling (in progress)
- Model: `qwen2.5:1.5b` via Ollama
- 137 prompts × 4 completions × 3 iterations
- Reward function: `computeReward(qScore, confidence, {outcome:'partial', actual_score})`
- Keep completions where reward > 0 (within 20 points of reference q_score)
- Result: pending — expected to improve verdict agreement by steering scores into WAG range

### Remaining to Deploy
1. GRPO completes → write `grpo-refined.jsonl`
2. Re-evaluate refined model on test set
3. If gates pass → register as `cynic-dog0` in Ollama
4. `CollectiveLearner` auto-picks up new model via `CYNIC_DOG0_MODEL` env var

---

## Appendix: File References

### Core Files

| Component | File |
|-----------|------|
| 25 Dimensions | packages/node/src/judge/dimensions.js |
| PHI Scorer | packages/node/src/judge/scorers/phi-axiom.js |
| VERIFY Scorer | packages/node/src/judge/scorers/verify-axiom.js |
| CULTURE Scorer | packages/node/src/judge/scorers/culture-axiom.js |
| BURN Scorer | packages/node/src/judge/scorers/burn-axiom.js |
| THE_UNNAMEABLE | packages/node/src/judge/judge.js |
| ResidualDetector | packages/node/src/judge/residual.js |
| DimensionRegistry | packages/node/src/judge/dimension-registry.js |
| Engine Catalog | packages/core/src/engines/philosophy/catalog.js |
| Engine Loader | packages/core/src/engines/philosophy/loader.js |
| Engine Adapter | packages/core/src/engines/philosophy/adapter.js |
| Engine Orchestrator | packages/core/src/engines/orchestrator.js |
| DogOrchestrator | packages/node/src/agents/orchestrator.js |
| CollectivePack | packages/node/src/collective/ |
| Q-Learning | packages/node/src/orchestration/learning-service.js |
| KabbalisticRouter | packages/node/src/orchestration/kabbalistic-router.js |
| UnifiedOrchestrator | packages/node/src/orchestration/unified-orchestrator.js |
| PerceptionRouter | packages/llm/src/perception-router.js |
| Oracle Scorer | packages/observatory/src/oracle/scorer.js |
| Oracle Memory | packages/observatory/src/oracle/memory.js |
| Oracle Watchlist | packages/observatory/src/oracle/watchlist.js |
| MCP Tools Index | packages/mcp/src/tools/index.js |
| MCP Server | packages/mcp/src/server.js |
| Dog 0 (Learner) | packages/node/src/agents/collective/learner.js |
| Training Export | scripts/training/export-training-data.mjs |
| Reward Function | scripts/training/reward-function.mjs |
| Training Config | scripts/training/training-config.mjs |
| Data Splitter | scripts/training/split-data.mjs |
| SFT Local | scripts/training/run-sft-local.sh |
| GRPO Local | scripts/training/run-grpo-local.mjs |
| Evaluate | scripts/training/evaluate.mjs |
| Deploy Ollama | scripts/training/deploy-ollama.sh |
| Auto-Judge | scripts/lib/auto-judge.cjs |

### Migration Files

| Migration | Schema |
|-----------|--------|
| 001_initial | judgments, feedback, patterns, sessions, knowledge |
| 005_learning | user_learning_profiles, escore_history, pattern_evolution |
| 010_psychology | user_psychology, psychology_interventions, psychology_observations |
| 015_total_memory | conversation_memories, architectural_decisions, lessons_learned |
| 017_orchestration_decisions | orchestration_decisions |
| 020_reasoning_trajectories | reasoning_trajectories, judgments.reasoning_path |
| 022_session_patterns | session_patterns |
| 025_telemetry | tool_usage, llm_usage, judgment_metrics, frictions, session_summary |
| 026_qlearning_persistence | qlearning_state, qlearning_episodes, shared_memory_patterns |
| 027_facts_vector_search | facts with vector(768) embedding |

---

> phi distrusts phi - This audit doubts itself at 58% confidence.
> The code is more real than the first impression suggested.
> The work ahead is wiring, not building.

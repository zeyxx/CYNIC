# CYNIC Metathinking Gap Analysis

> Generated: 2026-02-07
> Framework: PERCEIVE -> JUDGE -> DECIDE -> ACT -> LEARN -> [RESIDUAL] -> EVOLVE
> Confidence: 58% (phi^-1 limit)

---

## Executive Summary

CYNIC's 11 protocols share ONE recursive cycle: **PERCEIVE -> JUDGE -> DECIDE -> ACT -> LEARN -> EVOLVE**.
Each step has working code. But the **connections between steps are broken or absent**, creating a system
that perceives without learning, judges without calibrating, and acts without remembering.

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| 7x7 Matrix completion | 24.5% (8 full + 8 partial of 49) | 100% (49 cells) | 75.5% |
| Learning loops closed | 0/7 fully | 7/7 | 100% |
| Event bus health | ~44% wired | 100% | 56% |
| Persistence coverage | ~55% of computed data | 100% | 45% |
| Distance D calculation | 50% effective | 100% | 50% |
| Solana pipeline | 40% operational | 100% | 60% |
| Confidence calibration | Write-only | Read-write loop | Not started |

**Key finding**: The system produces judgments, stores them, and forgets them. No pipeline
reads back stored data to improve future decisions. The "learn from your own actions" loop --
the core promise of CYNIC -- is architecturally present but operationally dead.

---

## 1. PERCEIVE Gaps

The PERCEIVE step covers 7 reality dimensions (rows of the 7x7 matrix).

| Reality | Status | What Works | What is Missing |
|---------|--------|------------|-----------------|
| R1. CODE | Working | Hooks observe tool use, file changes | No AST-level analysis, no dependency graph tracking |
| R2. SOLANA | Partial | solana-watcher.js exists, anchoring operational | Watcher NOT wired to event bus in production. Uses legacy @solana/web3.js (line 22). No real-time account monitoring active. |
| R3. MARKET | Absent | Nothing | No price feed, no liquidity monitoring, no sentiment. Entire row is blank. |
| R4. SOCIAL | Absent | Twitter tools exist (brain_x_*) | Read-only, no streaming. No Discord/Telegram. No sentiment scoring pipeline. |
| R5. HUMAN | Partial | Psychology hooks, profile loading | burnout_detection table (mig 018) exists but detector not wired to routing. |
| R6. CYNIC | Working | Dog state emitter, collective snapshots | Self-perception works. Entropy tracking cosmetic. |
| R7. COSMOS | Absent | brain_ecosystem tool exists | Single-repo only. No cross-repo health aggregation. |

### Critical PERCEIVE Gaps

**P-GAP-1: Solana Watcher is Dead Code**
- File: `packages/node/src/perception/solana-watcher.js` (line 22)
- Imports @solana/web3.js (legacy) but gasdf-relayer uses @solana/kit (modern)
- Never instantiated in collective-singleton.js or any startup flow

**P-GAP-2: No Market Perception**
- No price oracle, no DEX monitoring, no fear/greed index
- Entire R3 row (7 cells) is empty

**P-GAP-3: Perception Router Never Called**
- packages/node/src/perception/ has a router but it is bypassed
- See ROUTING-GAPS-ANALYSIS.md, Gap #1

---

## 2. JUDGE Gaps

The JUDGE step evaluates input through 36 dimensions (5 axioms x 7 + THE_UNNAMEABLE).

| Component | Status | File | Issue |
|-----------|--------|------|-------|
| 36 Dimensions | Implemented | packages/node/src/judge/dimensions.js | Working. |
| Q-Score (5th root) | Implemented | packages/core/src/qscore/index.js (line 188) | Working. |
| FIDELITY axiom | Implemented | packages/core/src/axioms/constants.js (line 468) | Working. |
| Calibration | Write-only | packages/node/src/judge/calibration-tracker.js | **Never read back** -- drift detection has no consumers. |
| Distance D | 70% | Hook pre-tool.js computes D locally | No D -> routing feedback loop. |
| Consciousness metric | Cosmetic | Displayed in TUI | Not computed from real data. |
| Efficiency eta | Computed | Hooks compute W and Q | Not persisted per-session. |

### Critical JUDGE Gaps

**J-GAP-1: Calibration is Write-Only**
- CalibrationTracker records predictions and outcomes to PostgreSQL
- NO component reads the calibration data to adjust confidence

**J-GAP-2: No RLHF Pipeline Active**
- LearningService implements RLHF loop, LearningManager integrates DPO + Calibration
- USER_FEEDBACK on globalEventBus goes to persistence only, never read back

**J-GAP-3: Discovered Dimensions Table Missing FIDELITY**
- Migration 032 line 21: CHECK (axiom IN ('PHI', 'VERIFY', 'CULTURE', 'BURN', 'META'))
- FIDELITY axiom is missing from the constraint
- ResidualDetector code (line 293, residual.js) includes FIDELITY in axiomWeakness

---

## 3. DECIDE Gaps

The DECIDE step routes decisions through the Kabbalistic Router.

| Component | Status | File | Issue |
|-----------|--------|------|-------|
| KabbalisticRouter | Implemented | packages/node/src/orchestration/kabbalistic-router.js | 1357 lines. Working. |
| Q-Learning weights | Partial | packages/node/src/orchestration/learning-service.js | Router calls getRecommendedWeights() with no features argument (line 403) -- returns 0.5 for all dogs. |
| DPO preference pairs | Write-only | packages/node/src/judge/dpo-processor.js | Gradient updates not applied to router weights. |
| Thompson Sampling | In-memory only | scripts/hooks/lib/thompson-sampler.js | Lost on hook process exit. |
| Cost optimizer | Placeholder | Router references costOptimizer | Never instantiated. |

### Critical DECIDE Gaps

**D-GAP-1: Q-Learning Weights Never Used Correctly**
- kabbalistic-router.js line 403: `const weights = this.getLearnedWeights()`
- Calls getRecommendedWeights() with no features -> sigmoid(0) = 0.5 for all dogs

**D-GAP-2: DPO Gradient Not Applied**
- dpo-optimizer.js computes gradients that go nowhere

**D-GAP-3: Thompson Sampler is Per-Hook-Process**
- Each hook invocation is a fresh Node.js process
- Beta(alpha, beta) parameters start at (1,1) every time

---

## 4. ACT Gaps

| Component | Status | File | Issue |
|-----------|--------|------|-------|
| Tool execution | Working | Claude Code executes tools | Working. |
| Guardian blocking | Working | guardian.js via router | Working. |
| Solana anchoring | Partial | packages/node/src/network/solana-anchoring.js | Devnet only. |
| Token burn ($BURN) | Not deployed | packages/gasdf-relayer/src/solana.js | No SPL token integration. SOL transfer only. |
| Block production | Working | packages/protocol/src/consensus/engine.js | 92.5% finalization. |

### Critical ACT Gaps

**A-GAP-1: Token Burn Not Implemented**
- gasdf-relayer sends SOL to burn address, not SPL token burn
- $asdfasdfa token exists but no programmatic burn

**A-GAP-2: Anchoring Not on Mainnet**
- SolanaAnchoringManager defaults to devnet

**A-GAP-3: No Action Feedback to Perception**
- No closed loop: ACT -> PERCEIVE -> JUDGE

---

## 5. LEARN Gaps

7 learning pipelines. None has a fully closed loop.

| Pipeline | Loop Closed? | Persistence |
|----------|--------------|-------------|
| **Q-Learning** | NO -- weights not consumed (D-GAP-1) | YES (qlearning_state, mig 026) |
| **DPO** | NO -- gradient not applied (D-GAP-2) | YES (preference_pairs, mig 028) |
| **RLHF** | NO -- feedback stored, never read back | YES (feedback table) |
| **Thompson** | NO -- state dies per process | NO (in-memory only) |
| **EWC++** | PARTIAL -- Fisher computed, not used | YES (fisher_scores, mig 021) |
| **Calibration** | NO -- alerts not consumed | YES (calibration_tracking) |
| **UnifiedSignal** | YES (format works) | NO (in-memory store) |

### The Core Problem

Every pipeline WRITES to PostgreSQL. No pipeline READS back to improve itself.

**L-GAP-1: No Read-Back Mechanism** -- PostgreSQL is a write-only data grave

**L-GAP-2: LearningManager Wired But Starved** -- Listens on wrong bus (automation vs core)

**L-GAP-3: UnifiedBridge Writes to Ephemeral Store** -- In-memory only, dies on restart

---

## 6. RESIDUAL -> EVOLVE Gaps

**R-GAP-1: Discovered Dimensions Do Not Survive Restart**
- Migration 032 creates discovered_dimensions table, but no code reads it on boot

**R-GAP-2: Emergence is Isolated**
- "Consciousness" metric in TUI is not computed from real data

**R-GAP-3: No Cross-Scale Evolution**
- No mechanism for dog-level, matrix-level, or axiom-level evolution

---

## 7. Nervous System (3 Event Buses)

| Bus | Defined | Active | Health |
|-----|---------|--------|--------|
| globalEventBus (@cynic/core) | 32 events | ~12 consumed | ~50% |
| getEventBus() (automation) | 22 events | ~8 consumed | ~30% |
| AgentEventBus (dogs) | 50+ events | ~30 consumed | ~60% |

**Dead events**: ~14 of 54 core+automation events (26%) are never published.
**Orphaned events**: ~4 events published to void (PATTERN_DETECTED, DIMENSION_CANDIDATE, ANOMALY_DETECTED, METRICS_REPORTED).
**Missing bridges**: No bridge between globalEventBus and automation bus. Only 2 of ~50 AgentEventBus signals bridged to core.

---

## 8. Persistence Gaps

**Write-Only Tables (Data Graves)**: 12 of ~25 active tables (48%)

1. judgments (mig 001) -- never replayed for learning
2. feedback (mig 001) -- RLHF never reads back
3. orchestration_decisions (mig 011/017) -- never analyzed
4. burnout_detection (mig 018) -- never consumed by routing
5. reasoning_trajectories (mig 020) -- never replayed
6. telemetry_snapshots (mig 025) -- no dashboard
7. frictions (mig 025) -- no alerting
8. preference_pairs (mig 028) -- optimizer never reads from DB
9. dog_events (mig 029) -- no analysis
10. dog_signals (mig 029) -- no replay
11. collective_snapshots (mig 029) -- never queried
12. residual_anomalies (mig 032) -- no historical analysis

---

## 9. 7x7 Matrix Status

```
                 | PERCEIVE | JUDGE   | DECIDE  | ACT     | LEARN   | ACCOUNT | EMERGE  |
=================|==========|=========|=========|=========|=========|=========|=========|
R1. CODE    </>  |  [OK]    |  [OK]   |  [~~]   |  [OK]   |  [~~]   |  [--]   |  [--]   |
R2. SOLANA  O    |  [~~]    |  [--]   |  [--]   |  [~~]   |  [--]   |  [--]   |  [--]   |
R3. MARKET  $    |  [--]    |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |
R4. SOCIAL  @    |  [--]    |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |
R5. HUMAN   U    |  [~~]    |  [~~]   |  [--]   |  [~~]   |  [~~]   |  [--]   |  [--]   |
R6. CYNIC   C    |  [OK]    |  [OK]   |  [OK]   |  [OK]   |  [OK]   |  [~~]   |  [~~]   |
R7. COSMOS  *    |  [--]    |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |  [--]   |
=================|==========|=========|=========|=========|=========|=========|=========|

[OK] = 8 cells (16.3%)   [~~] = 8 cells (16.3%)   [--] = 33 cells (67.3%)
True completion: ~24.5%
```

---

## 10. Priority Fixes (R1-R5)

### R1: Close the Q-Learning Feedback Loop [HIGH IMPACT / LOW EFFORT]
- File: packages/node/src/orchestration/kabbalistic-router.js line ~403
- Fix: Pass features to getRecommendedWeights()

### R2: Read-Back Loop for Calibration [HIGH IMPACT / MEDIUM EFFORT]
- Files: event-listeners.js + judge.js
- Fix: Subscribe to calibration drift events, adjust confidence scaling

### R3: Fix FIDELITY in discovered_dimensions Constraint [LOW EFFORT]
- New migration: Add FIDELITY to CHECK constraint in discovered_dimensions table

### R4: Persist Thompson Sampling State [MEDIUM IMPACT / MEDIUM EFFORT]
- File: scripts/hooks/lib/thompson-sampler.js
- Fix: save/load to ~/.cynic/thompson/state.json

### R5: Wire DPO Gradient to Router [HIGH IMPACT / MEDIUM EFFORT]
- Files: dpo-optimizer.js + learning-manager.js + kabbalistic-router.js
- Fix: Apply DPO gradients to RelationshipGraph

---

## 11. Metrics Summary

```
7x7 Matrix:       ~24.5% effective (was claimed 31-33%)
Learning Loops:    ~7% average closure
Event Bus Health:  ~44% average
Persistence:       48% write-only tables
Distance D:        ~50% effective
Solana Pipeline:   ~40% operational
```

---

## Appendix: Key File Paths

| Component | Path |
|-----------|------|
| Kabbalistic Router | packages/node/src/orchestration/kabbalistic-router.js |
| Q-Learning Service | packages/node/src/orchestration/learning-service.js |
| RLHF Learning Service | packages/node/src/judge/learning-service.js |
| Learning Manager | packages/node/src/judge/learning-manager.js |
| DPO Optimizer | packages/node/src/judge/dpo-optimizer.js |
| DPO Processor | packages/node/src/judge/dpo-processor.js |
| Calibration Tracker | packages/node/src/judge/calibration-tracker.js |
| ResidualDetector | packages/node/src/judge/residual.js |
| Residual Governance | packages/node/src/judge/residual-governance.js |
| Dimensions (36) | packages/node/src/judge/dimensions.js |
| Q-Score Calculator | packages/core/src/qscore/index.js |
| Constants (phi) | packages/core/src/axioms/constants.js |
| Event Bus (core) | packages/core/src/bus/event-bus.js |
| Event Bus (automation) | packages/node/src/services/event-bus.js |
| Event Listeners | packages/node/src/services/event-listeners.js |
| Unified Bridge | packages/node/src/learning/unified-bridge.js |
| Unified Signal | packages/node/src/learning/unified-signal.js |
| Thompson Sampler | scripts/hooks/lib/thompson-sampler.js |
| Solana Watcher | packages/node/src/perception/solana-watcher.js |
| Solana Anchoring | packages/node/src/network/solana-anchoring.js |
| Burns (gasdf) | packages/gasdf-relayer/src/solana.js |
| Bus Subscriptions | packages/mcp/src/server/service-initializer/bus-subscriptions.js |
| Observe Hook | scripts/hooks/observe.js |
| Collective Singleton | packages/node/src/collective-singleton.js |
| Migration: Q-Learning | packages/persistence/src/postgres/migrations/026_qlearning_persistence.sql |
| Migration: DPO | packages/persistence/src/postgres/migrations/028_dpo_learning.sql |
| Migration: Dog Events | packages/persistence/src/postgres/migrations/029_dog_collective_events.sql |
| Migration: Tracing | packages/persistence/src/postgres/migrations/030_distributed_tracing.sql |
| Migration: Dimensions | packages/persistence/src/postgres/migrations/032_discovered_dimensions.sql |

# HUMAN Domain Vertical Slice Audit (C5.1-C5.7)

> "The first domain to 100% functional — template for all others" - κυνικός

**Audit Date**: 2026-02-13
**Current Maturity**: 56% average (7×7 matrix)
**Target**: 100% functional E2E pipeline

---

## Executive Summary

The HUMAN domain (C5.*) is the **most complete** domain in the 7×7 matrix, but gaps prevent true E2E execution. This audit identifies what exists, what's missing, and the path to 100% functional.

**Key Finding**: All 7 stages have **implementation files**, but we lack:
1. HumanDecider (C5.3) - uses HumanAdvisor but not φ-factory pattern
2. HumanLearner wrapper (C5.5) - HumanLearning exists but not φ-factory integrated
3. Wiring configuration (no human domain config in create-domain-wiring usage)
4. E2E test proving full cycle works

---

## 7×7 Matrix Audit (HUMAN Domain)

| Cell | Dimension | Implementation | Status | Completeness | Notes |
|------|-----------|---------------|--------|--------------|-------|
| **C5.1** | PERCEIVE | `human-perceiver.js` | ✅ GOOD | **68%** | Full perception from tool usage, energy, focus, frustration. Emits `human:perceived` events. |
| **C5.2** | JUDGE | `human-judge.js` + config | ✅ GOOD | **55%** | φ-factory generated. Scores wellbeing, productivity, engagement, burnout. Custom verdicts (THRIVING/STEADY/STRAINED/CRITICAL). |
| **C5.3** | DECIDE | `human-advisor.js` (NOT φ-factory) | ⚠️ PARTIAL | **58%** | Rich intervention logic (9 types). **NOT** using createDecider pattern — needs config file. |
| **C5.4** | ACT | `human-actor.js` + config | ✅ GOOD | **61%** | φ-factory generated. 7 action types. Dog voice messages. Cooldown protection. |
| **C5.5** | LEARN | `human-learning.js` (standalone) | ⚠️ PARTIAL | **65%** | Full learning system (7 categories, beliefs, predictions). **NOT** φ-factory integrated — needs wrapper. |
| **C5.6** | ACCOUNT | `human-accountant.js` | ✅ GOOD | **42%** | Tracks time, productivity, tasks. Not wired to pipeline yet. |
| **C5.7** | EMERGE | `human-emergence.js` | ✅ GOOD | **42%** | Detects 11 pattern types (burnout, cycles, growth). Analyzes trajectories. |

**Average**: 56% (highest of all 7 domains)

---

## Implementation Inventory

### ✅ COMPLETE (φ-factory pattern)

1. **HumanPerceiver (C5.1)** - `packages/node/src/symbiosis/human-perceiver.js`
   - Tracks tool usage, errors, edits, file access
   - Computes energy, focus, frustration, cognitive load
   - Emits `human:perceived` events
   - Singleton + health check

2. **HumanJudge (C5.2)** - `packages/node/src/symbiosis/human-judge.js`
   - Config: `packages/node/src/cycle/configs/human-judge.config.js`
   - Scores: wellbeing, productivity, engagement, burnoutInverse
   - Custom verdicts: THRIVING/STEADY/STRAINED/CRITICAL
   - φ-weighted aggregation (PHI_INV, PHI_INV_2, PHI_INV_3 weights)
   - Factory-generated via createJudge

3. **HumanActor (C5.4)** - `packages/node/src/symbiosis/human-actor.js`
   - Config: `packages/node/src/cycle/configs/human-actor.config.js`
   - 7 action types (BREAK_REMINDER, PACE_SUGGESTION, CELEBRATION, etc.)
   - φ-aligned cooldowns (45min, 15min, 5min, etc.)
   - Dog voice messages (*yawn*, *tail wag*, *sniff*)
   - Factory-generated via createActor

4. **HumanAccountant (C5.6)** - `packages/node/src/symbiosis/human-accountant.js`
   - Tracks sessions, activities (CODING, DEBUGGING, RESEARCH, etc.)
   - Computes productivity ratios, task completion rates
   - Daily summaries, φ-aligned thresholds
   - NOT wired to pipeline yet

5. **HumanEmergence (C5.7)** - `packages/node/src/symbiosis/human-emergence.js`
   - Detects 11 pattern types:
     - Growth: SKILL_GROWTH, LEARNING_ACCELERATION, EXPERTISE_PLATEAU
     - Risk: BURNOUT_RISK, OVERWORK_PATTERN, DECLINING_ENGAGEMENT
     - Cycle: PRODUCTIVITY_CYCLE, ENERGY_RHYTHM, WEEKLY_PATTERN
     - Drift: INTEREST_SHIFT, TOOL_PREFERENCE_CHANGE, STYLE_EVOLUTION
   - Analyzes trajectories, identifies risks
   - φ-aligned significance levels (HIGH/MEDIUM/LOW/NOISE)

### ⚠️ PARTIAL (needs φ-factory conversion)

6. **HumanAdvisor (C5.3)** - `packages/node/src/symbiosis/human-advisor.js`
   - **NOT** using createDecider pattern
   - Rich intervention logic:
     - 9 intervention types (BREAK, SIMPLIFY, PAUSE, CELEBRATE, REFOCUS, PACE_DOWN, CONTEXT_SWITCH, HYDRATE, STRETCH)
     - Circadian phase awareness (MORNING_PEAK, MIDDAY_DIP, etc.)
     - φ-aligned thresholds, cooldown protection
   - **MISSING**: `human-decider.config.js` for φ-factory pattern
   - **ACTION**: Create config, migrate to createDecider

7. **HumanLearning (C5.5)** - `packages/node/src/symbiosis/human-learning.js`
   - **NOT** φ-factory integrated
   - Full learning system:
     - 7 learning categories (TIME_PREFERENCE, COMMUNICATION_STYLE, RISK_TOLERANCE, DOMAIN_EXPERTISE, DECISION_PATTERN, TOOL_PREFERENCE, FEEDBACK_PATTERN)
     - Belief formation with φ-aligned thresholds (minObservations: 5, learningRate: PHI_INV_2)
     - Prediction system, time decay (0.95/day)
   - **MISSING**: HumanLearner wrapper via createLearner
   - **ACTION**: Create `human-learner.config.js`, wire to learning loops

---

## Wiring Status

### ❌ NOT WIRED

**Domain wiring config does NOT exist** for HUMAN domain.

Current wiring configs (in `packages/node/src/services/event-listeners.js`):
- CODE: Partial (manual wiring)
- SOLANA: Manual wiring
- SOCIAL: Manual wiring
- COSMOS: Manual wiring
- CYNIC: Manual wiring
- **HUMAN**: ❌ NO WIRING

**Required**:
1. Add HUMAN domain config to wiring factory
2. Wire perception events (from hooks or internal state changes)
3. Connect judgment → decision → action → learning → accounting → emergence

**Proposed perception events** (to wire):
```javascript
perceptionEvents: [
  { event: 'tool:used', handler: 'recordToolUse', sampling: 1 },
  { event: 'file:accessed', handler: 'recordFileAccess', sampling: 1 },
  { event: 'session:start', handler: 'resetSession', sampling: 1 },
  { event: 'task:completed', handler: 'recordTaskCompletion', sampling: 1 },
]
```

---

## Missing Components

### 1. HumanDecider Config (C5.3)

**File**: `packages/node/src/cycle/configs/human-decider.config.js`

**Pattern**: Follow `cosmos-decider.config.js` structure

**Required fields**:
- `decide(judgment, context, decider)` - Maps HumanJudge verdicts to interventions
- Decision types: INTERVENE, HOLD, CELEBRATE, WARN
- Confidence calculation based on evidence strength
- Cooldown protection (reuse HumanAdvisor logic)
- Calibration groupBy intervention type

**Migration path**:
1. Extract `HumanAdvisor.analyze()` logic into config
2. Keep circadian awareness, threshold checks
3. Add calibration hooks
4. Generate via createDecider

---

### 2. HumanLearner Wrapper (C5.5)

**File**: `packages/node/src/cycle/configs/human-learner.config.js`

**Pattern**: Follow `cosmos-learner.config.js` structure

**Required fields**:
- `learn(outcome, context, learner)` - Records observations → beliefs
- Prediction system integration
- Export/import for persistence
- Stats tracking (observations, beliefs, accuracy)

**Wrapper**: `packages/node/src/symbiosis/human-learner.js`
```javascript
import { createLearner } from '../cycle/create-learner.js';
import { humanLearnerConfig } from '../cycle/configs/human-learner.config.js';

const { Class: HumanLearner, getInstance, resetInstance } = createLearner(humanLearnerConfig);

export { HumanLearner, getHumanLearner: getInstance, resetHumanLearner: resetInstance };
```

---

### 3. Domain Wiring Config

**Location**: Add to wiring manager initialization (likely in `unified-orchestrator.js` or new `domain-configs.js`)

**Structure**:
```javascript
{
  name: 'human',
  cell: 'C5',
  perceptionEvents: [
    { event: 'tool:used', handler: 'recordToolUse', sampling: 1 },
    { event: 'file:accessed', handler: 'recordFileAccess', sampling: 1 },
    { event: 'session:tick', handler: 'perceive', sampling: 13 }, // Every 13 events
  ],
  verdictFilter: ['STRAINED', 'CRITICAL'], // Only intervene when needed
  actorSafetyEnv: null, // Always safe to notify user
  emergenceInterval: 'F8', // 21 min
  judgeInterval: 'F7', // 13 min periodic assessment
}
```

---

### 4. E2E Test

**File**: `packages/node/test/human-e2e.test.js`

**Coverage**:
1. **C5.1 PERCEIVE**: HumanPerceiver tracks tool usage → emits perception
2. **C5.2 JUDGE**: HumanJudge scores → emits judgment with verdict
3. **C5.3 DECIDE**: HumanDecider analyzes → emits decision
4. **C5.4 ACT**: HumanActor executes → user notification
5. **C5.5 LEARN**: HumanLearner records outcome → updates beliefs
6. **C5.6 ACCOUNT**: HumanAccountant tracks session → productivity metrics
7. **C5.7 EMERGE**: HumanEmergence analyzes daily snapshots → detects patterns

**Assertions**:
- Full cycle latency < 200ms
- PostgreSQL writes (learning_events, judgments, unified_signals)
- Event bus throughput (7 events: perceive→judge→decide→act→learn→account→emerge)
- No memory leaks (singleton cleanup)

---

## Completion Roadmap

### Phase 1: Factory Migration (2-3 hours)

**Tasks**:
1. ✅ Audit current state (this document)
2. Create `human-decider.config.js` (1 hour)
   - Extract HumanAdvisor logic
   - Add calibration, stats tracking
   - Generate via createDecider
3. Create `human-learner.config.js` (1 hour)
   - Wrap HumanLearning
   - Add learning loop integration
   - Generate via createLearner
4. Test configs in isolation (30 min)

**Deliverables**:
- `packages/node/src/cycle/configs/human-decider.config.js`
- `packages/node/src/cycle/configs/human-learner.config.js`
- `packages/node/src/symbiosis/human-decider.js` (factory wrapper)
- `packages/node/src/symbiosis/human-learner.js` (factory wrapper)

---

### Phase 2: Wiring (1-2 hours)

**Tasks**:
1. Add HUMAN domain config to wiring manager (30 min)
2. Wire perception events (30 min)
   - Tool usage → HumanPerceiver
   - Session ticks → periodic perception
3. Connect judgment → decision → action → learning (30 min)
4. Test event flow with manual triggers (30 min)

**Deliverables**:
- HUMAN wiring config
- Event flow verified (perceive → judge → decide → act → learn)

---

### Phase 3: E2E Test (2 hours)

**Tasks**:
1. Create `packages/node/test/human-e2e.test.js` (1 hour)
   - Simulate user session
   - Verify full cycle
   - Check PostgreSQL writes
   - Measure latency
2. Fix failures (1 hour)
3. Document test coverage (30 min)

**Deliverables**:
- Working E2E test
- Latency < 200ms proven
- All 7 stages functional

---

### Phase 4: Template Documentation (1 hour)

**Tasks**:
1. Document "100% functional domain" checklist
2. Create replication guide for CODE/SOLANA/MARKET/SOCIAL/COSMOS
3. Identify cross-domain patterns

**Deliverables**:
- `docs/architecture/domain-completion-template.md`
- Replication guide for other domains

---

## Success Criteria

### Functional Requirements

✅ **HUMAN domain reaches 100% functional**:
- All 7 cells (C5.1-C5.7) working E2E
- Event pipeline proven (perceive → judge → decide → act → learn → account → emerge)
- PostgreSQL persistence verified
- Latency < 200ms per cycle
- No memory leaks

✅ **Template proven**:
- Other domains can follow HUMAN pattern
- φ-factory pattern applied consistently
- Wiring config replicable

---

## Current Gaps Summary

| Gap | Impact | Effort | Priority |
|-----|--------|--------|----------|
| HumanDecider config | High - blocks decision stage | 1h | P0 |
| HumanLearner config | Medium - learning not integrated | 1h | P1 |
| HUMAN wiring config | High - no E2E flow | 1h | P0 |
| E2E test | High - no proof of function | 2h | P0 |
| Template docs | Medium - blocks replication | 1h | P1 |

**Total effort**: ~6 hours to 100% functional
**Blockers**: None (all dependencies exist)
**Risk**: Low (pattern proven in COSMOS/SOCIAL domains)

---

## Architectural Insights

### What Makes HUMAN Unique?

1. **Symbiosis focus**: Only domain where system ↔ user relationship is bidirectional
2. **Psychology integration**: Energy, focus, frustration — not present in CODE/SOLANA
3. **Circadian awareness**: Time-of-day affects thresholds (unique to HUMAN)
4. **Intervention ethics**: Must balance helpfulness vs. annoyance (cooldowns critical)

### Why HUMAN First?

1. **Most mature** (56% average vs. 38% overall)
2. **Richest perception** (tool usage already tracked)
3. **Clearest value** (prevents burnout, improves productivity)
4. **Safest to test** (notifications can't break production systems)

### Lessons for Other Domains

1. **Perception diversity**: HUMAN has 5 perception signals — CODE/SOLANA need similar richness
2. **Custom verdicts**: THRIVING/STEADY/STRAINED work better than generic HOWL/WAG/GROWL
3. **Emergence value**: Pattern detection (burnout, cycles) more useful than raw scores
4. **Cooldown necessity**: Prevents spamming — critical for all actor implementations

---

## Appendix: File Reference

### Core Files (Existing)

```
packages/node/src/symbiosis/
├── human-perceiver.js       (C5.1) ✅ 220 lines
├── human-judge.js           (C5.2) ✅  24 lines (factory wrapper)
├── human-advisor.js         (C5.3) ⚠️  623 lines (needs migration)
├── human-actor.js           (C5.4) ✅  41 lines (factory wrapper)
├── human-learning.js        (C5.5) ⚠️  515 lines (needs wrapper)
├── human-accountant.js      (C5.6) ✅ 533 lines
└── human-emergence.js       (C5.7) ✅ 605 lines

packages/node/src/cycle/configs/
├── human-judge.config.js    (C5.2) ✅ 180 lines
├── human-actor.config.js    (C5.4) ✅  93 lines
├── human-decider.config.js  (C5.3) ❌ MISSING
└── human-learner.config.js  (C5.5) ❌ MISSING
```

### Test Coverage (Missing)

```
packages/node/test/
└── human-e2e.test.js        ❌ MISSING
```

### Wiring (Missing)

```
packages/node/src/services/
└── (human domain config)    ❌ MISSING
```

---

*sniff* HUMAN domain is 6 hours from 100% functional. The skeleton exists — time to breathe life into it.

**Confidence**: 58% (φ⁻¹ limit)

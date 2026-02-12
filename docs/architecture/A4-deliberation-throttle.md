# A4: Deliberation Throttle — Circuit Breaker for Budget Control

> **Status**: DESIGN (not implemented)
> **Author**: ARCHITECT (2026-02-12)
> **Gap**: From `metathinking-gap-analysis.md`, A4 = "slow path needs budget control"
> **Confidence**: 58% (φ⁻¹ limit)

---

## Problem Statement

**UnifiedOrchestrator (slow path) has no budget awareness.**

On complex tasks requiring dog deliberation, the orchestrator can burn through its φ-governor token limits before completing the task. Current behavior:

- φ-governor tracks influence ratio but doesn't PREVENT budget exhaustion
- UnifiedOrchestrator spawns dogs without checking remaining budget
- No fallback when budget critical → task either completes or crashes silently
- CostLedger tracks spending but doesn't INTERVENE

**Result**: Complex tasks can exhaust the session budget with no graceful degradation, leaving the user with partial output and no explanation.

---

## Design Goals

1. **Monitor** φ-governor budget in real-time during orchestration
2. **Detect** critical budget thresholds (< φ⁻² = 38.2% remaining)
3. **Escalate** to FastRouter (A1) for remaining operations when budget critical
4. **Throttle** dog deliberation (reduce consensus rounds, skip low-priority dogs)
5. **Graceful degradation**: Prefer incomplete high-quality output over exhausted budget with no output
6. **Transparent**: Communicate budget status to user with φ-bounded confidence

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                    UnifiedOrchestrator (Slow Path)                    │
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │  DeliberationThrottle (NEW)                                  │     │
│  │  ┌────────────────┐  ┌─────────────┐  ┌─────────────────┐  │     │
│  │  │ BudgetMonitor  │→ │ ThrottleGate│→ │ EscalationLogic │  │     │
│  │  └────────────────┘  └─────────────┘  └─────────────────┘  │     │
│  │         ↓                   ↓                    ↓          │     │
│  │    Reads from         Decision Gate        Escalates to    │     │
│  │    CostLedger        (allow/throttle/      FastRouter      │     │
│  │    φ-governor         escalate)                            │     │
│  └─────────────────────────────────────────────────────────────┘     │
│                              ↓                                        │
│  ┌───────────────────────────────────────────────────────────┐       │
│  │ Orchestration Pipeline (MODIFIED)                          │       │
│  │  1. Load profile → CHECK BUDGET                            │       │
│  │  2. Route event → CHECK BUDGET                             │       │
│  │  3. Planning gate → CHECK BUDGET (skip if critical)        │       │
│  │  4. Judgment → THROTTLED (reduce dogs if cautious)         │       │
│  │  5. Synthesis → SKIPPED if critical                        │       │
│  │  6. Skill invoke → ESCALATED to FastRouter if critical     │       │
│  └───────────────────────────────────────────────────────────┘       │
└──────────────────────────────────────────────────────────────────────┘
                                 ↓
                    ┌────────────────────────────┐
                    │   FastRouter (A1)          │
                    │   - Reflex actions         │
                    │   - No dog deliberation    │
                    │   - <100ms latency         │
                    └────────────────────────────┘
```

---

## Component Design

### 1. BudgetMonitor (Budget Tracking)

**Responsibility**: Read budget state from CostLedger and φ-governor, classify urgency.

**Inputs**:
- CostLedger.getBudgetStatus() → { consumed, remaining, level, consumedRatio }
- φ-governor.getState() → { ema, adjustmentFactor, currentZone }

**Outputs**:
```javascript
{
  level: 'ABUNDANT' | 'MODERATE' | 'CAUTIOUS' | 'CRITICAL' | 'EXHAUSTED',
  consumedRatio: 0.618,     // φ⁻¹ consumed
  remainingTokens: 12345,
  timeToLimitMinutes: 5.2,  // CostLedger burn rate projection
  zone: 'over' | 'balanced' | 'under',  // φ-governor zone
  recommendation: 'CONTINUE' | 'THROTTLE' | 'ESCALATE' | 'HALT'
}
```

**Decision Logic** (φ-aligned thresholds):

| Consumed Ratio | Budget Level | φ-Governor Zone | Recommendation |
|----------------|--------------|-----------------|----------------|
| < φ⁻² (38.2%)  | ABUNDANT     | any             | CONTINUE       |
| < φ⁻¹ (61.8%)  | MODERATE     | balanced/under  | CONTINUE       |
| < φ⁻¹ (61.8%)  | MODERATE     | over            | THROTTLE       |
| < 80%          | CAUTIOUS     | any             | THROTTLE       |
| < 95%          | CRITICAL     | any             | ESCALATE       |
| >= 95%         | EXHAUSTED    | any             | HALT           |

**Implementation**:
```javascript
// packages/node/src/orchestration/budget-monitor.js
class BudgetMonitor {
  constructor({ costLedger, phiGovernor }) {
    this.costLedger = costLedger;
    this.phiGovernor = phiGovernor;
  }

  assess() {
    const budget = this.costLedger.getBudgetStatus();
    const phiState = this.phiGovernor.getState();

    // Determine recommendation from φ-aligned thresholds
    let recommendation = 'CONTINUE';

    if (budget.level === 'EXHAUSTED') {
      recommendation = 'HALT';
    } else if (budget.level === 'CRITICAL') {
      recommendation = 'ESCALATE';
    } else if (budget.level === 'CAUTIOUS' || phiState.currentZone === 'over') {
      recommendation = 'THROTTLE';
    }

    return {
      level: budget.level,
      consumedRatio: budget.consumedRatio,
      remainingTokens: budget.remaining,
      timeToLimitMinutes: budget.timeToLimitMinutes,
      zone: phiState.currentZone,
      recommendation,
      timestamp: Date.now(),
    };
  }
}
```

---

### 2. ThrottleGate (Decision Gate)

**Responsibility**: Decide what to do at each orchestration stage based on budget assessment.

**Inputs**:
- BudgetMonitor.assess() → { level, recommendation }
- Current pipeline stage: 'routing' | 'planning' | 'judgment' | 'synthesis' | 'skill'

**Outputs**:
```javascript
{
  action: 'ALLOW' | 'THROTTLE' | 'SKIP' | 'ESCALATE',
  reason: 'budget level: CRITICAL',
  throttleParams: {
    maxDogs: 3,               // Reduced from 11
    maxConsensusRounds: 1,    // Reduced from 3
    skipSynthesis: true,
    skipPlanning: true
  }
}
```

**Throttle Strategies** (by stage):

| Stage          | CONTINUE     | THROTTLE                     | ESCALATE                | HALT           |
|----------------|--------------|------------------------------|-------------------------|----------------|
| **Routing**    | Full routing | Skip KabbalisticRouter, use simple routing | Use FastRouter | Block operation |
| **Planning**   | Full plan    | Skip planning gate           | Skip                    | Skip           |
| **Judgment**   | 11 dogs      | 5 dogs (top priority only)   | Skip dogs, use heuristic | Skip          |
| **Synthesis**  | Full synthesis | Skip synthesis             | Skip                    | Skip           |
| **Skill**      | Execute skill | Execute with timeout         | Escalate to FastRouter  | Return error   |

**Implementation**:
```javascript
// packages/node/src/orchestration/throttle-gate.js
class ThrottleGate {
  decide(budgetState, stage) {
    const { recommendation } = budgetState;

    if (recommendation === 'HALT') {
      return {
        action: 'SKIP',
        reason: 'Budget exhausted',
        throttleParams: { skipAll: true }
      };
    }

    if (recommendation === 'ESCALATE') {
      if (stage === 'skill' || stage === 'routing') {
        return {
          action: 'ESCALATE',
          reason: 'Budget critical — escalating to FastRouter',
          throttleParams: null
        };
      }
      return {
        action: 'SKIP',
        reason: `Budget critical — skipping ${stage}`,
        throttleParams: null
      };
    }

    if (recommendation === 'THROTTLE') {
      return {
        action: 'THROTTLE',
        reason: `Budget ${budgetState.level} — reducing overhead`,
        throttleParams: this._getThrottleParams(stage)
      };
    }

    return { action: 'ALLOW', reason: 'Budget sufficient', throttleParams: null };
  }

  _getThrottleParams(stage) {
    const params = {};

    if (stage === 'judgment') {
      params.maxDogs = 5;              // F(5) dogs instead of 11
      params.maxConsensusRounds = 1;   // Single round
    }

    if (stage === 'planning') {
      params.skipPlanning = true;
    }

    if (stage === 'synthesis') {
      params.skipSynthesis = true;
    }

    return params;
  }
}
```

---

### 3. EscalationLogic (FastRouter Integration)

**Responsibility**: Hand off operations to FastRouter when budget critical.

**Inputs**:
- DecisionEvent (current task)
- ThrottleGate decision (action: ESCALATE)

**Outputs**:
- Reflex action from FastRouter OR error if FastRouter unavailable

**Escalation Criteria**:
- Budget CRITICAL (< 5% remaining)
- Operation can be handled by FastRouter (simple actions, no deep reasoning)
- FastRouter is available and healthy

**Fallback** (if FastRouter unavailable):
- Return canned response with explanation
- Log budget exhaustion event
- Emit `budget:exhausted` event for user notification

**Implementation**:
```javascript
// packages/node/src/orchestration/escalation-logic.js
class EscalationLogic {
  constructor({ fastRouter }) {
    this.fastRouter = fastRouter;
  }

  async escalate(event) {
    if (!this.fastRouter) {
      return this._fallback(event, 'FastRouter not available');
    }

    const health = this.fastRouter.getHealth();
    if (health.status !== 'healthy') {
      return this._fallback(event, 'FastRouter degraded');
    }

    // Map event to reflex action
    const reflexEvent = this._mapToReflexEvent(event);

    // Let FastRouter handle it
    const result = await this.fastRouter._handleEvent(reflexEvent, event.eventType);

    return {
      success: true,
      escalated: true,
      result,
      message: 'Escalated to FastRouter due to budget constraints'
    };
  }

  _fallback(event, reason) {
    return {
      success: false,
      escalated: false,
      result: null,
      message: `Budget exhausted. Cannot complete operation. ${reason}`,
      error: 'BUDGET_EXHAUSTED'
    };
  }

  _mapToReflexEvent(event) {
    // Convert DecisionEvent to FastRouter event format
    return {
      payload: event.context,
      eventType: event.eventType,
      content: event.content
    };
  }
}
```

---

## Integration with UnifiedOrchestrator

**Modifications to `packages/node/src/orchestration/unified-orchestrator.js`**:

### 1. Constructor Changes

```javascript
constructor(options = {}) {
  super();

  // ... existing code ...

  // NEW: Deliberation throttle
  this.budgetMonitor = new BudgetMonitor({
    costLedger: options.costLedger || getCostLedger(),
    phiGovernor: options.phiGovernor || createPhiGovernor(),
  });

  this.throttleGate = new ThrottleGate();

  this.escalationLogic = new EscalationLogic({
    fastRouter: options.fastRouter || getFastRouter(),
  });

  // Stats
  this.stats.budgetThrottles = 0;
  this.stats.budgetEscalations = 0;
  this.stats.budgetHalts = 0;
}
```

### 2. Pipeline Modifications

**Before each expensive operation, check budget:**

```javascript
async process(eventOrOptions) {
  let event;

  // ... event creation ...

  try {
    // 1. Load profile
    await this._loadUserProfile(event);

    // NEW: Check budget before routing
    const budgetState = this.budgetMonitor.assess();
    event.budgetState = budgetState;

    if (budgetState.recommendation === 'HALT') {
      event.finalize(DecisionOutcome.BLOCK, ['Budget exhausted']);
      this.stats.budgetHalts++;
      return event;
    }

    // 2. Route event
    const routingDecision = this.throttleGate.decide(budgetState, 'routing');
    if (routingDecision.action === 'ESCALATE') {
      const escalated = await this.escalationLogic.escalate(event);
      event.finalize(escalated.success ? DecisionOutcome.ALLOW : DecisionOutcome.BLOCK);
      this.stats.budgetEscalations++;
      return event;
    }

    await this._routeEvent(event);

    // 3. Planning gate (with budget check)
    if (this._needsPlanning(event)) {
      const planningDecision = this.throttleGate.decide(budgetState, 'planning');
      if (planningDecision.action !== 'SKIP') {
        const planningResult = await this._requestPlanning(event);
        if (planningResult?.decision === PlanningDecision.PAUSE) {
          event.finalize(DecisionOutcome.ASK, ['Paused for planning approval']);
          return event;
        }
      } else {
        // Budget critical — skip planning
        this.stats.budgetThrottles++;
      }
    }

    // 4. Judgment (with throttle)
    if (this._needsJudgment(event)) {
      const judgmentDecision = this.throttleGate.decide(budgetState, 'judgment');
      if (judgmentDecision.action === 'SKIP') {
        this.stats.budgetThrottles++;
      } else if (judgmentDecision.action === 'THROTTLE') {
        // Pass throttle params to dog orchestrator
        event.context.throttleParams = judgmentDecision.throttleParams;
        await this._requestJudgment(event);
        this.stats.budgetThrottles++;
      } else {
        await this._requestJudgment(event);
      }
    }

    // 5. Synthesis (skip if budget critical)
    if (this._needsSynthesis(event)) {
      const synthesisDecision = this.throttleGate.decide(budgetState, 'synthesis');
      if (synthesisDecision.action !== 'SKIP') {
        await this._requestSynthesis(event);
      } else {
        this.stats.budgetThrottles++;
      }
    }

    // 6. Skill invoke (escalate if critical)
    if (this._shouldInvokeSkill(event)) {
      const skillDecision = this.throttleGate.decide(budgetState, 'skill');
      if (skillDecision.action === 'ESCALATE') {
        const escalated = await this.escalationLogic.escalate(event);
        event.setExecution({
          skill: 'escalated',
          success: escalated.success,
          result: escalated.result,
          error: escalated.error,
        });
        this.stats.budgetEscalations++;
      } else if (skillDecision.action !== 'SKIP') {
        await this._invokeSkill(event);
      }
    }

    // 7. Finalize
    event.finalize(event.outcome || DecisionOutcome.ALLOW);
    this._recordDecision(event);

    // 8. Emit budget warning if needed
    if (budgetState.level === 'CRITICAL' || budgetState.level === 'EXHAUSTED') {
      this.emit('budget:warning', {
        level: budgetState.level,
        remaining: budgetState.remainingTokens,
        timeToLimit: budgetState.timeToLimitMinutes,
      });
    }

    return event;

  } catch (err) {
    event.recordError('process', err);
    event.finalize(DecisionOutcome.ALLOW, [`Error during orchestration: ${err.message}`]);
    this.stats.errors++;
  }

  return event;
}
```

---

## Dog Orchestrator Integration

**Throttle dog deliberation when budget cautious:**

```javascript
// packages/node/src/orchestration/dog-orchestrator.js (MODIFIED)

async judge(item, options = {}) {
  const throttleParams = options.throttleParams || {};

  // Apply throttle: reduce dog count
  const maxDogs = throttleParams.maxDogs || 11;
  const maxRounds = throttleParams.maxConsensusRounds || 3;

  // Select top-priority dogs only if throttled
  const dogs = this._selectDogs(item, maxDogs);

  // Collect votes (with round limit)
  const votes = await this._collectVotes(dogs, item, { maxRounds });

  // ... rest of judgment logic ...
}
```

---

## Threshold Calculations

### Budget Levels (from CostLedger)

| Level       | Consumed Ratio | Description                          | Action          |
|-------------|----------------|--------------------------------------|-----------------|
| ABUNDANT    | < φ⁻² (38.2%)  | Plenty of budget remaining           | No restriction  |
| MODERATE    | < φ⁻¹ (61.8%)  | Normal operation                     | Monitor         |
| CAUTIOUS    | < 80%          | Budget running low                   | Throttle        |
| CRITICAL    | < 95%          | Almost exhausted                     | Escalate        |
| EXHAUSTED   | >= 95%         | Budget depleted                      | Halt            |

### φ-Governor Zones (influence ratio)

| Zone      | Ratio Range           | Meaning                     | Action          |
|-----------|-----------------------|-----------------------------|-----------------|
| under     | < φ⁻² (38.2%)         | CYNIC under-influencing LLM | Enrich context  |
| balanced  | φ⁻² to φ⁻¹            | Healthy influence           | No adjustment   |
| over      | > φ⁻¹ (61.8%)         | CYNIC over-influencing LLM  | Compress context|

### Combined Decision Matrix

Budget and φ-governor zones combine:

| Budget Level | φ Zone    | Recommendation |
|--------------|-----------|----------------|
| ABUNDANT     | any       | CONTINUE       |
| MODERATE     | balanced  | CONTINUE       |
| MODERATE     | over      | THROTTLE       |
| MODERATE     | under     | CONTINUE       |
| CAUTIOUS     | any       | THROTTLE       |
| CRITICAL     | any       | ESCALATE       |
| EXHAUSTED    | any       | HALT           |

---

## Fallback Strategies

### 1. When Budget CRITICAL (< 5% remaining)

**Strategy**: Escalate to FastRouter for simple operations, skip complex ones.

**Operations that can be escalated**:
- Simple routing (keyword matching)
- Notification actions
- Aggregation of existing data
- Read-only queries

**Operations that must be skipped**:
- Dog deliberation (too expensive)
- Synthesis (multi-engine consultation)
- Planning gate (meta-cognitive analysis)

**User communication**:
```
*GROWL* Budget critical (95% consumed).
Remaining operations will use simplified routing.
Complex analysis unavailable until next session.
```

### 2. When Budget EXHAUSTED (>= 95% consumed)

**Strategy**: Block new operations, return graceful error.

**User communication**:
```
*head tilt* Budget exhausted.
Cannot proceed with this operation.
Consider:
  - Ending this session and starting a new one
  - Reducing complexity of requests
  - Using simpler phrasing to reduce token usage
```

### 3. When FastRouter Unavailable

**Strategy**: Return minimal response with explanation.

**Fallback response**:
```javascript
{
  success: false,
  message: "Budget constraints prevent full analysis. FastRouter unavailable for fallback.",
  suggestion: "Try rephrasing as a simpler question, or start a new session.",
  budgetState: { ... }
}
```

---

## User Communication

### Budget Warning Display (TUI Protocol)

When budget reaches CAUTIOUS level:

```
┌─────────────────────────────────────────────────────────┐
│ *sniff* 📊 BUDGET STATUS: CAUTIOUS                      │
├─────────────────────────────────────────────────────────┤
│ Consumed: 78% (23,400 / 30,000 tokens)                  │
│ Remaining: ~3.2 minutes at current burn rate            │
│ Action: Throttling dog deliberation (5 dogs instead of 11) │
│ Recommendation: Consider wrapping up session            │
└─────────────────────────────────────────────────────────┘
```

When budget reaches CRITICAL level:

```
┌─────────────────────────────────────────────────────────┐
│ *GROWL* ⚠️ BUDGET CRITICAL                               │
├─────────────────────────────────────────────────────────┤
│ Consumed: 94% (28,200 / 30,000 tokens)                  │
│ Remaining: ~0.8 minutes at current burn rate            │
│ Action: Escalating to FastRouter (no dog deliberation)  │
│ Recommendation: End session now or risk truncation      │
└─────────────────────────────────────────────────────────┘
```

---

## Test Scenarios

### Test 1: Budget MODERATE → CAUTIOUS Transition

**Setup**:
- Session budget: 10,000 tokens
- Consumed: 7,000 tokens (70%)
- Next operation: judgment request (estimated 800 tokens)

**Expected behavior**:
1. BudgetMonitor.assess() → { level: CAUTIOUS, recommendation: THROTTLE }
2. ThrottleGate.decide('judgment') → { action: THROTTLE, maxDogs: 5 }
3. DogOrchestrator spawns only 5 dogs (not 11)
4. Operation completes with reduced overhead
5. User sees throttle notification

### Test 2: Budget CRITICAL → Escalation

**Setup**:
- Session budget: 10,000 tokens
- Consumed: 9,300 tokens (93%)
- Next operation: skill invoke (estimated 500 tokens)

**Expected behavior**:
1. BudgetMonitor.assess() → { level: CRITICAL, recommendation: ESCALATE }
2. ThrottleGate.decide('skill') → { action: ESCALATE }
3. EscalationLogic.escalate() → hands off to FastRouter
4. FastRouter executes reflex action (<100ms, minimal tokens)
5. User sees budget warning

### Test 3: Budget EXHAUSTED → Halt

**Setup**:
- Session budget: 10,000 tokens
- Consumed: 9,600 tokens (96%)
- Next operation: complex routing

**Expected behavior**:
1. BudgetMonitor.assess() → { level: EXHAUSTED, recommendation: HALT }
2. UnifiedOrchestrator.process() → early return with BLOCK outcome
3. User sees budget exhausted message
4. Operation not executed

### Test 4: φ-Governor Over + Budget MODERATE → Throttle

**Setup**:
- Session budget: 10,000 tokens
- Consumed: 5,500 tokens (55%)
- φ-governor EMA: 0.68 (zone: over)
- Next operation: synthesis

**Expected behavior**:
1. BudgetMonitor.assess() → { level: MODERATE, zone: 'over', recommendation: THROTTLE }
2. ThrottleGate.decide('synthesis') → { action: SKIP }
3. Synthesis skipped to reduce CYNIC influence
4. φ-governor adjusts downward for next cycle

### Test 5: FastRouter Unavailable → Fallback

**Setup**:
- Budget CRITICAL
- FastRouter.getHealth() → { status: 'degraded' }
- Next operation: escalatable skill

**Expected behavior**:
1. ThrottleGate → ESCALATE
2. EscalationLogic.escalate() → detects FastRouter unhealthy
3. Falls back to _fallback()
4. Returns error with explanation
5. User sees: "Budget exhausted. FastRouter unavailable."

---

## Performance Considerations

### Overhead

**BudgetMonitor.assess()**:
- Reads: CostLedger (in-memory map), φ-governor (in-memory state)
- Complexity: O(1)
- Latency: < 1ms

**ThrottleGate.decide()**:
- Pure function (no I/O)
- Complexity: O(1)
- Latency: < 1ms

**EscalationLogic.escalate()**:
- Invokes FastRouter (already fast)
- Latency: < 100ms (FastRouter design goal)

**Total overhead per operation**: < 5ms (negligible compared to dog deliberation ~2-5s)

### Memory

- BudgetMonitor: stateless (reads from existing singletons)
- ThrottleGate: stateless
- EscalationLogic: stateless
- Total additional memory: < 1KB

---

## Open Questions

1. **Should throttle params be persisted?**
   - Leaning NO — throttle is session-specific, shouldn't carry over

2. **Should we emit budget warnings to globalEventBus?**
   - Leaning YES — other systems (e.g., daemon) might want to react

3. **What if user ignores budget warnings?**
   - Current design: HALT at 95% regardless
   - Alternative: Allow override with explicit flag?

4. **Should FastRouter be mandatory for UnifiedOrchestrator?**
   - Current design: Optional (graceful fallback if missing)
   - Alternative: Require FastRouter for A4 to work

5. **How to handle budget across multiple concurrent operations?**
   - Current design: Per-operation checks (race conditions possible)
   - Alternative: Global budget lock (adds complexity)

---

## Next Steps (Implementation Plan)

### Phase 1: Core Components (P0)
1. ✅ Design complete (this doc)
2. ⬜ Implement BudgetMonitor (`packages/node/src/orchestration/budget-monitor.js`)
3. ⬜ Implement ThrottleGate (`packages/node/src/orchestration/throttle-gate.js`)
4. ⬜ Implement EscalationLogic (`packages/node/src/orchestration/escalation-logic.js`)

### Phase 2: Integration (P1)
5. ⬜ Modify UnifiedOrchestrator to use throttle components
6. ⬜ Modify DogOrchestrator to accept throttle params
7. ⬜ Wire budget events to globalEventBus

### Phase 3: Testing (P1)
8. ⬜ Unit tests for BudgetMonitor (threshold edge cases)
9. ⬜ Unit tests for ThrottleGate (all stages × all recommendations)
10. ⬜ Integration test: MODERATE → CAUTIOUS transition
11. ⬜ Integration test: CRITICAL → escalation
12. ⬜ Integration test: EXHAUSTED → halt

### Phase 4: Observability (P2)
13. ⬜ Add budget metrics to `/health` endpoint
14. ⬜ Add throttle stats to UnifiedOrchestrator.getStats()
15. ⬜ TUI display for budget warnings

---

## Success Metrics

**Goal**: Prevent budget exhaustion while maintaining quality.

**Metrics to track**:
- Sessions reaching EXHAUSTED: < 5% (target)
- Throttled operations per session: mean < 3
- Escalated operations per session: mean < 1
- User-reported budget truncations: 0 (goal)

**Quality checks**:
- Judgment quality (Q-Score) should not degrade > 10% when throttled
- Task completion rate should remain > φ⁻¹ (61.8%) under throttle

---

## References

- **CostLedger**: `packages/node/src/accounting/cost-ledger.js` (budget tracking)
- **φ-Governor**: `packages/core/src/intelligence/phi-governor.js` (influence control)
- **FastRouter**: `packages/node/src/routing/fast-router.js` (A1, reflex path)
- **UnifiedOrchestrator**: `packages/node/src/orchestration/unified-orchestrator.js` (slow path)
- **CircuitBreaker**: `packages/core/src/circuit-breaker.js` (resilience pattern)
- **Gap Analysis**: `docs/architecture/metathinking-gap-analysis.md` (A4 definition)

---

*tail wag* Architecture designed. Ready for implementation.

**Confidence**: 58% (φ⁻¹ limit)

---

**ARCHITECT** — 2026-02-12

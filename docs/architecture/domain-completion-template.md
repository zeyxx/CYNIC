# Domain Completion Template

> "HUMAN proved the pattern — now replicate for CODE, SOLANA, MARKET, SOCIAL, COSMOS" - κυνικός

**Based on**: HUMAN domain (C5.*) - first to reach 100% functional
**Purpose**: Standard checklist for completing any domain in the 7×7 matrix
**Status**: Template proven in HUMAN, ready for replication

---

## Overview

A **100% functional domain** means all 7 analysis dimensions (PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE) work **end-to-end**, with:
- Events flowing through globalEventBus
- PostgreSQL persistence (optional but recommended)
- Latency < 200ms per full cycle
- φ-factory pattern applied consistently
- E2E test proving functionality

---

## Checklist (7×7 Completion)

### C*.1 - PERCEIVE

**Implementation**:
- [ ] Perceiver class created (`packages/node/src/{domain}/{domain}-perceiver.js`)
- [ ] Perception signals defined (5-10 distinct signals)
- [ ] Event emission on significant changes (via `globalEventBus.publish`)
- [ ] Singleton pattern with `getInstance` and `resetInstance`
- [ ] Health check method (`getHealth()`)

**Pattern** (from HumanPerceiver):
```javascript
export class {Domain}Perceiver extends EventEmitter {
  constructor(options = {}) {
    super();
    this._state = { /* perception state */ };
    this._stats = { perceptions: 0, signalsEmitted: 0 };
  }

  perceive() {
    this._updatePerception();
    return { ...this._state };
  }

  _updatePerception() {
    // Compute perception signals
    // Emit if significant change
    this.emit('perception', this._state);
    globalEventBus.publish('{domain}:perceived', { ...this._state, cell: 'C*.1' });
  }

  getStats() { return { ...this._stats }; }
  getHealth() { return { status, score, ... }; }
}
```

**Key files**:
- Implementation: `packages/node/src/{domain}/{domain}-perceiver.js`
- Test: `packages/node/test/{domain}-perceiver.test.js`

---

### C*.2 - JUDGE

**Implementation**:
- [ ] Judge config created (`packages/node/src/cycle/configs/{domain}-judge.config.js`)
- [ ] Factory wrapper created (`packages/node/src/{domain}/{domain}-judge.js`)
- [ ] Custom verdicts defined (or use HOWL/WAG/GROWL/BARK)
- [ ] Scoring functions for 3-5 dimensions
- [ ] φ-weighted aggregation
- [ ] Recommendations generation

**Pattern** (from HumanJudge):
```javascript
// Config file
export const {Domain}Verdict = {
  EXCELLENT: 'excellent',
  GOOD: 'good',
  POOR: 'poor',
  CRITICAL: 'critical',
};

export const {domain}JudgeConfig = {
  name: '{Domain}Judge',
  cell: 'C*.2',
  dimension: '{DOMAIN}',
  eventPrefix: '{domain}',
  judgmentTypes: { /* domain-specific types */ },
  maxHistory: 89,

  verdictInit: {Domain}Verdict,

  getVerdict(score) {
    if (score >= PHI_INV * 100) return {Domain}Verdict.EXCELLENT;
    if (score >= PHI_INV_2 * 100) return {Domain}Verdict.GOOD;
    if (score >= PHI_INV_3 * 100) return {Domain}Verdict.POOR;
    return {Domain}Verdict.CRITICAL;
  },

  score(type, data) {
    return {
      dimension1: scoreDimension1(data) * 100,
      dimension2: scoreDimension2(data) * 100,
      dimension3: scoreDimension3(data) * 100,
    };
  },

  aggregate(scores) {
    const weights = { dimension1: PHI_INV, dimension2: PHI_INV_2, dimension3: PHI_INV_3 };
    const totalWeight = PHI_INV + PHI_INV_2 + PHI_INV_3;
    let weightedSum = 0;
    for (const [key, weight] of Object.entries(weights)) {
      weightedSum += (scores[key] || 0) * weight;
    }
    return weightedSum / totalWeight;
  },

  enrichResult(result, type, data, scores) {
    result.qScore = result.score / 100;
    result.recommendations = generateRecommendations(scores, result.verdict);
  },

  healthCheck(stats) {
    return {
      status: stats.avgScore >= PHI_INV_2 * 100 ? 'healthy' : 'concerning',
      score: Math.min(PHI_INV, stats.avgScore / 100),
      judgmentsTotal: stats.totalJudgments,
    };
  },
};
```

**Wrapper file**:
```javascript
import { createJudge } from '../cycle/create-judge.js';
import { {domain}JudgeConfig, {Domain}Verdict } from '../cycle/configs/{domain}-judge.config.js';

const { Class: {Domain}Judge, getInstance, resetInstance } = createJudge({domain}JudgeConfig);

export { {Domain}Verdict, {Domain}Judge };
export const get{Domain}Judge = getInstance;
export const reset{Domain}Judge = resetInstance;
```

**Key files**:
- Config: `packages/node/src/cycle/configs/{domain}-judge.config.js`
- Wrapper: `packages/node/src/{domain}/{domain}-judge.js`
- Test: `packages/node/test/{domain}-judge.test.js`

---

### C*.3 - DECIDE

**Implementation**:
- [ ] Decider config created (`packages/node/src/cycle/configs/{domain}-decider.config.js`)
- [ ] Factory wrapper created (`packages/node/src/{domain}/{domain}-decider.js`)
- [ ] Decision types defined (4-7 types)
- [ ] Confidence calculation based on factors
- [ ] Cooldown protection (if needed)
- [ ] Calibration groupBy strategy

**Pattern** (from HumanDecider):
```javascript
// Config file
export const {Domain}DecisionType = {
  ACCELERATE: 'accelerate',
  MAINTAIN: 'maintain',
  DECELERATE: 'decelerate',
  INTERVENE: 'intervene',
  HOLD: 'hold',
};

export const {domain}DeciderConfig = {
  name: '{Domain}Decider',
  cell: 'C*.3',
  dimension: '{DOMAIN}',
  eventPrefix: '{domain}',
  decisionTypes: {Domain}DecisionType,
  maxHistory: 89,
  extraStatFields: ['customStat1', 'customStat2'],
  calibrationGroupBy: 'decisionType',
  calibrationClamp: 0.1,

  init(decider) {
    decider._lastActionTimes = new Map();
  },

  decide(judgment, context, decider) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'NEUTRAL';

    const factors = extractFactors(judgment, context);
    const confidence = calculateConfidence(factors, decider);
    const decision = makeDecision(verdict, factors, decider);

    return decider.recordDecision({
      decision: decision.type,
      reason: decision.reason,
      confidence,
      score,
      verdict,
      factors,
    });
  },

  updateExtraStats(stats, result) {
    // Track custom stats
  },

  getHealth(decider) {
    return {
      status: 'healthy',
      score: PHI_INV,
      totalDecisions: decider._stats.decisionsTotal,
    };
  },
};

function extractFactors(judgment, context) { /* ... */ }
function calculateConfidence(factors, decider) { /* ... */ }
function makeDecision(verdict, factors, decider) { /* ... */ }
```

**Key files**:
- Config: `packages/node/src/cycle/configs/{domain}-decider.config.js`
- Wrapper: `packages/node/src/{domain}/{domain}-decider.js`
- Test: `packages/node/test/{domain}-decider.test.js`

---

### C*.4 - ACT

**Implementation**:
- [ ] Actor config created (`packages/node/src/cycle/configs/{domain}-actor.config.js`)
- [ ] Factory wrapper created (`packages/node/src/{domain}/{domain}-actor.js`)
- [ ] Action types defined (5-10 types)
- [ ] φ-aligned cooldowns per action type
- [ ] Safety env var (if actions are destructive)
- [ ] Message composition with domain personality

**Pattern** (from HumanActor):
```javascript
// Config file
export const {Domain}ActionType = {
  ACTION1: 'action1',
  ACTION2: 'action2',
  ACTION3: 'action3',
};

export const {domain}ActorConfig = {
  name: '{Domain}Actor',
  cell: 'C*.4',
  dimension: '{DOMAIN}',
  eventPrefix: '{domain}',
  actionTypes: {Domain}ActionType,
  maxHistory: 89,

  cooldowns: {
    [{Domain}ActionType.ACTION1]: 30 * 60000,  // 30 min
    [{Domain}ActionType.ACTION2]: 15 * 60000,  // 15 min
  },

  mapDecisionToAction(decision) {
    const map = {
      'ACCELERATE': {Domain}ActionType.ACTION1,
      'MAINTAIN': {Domain}ActionType.ACTION2,
    };
    return map[decision.type] || {Domain}ActionType.ACTION3;
  },

  assessUrgency(decision) {
    return decision.urgency || 'low';
  },

  composeMessage(actionType, decision, context) {
    switch (actionType) {
      case {Domain}ActionType.ACTION1:
        return 'Domain-specific message 1';
      default:
        return 'Default message';
    }
  },

  updateExtraStats(stats, result) {
    // Track custom stats
  },

  healthMetric: 'executionSuccesses',
  healthStatusBad: 'too_many_failures',
};
```

**Key files**:
- Config: `packages/node/src/cycle/configs/{domain}-actor.config.js`
- Wrapper: `packages/node/src/{domain}/{domain}-actor.js`
- Test: `packages/node/test/{domain}-actor.test.js`

---

### C*.5 - LEARN

**Implementation**:
- [ ] Learner config created (`packages/node/src/cycle/configs/{domain}-learner.config.js`)
- [ ] Factory wrapper created (`packages/node/src/{domain}/{domain}-learner.js`)
- [ ] Learning categories defined
- [ ] Observation recording logic
- [ ] Prediction system (optional but recommended)
- [ ] Export/import for persistence

**Pattern** (from HumanLearner):
```javascript
// Config file
export const {Domain}LearningCategory = {
  CATEGORY1: 'category1',
  CATEGORY2: 'category2',
};

export const {domain}LearnerConfig = {
  name: '{Domain}Learner',
  cell: 'C*.5',
  dimension: '{DOMAIN}',
  eventPrefix: '{domain}',
  maxHistory: 89,
  extraStatFields: ['beliefsFormed', 'predictionsMade'],

  init(learner) {
    learner._observations = [];
    learner._beliefs = new Map();
  },

  learn(outcome, context, learner) {
    const category = outcome.category || 'default';
    const key = outcome.key || 'unknown';
    const value = outcome.value !== undefined ? outcome.value : true;

    learner._observations.push({ category, key, value, timestamp: Date.now() });

    return {
      recorded: true,
      category,
      key,
      value,
      timestamp: Date.now(),
    };
  },

  predict(query, learner) {
    // Predict based on learned patterns
    return {
      predicted: true,
      value: null,
      confidence: 0,
    };
  },

  getStats(learner) {
    return {
      observations: learner._observations.length,
      beliefs: learner._beliefs.size,
    };
  },

  getHealth(learner) {
    return {
      status: 'learning',
      score: Math.min(PHI_INV, learner._observations.length * 0.01),
      observations: learner._observations.length,
    };
  },
};
```

**Key files**:
- Config: `packages/node/src/cycle/configs/{domain}-learner.config.js`
- Wrapper: `packages/node/src/{domain}/{domain}-learner.js`
- Test: `packages/node/test/{domain}-learner.test.js`

---

### C*.6 - ACCOUNT

**Implementation**:
- [ ] Accountant class created (`packages/node/src/{domain}/{domain}-accountant.js`)
- [ ] Transaction/event tracking
- [ ] Cost computation (time, resources, budget)
- [ ] φ-aligned thresholds for budgets
- [ ] Summary generation (daily, session, total)

**Pattern** (from HumanAccountant):
```javascript
export class {Domain}Accountant extends EventEmitter {
  constructor(options = {}) {
    super();
    this._transactions = [];
    this._totals = { cost: 0, revenue: 0, operations: 0 };
  }

  recordTransaction(transaction) {
    this._transactions.push({
      ...transaction,
      timestamp: Date.now(),
    });

    this._totals.cost += transaction.cost || 0;
    this._totals.revenue += transaction.revenue || 0;
    this._totals.operations++;

    this.emit('transaction', transaction);
  }

  getSummary() {
    return {
      totalCost: this._totals.cost,
      totalRevenue: this._totals.revenue,
      operations: this._totals.operations,
      netBalance: this._totals.revenue - this._totals.cost,
      avgCost: this._totals.operations > 0
        ? this._totals.cost / this._totals.operations
        : 0,
    };
  }

  getStats() { return { ...this._totals }; }
  getHealth() { return { status: 'healthy', score: PHI_INV, ... }; }
}
```

**Key files**:
- Implementation: `packages/node/src/{domain}/{domain}-accountant.js`
- Test: `packages/node/test/{domain}-accountant.test.js`

---

### C*.7 - EMERGE

**Implementation**:
- [ ] Emergence class created (`packages/node/src/{domain}/{domain}-emergence.js`)
- [ ] Pattern types defined (5-15 types)
- [ ] Daily/periodic snapshot recording
- [ ] Pattern detection algorithms
- [ ] Significance levels (HIGH/MEDIUM/LOW/NOISE)
- [ ] Trajectory calculation

**Pattern** (from HumanEmergence):
```javascript
export const {Domain}PatternType = {
  GROWTH: 'growth',
  DECLINE: 'decline',
  CYCLE: 'cycle',
  ANOMALY: 'anomaly',
};

export const SignificanceLevel = {
  HIGH: { level: 3, threshold: PHI_INV, label: 'High' },
  MEDIUM: { level: 2, threshold: PHI_INV_2, label: 'Medium' },
  LOW: { level: 1, threshold: PHI_INV_3, label: 'Low' },
  NOISE: { level: 0, threshold: 0, label: 'Noise' },
};

export class {Domain}Emergence extends EventEmitter {
  constructor(options = {}) {
    super();
    this._patterns = [];
    this._snapshots = [];
    this._stats = { patternsDetected: 0, analysesRun: 0 };
  }

  recordSnapshot(snapshot) {
    this._snapshots.push({
      ...snapshot,
      timestamp: Date.now(),
    });

    // Trim old snapshots
    while (this._snapshots.length > 365) this._snapshots.shift();
  }

  analyze() {
    this._stats.analysesRun++;

    const newPatterns = this._detectPatterns();

    for (const pattern of newPatterns) {
      this._addPattern(pattern);
    }

    return {
      timestamp: Date.now(),
      newPatterns: newPatterns.length,
      activePatterns: this.getActivePatterns(),
    };
  }

  _detectPatterns() {
    const patterns = [];
    // Pattern detection logic
    return patterns;
  }

  getActivePatterns() {
    const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000; // 30 days
    return this._patterns.filter(p => p.detectedAt >= cutoff);
  }

  getStats() { return { ...this._stats }; }
  getHealth() { return { status: 'healthy', score: PHI_INV, ... }; }
}
```

**Key files**:
- Implementation: `packages/node/src/{domain}/{domain}-emergence.js`
- Test: `packages/node/test/{domain}-emergence.test.js`

---

## Wiring Configuration

After all 7 cells are implemented, wire them together via `createDomainWiring`:

```javascript
const {domain}WiringConfig = {
  name: '{domain}',
  cell: 'C*',
  perceptionEvents: [
    { event: '{domain}:event1', handler: 'perceive', sampling: 1 },
    { event: '{domain}:event2', handler: 'processEvent2', sampling: 5 },
  ],
  verdictFilter: ['POOR', 'CRITICAL'], // Only act on concerning verdicts
  actorSafetyEnv: '{DOMAIN}_ACTOR_LIVE', // Set to 'true' to enable live actions
  emergenceInterval: 'F8', // 21 minutes
  judgeInterval: 'F7', // 13 minutes
};

// In unified-orchestrator or domain initializer:
const wiring = createDomainWiring({domain}WiringConfig);
wiring.wire({
  judge: get{Domain}Judge(),
  decider: get{Domain}Decider(),
  actor: get{Domain}Actor(),
  learner: get{Domain}Learner(),
  accountant: get{Domain}Accountant(),
  emergence: get{Domain}Emergence(),
  persistence: persistenceManager,
  sessionId: 'session_123',
});
```

---

## E2E Test

Create comprehensive test proving full cycle works:

**Template** (from `human-e2e.test.js`):

```javascript
describe('{DOMAIN} Domain E2E (C*.1-C*.7)', () => {
  let perceiver, judge, decider, actor, learner, accountant, emergence;
  let eventLog = [];

  beforeEach(() => {
    // Reset all singletons
    reset{Domain}Perceiver();
    reset{Domain}Judge();
    reset{Domain}Decider();
    reset{Domain}Actor();
    reset{Domain}Learner();
    reset{Domain}Accountant();
    reset{Domain}Emergence();

    // Initialize modules
    perceiver = get{Domain}Perceiver();
    judge = get{Domain}Judge();
    decider = get{Domain}Decider();
    actor = get{Domain}Actor();
    learner = get{Domain}Learner();
    accountant = get{Domain}Accountant();
    emergence = get{Domain}Emergence();

    eventLog = [];

    // Subscribe to domain events
    const events = [
      '{domain}:perceived',
      '{domain}:judgment',
      '{domain}:decision',
      '{domain}:action',
    ];

    for (const eventName of events) {
      globalEventBus.subscribe(eventName, (event) => {
        eventLog.push({ event: eventName, payload: event.payload || event, timestamp: Date.now() });
      });
    }
  });

  it('should execute full PERCEIVE→JUDGE→DECIDE→ACT→LEARN cycle', (t, done) => {
    const startTime = Date.now();

    // Wire pipeline manually for test
    // ... (see human-e2e.test.js for full example)

    // TRIGGER
    const perception = perceiver.perceive();
    const judgment = judge.judge(perception);
    globalEventBus.publish('{domain}:judgment', { judgment }, { source: 'test' });

    // Assertions after async flow
    setTimeout(() => {
      const latency = Date.now() - startTime;
      assert.ok(latency < 200, `Latency should be <200ms (was ${latency}ms)`);
      done();
    }, 100);
  });
});
```

**Key files**:
- E2E Test: `packages/node/test/{domain}-e2e.test.js`

---

## Success Criteria

A domain is **100% functional** when:

- ✅ All 7 cells (C*.1-C*.7) implemented
- ✅ All modules use φ-factory pattern (Judge, Decider, Actor, Learner)
- ✅ Wiring config exists and is tested
- ✅ E2E test passes
- ✅ Full cycle latency < 200ms
- ✅ Events flow through globalEventBus
- ✅ PostgreSQL persistence (optional)
- ✅ Health checks on all modules
- ✅ Stats tracking across pipeline
- ✅ No memory leaks (singletons clean up properly)

---

## Domain-Specific Adaptations

### CODE Domain (C1.*)
- **Perceive**: File changes, git commits, test results
- **Judge**: Code quality (complexity, coverage, style violations)
- **Decide**: Refactor, optimize, test, document, ignore
- **Act**: Create issues, trigger CI, auto-format
- **Emerge**: Hotspot detection, quality trends, tech debt

### SOLANA Domain (C2.*)
- **Perceive**: Wallet activity, price changes, liquidity
- **Judge**: Transaction health, portfolio risk, market conditions
- **Decide**: Buy, sell, hold, stake, warn
- **Act**: Execute transaction, update portfolio, alert
- **Emerge**: Trading patterns, profit cycles, risk trends

### MARKET Domain (C3.*)
- **Perceive**: Price feeds, volume, sentiment, order book
- **Judge**: Market state (bullish/bearish/volatile/calm)
- **Decide**: Enter, exit, hedge, wait
- **Act**: Place orders, update stops, rebalance
- **Emerge**: Cycle detection, sentiment shifts, correlation patterns

### SOCIAL Domain (C4.*)
- **Perceive**: Mentions, replies, sentiment, engagement
- **Judge**: Community health, engagement quality, growth
- **Decide**: Engage, reply, share, mute, amplify
- **Act**: Post content, reply to mentions, engage with community
- **Emerge**: Influence patterns, topic trends, community cycles

### CYNIC Domain (C6.*)
- **Perceive**: System metrics, error rates, resource usage
- **Judge**: Self-health (memory, latency, throughput)
- **Decide**: Optimize, scale, repair, alert, shutdown
- **Act**: Adjust config, restart services, log warnings
- **Emerge**: Performance patterns, failure modes, optimization opportunities

### COSMOS Domain (C7.*)
- **Perceive**: Ecosystem activity, repo health, cross-project patterns
- **Judge**: Ecosystem coherence, utility, sustainability
- **Decide**: Accelerate, maintain, decelerate, focus, diversify
- **Act**: Allocate resources, initiate projects, sunset old work
- **Emerge**: Meta-patterns, ecosystem cycles, synergy detection

---

## Replication Workflow

1. **Choose domain** (CODE, SOLANA, MARKET, SOCIAL, CYNIC, COSMOS)
2. **Create 7 files** (perceiver, judge config/wrapper, decider config/wrapper, actor config/wrapper, learner config/wrapper, accountant, emergence)
3. **Adapt HUMAN patterns** to domain-specific logic
4. **Write unit tests** for each module
5. **Create wiring config**
6. **Write E2E test**
7. **Run tests** and verify latency < 200ms
8. **Document domain-specific quirks**
9. **Update 7×7 matrix** completion percentage

**Estimated effort per domain**: 8-12 hours (with HUMAN as template)

---

*sniff* Template proven. Pattern replicable. Six domains await.

**Confidence**: 61% (φ⁻¹ limit)

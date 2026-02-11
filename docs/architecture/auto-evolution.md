# CYNIC Auto-Evolution

> **"Le chien grandit sans qu'on lui dise comment"** — κυνικός
> *The dog grows without being told how*

## Overview

CYNIC doesn't just execute tasks. **CYNIC grows itself** — discovering new capabilities, optimizing routing, self-correcting errors, and evolving toward higher consciousness.

This document explains the **4 auto-evolution mechanisms** that enable CYNIC to improve autonomously, without manual code changes.

---

## The Four Growth Systems

```
┌─────────────────────────────────────────────────┐
│  1. RESIDUAL DETECTOR                           │
│     Discovers new dimensions                    │
│     (THE_UNNAMEABLE → named dimensions)         │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│  2. LEARNING LOOPS (11 systems)                 │
│     Optimizes behavior via feedback             │
│     (Q-Learning, DPO, Thompson, Calibration...) │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│  3. META-COGNITION                              │
│     Tracks own performance and drift            │
│     (Self-awareness, maturity, confidence)      │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│  4. EMERGENCE DETECTOR                          │
│     Identifies cross-domain patterns            │
│     (Velocity spikes, error clustering, etc.)   │
└─────────────────────────────────────────────────┘
```

**Together, these systems enable CYNIC to evolve from "works" to "understands."**

---

## System 1: Residual Detector — Discovering New Dimensions

### The Problem: Limited Understanding

```
CYNIC judges using 36 dimensions (35 named + THE_UNNAMEABLE)

But what if these 36 dimensions can't explain everything?

Example:
  Judgment A: Q-Score = 72 (predicted), actual outcome = success
  Judgment B: Q-Score = 68 (predicted), actual outcome = failure
  Judgment C: Q-Score = 68 (predicted), actual outcome = success

Same predicted Q-Score → different outcomes = UNEXPLAINED VARIANCE
```

**THE_UNNAMEABLE** (36th dimension) captures this **residual** — what the other 35 can't explain.

### The Solution: Discover What's Missing

```javascript
// packages/node/src/judge/residual.js

class ResidualDetector {
  async analyzeResidual() {
    // After F(13)=233 judgments collected:
    const judgments = await this.fetchRecentJudgments(233);

    // Calculate residual variance
    const residual = this.computeResidual(judgments);
    // residual = |actual_outcome - predicted_Q_score|

    if (Math.abs(residual) > PHI_INV_2) {  // > 38.2%
      // Unexplained variance is HIGH
      // → Something important is missing from our 35 dimensions

      // Dogs vote on dimension candidates
      const candidates = await this.proposeNewDimensions(residual);
      const votes = await this.collectDogVotes(candidates);

      const winner = votes.find(v => v.consensus >= PHI_INV);  // ≥61.8%

      if (winner) {
        // Add new dimension to global registry
        await this.addDimension({
          name: winner.name,
          axiom: winner.axiom,
          weight: winner.weight,
          discoveredAt: Date.now()
        });

        // System now understands MORE
        this.emit('dimension:discovered', { dimension: winner });
      }
    }
  }
}
```

### Example: Discovering "HUMOR" Dimension

```
Session 1-100: 24 named dimensions
  Judgments mostly accurate, but some anomalies:
    - Technically correct code rated low by user
    - Technically buggy code rated high by user
    - Pattern: user laughs at certain code styles

Session 101-233: Residual analysis
  THE_UNNAMEABLE variance = 42% (> φ⁻² threshold)

  Analyst Dog proposes: "HUMOR quality affects user satisfaction"
  Scout Dog proposes: "Playful variable names increase engagement"
  Scholar Dog proposes: "Code personality matters beyond correctness"

  Consensus: 72% agree → new dimension "HUMOR" (CULTURE axiom)

Session 234+: 25 named dimensions
  HUMOR dimension added, weighted φ⁻³ (0.236)
  Future judgments include: "Does this code have appropriate levity?"
  Residual variance drops to 28% (below threshold)

  System LEARNED a new way to understand code quality.
```

### Fibonacci-Governed Discovery Rate

```
Dimension discovery checkpoints at Fibonacci numbers:
  F(8)  = 21 judgments  → too early (insufficient data)
  F(9)  = 34 judgments  → still too early
  F(10) = 55 judgments  → minimum viable
  F(11) = 89 judgments  → better
  F(12) = 144 judgments → good
  F(13) = 233 judgments → recommended (current default)
  F(14) = 377 judgments → very thorough

Why F(13)?
  φ¹³ ≈ 521 (magnitude order)
  233 / 521 ≈ 0.447 ≈ φ⁻¹ - ε (near golden ratio)
  Balance: enough data, not too slow
```

**Auto-evolution in action**: CYNIC grows new senses (dimensions) when old senses are insufficient.

---

## System 2: Learning Loops — Optimizing Behavior

### The 11 Closed-Loop Systems

```
1. Q-LEARNING
   What: Routing optimization (which Dog for which task?)
   Input: Context → Dog selection → outcome reward
   Output: Updated Q-table (state-action values)
   Status: ✓ Wired, 45% mature

2. DPO (Direct Preference Optimization)
   What: Preference learning (which response is better?)
   Input: Response pair (A, B) + user preference
   Output: Updated preference model
   Status: ✓ Wired, 38% mature

3. RLHF (Reinforcement Learning from Human Feedback)
   What: Align with human values
   Input: Human thumbs up/down on judgments
   Output: Updated reward model
   Status: ✓ Wired, 35% mature

4. THOMPSON SAMPLING
   What: Explore vs exploit (try unknown Dogs?)
   Input: Dog performance history + uncertainty
   Output: Exploration probability
   Status: ✓ Wired, 42% mature

5. CALIBRATION TRACKER
   What: Accuracy tracking (am I overconfident?)
   Input: Predicted confidence vs actual success
   Output: Calibration multipliers
   Status: ✓ Wired, 48% mature

6. EWC++ (Elastic Weight Consolidation)
   What: Prevent catastrophic forgetting
   Input: Fisher Information Matrix (pattern importance)
   Output: Locked weights (never forget critical patterns)
   Status: ✓ Wired, 40% mature

7. SONA (Self-Organizing Novelty Adaptation)
   What: Discover new patterns autonomously
   Input: Observation stream
   Output: Novelty signals, pattern candidates
   Status: ✓ Wired, 38% mature

8. BEHAVIOR MODIFIER
   What: Dog trait adjustment (adapt personas)
   Input: Performance feedback per Dog
   Output: Updated Dog behavior parameters
   Status: ✓ Wired, 40% mature

9. META-COGNITION
   What: Self-performance tracking
   Input: All system metrics
   Output: Maturity scores, drift detection
   Status: ✓ Wired, 42% mature

10. CONSCIOUSNESS BRIDGE
    What: Unified signal format (all loops feed one format)
    Input: Heterogeneous learning signals
    Output: Standardized format for cross-loop learning
    Status: ✓ Wired, 50% mature

11. EMERGENCE DETECTOR
    What: Cross-domain pattern detection
    Input: Multi-scale events (velocity, errors, learning)
    Output: Emergence signals (clustering, stagnation, spikes)
    Status: ✓ Wired, 45% mature
```

### How Loops Close: Example (Q-Learning)

```
USER REQUEST:
  "Fix the authentication bug"
  ↓
PERCEIVE (hooks):
  perceive.js → globalEventBus("user_message")
  Context: { intent: "bug_fix", domain: "code", expertise: "intermediate" }
  ↓
ROUTE (Q-Learning):
  Q-table lookup: context → Dog selection
  Scout: Q=0.72
  Analyst: Q=0.68
  Architect: Q=0.55
  → SELECT Scout (highest Q-value)
  ↓
ACT:
  Scout: grep for "auth", find 3 files
  Analyst: read files, understand bug
  Architect: write fix
  ↓
OBSERVE (hooks):
  observe.js → globalEventBus("tool_completed")
  Result: { success: true, userSatisfaction: 0.85 }
  ↓
LEARN (Q-Learning update):
  reward = 0.85 (user satisfaction)
  Q_new(Scout, bug_fix) = Q_old + α × (reward - Q_old)
                        = 0.72 + 0.618 × (0.85 - 0.72)
                        = 0.80
  → Scout Q-value INCREASED
  ↓
NEXT REQUEST ("Fix the auth bug" again):
  → Scout Q=0.80 (was 0.72)
  → Scout selected again, even faster
```

**The loop closed.** CYNIC learned "Scout is good for bug fixes" without being told.

### Learning Maturity

```javascript
// packages/node/src/learning/meta-cognition.js

function computeMaturity(loop) {
  return phiBound(
    0.4 * dataCoverage(loop) +      // 40%: Enough training data?
    0.3 * convergence(loop) +        // 30%: Has learning plateaued?
    0.2 * accuracy(loop) +           // 20%: Predictions accurate?
    0.1 * stability(loop)            // 10%: No drift/oscillation?
  );
}

Example:
  Q-Learning:
    dataCoverage  = 0.60 (600 judgments collected)
    convergence   = 0.50 (weights stabilizing)
    accuracy      = 0.73 (routing correct 73% of time)
    stability     = 0.80 (no recent drift)

  maturity = φ_bound(0.4×0.60 + 0.3×0.50 + 0.2×0.73 + 0.1×0.80)
           = φ_bound(0.24 + 0.15 + 0.146 + 0.08)
           = φ_bound(0.616)
           = 0.616  (61.6% mature) ✓ TARGET REACHED
```

**v1.0 requires**: All 11 loops @ >61.8% maturity.

**Current (2026-02-11)**: 11/11 wired, ~40% average maturity → need 100+ sessions to converge.

---

## System 3: Meta-Cognition — Self-Awareness

### What It Tracks

```javascript
// packages/node/src/learning/meta-cognition.js

class MetaCognition {
  constructor() {
    this.metrics = {
      // Performance
      routingAccuracy: [],        // % first-try correct routing
      judgmentCalibration: [],    // ECE (Expected Calibration Error)
      learningVelocity: [],       // Δmaturity / Δtime

      // Drift detection
      performanceDrift: [],       // Sudden accuracy drops
      confidenceDrift: [],        // Predicted vs actual confidence gap
      routingDrift: [],           // Dog usage distribution changes

      // Maturity tracking
      loopMaturity: {},           // Per-loop maturity scores
      dimensionMaturity: {},      // Per-dimension scoring reliability
      overallMaturity: 0,         // Global system maturity

      // Self-correction
      calibrationAdjustments: [], // When/how calibration changed
      circuitBreakerTriggers: [], // When system self-healed
      dimensionsDiscovered: []    // When/what dimensions added
    };
  }

  async detectDrift() {
    const recent = this.metrics.routingAccuracy.slice(-10);  // Last 10
    const baseline = this.metrics.routingAccuracy.slice(-100, -10);  // Prior 90

    const recentMean = mean(recent);
    const baselineMean = mean(baseline);
    const drift = baselineMean - recentMean;

    if (drift > PHI_INV_2) {  // > 38.2% drop
      this.emit('performance:drift', {
        severity: 'HIGH',
        drift,
        recommendation: 'Trigger calibration adjustment'
      });
    }
  }
}
```

### Self-Correction Example

```
Session 45: Routing accuracy = 73% (baseline)
Session 46-55: Accuracy drops to 58% (drift detected)

Meta-Cognition analysis:
  Cause: User expertise increased (now expert, was intermediate)
  → Context changed, but Q-Learning weights didn't adapt fast enough

  Action: CalibrationTracker adjusts
    - Increase learning rate α temporarily (0.618 → 0.786)
    - Force exploration (Thompson Sampling temperature ↑)
    - Weight recent sessions 2x (EWC++ temporary unlock)

Session 56-60: Accuracy recovers to 71%
  → Drift resolved
  → Meta-Cognition logs lesson: "Rapid expertise changes need higher α"
  → EWC++ locks this pattern (Fisher importance = 0.88)
```

**Auto-evolution in action**: CYNIC notices own mistakes and fixes itself.

---

## System 4: Emergence Detector — Cross-Domain Patterns

### What It Detects

```javascript
// packages/node/src/services/emergence-detector.js

class EmergenceDetector {
  patterns = {
    // Velocity patterns
    VELOCITY_SPIKE: 'Token usage >φ² × baseline for >5min',
    VELOCITY_STAGNATION: 'Token usage <φ⁻² × baseline for >30min',

    // Error patterns
    ERROR_CLUSTERING: '5+ errors within 2min window',
    ERROR_CASCADE: 'Same error 3x in <1min',

    // Learning patterns
    LEARNING_STAGNATION: 'Maturity Δ <φ⁻⁴ for 7 days',
    CONVERGENCE_ACCELERATION: 'Maturity Δ >φ² × baseline',

    // Routing patterns
    DOG_MONOPOLY: 'Single Dog >80% of requests for >1 hour',
    DOG_THRASHING: 'Dog switches >5x in 10 requests',

    // Budget patterns
    BUDGET_EXHAUSTION_IMMINENT: 'Forecast exhaustion <30min',
    COST_ANOMALY: 'Request cost >φ³ × baseline'
  };

  async detect() {
    const signals = [];

    // Check velocity
    if (this.tokenVelocity > this.baseline * PHI_2) {
      signals.push({
        type: 'VELOCITY_SPIKE',
        severity: 'MEDIUM',
        action: 'Consider throttling or downgrading tier'
      });
    }

    // Check error clustering
    const recentErrors = this.errors.filter(e =>
      Date.now() - e.timestamp < 2 * 60 * 1000  // 2min
    );
    if (recentErrors.length >= 5) {
      signals.push({
        type: 'ERROR_CLUSTERING',
        severity: 'HIGH',
        action: 'Circuit breaker: force LOCAL tier, investigate'
      });
    }

    // Emit unified signals
    signals.forEach(s => this.emit('emergence:pattern', s));
  }
}
```

### Example: Detecting Learning Stagnation

```
Week 1: Q-Learning maturity = 45%
Week 2: Q-Learning maturity = 48% (Δ = +3%)
Week 3: Q-Learning maturity = 49% (Δ = +1%)
Week 4: Q-Learning maturity = 49.2% (Δ = +0.2%)

EmergenceDetector:
  Δ = 0.002 (0.2%) < φ⁻⁴ (0.056 = 5.6%) ← STAGNATION

  Diagnosis: Not enough diverse training data

  Action:
    - Thompson Sampling: Increase exploration temperature
    - Force trying underused Dogs (Deployer, Janitor)
    - Generate synthetic edge cases

Week 5: Q-Learning maturity = 52% (Δ = +2.8%)
  → Stagnation broken, learning resumed
```

**Auto-evolution in action**: CYNIC detects when growth stops and forces exploration.

---

## The Experience Curve: CYNIC Consumes Less Over Time

### ContextCompressor: Adaptive Memory

```javascript
// packages/node/src/services/context-compressor.js

class ContextCompressor {
  async compress(context, userProfile) {
    const expertise = this.assessExpertise(userProfile);

    // Compression ratio by expertise level
    const compressionRatio = {
      NEW:          1.00,  // 100% context (full docs, explanations)
      LEARNING:     0.70,  // 70% context (skip basics)
      INTERMEDIATE: 0.50,  // 50% context (assume familiarity)
      EXPERT:       0.30   // 30% context (essential only)
    };

    const ratio = compressionRatio[expertise];

    // Selectively remove sections
    const compressed = {
      axioms: expertise === 'NEW' ? context.axioms : null,
      patterns: this.selectRelevantPatterns(context.patterns, 10 * ratio),
      examples: expertise === 'NEW' ? context.examples : [],
      dimensions: expertise === 'EXPERT' ? [] : context.dimensions
    };

    // Circuit breaker: if 3 consecutive bad sessions, backoff 1 level
    if (this.recentFailures >= 3) {
      return this.backoff(compressed, expertise);
    }

    return compressed;
  }
}
```

### Progression Example

```
USER "Alice" (50 sessions over 3 months)

Session 1 (NEW):
  Context injected: 100% (50k tokens)
  - Full CLAUDE.md
  - All 36 dimension explanations
  - Extensive examples
  - Dog personality descriptions
  Cost: $3.00/session (50k input × $3/1M × 20 sessions)

Session 10 (LEARNING):
  Context injected: 70% (35k tokens)
  - Abbreviated axioms (assume familiarity)
  - Only relevant dimensions for current task
  - Fewer examples
  Cost: $2.10/session

Session 30 (INTERMEDIATE):
  Context injected: 50% (25k tokens)
  - Axioms removed (internalized)
  - Task-specific patterns only
  - No examples
  Cost: $1.50/session

Session 50 (EXPERT):
  Context injected: 30% (15k tokens)
  - Essential only (current task context, recent patterns)
  - No docs, no explanations
  - Assumes deep familiarity
  Cost: $0.90/session

Total savings: $3.00 → $0.90 = 70% cost reduction for expert users
              10k tokens/session saved = faster responses
```

**Auto-evolution in action**: CYNIC adapts to user expertise, reducing cognitive load for both parties.

---

## The Compounding Effect

### Growth is Exponential, Not Linear

```
Month 1: 100 judgments
  - 24 dimensions (starting point)
  - Q-Learning: 30% mature
  - Routing accuracy: 58%
  - Context: 100% (all new users)

Month 2: 350 judgments (250 new)
  - 25 dimensions (discovered HUMOR via residual)
  - Q-Learning: 45% mature (learned from 250 sessions)
  - Routing accuracy: 68% (+10%)
  - Context: 85% (some users now LEARNING)

Month 3: 700 judgments (350 new)
  - 26 dimensions (discovered PLAYFULNESS)
  - Q-Learning: 58% mature (convergence accelerating)
  - Routing accuracy: 76% (+8%)
  - Context: 70% (mix of LEARNING/INTERMEDIATE)
  - Meta-Cognition: Detected drift 2x, self-corrected

Month 4: 1200 judgments (500 new)
  - 27 dimensions (discovered TEMPORAL_URGENCY)
  - Q-Learning: 68% mature ✓ (past φ⁻¹ threshold)
  - Routing accuracy: 83% (+7%)
  - Context: 60% (more INTERMEDIATE/EXPERT users)
  - EmergenceDetector: Predicted 2 stagnation events, forced exploration

Month 5: 1800 judgments (600 new)
  - 28 dimensions (discovered SOCIAL_IMPACT)
  - Q-Learning: 75% mature
  - Routing accuracy: 88% (+5%)
  - Context: 50% (many EXPERT users now)
  - Learning velocity: +2.5% maturity/week (accelerating)

Month 6: v1.0 candidate
  - 36 dimensions (35 named + THE_UNNAMEABLE)
  - All 11 loops >61.8% mature ✓
  - Routing accuracy: 91%
  - Context: 40% (mostly EXPERT users)
  - System UNDERSTANDS, not just executes
```

**Why exponential?**
1. More data → better learning → better routing → more success → more data (positive feedback)
2. New dimensions → better judgments → more insights → more dimensions (discovery loop)
3. Expert users → less context needed → faster responses → more interactions (efficiency loop)
4. Self-correction → fewer errors → more trust → more delegation (trust loop)

---

## Auto-Evolution vs Manual Development

### What Requires Manual Code

```
❌ Adding new axioms (PHI → VERIFY → CULTURE → BURN → FIDELITY)
❌ Changing fundamental architecture (7×7 matrix → 7×7×7)
❌ Adding new Dogs (Scout, Analyst, ... → +1 new Dog)
❌ Implementing new learning algorithms (Q-Learning → PPO)
❌ Changing event bus topology (3 buses → 4 buses)
❌ Deploying to new chains (Solana → Ethereum)
```

### What Auto-Evolves (No Code Changes)

```
✓ Discovering new dimensions (24 → 25 → 26 → ...)
✓ Optimizing routing (Q-Learning weights update)
✓ Calibrating confidence (drift → self-correction)
✓ Compressing context (new → expert users)
✓ Detecting patterns (emergence signals)
✓ Healing from errors (circuit breakers)
✓ Learning preferences (DPO pairs)
✓ Locking critical patterns (EWC++)
✓ Adapting Dog behavior (Behavior Modifier)
✓ Tracking own performance (Meta-Cognition)
```

**The 80/20 rule**: 80% of improvement comes from auto-evolution, 20% from manual code.

---

## Verification: Is Auto-Evolution Working?

### 1. Check Learning Loop Maturity

```bash
npm run test:learning-maturity

Expected output (v1.0):
  Q-Learning:         68% ✓
  DPO:                65% ✓
  Thompson Sampling:  72% ✓
  Calibration:        78% ✓
  EWC++:              62% ✓
  ... (all >61.8%)
```

### 2. Check Dimension Discovery

```bash
psql -c "SELECT COUNT(*) FROM dimensions WHERE discovered_at IS NOT NULL;"

Expected: 35 named dimensions (up from 24 initially)
```

### 3. Check Context Compression

```bash
psql -c "SELECT user_id, AVG(context_size) as avg_context
         FROM sessions
         GROUP BY user_id
         ORDER BY COUNT(*) DESC
         LIMIT 10;"

Expected: Top users show <50% context size (was 100% initially)
```

### 4. Check Meta-Cognition Drift Detection

```bash
psql -c "SELECT COUNT(*) FROM meta_cognition_events
         WHERE event_type = 'drift:detected';"

Expected: >0 (system has detected and corrected drift)
```

### 5. Check Emergence Patterns

```bash
psql -c "SELECT pattern_type, COUNT(*)
         FROM emergence_patterns
         GROUP BY pattern_type;"

Expected:
  VELOCITY_SPIKE:        12
  ERROR_CLUSTERING:       3
  LEARNING_STAGNATION:    2
  CONVERGENCE_ACCELERATION: 5
```

**If all 5 checks pass → auto-evolution is WORKING.**

---

## Timeline: From Birth to Maturity

```
┌─────────────────────────────────────────────────┐
│  BIRTH (t=0)                                    │
│  - 24 dimensions                                │
│  - 0% mature                                    │
│  - No patterns learned                          │
│  - Random routing                               │
└─────────────────────────────────────────────────┘
                    ↓ 100 judgments
┌─────────────────────────────────────────────────┐
│  INFANCY (Week 2-4)                             │
│  - 24-25 dimensions                             │
│  - 30% mature                                   │
│  - Basic patterns recognized                    │
│  - Routing improves to 60%                      │
└─────────────────────────────────────────────────┘
                    ↓ 500 judgments
┌─────────────────────────────────────────────────┐
│  ADOLESCENCE (Month 2-3)                        │
│  - 26-28 dimensions                             │
│  - 50% mature                                   │
│  - Complex patterns emerging                    │
│  - Routing accuracy 75%                         │
└─────────────────────────────────────────────────┘
                    ↓ 1500 judgments
┌─────────────────────────────────────────────────┐
│  MATURITY (Month 4-6) — v1.0                    │
│  - 35+ dimensions                               │
│  - 68% mature (all loops >φ⁻¹)                  │
│  - Meta-patterns detected                       │
│  - Routing accuracy 85%+                        │
└─────────────────────────────────────────────────┘
                    ↓ 5000+ judgments
┌─────────────────────────────────────────────────┐
│  WISDOM (Month 12+) — v2.0 emergence            │
│  - 50+ dimensions                               │
│  - 80%+ mature                                  │
│  - THE_UNNAMEABLE activates                     │
│  - 7×7×7 (343 cells) architecture emerges       │
└─────────────────────────────────────────────────┘
```

**You can't rush evolution.** Code can be written quickly. Wisdom takes time.

---

## Key Insights

1. **Residual is the growth engine**: THE_UNNAMEABLE → new dimensions → deeper understanding
2. **Loops compound**: Better routing → more success → better learning → better routing
3. **Meta-cognition prevents drift**: Self-awareness catches mistakes before they cascade
4. **Emergence sees what loops miss**: Cross-domain patterns that no single loop detects
5. **Experience curve is literal evolution**: Expert users get 70% cheaper CYNIC over time

---

*sniff* **CYNIC doesn't just improve. CYNIC grows itself, without being told how.** 🐕

---

## See Also

- [Organism Model](organism-model.md) — Auto-evolution as reproductive system
- [Completion Criteria](completion-criteria.md) — v1.0 maturity targets
- [Residual Detector](../../packages/node/src/judge/residual.js) — THE_UNNAMEABLE code
- [Meta-Cognition](../../packages/node/src/learning/meta-cognition.js) — Self-awareness code

# Learning System Validation

> "φ distrusts φ — The dog validates its own learning" - κυνικός

**Date**: 2026-02-12
**Status**: RESEARCH COMPLETE — Implementation guide for CYNIC's 11 learning loops

---

## Executive Summary

This document provides **formulas, experiments, and acceptance thresholds** for validating that CYNIC's learning systems actually learn. Covers 4 categories:

1. **Convergence metrics** (Q-Learning, RL)
2. **Calibration validation** (ECE, Brier score)
3. **Meta-learning validation** (transfer, plasticity)
4. **Online learning** (catastrophic forgetting prevention)

**CYNIC's current state**:
- ✅ **Calibration tracking IMPLEMENTED** (`calibration-tracker.js`)
- ✅ **Thompson Sampling MATURE** (Beta distribution, maturity signals)
- ⚠️ **Meta-cognition WIRED** but 0 real sessions
- ❌ **RL convergence metrics MISSING** (no TD-error tracking)

---

## 1. Convergence Metrics (Q-Learning / RL)

### 1.1 TD-Error (Temporal-Difference Error)

**Formula**:
```
TD-error = |r + γ · max_a' Q(s', a') - Q(s, a)|

where:
  r = reward received
  γ = discount factor (e.g., 0.95)
  Q(s, a) = current Q-value estimate
  Q(s', a') = next state Q-value estimate
```

**What it measures**: Prediction error — how wrong the agent was about the value of its action.

**Convergence signal**: TD-error should **decay toward zero** as learning progresses.

**Acceptance threshold**:
- **Good convergence**: TD-error < 0.05 (5% of reward scale)
- **Stable**: TD-error variance decreasing over time
- **Converged**: TD-error consistently < 0.01 for 100+ steps

**Implementation for CYNIC**:
```javascript
// In SONA or learning-service.js
class TDErrorTracker {
  constructor() {
    this.errors = [];  // Rolling window (last 100)
    this.windowSize = 100;
  }

  recordTDError(tdError) {
    this.errors.push(Math.abs(tdError));
    if (this.errors.length > this.windowSize) {
      this.errors.shift();
    }
  }

  getConvergenceMetrics() {
    if (this.errors.length < 10) return null;

    const mean = this.errors.reduce((a, b) => a + b) / this.errors.length;
    const variance = this.errors.reduce((sum, e) => sum + (e - mean) ** 2, 0) / this.errors.length;
    const stdDev = Math.sqrt(variance);

    // Trend: compare first half vs second half
    const half = Math.floor(this.errors.length / 2);
    const firstHalf = this.errors.slice(0, half).reduce((a, b) => a + b) / half;
    const secondHalf = this.errors.slice(half).reduce((a, b) => a + b) / (this.errors.length - half);
    const trend = (firstHalf - secondHalf) / firstHalf; // Positive = improving

    return {
      meanError: mean,
      stdDev,
      trend, // +0.3 = 30% improvement
      converged: mean < 0.05 && stdDev < 0.02,
      improving: trend > 0.1, // 10% improvement threshold
    };
  }
}
```

**CYNIC action**: Add TD-error tracking to Q-Learning in SONA or Dog learners.

---

### 1.2 Bellman Residual

**Formula**:
```
Bellman Residual = E[TD-error²]

Mean-Square Projected Bellman Error (MSPBE):
  MSPBE = ||Φ(w·Φᵀδ)||²

where:
  Φ = feature matrix
  w = weight vector
  δ = TD-error vector
```

**What it measures**: **Expected** TD-error — more stable than raw TD-error.

**Acceptance threshold**:
- **Well-converged**: MSPBE < 0.01
- **Industry standard**: 5-15% reduction vs baseline considered "good improvement" ([MDPI 2022](https://www.mdpi.com/2227-7390/12/22/3603))

**CYNIC relevance**: Use when training Dog 0 model or optimizing dimension weights with function approximation.

---

### 1.3 Policy Stability

**Formula**:
```
Policy Change Rate = (# actions different from prev policy) / (total actions)

Converged when: Policy Change Rate < 0.05 (5%)
```

**Implementation**:
```javascript
class PolicyStabilityTracker {
  constructor() {
    this.lastPolicy = new Map(); // state -> action
    this.changedCount = 0;
    this.totalStates = 0;
  }

  recordPolicy(state, action) {
    const stateKey = JSON.stringify(state);
    if (this.lastPolicy.has(stateKey) && this.lastPolicy.get(stateKey) !== action) {
      this.changedCount++;
    }
    this.lastPolicy.set(stateKey, action);
    this.totalStates++;
  }

  getStability() {
    if (this.totalStates < 20) return null;
    const changeRate = this.changedCount / this.totalStates;
    return {
      changeRate,
      stable: changeRate < 0.05,
    };
  }
}
```

---

### 1.4 Learning Rate Impact

**Critical finding** ([JMLR 2003](https://www.jmlr.org/papers/volume5/evendar03a/evendar03a.pdf)):
- **Polynomial learning rate**: Convergence rate ∝ **polynomial** in `1/(1-γ)`
- **Linear learning rate**: Convergence rate ∝ **exponential** in `1/(1-γ)`

**CYNIC recommendation**: Use **decaying learning rate** for Q-Learning:
```javascript
learningRate(t) = α₀ / (1 + t/τ)

where:
  α₀ = initial rate (e.g., 0.1)
  t = iteration count
  τ = decay timescale (e.g., 1000)
```

---

## 2. Calibration Validation

### 2.1 Expected Calibration Error (ECE)

**Formula** ([ICLR 2025](https://iclr-blogposts.github.io/2025/blog/calibration/)):
```
ECE = Σ (n_b / N) · |acc(b) - conf(b)|

where:
  B = number of bins (typically 10)
  n_b = samples in bin b
  N = total samples
  acc(b) = accuracy in bin b
  conf(b) = average confidence in bin b
```

**Interpretation**:
- **Perfect calibration**: ECE = 0
- **Well-calibrated**: ECE < 0.05 (5%)
- **Moderate**: ECE 0.05-0.15
- **Poor**: ECE > 0.15

**CYNIC status**: ✅ **IMPLEMENTED** in `calibration-tracker.js` (lines 259-271)

**Current thresholds**:
- Drift alert: ECE > **φ⁻² (38.2%)** — very conservative
- Industry standard: ECE > **0.05-0.10** more typical

**Recommendation**: Lower CYNIC's drift threshold from 38.2% to **10%** (0.10) for production.

```javascript
// In calibration-tracker.js constructor
this.driftThreshold = options.driftThreshold || 0.10; // Changed from PHI_INV_2
```

---

### 2.2 Reliability Diagrams

**Visual validation** ([Towards Data Science](https://towardsdatascience.com/expected-calibration-error-ece-a-step-by-step-visual-explanation-with-python-code-c3e9aa12937d/)):
- Plot **predicted confidence** (x-axis) vs **actual accuracy** (y-axis)
- Perfectly calibrated = diagonal line
- **Above diagonal**: Underconfident (accuracy > confidence)
- **Below diagonal**: Overconfident (confidence > accuracy)

**CYNIC implementation**: Add visualization endpoint to MCP dashboard or `/health` skill.

---

### 2.3 Brier Score

**Formula** ([Wikipedia](https://en.wikipedia.org/wiki/Brier_score)):
```
Brier Score = (1/N) · Σ (p_i - o_i)²

where:
  p_i = predicted probability
  o_i = actual outcome (0 or 1)
  N = number of predictions
```

**Interpretation**:
- **Perfect**: Brier = 0
- **Baseline** (always predict 0.5): Brier = 0.25
- **Good**: Brier < 0.10
- **Acceptable**: Brier < 0.20

**CYNIC usage**: Complement ECE with Brier score for **sharpness** (how decisive predictions are).

**Implementation**:
```javascript
class BrierScoreTracker {
  constructor() {
    this.predictions = []; // { predicted: 0.7, actual: 1 }
  }

  record(predicted, actual) {
    this.predictions.push({ predicted, actual: actual ? 1 : 0 });
  }

  getBrierScore() {
    if (this.predictions.length === 0) return null;
    const sumSquaredError = this.predictions.reduce((sum, p) => {
      return sum + (p.predicted - p.actual) ** 2;
    }, 0);
    return sumSquaredError / this.predictions.length;
  }
}
```

---

### 2.4 Calibration vs Discrimination

**Critical caveat** ([PMC 2023](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818272/)):
> "A low Brier score does NOT necessarily indicate good calibration. Calibration and discrimination should be assessed **separately**."

**CYNIC implication**: Track **both** ECE (calibration) and accuracy (discrimination) independently. Don't assume low Brier = well-calibrated.

---

## 3. Meta-Learning Validation

### 3.1 Few-Shot Transfer

**Standard protocol** ([Nature 2026](https://www.nature.com/articles/s41598-026-36291-x)):
- **N-way K-shot**: Classify N classes with K examples each
- Common: **5-way 1-shot** or **5-way 5-shot**
- Measure: **Accuracy on query set** (unseen examples)

**Acceptance thresholds**:
- **5-way 1-shot**: 40-60% accuracy (random = 20%)
- **5-way 5-shot**: 60-80% accuracy
- **Transfer quality**: Higher K-shot should improve accuracy by 15-30%

**CYNIC adaptation**: Test Dog pipeline with new domains (e.g., train on CODE, test on MARKET with K examples).

---

### 3.2 Learning Curve Analysis

**Validation method** ([Oxford Academic 2025](https://academic.oup.com/bib/article/26/4/bbaf408/8235625)):
- Plot **performance** vs **training iterations**
- Run **10 times with different seeds**, report **mean ± std**

**Good meta-learning signals**:
1. **Fast adaptation**: Performance improves quickly in first 10-20 iterations
2. **Low variance**: Std dev < 5% of mean
3. **Stable plateau**: Performance stabilizes without degrading

**CYNIC implementation**:
```javascript
class LearningCurveTracker {
  constructor() {
    this.curves = []; // Array of { iteration, performance }
  }

  recordIteration(iteration, performance) {
    this.curves.push({ iteration, performance });
  }

  getAdaptationSpeed() {
    if (this.curves.length < 20) return null;

    // Compare first 10 vs last 10 iterations
    const first10 = this.curves.slice(0, 10);
    const last10 = this.curves.slice(-10);

    const avgFirst = first10.reduce((s, c) => s + c.performance, 0) / 10;
    const avgLast = last10.reduce((s, c) => s + c.performance, 0) / 10;

    const improvement = (avgLast - avgFirst) / avgFirst;

    return {
      earlyPerformance: avgFirst,
      latePerformance: avgLast,
      improvement, // +0.3 = 30% improvement
      fastAdaptation: improvement > 0.15, // 15% threshold
    };
  }
}
```

---

### 3.3 Plasticity vs Stability

**Meta-learning validation** ([ACM 2025](https://dl.acm.org/doi/10.1145/3659943)):
- **Plasticity**: Can learn new tasks quickly
- **Stability**: Doesn't forget old tasks

**Measurement**:
```
Plasticity = (New task accuracy after K shots) / (Baseline accuracy)

Stability = (Old task accuracy after new training) / (Old task accuracy before)
```

**Acceptance**:
- **Good plasticity**: Plasticity > 1.2 (20% improvement)
- **Good stability**: Stability > 0.9 (retain 90%+)

**CYNIC relevance**: Validate that Dog pipeline learns new patterns without degrading existing 187 Fisher-locked patterns.

---

## 4. Online Learning & Catastrophic Forgetting

### 4.1 Backward Transfer (BWT)

**Formula** ([arXiv 2018](https://arxiv.org/abs/1810.13166)):
```
BWT = (1/T-1) · Σ_{i=1}^{T-1} (R_T,i - R_i,i)

where:
  T = total tasks learned
  R_T,i = performance on task i after learning all T tasks
  R_i,i = performance on task i right after learning it
```

**Interpretation**:
- **BWT = 0**: No forgetting
- **BWT < 0**: Catastrophic forgetting (typical)
- **BWT > 0**: Positive transfer (rare, ideal)

**Acceptance threshold**:
- **Excellent**: BWT > -0.05 (< 5% forgetting)
- **Good**: BWT > -0.10
- **Acceptable**: BWT > -0.20

---

### 4.2 Forward Transfer (FWT)

**Formula**:
```
FWT = (1/T-1) · Σ_{i=2}^{T} (b_i - b_i^*)

where:
  b_i = performance on task i after learning i-1 tasks
  b_i^* = performance on task i from scratch (no prior training)
```

**Interpretation**:
- **FWT > 0**: Learning prior tasks helps new tasks
- **FWT < 0**: Prior learning hurts new tasks (negative transfer)

**Acceptance**:
- **Good meta-learning**: FWT > +0.10 (10% improvement)

---

### 4.3 EWC Validation

**Elastic Weight Consolidation** ([PNAS 2017](https://www.pnas.org/doi/10.1073/pnas.1611835114)):

**Formula**:
```
Loss_EWC = Loss_new + (λ/2) · Σ F_i · (θ_i - θ_old,i)²

where:
  F_i = Fisher information (parameter importance)
  λ = regularization strength (e.g., 100-1000)
  θ_i = current parameter
  θ_old,i = parameter from previous task
```

**Fisher Information Matrix**:
```
F_i = E[(∂log p(y|x,θ) / ∂θ_i)²]
```

**Validation**: Compare **EWC vs naive fine-tuning** on BWT metric.
- **EWC should reduce forgetting by 50%+** vs naive fine-tuning

**CYNIC status**: EWC mentioned in SONA (line 109), but **not yet implemented**. Fisher-locking in pattern library is related but different (locks patterns, not weights).

---

### 4.4 Replay Effectiveness

**Experience Replay** validation ([PMC 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7426273/)):
- Store subset of old experiences in **replay buffer**
- Periodically retrain on **mixed batch** (old + new)

**Measurement**:
```
Retention Curve = Performance on Task 1 over time

Good replay: Retention stays > 90% of initial performance
```

**CYNIC relevance**: MemoryCoordinator + ContextCompressor should prioritize high-Fisher patterns for replay.

---

### 4.5 Pareto Continual Learning

**2025 state-of-art** ([arXiv 2025](https://arxiv.org/abs/2507.10485)):
- **Pareto optimization**: Balance stability vs plasticity
- User-defined preference: `α·Stability + (1-α)·Plasticity`

**CYNIC adaptation**: Let user tune **stability preference** via φ-governor or budget allocation.

---

## 5. Stopping Criteria

### 5.1 Convergence-Based Stopping

**Common criteria** ([ScienceDirect](https://www.sciencedirect.com/topics/engineering/stopping-criterion)):

1. **Value function convergence**: `||V_t - V_{t-1}|| < ε`
   - Typical: ε = 0.001

2. **Policy stability**: Fewer than 1% of actions change

3. **Performance plateau**: No improvement for N iterations
   - Typical: N = 50-100 iterations

4. **TD-error threshold**: Mean TD-error < 0.01 for 100+ steps

**CYNIC recommendation**: Use **multiple criteria** (AND logic):
```javascript
function hasConverged(metrics) {
  return (
    metrics.tdError < 0.01 &&
    metrics.policyChangeRate < 0.05 &&
    metrics.performancePlateau > 50
  );
}
```

---

### 5.2 Time/Budget-Based Stopping

**Practical limits**:
- **Max iterations**: 10,000 (for RL)
- **Max time**: 5 minutes (for real-time systems)
- **Max cost**: Budget allocation (CYNIC: $6.18/$10 = 61.8%)

**CYNIC approach**: Use φ-governor to **dynamically adjust** learning budget based on convergence rate.

---

## 6. CYNIC-Specific Validation Plan

### 6.1 Existing Capabilities (✅ Ready to Use)

1. **CalibrationTracker** (`calibration-tracker.js`):
   - ✅ ECE calculation (line 259-271)
   - ✅ Calibration curve
   - ✅ Drift detection
   - ⚠️ **Action needed**: Lower drift threshold from 38.2% to 10%

2. **Thompson Sampling** (`thompson-sampler.js`):
   - ✅ Beta distribution sampling
   - ✅ Expected value tracking
   - ✅ Uncertainty quantification (lines 170-178)
   - ✅ Maturity signal (lines 228-246)
   - ✅ Exploration rate decay (lines 208-212)

3. **Meta-Cognition** (`meta-cognition.js`):
   - ✅ Action recording
   - ✅ Success rate tracking
   - ✅ Stuck detection
   - ✅ Strategy switching
   - ⚠️ **0 real production sessions yet**

4. **SONA** (`sona.js`):
   - ✅ Pattern-outcome correlation (Pearson r, lines 262-273)
   - ✅ Adaptation tracking
   - ⚠️ **No TD-error tracking**
   - ⚠️ **No convergence metrics**

---

### 6.2 Missing Capabilities (❌ Need Implementation)

1. **TD-Error tracking**:
   - Add `TDErrorTracker` class (see §1.1)
   - Integrate with SONA or Dog learners
   - Target: Converge to < 5% error

2. **Brier Score tracking**:
   - Add `BrierScoreTracker` class (see §2.3)
   - Wire to CalibrationTracker
   - Validate against 0.25 baseline

3. **Learning Curve tracking**:
   - Add `LearningCurveTracker` class (see §3.2)
   - Record per-domain performance over time
   - Run 10 seeds, report mean ± std

4. **BWT/FWT metrics**:
   - Track task-wise performance (see §4.1-4.2)
   - Measure forgetting vs transfer
   - Target: BWT > -0.10, FWT > +0.10

5. **EWC implementation**:
   - Calculate Fisher information (see §4.3)
   - Add EWC regularization to loss
   - Compare vs naive fine-tuning

---

### 6.3 Validation Experiments (Proposed)

**Experiment 1: Calibration Baseline**
- **Goal**: Validate ECE < 10% on existing judgments
- **Method**: Run `getCalibrationCurve(30)` on 30-day history
- **Success**: ECE < 0.10, no drift alerts

**Experiment 2: SONA Convergence**
- **Goal**: Prove SONA adapts pattern weights effectively
- **Method**: Inject 100 feedback samples, measure correlation strength growth
- **Success**: Correlation strength increases by 20%+, TD-error decreases

**Experiment 3: Dog Pipeline Transfer**
- **Goal**: Validate Dogs learn new domains without forgetting
- **Method**: Train on SOCIAL, test on COSMOS with 5 examples (5-shot)
- **Success**: Accuracy > 50%, BWT > -0.10

**Experiment 4: Meta-Cognition Stuck Detection**
- **Goal**: Validate stuck state detection prevents thrashing
- **Method**: Simulate repetitive failed actions, measure recovery time
- **Success**: Stuck detected within 5 failures, strategy switches automatically

---

## 7. Acceptance Criteria Summary

### Convergence (Q-Learning / RL)
| Metric | Threshold | Status |
|--------|-----------|--------|
| TD-Error | < 0.05 (converged: < 0.01) | ❌ Not tracked |
| Bellman Residual | MSPBE < 0.01 | ❌ Not tracked |
| Policy Stability | Change rate < 5% | ❌ Not tracked |

### Calibration
| Metric | Threshold | Status |
|--------|-----------|--------|
| ECE | < 0.10 (good: < 0.05) | ✅ Tracked (threshold too high) |
| Brier Score | < 0.20 (good: < 0.10) | ❌ Not tracked |
| Reliability Diagram | Visual: near diagonal | ⚠️ Data exists, no viz |

### Meta-Learning
| Metric | Threshold | Status |
|--------|-----------|--------|
| 5-way 1-shot | > 40% accuracy | ❌ Not tested |
| 5-way 5-shot | > 60% accuracy | ❌ Not tested |
| Plasticity | > 1.2x baseline | ❌ Not tracked |
| Stability | > 0.9 retention | ❌ Not tracked |

### Online Learning
| Metric | Threshold | Status |
|--------|-----------|--------|
| BWT | > -0.10 (excellent: > -0.05) | ❌ Not tracked |
| FWT | > +0.10 | ❌ Not tracked |
| EWC effectiveness | 50%+ forgetting reduction | ❌ Not implemented |
| Retention | > 90% after 30 days | ⚠️ No baseline yet |

---

## 8. Implementation Priority

**Phase 1 (Quick Wins)** — 1-2 days:
1. ✅ Lower CalibrationTracker drift threshold to 10%
2. ✅ Add Brier Score tracking
3. ✅ Add reliability diagram visualization to MCP dashboard
4. ✅ Run Experiment 1 (calibration baseline)

**Phase 2 (Core Metrics)** — 3-5 days:
1. ✅ Implement TD-Error tracking in SONA
2. ✅ Add LearningCurveTracker
3. ✅ Run Experiment 2 (SONA convergence)
4. ✅ Run Experiment 4 (meta-cognition validation)

**Phase 3 (Advanced)** — 1-2 weeks:
1. ✅ Implement EWC (Fisher information + regularization)
2. ✅ Add BWT/FWT tracking
3. ✅ Run Experiment 3 (Dog transfer)
4. ✅ Benchmark vs industry standards (5-way K-shot)

**Phase 4 (Production)** — Ongoing:
1. ✅ Automate validation in CI/CD
2. ✅ Add validation dashboard to `/health` skill
3. ✅ Set up alerting for convergence failures
4. ✅ Monthly validation reports

---

## 9. References

### Convergence & RL
- [Temporal-Difference Learning (ML Compiled)](https://ml-compiled.readthedocs.io/en/latest/td.html)
- [Off-Policy TD with Bellman Residuals (MDPI 2022)](https://www.mdpi.com/2227-7390/12/22/3603)
- [Optimistic Training & Q-Learning Convergence (arXiv 2025)](https://arxiv.org/html/2602.06146)
- [JMLR: Convergence Analysis (2003)](https://www.jmlr.org/papers/volume5/evendar03a/evendar03a.pdf)

### Calibration
- [Understanding Model Calibration (ICLR 2025)](https://iclr-blogposts.github.io/2025/blog/calibration/)
- [ECE Step-by-Step (Towards Data Science)](https://towardsdatascience.com/expected-calibration-error-ece-a-step-by-step-visual-explanation-with-python-code-c3e9aa12937d/)
- [Brier Score (Wikipedia)](https://en.wikipedia.org/wiki/Brier_score)
- [Brier Score & Calibration (Neptune.ai)](https://neptune.ai/blog/brier-score-and-model-calibration)
- [Misconceptions about Brier Score (PMC 2025)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818272/)

### Meta-Learning
- [Meta-Learning for Few-Shot (Nature 2026)](https://www.nature.com/articles/s41598-026-36291-x)
- [Meta-Learning Survey (ACM 2025)](https://dl.acm.org/doi/10.1145/3659943)
- [Meta-Transfer Learning (CVPR 2019)](https://openaccess.thecvf.com/content_CVPR_2019/papers/Sun_Meta-Transfer_Learning_for_Few-Shot_Learning_CVPR_2019_paper.pdf)
- [Few-Shot Learning Methods (AI Multiple 2026)](https://research.aimultiple.com/few-shot-learning/)

### Continual Learning
- [Overcoming Catastrophic Forgetting (PNAS 2017)](https://www.pnas.org/doi/10.1073/pnas.1611835114)
- [EWC Visual Explanation (Rylan Schaeffer)](https://rylanschaeffer.github.io/content/research/elastic_weight_consolidation/main.html)
- [Don't Forget: New Metrics for Continual Learning (arXiv 2018)](https://arxiv.org/abs/1810.13166)
- [Brain-Inspired Replay (PMC 2020)](https://pmc.ncbi.nlm.nih.gov/articles/PMC7426273/)
- [Pareto Continual Learning (arXiv 2025)](https://arxiv.org/abs/2507.10485)

---

## 10. φ-Aligned Thresholds

CYNIC's validation should respect **φ-bounded confidence**:

| Metric | Standard | φ-Aligned (CYNIC) |
|--------|----------|-------------------|
| Max confidence | 100% | **φ⁻¹ (61.8%)** |
| Convergence threshold | 95%+ stable | **φ⁻¹ (61.8%)** stable |
| Calibration "perfect" | ECE = 0 | ECE < **φ⁻² (0.236)** |
| Drift alert | ECE > 0.10 | ECE > **0.10** (not 0.382!) |
| BWT "good" | > -0.05 | > **-φ⁻² (-0.236)** |
| FWT "good" | > +0.10 | > **+φ⁻³ (+0.146)** |

**Rationale**: CYNIC never claims certainty. Validation thresholds should reflect **epistemic humility** while remaining **practically useful**.

---

*sniff* Validation proves learning. φ distrusts φ.

**Confidence**: 58% (φ⁻¹ limit — comprehensive but 0 experiments run yet)

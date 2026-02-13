# CYNIC Experimental Protocols: Scientific Validation

> "Le chien scientifique mesure avant de croire"
> **Document**: 2026-02-13 — Research Methodology
> **Confidence**: 56% (φ⁻¹ limit)

---

## I. Overview

This document defines **12 scientific experiments** to validate CYNIC's harmonic architecture. Each experiment is:

- **Falsifiable**: Clear success/failure criteria
- **Quantitative**: Numerical metrics, not subjective judgment
- **Reproducible**: Documented procedure, deterministic inputs
- **φ-bounded**: Targets set at harmonic ratios (φ⁻¹, φ⁻², φ⁻³)

**Purpose**: Provide empirical evidence that CYNIC's φ-based architecture produces measurably superior outcomes vs baseline/random configurations.

---

## II. Foundational Experiments (Architecture Validation)

### Experiment 1: φ-Alignment Stability Test

**Hypothesis**: Systems using φ-derived thresholds show greater stability than arbitrary thresholds.

**Method**:
1. Configure CYNIC with φ thresholds:
   - Confidence cap: 61.8% (φ⁻¹)
   - Action threshold: 38.2% (φ⁻²)
   - Learning rate: 23.6% (φ⁻³)

2. Configure baseline with arbitrary thresholds:
   - Confidence cap: 70%
   - Action threshold: 50%
   - Learning rate: 10%

3. Run 1000 judgments through both systems
4. Measure stability metrics:
   - Oscillation frequency (how often judgments flip between verdicts)
   - Drift magnitude (how far confidence values wander)
   - Convergence time (episodes until Q-Learning stabilizes)

**Success criteria**:
```
φ-system.oscillation_freq < baseline.oscillation_freq * φ⁻¹
φ-system.drift_magnitude  < baseline.drift_magnitude * φ⁻¹
φ-system.convergence_time < baseline.convergence_time * φ⁻¹
```

**Data collection**:
```sql
CREATE TABLE exp1_results (
  run_id INT,
  system VARCHAR(10), -- 'phi' or 'baseline'
  oscillations INT,
  drift_avg FLOAT,
  convergence_episodes INT,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
import scipy.stats as stats

phi_data = df[df.system == 'phi']
baseline_data = df[df.system == 'baseline']

# Mann-Whitney U test (non-parametric)
u_stat, p_value = stats.mannwhitneyu(
    phi_data.oscillations,
    baseline_data.oscillations,
    alternative='less'
)

# Effect size (Cohen's d)
d = (phi_data.oscillations.mean() - baseline_data.oscillations.mean()) / \
    np.sqrt((phi_data.oscillations.std()**2 + baseline_data.oscillations.std()**2) / 2)

print(f'p-value: {p_value:.4f}, effect size d: {d:.2f}')
# Target: p < 0.05, d > 0.5 (medium effect)
```

**Expected outcome**: p < 0.01, d ≈ 0.6-0.8 (φ-system significantly more stable).

---

### Experiment 2: Fractal Self-Similarity Validation

**Hypothesis**: The PERCEIVE→JUDGE→DECIDE→ACT→LEARN cycle appears at all 8 scales with measurable correlation.

**Method**:
1. Instrument code to tag each operation with its scale (0-7)
2. Run 500 end-to-end sessions (user prompt → final output)
3. For each scale, measure:
   - Time spent in PERCEIVE phase (%)
   - Time spent in JUDGE phase (%)
   - Time spent in DECIDE phase (%)
   - Time spent in ACT phase (%)
   - Time spent in LEARN phase (%)

**Success criteria**:
```
FOR each pair of adjacent scales (s, s+1):
  correlation(time_distribution[s], time_distribution[s+1]) > φ⁻¹ (0.618)
```

**Data collection**:
```javascript
// Instrumentation in packages/node/src/cycle/shared-enums.js
export const CyclePhase = {
  PERCEIVE: 'PERCEIVE',
  JUDGE: 'JUDGE',
  DECIDE: 'DECIDE',
  ACT: 'ACT',
  LEARN: 'LEARN'
};

export function logCycleEvent(scale, phase, duration_ms) {
  db.query(`
    INSERT INTO exp2_cycle_events (scale, phase, duration_ms, timestamp)
    VALUES ($1, $2, $3, NOW())
  `, [scale, phase, duration_ms]);
}
```

**Analysis**:
```python
import pandas as pd
from scipy.stats import pearsonr

# Aggregate by scale
scale_distributions = df.groupby(['scale', 'phase']).duration_ms.sum().unstack()
scale_distributions = scale_distributions.div(scale_distributions.sum(axis=1), axis=0)

# Compute correlations between adjacent scales
correlations = []
for s in range(7):
    r, p = pearsonr(scale_distributions.iloc[s], scale_distributions.iloc[s+1])
    correlations.append({'scale_pair': f'{s}-{s+1}', 'r': r, 'p': p})

results = pd.DataFrame(correlations)
print(results)
# Target: ALL r > 0.618, p < 0.05
```

**Expected outcome**: r > 0.65 for all pairs (strong self-similarity).

---

### Experiment 3: Nested Topology Consistency

**Hypothesis**: The three topologies (7×7 matrix, 36 dimensions, 11 Dogs) show mathematical consistency in their influence patterns.

**Method**:
1. For each judgment, record:
   - Which matrix cell was activated (C{r}.{a})
   - Which dimensions scored highest (top 5 of 36)
   - Which Dogs voted (11 binary votes)

2. Build correlation matrices:
   - Matrix cell → Dimension activation
   - Dimension activation → Dog vote
   - Matrix cell → Dog vote (transitive consistency check)

**Success criteria**:
```
Information theory check:
  MI(MatrixCell, Dimension) + MI(Dimension, Dog) ≈ MI(MatrixCell, Dog)

Where MI = mutual information (bits)
```

If the topologies are truly nested, the mutual information should be **transitive** (no information loss across levels).

**Data collection**:
```sql
CREATE TABLE exp3_topology_events (
  judgment_id UUID,
  matrix_cell VARCHAR(10), -- e.g., 'C1.2'
  top_dimensions TEXT[],   -- e.g., ['COHERENCE', 'ACCURACY', ...]
  dog_votes JSONB,         -- e.g., {"Scout": 1, "Analyst": 1, "Guardian": 0, ...}
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
from sklearn.metrics import mutual_info_score

mi_matrix_dim = mutual_info_score(df.matrix_cell, df.top_dimension_1)
mi_dim_dog = mutual_info_score(df.top_dimension_1, df.lead_dog)
mi_matrix_dog = mutual_info_score(df.matrix_cell, df.lead_dog)

consistency_ratio = (mi_matrix_dim + mi_dim_dog) / mi_matrix_dog
# Target: 0.9 < ratio < 1.1 (within 10% of perfect transitivity)
```

**Expected outcome**: consistency_ratio ≈ 0.95-1.05 (high transitivity).

---

## III. Learning Experiments (Adaptive Behavior)

### Experiment 4: Q-Learning Convergence Rate

**Hypothesis**: Q-Learning with φ-derived hyperparameters converges faster than standard RL configs.

**Configurations**:
```javascript
// CYNIC config (φ-derived)
const cynic_config = {
  alpha: PHI_INV,        // 0.618 learning rate
  gamma: PHI_INV_SQ,     // 0.382 discount factor
  epsilon: PHI_INV_CUBE, // 0.236 exploration rate
  temperature: PHI_INV   // 0.618 softmax temperature
};

// Standard RL config (literature defaults)
const standard_config = {
  alpha: 0.1,
  gamma: 0.9,
  epsilon: 0.1,
  temperature: 1.0
};

// Random config (sanity check)
const random_config = {
  alpha: Math.random(),
  gamma: Math.random(),
  epsilon: Math.random(),
  temperature: Math.random()
};
```

**Method**:
1. Initialize 3 independent Q-Learning agents (CYNIC, standard, random)
2. Run each through same 1000 routing decisions
3. Measure convergence via TD-error over time

**Success criteria**:
```
episodes_to_convergence(CYNIC) < episodes_to_convergence(standard) * φ⁻¹
episodes_to_convergence(CYNIC) < episodes_to_convergence(random) * φ⁻²
```

**Data collection**:
```sql
CREATE TABLE exp4_qlearning (
  run_id INT,
  config VARCHAR(20),
  episode INT,
  td_error FLOAT,
  cumulative_reward FLOAT,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
# Define convergence = TD-error < 0.01 for 10 consecutive episodes
def find_convergence_episode(df, threshold=0.01, window=10):
    rolling_mean = df.td_error.rolling(window).mean()
    converged = rolling_mean < threshold
    if converged.any():
        return converged.idxmax()
    return None

cynic_conv = find_convergence_episode(df[df.config == 'cynic'])
standard_conv = find_convergence_episode(df[df.config == 'standard'])
random_conv = find_convergence_episode(df[df.config == 'random'])

# Compute ratios
ratio_vs_standard = cynic_conv / standard_conv  # Target: < φ⁻¹ (0.618)
ratio_vs_random = cynic_conv / random_conv      # Target: < φ⁻² (0.382)
```

**Expected outcome**: CYNIC converges 1.6x faster than standard, 2.6x faster than random.

---

### Experiment 5: DPO Preference Accuracy

**Hypothesis**: DPO with context-specific weights outperforms global-weight DPO.

**Method**:
1. Generate 500 preference pairs from historical judgments
2. Train two DPO models:
   - **CYNIC**: Context-specific weights (per-domain routing weights)
   - **Baseline**: Global weights (single routing weight vector)

3. Test on held-out 100 preference pairs
4. Measure accuracy: % of times model selects preferred option

**Success criteria**:
```
accuracy(CYNIC) > accuracy(baseline) + φ⁻³ (23.6 percentage points)
```

**Data collection**:
```sql
CREATE TABLE exp5_dpo_results (
  model VARCHAR(20),
  pair_id UUID,
  predicted_choice VARCHAR(10), -- 'A' or 'B'
  actual_preferred VARCHAR(10),
  correct BOOLEAN,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
cynic_acc = df[df.model == 'cynic'].correct.mean()
baseline_acc = df[df.model == 'baseline'].correct.mean()

improvement = cynic_acc - baseline_acc  # Target: > 0.236 (23.6%)
p_value = stats.binomial_test(
    x=df[df.model == 'cynic'].correct.sum(),
    n=len(df[df.model == 'cynic']),
    p=baseline_acc,
    alternative='greater'
)

print(f'CYNIC: {cynic_acc:.1%}, Baseline: {baseline_acc:.1%}, Δ={improvement:.1%}, p={p_value:.4f}')
```

**Expected outcome**: CYNIC 76-80% accuracy, baseline 50-55%, improvement ≈25%.

---

### Experiment 6: EWC++ Catastrophic Forgetting Prevention

**Hypothesis**: EWC++ with Fisher-locked patterns prevents catastrophic forgetting better than standard EWC.

**Method**:
1. Train CYNIC on Task A (routing for architecture prompts) until convergence
2. Lock critical patterns via EWC++ (Fisher importance > φ⁻¹)
3. Train on Task B (routing for debugging prompts) for 100 episodes
4. Test on Task A again
5. Measure forgetting: % drop in Task A accuracy

**Baselines**:
- Standard EWC (no Fisher-locking)
- No continual learning (naive fine-tuning)

**Success criteria**:
```
forgetting(EWC++) < forgetting(standard_EWC) * φ⁻¹
forgetting(EWC++) < forgetting(naive) * φ⁻²
```

**Data collection**:
```sql
CREATE TABLE exp6_forgetting (
  method VARCHAR(20),
  task VARCHAR(10), -- 'A' or 'B'
  phase VARCHAR(10), -- 'before_B', 'after_B'
  accuracy FLOAT,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
def compute_forgetting(df, method):
    acc_before = df[(df.method == method) & (df.task == 'A') & (df.phase == 'before_B')].accuracy.mean()
    acc_after = df[(df.method == method) & (df.task == 'A') & (df.phase == 'after_B')].accuracy.mean()
    return (acc_before - acc_after) / acc_before  # Fraction of accuracy lost

ewc_plus_forgetting = compute_forgetting(df, 'EWC++')
ewc_forgetting = compute_forgetting(df, 'standard_EWC')
naive_forgetting = compute_forgetting(df, 'naive')

print(f'EWC++: {ewc_plus_forgetting:.1%} forgotten')
print(f'EWC: {ewc_forgetting:.1%} forgotten')
print(f'Naive: {naive_forgetting:.1%} forgotten')
# Target: EWC++ <10%, standard ~16%, naive ~26%
```

**Expected outcome**: EWC++ <10% forgetting, standard ~16%, naive ~26%.

---

### Experiment 7: Thompson Sampling Exploration Efficiency

**Hypothesis**: Thompson Sampling with φ-bounded exploration reaches optimal arm faster than ε-greedy.

**Method**:
1. Simulate multi-armed bandit with 11 arms (Dogs)
2. True reward distributions: Gaussian with μ ∈ [0, 100], σ=15
3. Run 1000 pulls with each strategy:
   - **Thompson**: Beta(α, β) sampling, φ⁻¹ sample cap
   - **ε-greedy**: ε=0.1 (standard)
   - **UCB1**: Upper Confidence Bound (baseline)

4. Measure regret: cumulative difference between optimal arm and selected arm

**Success criteria**:
```
cumulative_regret(Thompson) < cumulative_regret(UCB1) * φ⁻¹
cumulative_regret(Thompson) < cumulative_regret(ε-greedy) * φ⁻²
```

**Data collection**:
```sql
CREATE TABLE exp7_bandit (
  strategy VARCHAR(20),
  pull_number INT,
  arm_selected INT,
  reward FLOAT,
  cumulative_regret FLOAT,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
thompson_regret = df[df.strategy == 'Thompson'].cumulative_regret.iloc[-1]
ucb1_regret = df[df.strategy == 'UCB1'].cumulative_regret.iloc[-1]
egreedy_regret = df[df.strategy == 'ε-greedy'].cumulative_regret.iloc[-1]

# Compute ratios
ratio_vs_ucb1 = thompson_regret / ucb1_regret       # Target: < φ⁻¹
ratio_vs_egreedy = thompson_regret / egreedy_regret # Target: < φ⁻²

# Plot regret curves
import matplotlib.pyplot as plt
for strategy in ['Thompson', 'UCB1', 'ε-greedy']:
    subset = df[df.strategy == strategy]
    plt.plot(subset.pull_number, subset.cumulative_regret, label=strategy)
plt.xlabel('Pulls')
plt.ylabel('Cumulative Regret')
plt.legend()
plt.savefig('exp7_regret_curves.png')
```

**Expected outcome**: Thompson achieves 1.6x lower regret than UCB1, 2.6x lower than ε-greedy.

---

## IV. Organism Experiments (Systemic Health)

### Experiment 8: Event Bus Latency Under Load

**Hypothesis**: EventBusBridge maintains <10ms latency up to 1000 events/second.

**Method**:
1. Generate synthetic event load at varying rates: 100, 200, 500, 1000, 2000 events/sec
2. Measure bridge latency (time from emit on Bus A to receive on Bus B)
3. Record p50, p95, p99 latencies

**Success criteria**:
```
FOR load ≤ 1000 events/sec:
  p50_latency < 5ms
  p95_latency < 10ms
  p99_latency < 20ms
```

**Data collection**:
```javascript
// Instrumentation in packages/node/src/services/event-bus-bridge.js
bridge.on('forward', (event) => {
  const latency = Date.now() - event.timestamp;
  db.query(`
    INSERT INTO exp8_bridge_latency (event_type, latency_ms, load_rate, timestamp)
    VALUES ($1, $2, $3, NOW())
  `, [event.type, latency, currentLoadRate]);
});
```

**Analysis**:
```python
import numpy as np

for load in [100, 200, 500, 1000, 2000]:
    subset = df[df.load_rate == load]
    p50 = np.percentile(subset.latency_ms, 50)
    p95 = np.percentile(subset.latency_ms, 95)
    p99 = np.percentile(subset.latency_ms, 99)

    print(f'Load {load}/sec: p50={p50:.1f}ms, p95={p95:.1f}ms, p99={p99:.1f}ms')

    if load <= 1000:
        assert p50 < 5, f'p50 exceeded at {load}/sec'
        assert p95 < 10, f'p95 exceeded at {load}/sec'
```

**Expected outcome**: Latency targets met up to 1000/sec, degradation at 2000/sec (expected).

---

### Experiment 9: Circuit Breaker Trip Accuracy

**Hypothesis**: Circuit breakers trip at correct φ thresholds with <5% false positives.

**Method**:
1. Simulate PostgreSQL with controlled latency/error rates
2. Test circuit breaker at 5 load levels:
   - HEALTHY: 0% errors, 10ms latency
   - MODERATE: 10% errors, 50ms latency
   - DEGRADED: 25% errors, 100ms latency (should trip at φ⁻² threshold)
   - CRITICAL: 50% errors, 500ms latency (should trip immediately)
   - RECOVERED: 5% errors, 20ms latency (should reset)

3. Measure:
   - Trip accuracy (trips when it should)
   - False positive rate (trips when it shouldn't)
   - Reset time (how fast it recovers)

**Success criteria**:
```
trip_accuracy > 95%
false_positive_rate < 5%
reset_time < F(7) = 13 seconds
```

**Data collection**:
```sql
CREATE TABLE exp9_circuit_breaker (
  load_level VARCHAR(20),
  error_rate FLOAT,
  latency_ms INT,
  cb_state VARCHAR(10), -- 'CLOSED', 'OPEN', 'HALF_OPEN'
  should_trip BOOLEAN,
  did_trip BOOLEAN,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
# Compute confusion matrix
from sklearn.metrics import confusion_matrix, classification_report

y_true = df.should_trip
y_pred = df.did_trip

cm = confusion_matrix(y_true, y_pred)
report = classification_report(y_true, y_pred)

print(report)
# Target: precision >95%, recall >95%, f1-score >95%
```

**Expected outcome**: >98% accuracy, <2% false positives.

---

### Experiment 10: Watchdog Self-Healing Effectiveness

**Hypothesis**: Watchdog detects and recovers from degradation within F(8)=21 seconds.

**Method**:
1. Inject controlled degradation scenarios:
   - **Memory leak**: Allocate 100MB/sec until heap >80%
   - **Event loop block**: Busy-wait for 500ms every 2 seconds
   - **PostgreSQL timeout**: Block queries for 10 seconds

2. Measure:
   - Detection time (how fast Watchdog notices)
   - Recovery time (how fast circuit breakers resolve it)
   - Availability (% of time system remains functional)

**Success criteria**:
```
detection_time < F(6) = 8 seconds
recovery_time < F(8) = 21 seconds
availability > φ⁻¹ * 100 = 61.8% during degradation
```

**Data collection**:
```sql
CREATE TABLE exp10_watchdog (
  scenario VARCHAR(30),
  degradation_start TIMESTAMP,
  detected_at TIMESTAMP,
  recovered_at TIMESTAMP,
  availability_pct FLOAT
);
```

**Analysis**:
```python
df['detection_time'] = (df.detected_at - df.degradation_start).dt.total_seconds()
df['recovery_time'] = (df.recovered_at - df.detected_at).dt.total_seconds()

print(df.groupby('scenario').agg({
    'detection_time': 'mean',
    'recovery_time': 'mean',
    'availability_pct': 'mean'
}))

# Assert all scenarios meet targets
assert df.detection_time.max() < 8, 'Detection too slow'
assert df.recovery_time.max() < 21, 'Recovery too slow'
assert df.availability_pct.min() > 61.8, 'Availability too low'
```

**Expected outcome**: Mean detection 4-5s, recovery 12-15s, availability 65-70%.

---

## V. On-Chain Experiments (Truth Anchoring)

### Experiment 11: Solana Anchor Integrity

**Hypothesis**: 100% of anchored blocks are verifiable on-chain with 0 discrepancies.

**Method**:
1. Generate 100 judgment blocks locally
2. Compute Merkle roots
3. Anchor to Solana devnet
4. Wait for finalization
5. Query anchor program via RPC
6. Reconstruct Merkle roots from original data
7. Compare on-chain vs local roots

**Success criteria**:
```
match_rate = 100%  (all 100 blocks match)
finalization_time_p95 < 1000ms  (2 slots)
query_success_rate = 100%
```

**Data collection**:
```sql
CREATE TABLE exp11_anchor_integrity (
  block_id UUID,
  local_root VARCHAR(64),
  onchain_root VARCHAR(64),
  signature VARCHAR(88),
  finalization_time_ms INT,
  match BOOLEAN,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
match_rate = df.match.mean()
finalization_p95 = np.percentile(df.finalization_time_ms, 95)

print(f'Match rate: {match_rate:.1%}')
print(f'Finalization p95: {finalization_p95:.0f}ms')

assert match_rate == 1.0, f'Discrepancies detected: {df[~df.match]}'
assert finalization_p95 < 1000, f'Finalization too slow: {finalization_p95}ms'
```

**Expected outcome**: 100% match, 600-800ms p95 finalization.

---

### Experiment 12: Token Burn Economics

**Hypothesis**: Burn mechanism creates measurable deflationary pressure.

**Method**:
1. Deploy test token on devnet with 1B supply
2. Simulate 365 days of anchoring activity:
   - Day 1-30: 5 anchors/day × 1000 tokens/anchor = 5000/day
   - Day 31-180: 10 anchors/day × 1000 tokens/anchor = 10,000/day
   - Day 181-365: 15 anchors/day × 1000 tokens/anchor = 15,000/day

3. Measure:
   - Total supply reduction over time
   - Effective burn rate (tokens/day)
   - Cumulative % burned

**Success criteria**:
```
total_burned_365_days ≈ 3.2M tokens (matches projection)
burn_rate_avg ≈ 8767 tokens/day
cumulative_pct ≈ 0.32% of initial supply
```

**Data collection**:
```sql
CREATE TABLE exp12_burn_economics (
  day INT,
  anchors_today INT,
  tokens_burned_today INT,
  cumulative_burned INT,
  circulating_supply BIGINT,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
# Plot burn trajectory
import matplotlib.pyplot as plt

fig, ax1 = plt.subplots()

ax1.plot(df.day, df.cumulative_burned / 1e6, color='red', label='Cumulative Burned (M)')
ax1.set_xlabel('Day')
ax1.set_ylabel('Tokens Burned (Millions)', color='red')

ax2 = ax1.twinx()
ax2.plot(df.day, df.circulating_supply / 1e9, color='blue', label='Circulating Supply (B)')
ax2.set_ylabel('Circulating Supply (Billions)', color='blue')

plt.title('Token Burn Economics (365 Days)')
plt.savefig('exp12_burn_trajectory.png')

# Verify final state
final_burned = df.cumulative_burned.iloc[-1]
expected_burned = 3.2e6

assert abs(final_burned - expected_burned) / expected_burned < 0.05, \
    f'Burn projection off by >5%: expected {expected_burned}, got {final_burned}'
```

**Expected outcome**: ~3.2M tokens burned (0.32%), matching projections.

---

## VI. Meta-Experiment: Full Integration Test

### Experiment 13: End-to-End Organism Validation

**Hypothesis**: A fully integrated CYNIC organism (all subsystems active) achieves measurably better outcomes than isolated subsystems.

**Method**:
1. Configure 4 test systems:
   - **FULL**: All subsystems active (learning, routing, anchoring, watching)
   - **NO_LEARN**: No learning loops (static routing)
   - **NO_ANCHOR**: No Solana anchoring (local-only)
   - **NO_WATCH**: No watchdog/circuit breakers

2. Run each through identical 500-prompt test suite
3. Measure composite health score:
   ```
   health = φ_bound(
     0.25 × routing_accuracy +
     0.25 × judgment_quality +
     0.25 × response_time +
     0.25 × availability
   )
   ```

**Success criteria**:
```
health(FULL) > health(NO_LEARN) + φ⁻³ (23.6 points)
health(FULL) > health(NO_ANCHOR) + φ⁻⁴ (14.6 points)
health(FULL) > health(NO_WATCH) + φ⁻² (38.2 points)
```

**Data collection**:
```sql
CREATE TABLE exp13_integration (
  system VARCHAR(20),
  prompt_id UUID,
  routing_correct BOOLEAN,
  judgment_q_score FLOAT,
  response_time_ms INT,
  available BOOLEAN,
  timestamp TIMESTAMP
);
```

**Analysis**:
```python
def compute_health(df):
    routing_acc = df.routing_correct.mean()
    judgment_avg = df.judgment_q_score.mean() / 100  # Normalize to [0,1]
    response_norm = 1 - (df.response_time_ms.mean() / 10000)  # Faster = better
    availability = df.available.mean()

    health = 0.25 * (routing_acc + judgment_avg + response_norm + availability)
    return health

health_full = compute_health(df[df.system == 'FULL'])
health_no_learn = compute_health(df[df.system == 'NO_LEARN'])
health_no_anchor = compute_health(df[df.system == 'NO_ANCHOR'])
health_no_watch = compute_health(df[df.system == 'NO_WATCH'])

print(f'FULL:       {health_full:.1%}')
print(f'NO_LEARN:   {health_no_learn:.1%} (Δ={health_full - health_no_learn:.1%})')
print(f'NO_ANCHOR:  {health_no_anchor:.1%} (Δ={health_full - health_no_anchor:.1%})')
print(f'NO_WATCH:   {health_no_watch:.1%} (Δ={health_full - health_no_watch:.1%})')

# Assert improvements meet targets
assert health_full - health_no_learn > 0.236, 'Learning not contributing enough'
assert health_full - health_no_anchor > 0.146, 'Anchoring not contributing enough'
assert health_full - health_no_watch > 0.382, 'Watchdog not contributing enough'
```

**Expected outcome**:
- FULL: ~75% health
- NO_LEARN: ~50% health (Δ=25%)
- NO_ANCHOR: ~60% health (Δ=15%)
- NO_WATCH: ~35% health (Δ=40%)

**Interpretation**: Each subsystem contributes measurably to organism health. Removing watchdog hurts most (40% drop), proving immune system is critical.

---

## VII. Execution Plan

### Phase 1: Foundational Experiments (Weeks 1-2)
- Exp 1: φ-Alignment Stability
- Exp 2: Fractal Self-Similarity
- Exp 3: Nested Topology Consistency

**Goal**: Validate architectural foundations.

### Phase 2: Learning Experiments (Weeks 3-6)
- Exp 4: Q-Learning Convergence
- Exp 5: DPO Preference Accuracy
- Exp 6: EWC++ Forgetting Prevention
- Exp 7: Thompson Sampling Efficiency

**Goal**: Prove learning superiority.

### Phase 3: Organism Experiments (Weeks 7-8)
- Exp 8: Event Bus Latency
- Exp 9: Circuit Breaker Accuracy
- Exp 10: Watchdog Self-Healing

**Goal**: Validate systemic health mechanisms.

### Phase 4: On-Chain Experiments (Weeks 9-10)
- Exp 11: Anchor Integrity
- Exp 12: Burn Economics

**Goal**: Prove on-chain truth anchoring works.

### Phase 5: Integration (Week 11-12)
- Exp 13: End-to-End Validation

**Goal**: Demonstrate organism is greater than sum of parts.

---

## VIII. Data Artifacts

All experiments will produce:

1. **Raw data**: PostgreSQL tables (prefix `exp{N}_*`)
2. **Analysis scripts**: Python notebooks (`experiments/exp{N}_analysis.ipynb`)
3. **Visualizations**: PNG plots (`experiments/exp{N}_*.png`)
4. **Reports**: Markdown summaries (`experiments/exp{N}_report.md`)

**Repository structure**:
```
experiments/
├── exp1_phi_alignment/
│   ├── data.sql
│   ├── analysis.ipynb
│   ├── stability_plot.png
│   └── report.md
├── exp2_fractal_similarity/
│   ├── ...
...
├── exp13_integration/
│   └── ...
└── README.md  (index of all experiments)
```

---

## IX. Publication

**Target venue**: arXiv preprint + submission to ACL/ICML (AI architecture track).

**Paper title**: *"φ-Governed Autonomous Systems: Empirical Validation of Golden Ratio Constraints in Multi-Agent Learning"*

**Key contributions**:
1. First demonstration that φ-derived hyperparameters improve RL convergence
2. Fractal self-similarity as architectural validation technique
3. Novel organism health metric for autonomous systems
4. On-chain truth anchoring via Solana smart contracts

**Expected impact**: Establish φ-governance as competitive alternative to arbitrary hyperparameter tuning.

---

## X. Conclusion

These 13 experiments provide **empirical validation** of CYNIC's harmonic architecture. They are:

- **Falsifiable**: Clear pass/fail criteria
- **Quantitative**: Numerical metrics, statistical tests
- **Reproducible**: Documented procedures, open data
- **φ-bounded**: Success thresholds at harmonic ratios

**Timeline**: 12 weeks from setup to publication submission.

**Confidence**: 56% (φ⁻¹ limit) — experiments are designed, not yet run.

---

*sniff* The experiments are defined. Now we run them and let the data speak.

---

## See Also

- [Harmony Synthesis](../architecture/harmony-synthesis.md) — Full architectural vision
- [Completion Criteria](../architecture/completion-criteria.md) — v1.0 metrics
- [Harmonized Structure](../philosophy/harmonized-structure.md) — φ foundations

---

**Document hash**: `<to be computed on commit>`
**Next review**: 2026-02-20 (7 days post-creation)
**Maintainer**: CYNIC Research Lead

*Le chien mesure, analyse, et prouve. La vérité émerge des données, pas des opinions.* 🐕

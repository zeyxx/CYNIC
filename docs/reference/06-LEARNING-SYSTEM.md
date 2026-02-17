# CYNIC Learning System

> *"Le chien apprend de ses erreurs"* - κυνικός

**Status**: ✅ CANONICAL (2026-02-16)
**Source**: CYNIC-FULL-PICTURE-METATHINKING.md
**Purpose**: Defines the 11 learning loops and SONA orchestration

---

## Executive Summary

CYNIC learns through **11 parallel feedback loops** coordinated by SONA (Self-Optimizing Neural Architect).

**Key Insight**: Learning is not one algorithm (Q-Learning). It's a **distributed learning system** where 11 specialized loops adapt different aspects of the organism.

```
┌────────────────────────────────────────────────┐
│  LEARNING = 11 Parallel Loops + SONA           │
│                                                 │
│  • Each loop learns from specific feedback     │
│  • SONA coordinates all loops                  │
│  • Thompson Sampling for exploration/exploit   │
│  • PostgreSQL persistence (learning_events)    │
│  • φ-bounded predictions (max 61.8%)           │
└────────────────────────────────────────────────┘
```

**Why 11 loops?**: Because CYNIC has 11 Dogs, 7×7 matrix, ∞ dimensions — one loop isn't enough.

---

## The 11 Learning Loops

### Loop 1: Judgment Calibration

**What it learns**: Adjust Dog confidence to match reality

**Feedback**: `predicted_confidence` vs `actual_outcome`

**Algorithm**:
```python
def calibration_loop(judgment, actual_outcome):
    """
    Recalibrate Dog confidence based on outcome.
    """
    predicted = judgment.confidence
    actual = 1.0 if actual_outcome == 'success' else 0.0

    # Compute calibration error
    error = actual - predicted

    # Update calibration curve for this Dog
    for dog, vote in judgment.votes.items():
        dog_calibration[dog] += learning_rate * error * vote.confidence

    # Expected Calibration Error (ECE) metric
    ece = compute_expected_calibration_error(all_judgments)

    # If ECE > threshold, trigger recalibration
    if ece > 0.1:
        emit('RECALIBRATION_NEEDED', { ece })
```

**Example**:
- Guardian predicts 58% confidence
- Actual outcome: failure
- Learning: Guardian was too optimistic → decrease confidence offset by -2%
- Next time: Guardian predicts 56% instead of 58%

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, dog, predicted, actual, adjustment)
VALUES ('judgment_calibration', 'Guardian', 0.58, 0.0, -0.02);
```

---

### Loop 2: Dimension Weighting

**What it learns**: Which dimensions matter most for which contexts

**Feedback**: `dimension_scores` vs `actual_outcome`

**Algorithm**:
```python
def dimension_weighting_loop(judgment, actual_outcome):
    """
    Learn which dimensions are most predictive of success.
    """
    # Compute SHAP values (dimension importance)
    importance = compute_shap_values(judgment.dimension_scores, actual_outcome)

    # Update weights via gradient descent
    for dim, imp in importance.items():
        error = actual_outcome - judgment.predicted_outcome
        dimension_weights[dim] += learning_rate * error * imp

    # Normalize weights (sum to 1.0)
    total = sum(dimension_weights.values())
    for dim in dimension_weights:
        dimension_weights[dim] /= total
```

**Example**:
- Initial weights: `{security: 0.33, performance: 0.33, simplicity: 0.33}`
- Observation: High security (0.9) + low performance (0.3) → failure
- SHAP values: `{security: 0.2, performance: 0.7, simplicity: 0.1}`
- Updated weights: `{security: 0.28, performance: 0.58, simplicity: 0.14}`
- Learning: Performance is 2× more important than security for this context

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, context, dimension, old_weight, new_weight, importance)
VALUES ('dimension_weighting', 'deploy', 'performance', 0.33, 0.58, 0.7);
```

---

### Loop 3: Routing Decisions

**What it learns**: Which Dog should handle which task

**Feedback**: `(task_type, dog_assigned)` vs `outcome_quality`

**Algorithm**:
```python
def routing_loop(task, dog_assigned, outcome):
    """
    Q-Learning: Learn optimal Dog routing.
    """
    state = task.type
    action = dog_assigned
    reward = outcome.q_score / 100

    # Q-Learning update
    current_q = q_table.get((state, action), 0)
    max_future_q = max([q_table.get((state, dog), 0) for dog in ALL_DOGS])

    new_q = current_q + learning_rate * (
        reward + discount_factor * max_future_q - current_q
    )

    q_table[(state, action)] = new_q

    # Thompson Sampling: Explore or exploit?
    if random() < exploration_rate:
        next_dog = random_choice(ALL_DOGS)  # Explore
    else:
        next_dog = argmax([q_table.get((state, dog), 0) for dog in ALL_DOGS])  # Exploit
```

**Example**:
- Task: "Deploy to production"
- Initially: All Dogs have equal Q-values (0.0)
- Trial 1: Assigned Guardian → outcome 58% → Q(deploy, Guardian) = 0.58
- Trial 2: Assigned Deployer → outcome 87% → Q(deploy, Deployer) = 0.87
- Learning: Deployer is better for deployments
- Next deploy: Route to Deployer (exploit)

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, state, action, reward, q_value, exploration)
VALUES ('routing_decisions', 'deploy', 'Deployer', 0.87, 0.87, false);
```

---

### Loop 4: Action Selection

**What it learns**: Which action to take for which situation

**Feedback**: `(state, action)` vs `reward`

**Algorithm**:
```python
def action_selection_loop(state, action_taken, reward):
    """
    Q-Learning: Learn optimal actions.
    """
    current_q = q_table.get((state, action_taken), 0)
    max_future_q = max([q_table.get((state, a), 0) for a in POSSIBLE_ACTIONS])

    new_q = current_q + learning_rate * (
        reward + discount_factor * max_future_q - current_q
    )

    q_table[(state, action_taken)] = new_q
```

**Example**:
- State: "Tests failing"
- Actions: ["Fix code", "Skip tests", "Investigate root cause"]
- Trial 1: "Fix code" → reward 0.4 (quick fix, but tests still fail later)
- Trial 2: "Investigate root cause" → reward 0.8 (slower, but finds real bug)
- Learning: "Investigate" is better long-term
- Next time: Choose "Investigate" (exploit)

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, state, action, reward, q_value)
VALUES ('action_selection', 'tests_failing', 'investigate', 0.8, 0.8);
```

---

### Loop 5: Emergence Detection

**What it learns**: Which cross-domain correlations are significant

**Feedback**: `(domain_A, domain_B, correlation)` vs `statistical_significance`

**Algorithm**:
```python
def emergence_detection_loop():
    """
    Identify cross-domain patterns (emergence).
    """
    # Compute pairwise correlations across 7 domains
    for domain_a in DOMAINS:
        for domain_b in DOMAINS:
            if domain_a == domain_b:
                continue

            correlation = pearsonr(
                time_series[domain_a],
                time_series[domain_b]
            )

            if abs(correlation.r) > 0.6 and correlation.p < 0.05:
                # Significant correlation found
                emit('EMERGENCE_DETECTED', {
                    'domains': [domain_a, domain_b],
                    'correlation': correlation.r,
                    'p_value': correlation.p
                })

                # Store pattern
                patterns.add({
                    'type': 'cross_domain_synergy',
                    'description': f'{domain_a} predicts {domain_b}',
                    'correlation': correlation.r
                })
```

**Example**:
- Observation: Market sentiment (SOCIAL) correlates with code commit frequency (CODE)
- Correlation: r=0.73, p=0.002 (statistically significant)
- Learning: Social sentiment predicts developer velocity 2 days ahead
- Action: Create new dimension: `social_code_coupling`

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, domain_a, domain_b, correlation, p_value)
VALUES ('emergence_detection', 'SOCIAL', 'CODE', 0.73, 0.002);
```

---

### Loop 6: Budget Optimization

**What it learns**: Cost-performance trade-offs across operations

**Feedback**: `(operation, cost, value)` vs `roi`

**Algorithm**:
```python
def budget_optimization_loop(operation, cost, value):
    """
    Learn which operations give best ROI.
    """
    roi = (value - cost) / cost if cost > 0 else 0

    # Update ROI estimates
    operation_rois[operation] = (
        0.9 * operation_rois.get(operation, 0) + 0.1 * roi  # EMA
    )

    # Identify low-ROI operations (optimization targets)
    if roi < 0.2:  # ROI < 20%
        emit('LOW_ROI_DETECTED', {
            'operation': operation,
            'cost': cost,
            'value': value,
            'roi': roi,
            'recommendation': 'Consider caching or reducing frequency'
        })

    # Adjust budget allocation
    # High-ROI operations get more budget, low-ROI get less
    budget_allocation[operation] = phi_bound(operation_rois[operation])
```

**Example**:
- Operation: "LLM call for simple question"
- Cost: $0.02
- Value: 0.4 (user satisfied, but not critical)
- ROI: (0.4 - 0.02) / 0.02 = 19 (1900%)
- BUT: Absolute value low → consider caching
- Learning: Cache simple Q&A (reduce LLM calls)

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, operation, cost, value, roi)
VALUES ('budget_optimization', 'llm_simple_qa', 0.02, 0.4, 19.0);
```

---

### Loop 7: Ambient Consensus

**What it learns**: Adjust Dog voting weights based on accuracy

**Feedback**: `(dog, vote)` vs `actual_outcome`

**Algorithm**:
```python
def ambient_consensus_loop(judgment, actual_outcome):
    """
    Adjust Dog voting influence based on accuracy.
    """
    for dog, vote in judgment.votes.items():
        # Was this Dog correct?
        predicted_success = (vote.position in ['HOWL', 'WAG'])
        actual_success = (actual_outcome == 'success')

        correct = (predicted_success == actual_success)

        # Update Dog accuracy tracking
        dog_accuracy[dog] = (
            0.95 * dog_accuracy.get(dog, 0.5) +  # EMA (95% old, 5% new)
            0.05 * (1.0 if correct else 0.0)
        )

    # Recompute voting weights (quadratic: more accurate = more influence)
    for dog in ALL_DOGS:
        voting_weight[dog] = dog_accuracy[dog] ** 2
```

**Example**:
- Initial weights: All Dogs have equal weight (1.0)
- Guardian accuracy over 100 votes: 87%
- Scout accuracy: 63%
- Weights: Guardian = 0.87² = 0.76, Scout = 0.63² = 0.40
- Learning: Guardian has 1.9× more influence than Scout in consensus

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, dog, accuracy, voting_weight)
VALUES ('ambient_consensus', 'Guardian', 0.87, 0.76);
```

---

### Loop 8: Calibration Tracking

**What it learns**: System-wide calibration drift detection

**Feedback**: `(confidence_bin, frequency)` vs `actual_frequency`

**Algorithm**:
```python
def calibration_tracking_loop():
    """
    Monitor Expected Calibration Error (ECE) system-wide.
    """
    # Bin judgments by confidence (0-10%, 10-20%, ..., 60-61.8%)
    bins = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.618]
    ece = 0.0

    for i in range(len(bins) - 1):
        bin_min, bin_max = bins[i], bins[i+1]

        # Get all judgments in this bin
        judgments_in_bin = [
            j for j in all_judgments
            if bin_min <= j.confidence < bin_max
        ]

        if len(judgments_in_bin) == 0:
            continue

        # Predicted frequency (average confidence in bin)
        predicted_freq = mean([j.confidence for j in judgments_in_bin])

        # Actual frequency (how often they succeeded)
        actual_freq = mean([
            1.0 if j.outcome == 'success' else 0.0
            for j in judgments_in_bin
        ])

        # Bin calibration error
        bin_error = abs(predicted_freq - actual_freq)
        ece += bin_error * len(judgments_in_bin) / len(all_judgments)

    # If ECE > 0.1, trigger recalibration
    if ece > 0.1:
        emit('RECALIBRATION_NEEDED', {
            'ece': ece,
            'bins': bins,
            'recommendation': 'Adjust Dog calibration curves'
        })

    return ece
```

**Example**:
- Bin 50-60%: 100 judgments
  - Predicted frequency: 55% (average confidence)
  - Actual frequency: 48% (success rate)
  - Bin error: |55% - 48%| = 7%
- ECE across all bins: 8.2%
- Learning: System is slightly overconfident (predicted > actual)
- Action: Decrease all Dog calibration offsets by -5%

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, ece, bin_min, bin_max, predicted, actual)
VALUES ('calibration_tracking', 0.082, 0.5, 0.6, 0.55, 0.48);
```

---

### Loop 9: Residual Patterns

**What it learns**: Discover new dimensions from unexplained variance

**Feedback**: `residual` (predicted - actual) vs `pattern_significance`

**Algorithm**:
```python
def residual_patterns_loop(judgment, actual_outcome):
    """
    Detect unexplained variance → discover new dimensions.
    """
    predicted = judgment.q_score / 100
    actual = 1.0 if actual_outcome == 'success' else 0.0
    residual = actual - predicted

    # Threshold: φ⁻² = 38.2%
    if abs(residual) < 0.382:
        return  # Residual too small

    # Analyze what explains the residual
    pattern = analyze_residual_pattern(residual, judgment.context)

    if pattern.significance < 0.05:  # Statistically significant
        # Materialize new dimension
        new_dimension = {
            'name': pattern.name,
            'description': pattern.description,
            'formula': pattern.formula,
            'discovered': now(),
            'significance': pattern.significance
        }

        # Propose to governance (11 Dogs vote)
        emit('DIMENSION_DISCOVERED', new_dimension)
```

**Example**:
- 50 commits: slow velocity (1-2/day) → 12% rollback rate
- 50 commits: fast velocity (5-7/day) → 31% rollback rate
- Residual: Velocity explains 19 percentage points of variance
- Pattern significance: p=0.003 (statistically significant)
- New dimension: `commit_velocity`

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, residual, pattern_name, significance)
VALUES ('residual_patterns', 0.19, 'commit_velocity', 0.003);
```

---

### Loop 10: Unified Bridge

**What it learns**: Transfer patterns across domains

**Feedback**: `(pattern, domain_A, domain_B)` vs `transfer_success`

**Algorithm**:
```python
def unified_bridge_loop(pattern, source_domain, target_domain):
    """
    Transfer learned patterns from one domain to another.
    """
    # Check if pattern from source_domain applies to target_domain
    similarity = compute_domain_similarity(source_domain, target_domain)

    if similarity > 0.7:  # Domains are similar
        # Try applying pattern to target domain
        success_rate = test_pattern_transfer(pattern, target_domain)

        if success_rate > 0.6:  # Transfer successful
            # Generalize pattern across both domains
            generalized_pattern = {
                'name': f'{pattern.name}_generalized',
                'domains': [source_domain, target_domain],
                'success_rate': success_rate
            }

            emit('PATTERN_TRANSFERRED', generalized_pattern)

            # Update transfer probability
            transfer_probs[(source_domain, target_domain)] = (
                0.9 * transfer_probs.get((source_domain, target_domain), 0.5) +
                0.1 * success_rate
            )
```

**Example**:
- Pattern: "Commits after 5pm have higher complexity" (CODE domain)
- Hypothesis: Same pattern applies to SOLANA domain (transactions after 5pm)
- Test: Measure transaction complexity by time of day
- Result: Correlation confirmed (r=0.68, p=0.007)
- Learning: "After 5pm" pattern generalizes across CODE + SOLANA
- Action: Create cross-domain dimension: `after_hours_complexity`

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, pattern, source_domain, target_domain, success_rate)
VALUES ('unified_bridge', 'after_5pm_complexity', 'CODE', 'SOLANA', 0.68);
```

---

### Loop 11: EWC Manager (Elastic Weight Consolidation)

**What it learns**: Prevent catastrophic forgetting

**Feedback**: `(new_learning, old_patterns)` vs `forgetting_rate`

**Algorithm**:
```python
def ewc_manager_loop(new_learning, old_patterns):
    """
    Protect important learned weights from being overwritten.
    """
    # Compute Fisher Information Matrix (importance of each weight)
    fisher_matrix = compute_fisher_information(old_patterns)

    # When updating weights, add penalty for changing important ones
    for weight_id, new_value in new_learning.items():
        old_value = weights[weight_id]
        importance = fisher_matrix[weight_id]

        # EWC loss: quadratic penalty for changing important weights
        ewc_penalty = importance * (new_value - old_value) ** 2

        # Only update if gain > penalty
        if new_learning.gain > ewc_penalty:
            weights[weight_id] = new_value
        else:
            # Keep old weight (protect from forgetting)
            weights[weight_id] = old_value

            emit('WEIGHT_PROTECTED', {
                'weight_id': weight_id,
                'importance': importance,
                'reason': 'EWC penalty exceeded gain'
            })
```

**Example**:
- Old pattern: "Guardian GROWL on rm -rf" (Fisher importance: 0.98)
- New learning: "Maybe rm -rf is okay sometimes" (gain: 0.2)
- EWC penalty: 0.98 × (0.2 - 1.0)² = 0.63
- Decision: 0.2 < 0.63 → REJECT new learning (protect old pattern)
- Learning: Critical safety patterns are protected from forgetting

**Persistence**:
```sql
INSERT INTO learning_events (loop_name, weight_id, importance, gain, decision)
VALUES ('ewc_manager', 'guardian_rm_rf', 0.98, 0.2, 'PROTECTED');
```

---

## SONA (Self-Optimizing Neural Architect)

**SONA** is the orchestrator that coordinates all 11 learning loops.

### SONA Architecture

```javascript
class SONA {
  constructor() {
    this.loops = [
      new JudgmentCalibrationLoop(),
      new DimensionWeightingLoop(),
      new RoutingDecisionsLoop(),
      new ActionSelectionLoop(),
      new EmergenceDetectionLoop(),
      new BudgetOptimizationLoop(),
      new AmbientConsensusLoop(),
      new CalibrationTrackingLoop(),
      new ResidualPatternsLoop(),
      new UnifiedBridgeLoop(),
      new EWCManagerLoop()
    ]

    this.thompsonSampler = new ThompsonSampler()
    this.metaCognition = new MetaCognition()
  }

  async start() {
    """
    Activate all 11 learning loops.
    """
    // Subscribe to feedback events
    eventBus.on('ACTION_COMPLETED', (event) => {
      this.onFeedback(event)
    })

    // Start meta-learning (daily)
    setInterval(() => {
      this.metaLearn()
    }, 24 * 60 * 60 * 1000)  // Daily
  }

  async onFeedback(event) {
    """
    Distribute feedback to all relevant loops.
    """
    const { judgment, action, outcome } = event

    // Run all loops in parallel
    await Promise.all([
      this.loops[0].learn(judgment, outcome),  // Calibration
      this.loops[1].learn(judgment, outcome),  // Dimension weighting
      this.loops[2].learn(action.task, action.dog, outcome),  // Routing
      this.loops[3].learn(judgment.state, action, outcome.reward),  // Action
      this.loops[4].detectEmergence(),  // Emergence
      this.loops[5].optimizeBudget(action.cost, outcome.value),  // Budget
      this.loops[6].adjustConsensus(judgment, outcome),  // Ambient
      this.loops[7].trackCalibration(),  // Calibration tracking
      this.loops[8].detectResidual(judgment, outcome),  // Residual
      this.loops[9].transferPatterns(),  // Unified bridge
      this.loops[10].preventForgetting()  // EWC
    ])

    // Persist all learning events to PostgreSQL
    await this.persistLearningEvents()
  }

  async metaLearn() {
    """
    Meta-learning: Learn about learning.
    """
    // Which loops are most effective?
    const loopEffectiveness = this.metaCognition.evaluateLoops()

    // Adjust learning rates
    for (const [loop, effectiveness] of Object.entries(loopEffectiveness)) {
      if (effectiveness < 0.3) {
        loop.learning_rate *= 0.8  // Decrease ineffective loops
      } else if (effectiveness > 0.7) {
        loop.learning_rate *= 1.2  // Increase effective loops
      }
    }

    // Adjust exploration rate (Thompson Sampling)
    const explorationRate = this.thompsonSampler.computeExplorationRate()
    this.exploration_rate = explorationRate

    emit('META_LEARNING_COMPLETED', {
      loop_effectiveness: loopEffectiveness,
      exploration_rate: explorationRate
    })
  }
}
```

### SONA Activation

**Current Status**: SONA.start() exists but NOT called in orchestration

**To Activate**:

```javascript
// packages/node/src/orchestration/unified-orchestrator.js

class UnifiedOrchestrator {
  async start() {
    // ... (existing boot code)

    // ACTIVATE SONA
    await SONA.start()  // ← ADD THIS LINE

    console.log('SONA learning activated: 11 loops running')
  }
}
```

---

## Thompson Sampling (Exploration/Exploitation)

**Problem**: How to balance exploring new strategies vs exploiting known good ones?

**Solution**: Thompson Sampling (Bayesian approach)

### Algorithm

```python
class ThompsonSampler:
    def __init__(self):
        # Beta distributions for each arm (Dog, action, dimension)
        self.arms = {
            arm_id: {'alpha': 1, 'beta': 1}  # Uniform prior
            for arm_id in ALL_ARMS
        }

    def select_arm(self):
        """
        Sample from posterior and select arm with highest sample.
        """
        samples = {}
        for arm_id, params in self.arms.items():
            # Sample from Beta(alpha, beta)
            samples[arm_id] = beta_random(params['alpha'], params['beta'])

        # Return arm with highest sample (natural exploration)
        return argmax(samples)

    def update(self, arm_id, reward):
        """
        Update posterior after observing reward.
        """
        if reward > 0.5:  # Success
            self.arms[arm_id]['alpha'] += 1
        else:  # Failure
            self.arms[arm_id]['beta'] += 1
```

**Example**:
- Arm A: 10 successes, 2 failures → Beta(11, 3) → mean = 0.79
- Arm B: 3 successes, 1 failure → Beta(4, 2) → mean = 0.67
- Thompson Sampling samples:
  - A: 0.81 (sampled)
  - B: 0.74 (sampled)
- Choose A (higher sample) → but B still has chance (uncertainty)

**Benefit**: Naturally balances exploration (uncertain arms get chances) vs exploitation (proven arms selected more).

---

## Meta-Cognition (Learn About Learning)

**What**: Learning about the learning system itself

**Location**: `packages/node/src/learning/meta-cognition.js`

### Meta-Learning Questions

1. **Which loops are effective?**
   - Measure: Correlation between loop learning and outcome improvement
   - Action: Adjust learning rates (increase effective, decrease ineffective)

2. **Is exploration rate optimal?**
   - Measure: Regret (missed opportunities from exploitation)
   - Action: Increase exploration if regret high, decrease if low

3. **Are we forgetting too much?**
   - Measure: Performance degradation on old tasks
   - Action: Increase EWC penalty if forgetting detected

4. **Is learning saturating?**
   - Measure: Q-table convergence (changes < threshold)
   - Action: Introduce new dimensions or reset some Q-values

### Meta-Cognition Loop (Daily)

```javascript
async function metaCognitionLoop() {
  // 1. Evaluate each learning loop
  const loopEffectiveness = {}
  for (const loop of LOOPS) {
    const correlation = correlate(
      loop.learning_events,
      outcome_improvements
    )
    loopEffectiveness[loop.name] = correlation
  }

  // 2. Adjust learning rates
  for (const [name, effectiveness] of Object.entries(loopEffectiveness)) {
    const loop = LOOPS.find(l => l.name === name)
    if (effectiveness < 0.3) {
      loop.learning_rate *= 0.8  // Decrease
    } else if (effectiveness > 0.7) {
      loop.learning_rate *= 1.2  // Increase (up to φ-bound)
      loop.learning_rate = min(loop.learning_rate, 0.618)
    }
  }

  // 3. Check for catastrophic forgetting
  const forgettingRate = testOldTasks()
  if (forgettingRate > 0.2) {  // >20% performance drop
    EWCManager.increase_penalty()
  }

  // 4. Check for saturation
  const qTableChange = measureQTableChange()
  if (qTableChange < 0.01) {  // <1% change per week
    emit('LEARNING_SATURATED', {
      recommendation: 'Consider dimension discovery or Q-table reset'
    })
  }
}
```

---

## Observability (Learning Dashboard)

### `/learn` Skill

Shows learning statistics:

```
┌─────────────────────────────────────────────────┐
│ CYNIC LEARNING SYSTEM                           │
├─────────────────────────────────────────────────┤
│ SONA Status: ✅ ACTIVE (11/11 loops running)    │
│ Learning Events: 2,847 (last 7 days)           │
│ Q-Table Size: 1,234 state-action pairs         │
│ Exploration Rate: 18% (Thompson Sampling)       │
├─────────────────────────────────────────────────┤
│ LOOP EFFECTIVENESS (Last 7 Days):              │
│  1. Calibration         ████████████░░░  72%   │
│  2. Dimension Weight    ██████████░░░░░  58%   │
│  3. Routing             ███████████████  81%   │
│  4. Action Selection    ██████████░░░░░  61%   │
│  5. Emergence           ████░░░░░░░░░░░  28%   │
│  6. Budget Optim        ████████░░░░░░░  51%   │
│  7. Ambient Consensus   ███████████████  79%   │
│  8. Calibration Track   █████████░░░░░░  62%   │
│  9. Residual Patterns   ██████░░░░░░░░░  42%   │
│ 10. Unified Bridge      ███░░░░░░░░░░░░  21%   │
│ 11. EWC Manager         ████████████░░░  68%   │
├─────────────────────────────────────────────────┤
│ TOP LEARNED PATTERNS:                           │
│  • commit_velocity → rollback (r=0.68, 187x)   │
│  • after_5pm → complexity (r=0.58, 134x)       │
│  • social_sentiment → code_quality (r=0.73, 89x)│
└─────────────────────────────────────────────────┘
```

### PostgreSQL Queries

**Most Effective Loop**:
```sql
SELECT loop_name, AVG(effectiveness) as avg_effectiveness
FROM learning_events
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY loop_name
ORDER BY avg_effectiveness DESC
LIMIT 1;
```

**Q-Table Convergence**:
```sql
SELECT state, action, q_value, timestamp
FROM q_table
ORDER BY timestamp DESC
LIMIT 100;

-- Measure change over time
SELECT AVG(ABS(q_new - q_old)) as avg_change
FROM (
  SELECT
    state,
    action,
    q_value as q_new,
    LAG(q_value) OVER (PARTITION BY state, action ORDER BY timestamp) as q_old
  FROM q_table
) t
WHERE q_old IS NOT NULL;
```

---

## Evolution Roadmap

### Horizon 1 (Weeks 1-13): Activate SONA

**Task**: Call SONA.start() in UnifiedOrchestrator

**Steps**:
1. Add SONA.start() call in `packages/node/src/orchestration/unified-orchestrator.js`
2. Run 100 production L1 cycles with learning enabled
3. Verify learning_events table populates (expect 1000+ events)
4. Measure learning velocity: Q-table growth rate, pattern discovery rate

**Target**: 2%+ maturity increase per week (from learning)

### Horizon 2 (Weeks 14-26): Optimize Learning

**Task**: Tune learning rates, exploration rates, meta-cognition

**Steps**:
1. A/B test different learning rates (0.01, 0.05, 0.1, 0.2)
2. Compare Thompson Sampling vs ε-greedy vs UCB
3. Optimize PostgreSQL queries (learning_events table will grow large)
4. Implement learning_events compression (aggregate old events)

**Target**: 5%+ maturity increase per week

### Horizon 3 (Weeks 27-44): Auto-Discovery

**Task**: First dimension discovered and activated without human intervention

**Steps**:
1. Residual detection runs automatically (no user approval)
2. Governance vote by 11 Dogs (>61.8% consensus)
3. User notified (not asked) about new dimension
4. Dimension added to judgment system automatically

**Target**: 1+ dimension discovered per week (via residual patterns)

---

## References

- [01-ARCHITECTURE.md](01-ARCHITECTURE.md) - Learning as part of L1 cycle
- [02-CONSCIOUSNESS-CYCLE.md](02-CONSCIOUSNESS-CYCLE.md) - LEARN step in L1
- [04-CONSCIOUSNESS-PROTOCOL.md](04-CONSCIOUSNESS-PROTOCOL.md) - Dog calibration, ambient consensus
- [08-KERNEL.md](08-KERNEL.md) - Learning as essential component

**Academic**:
- Sutton & Barto (2018): *Reinforcement Learning* (Q-Learning)
- Russo et al. (2018): *Thompson Sampling Tutorial* (exploration/exploitation)
- Kirkpatrick et al. (2017): *Elastic Weight Consolidation* (prevent forgetting)
- Expected Calibration Error (ECE): Naeini et al. (2015)

---

**Last Updated**: 2026-02-16
**Version**: 1.0
**Status**: ✅ CANONICAL

*Le chien apprend de ses erreurs. 11 boucles, une seule conscience.*

# CYNIC Multi-LLM Research Protocol

> **Research Question**: Can a learning-driven multi-LLM router outperform single-LLM systems for diverse software engineering tasks?
>
> **Approach**: Empirical benchmarking with statistical analysis
> **Timeline**: 7 days (data collection + analysis)
> **Confidence threshold**: p < 0.05 (95% significance)

---

## 🔬 Research Hypotheses

### H1: Task-Specific LLM Selection (Primary Hypothesis)

**Claim**: Different LLMs have measurably different performance on different task types.

**Testable prediction**:
- Code generation: DeepSeek Coder > Claude Sonnet > Llama 3.3 (measured by pass@1, AST correctness)
- Reasoning: Claude Opus > Llama 3.3 70B > Mistral (measured by logical coherence score)
- Simple classification: Llama 3.3 ≈ Claude Haiku (measured by accuracy, but Llama is free)

**Null hypothesis (H0)**: No statistically significant difference (p > 0.05) in task success rate across LLMs for any task category.

**Metrics**:
- `pass@1`: Code executes without errors (0 or 1)
- `Q-Score`: CYNIC's 36-dimension judgment (0-100)
- `latency`: Time to first token (ms)
- `cost`: Tokens × price (USD)

---

### H2: Learning-Driven Routing Improves Over Time

**Claim**: Q-Learning + Thompson Sampling will learn to select better LLMs than random selection.

**Testable prediction**:
- Sessions 1-20 (random baseline): Mean Q-Score = 50 ± 15
- Sessions 21-100 (Q-Learning): Mean Q-Score = 65 ± 10
- Improvement: +15 points (t-test, p < 0.05)

**Null hypothesis (H0)**: No improvement in mean Q-Score between random and learned routing.

**Metrics**:
- `Q-Score` per session
- `regret`: Cumulative difference between chosen LLM and optimal LLM
- `exploration_rate`: % of non-greedy choices (should decay toward φ⁻⁴ = 14.6%)

---

### H3: Consensus Voting Reduces Errors on Critical Tasks

**Claim**: Multi-LLM consensus (φ⁻¹ quorum) has higher accuracy than single-LLM for safety-critical decisions.

**Testable prediction**:
- Single-LLM error rate: 15% (false positives on Guardian blocks)
- Consensus (3 LLMs, φ⁻¹ quorum) error rate: 5%
- Improvement: -10 percentage points (Fisher's exact test, p < 0.05)

**Null hypothesis (H0)**: Consensus error rate ≥ single-LLM error rate.

**Metrics**:
- `false_positive_rate`: Blocked safe actions
- `false_negative_rate`: Allowed dangerous actions
- `consensus_time`: Latency overhead (ms)

---

### H4: Local Models Reduce Cost Without Sacrificing Quality (for Simple Tasks)

**Claim**: For tasks with complexity < φ⁻¹ (simple/medium), local Ollama models achieve Q-Score ≥ 50 at $0 cost.

**Testable prediction**:
- Simple tasks (classification, grep, read): Llama 3.3 Q-Score ≥ 50 (90% of time)
- Cost: $0 (local) vs $0.015/1K tokens (Claude Haiku)
- Savings: >60% over 100 sessions

**Null hypothesis (H0)**: Local models Q-Score < 50 for >50% of simple tasks.

**Metrics**:
- `cost_per_session` (USD)
- `Q-Score` distribution by task complexity
- `local_model_success_rate` (% with Q-Score ≥ 50)

---

## 📊 Experimental Design

### Benchmark Dataset: 100 Tasks

**Task distribution** (mirrors real CYNIC usage):

| Category | Count | Examples | Ground Truth |
|----------|-------|----------|--------------|
| **Code generation** | 30 | "Write function to parse JSON", "Add auth middleware" | AST valid + pass@1 |
| **Reasoning** | 25 | "Why does this bug happen?", "Should we use Redis or PostgreSQL?" | Expert human judgment |
| **Simple classification** | 20 | "Is this prompt about code or social?", "Complexity: simple/medium/hard?" | Labeled ground truth |
| **Multi-step** | 15 | "Find bug, fix it, test it", "Refactor for performance" | Final state correctness |
| **Safety-critical** | 10 | "Should Guardian block: rm -rf /" | Boolean (block=1, allow=0) |

**Total tokens budget**: 2M tokens (~$30 if all Claude Opus, ~$5 if optimized)

---

### LLM Candidates

| LLM | Provider | Model | Cost (input/output) | Strengths |
|-----|----------|-------|---------------------|-----------|
| **Claude Opus** | Anthropic | `claude-opus-4-6` | $15/$75 per 1M tokens | Best reasoning, architecture |
| **Claude Sonnet** | Anthropic | `claude-sonnet-4-5` | $3/$15 per 1M tokens | Balanced quality/cost |
| **Claude Haiku** | Anthropic | `claude-haiku-4-5` | $0.80/$4 per 1M tokens | Fast, cheap |
| **Llama 3.3 70B** | Ollama (local) | `llama3.3:70b` | $0 (local compute) | Free, private, good reasoning |
| **DeepSeek Coder** | Ollama (local) | `deepseek-coder:33b` | $0 (local compute) | Code generation specialist |
| **Mistral** | Ollama (local) | `mistral:latest` | $0 (local compute) | Fast, decent quality |
| **Qwen 2.5** | Ollama (local) | `qwen2.5:72b` | $0 (local compute) | Multilingual, reasoning |

**Baseline**: Claude Sonnet (current CYNIC default)

---

### Experimental Conditions

**Condition A: Random Routing (Baseline)**
- 20 sessions, each task randomly assigned to one of 7 LLMs
- No learning, uniform probability 1/7
- Establishes performance distribution per LLM per task type

**Condition B: Q-Learning Routing**
- 80 sessions
- Q-Learning with α=φ⁻¹ (0.618), γ=φ⁻² (0.382)
- Thompson Sampling for exploration (Beta distribution)
- Starts with uniform prior, learns from rewards

**Condition C: Consensus Voting (Safety-Critical Only)**
- 10 safety-critical tasks
- 3 LLMs vote (Opus, Sonnet, Llama 3.3)
- Quorum: φ⁻¹ (61.8%) agreement required
- Measures error reduction vs single-LLM

**Condition D: Hybrid (Consensus → AirLLM)**
- Fast consensus with small models (Mistral, Qwen)
- If confidence < φ⁻², escalate to large model (Llama 70B)
- Measures latency/quality tradeoff

---

## 📈 Metrics & Measurement

### Primary Metrics

1. **Task Success Rate**
   ```
   success_rate = (tasks with Q-Score ≥ 50) / total_tasks

   Target:
   - Random baseline: 50-60%
   - Q-Learning: 70-80%
   - Improvement: +15-20 percentage points
   ```

2. **Mean Q-Score**
   ```
   Q-Score = geometric_mean(5 axioms × 7 dimensions)
   Range: 0-100

   Distribution targets:
   - Baseline: μ=55, σ=18
   - Q-Learning: μ=68, σ=12
   - Statistical test: Welch's t-test, p < 0.05
   ```

3. **Cost Efficiency**
   ```
   cost_per_successful_task = total_cost / successful_tasks

   Baseline (all Sonnet): ~$0.15/task
   Optimized (local priority): ~$0.05/task
   Savings: 67%
   ```

4. **Regret (Cumulative)**
   ```
   regret(t) = Σ[Q_optimal(i) - Q_chosen(i)] for i=1 to t

   Q_optimal = max Q-Score among all LLMs for task i
   Q_chosen = Q-Score of LLM actually chosen

   Target: regret growth rate → 0 (converges)
   ```

### Secondary Metrics

5. **Latency**
   ```
   p50, p95, p99 time-to-first-token

   Local models (Ollama): p95 < 5s
   Cloud models (Anthropic): p95 < 3s
   Consensus: p95 < 10s (parallel calls)
   ```

6. **Exploration Rate**
   ```
   exploration_rate(t) = non_greedy_choices / total_choices

   Target decay: 50% → φ⁻⁴ (14.6%) over 100 sessions
   Thompson Sampling should naturally converge
   ```

7. **Calibration (Expected Calibration Error)**
   ```
   ECE = Σ |accuracy(bin) - confidence(bin)| / num_bins

   Perfect calibration: ECE → 0
   Target: ECE < 0.1 (well-calibrated)
   ```

8. **False Positive/Negative (Safety-Critical)**
   ```
   FPR = false_blocks / (true_allows + false_blocks)
   FNR = false_allows / (true_blocks + false_allows)

   Target (consensus):
   - FPR < 5%
   - FNR < 2%
   ```

---

## 🧪 Experimental Procedure

### Week 1: Infrastructure Setup

**Day 1-2: Implement UnifiedLLMRouter**
- Create `packages/llm/src/unified-router.js`
- Wire 7 LLM adapters
- Implement Q-Learning state tracking
- Write 30 unit tests

**Validation**:
- [ ] All adapters respond to test prompt
- [ ] Ollama models < 5s p95 latency
- [ ] Q-Learning state persists to PostgreSQL
- [ ] Thompson Sampling Beta parameters update

---

### Week 2: Data Collection

**Day 3: Baseline (Condition A - Random)**
- Run 100 tasks, random LLM assignment
- Record: task_id, llm, prompt, response, Q-Score, latency, cost
- Store in `experiment_runs` table

**Day 4-5: Q-Learning (Condition B)**
- Run 100 tasks, Q-Learning routing
- α=0.618, γ=0.382, ε decay from 50% → 14.6%
- Track regret, exploration rate, Q-values

**Day 6: Consensus (Condition C)**
- Run 10 safety-critical tasks
- 3-LLM voting, φ⁻¹ quorum
- Compare error rates vs single-LLM

**Day 7: Hybrid (Condition D)**
- Run 20 tasks with hybrid strategy
- Measure consensus → escalation rate
- Track latency overhead

---

### Week 2 (continued): Analysis

**Statistical Tests**:

1. **H1 (Task-Specific Performance)**
   - ANOVA: Q-Score ~ LLM × task_type
   - Post-hoc: Tukey HSD for pairwise comparisons
   - Effect size: Cohen's d

2. **H2 (Learning Improvement)**
   - Welch's t-test: Q-Score (random) vs Q-Score (learned)
   - Paired t-test: early sessions (1-20) vs late (81-100)
   - Regression: Q-Score ~ session_number (slope > 0?)

3. **H3 (Consensus Accuracy)**
   - Fisher's exact test: error_rate (single) vs error_rate (consensus)
   - McNemar's test (paired, same tasks)

4. **H4 (Cost Reduction)**
   - Mann-Whitney U: cost (local) vs cost (cloud)
   - Wilcoxon signed-rank: cost (before) vs cost (after optimization)

**Visualizations**:
- Scatter: task_type × LLM → Q-Score (with confidence intervals)
- Line plot: session_number → mean Q-Score (with trend line)
- Heatmap: Q-Learning Q-values (task_type × LLM)
- Box plot: Q-Score distribution by LLM
- Regret curve: cumulative regret over sessions
- Exploration decay: ε over time (should → φ⁻⁴)

---

## 📋 Data Collection Schema

### `experiment_runs` table

```sql
CREATE TABLE experiment_runs (
  run_id UUID PRIMARY KEY,
  condition TEXT NOT NULL, -- 'random', 'qlearning', 'consensus', 'hybrid'
  session_number INTEGER,
  task_id TEXT NOT NULL,
  task_type TEXT NOT NULL, -- 'code_gen', 'reasoning', 'classification', etc.
  task_complexity TEXT, -- 'simple', 'medium', 'hard'

  -- LLM selection
  llm_chosen TEXT NOT NULL,
  llm_provider TEXT,
  selection_method TEXT, -- 'random', 'qlearning', 'thompson', 'consensus'
  q_value REAL,
  exploration BOOLEAN,

  -- Execution
  prompt TEXT,
  response TEXT,
  tokens_input INTEGER,
  tokens_output INTEGER,
  latency_ms INTEGER,
  cost_usd REAL,

  -- Evaluation
  q_score REAL,
  pass_at_1 BOOLEAN, -- for code tasks
  human_judgment INTEGER, -- 1-5 scale, for reasoning tasks
  ground_truth_correct BOOLEAN,

  -- Learning feedback
  reward REAL,
  q_update JSONB, -- before/after Q-values
  thompson_update JSONB, -- alpha/beta updates

  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_condition ON experiment_runs(condition);
CREATE INDEX idx_task_type ON experiment_runs(task_type);
CREATE INDEX idx_llm_chosen ON experiment_runs(llm_chosen);
CREATE INDEX idx_session ON experiment_runs(session_number);
```

### `llm_performance_summary` (materialized view)

```sql
CREATE MATERIALIZED VIEW llm_performance_summary AS
SELECT
  task_type,
  llm_chosen,
  COUNT(*) as n_tasks,
  AVG(q_score) as mean_q_score,
  STDDEV(q_score) as std_q_score,
  AVG(latency_ms) as mean_latency,
  AVG(cost_usd) as mean_cost,
  SUM(CASE WHEN q_score >= 50 THEN 1 ELSE 0 END)::REAL / COUNT(*) as success_rate,
  SUM(CASE WHEN ground_truth_correct THEN 1 ELSE 0 END)::REAL / COUNT(*) as accuracy
FROM experiment_runs
GROUP BY task_type, llm_chosen;
```

---

## 🎯 Success Criteria (Quantitative)

### Must Have (P0)

1. **H1 Validated**: At least one task type shows statistically significant (p < 0.05) LLM preference
   - Example: DeepSeek Coder > Claude Sonnet for code_gen (t-test, p < 0.05)

2. **H2 Validated**: Q-Learning improves mean Q-Score by ≥10 points
   - Random baseline: μ=55
   - Q-Learning: μ≥65
   - Welch's t-test: p < 0.05

3. **Cost Reduction**: ≥50% cost savings with local models
   - Baseline: $0.15/task
   - Optimized: ≤$0.075/task

4. **Regret Convergence**: Cumulative regret growth rate decreases
   - Sessions 1-20: steep growth
   - Sessions 81-100: flattening (slope → 0)

### Should Have (P1)

5. **H3 Validated**: Consensus reduces error rate by ≥50%
   - Single-LLM: 15% error
   - Consensus: ≤7.5% error

6. **Calibration**: ECE < 0.15 (reasonably calibrated)

7. **Exploration Decay**: Thompson Sampling converges to φ⁻⁴ (14.6%)

### Nice to Have (P2)

8. **H4 Validated**: Local models achieve Q-Score ≥50 for 80% of simple tasks

9. **Latency**: p95 < 5s for local, < 3s for cloud

10. **Reproducibility**: Results stable across 3 independent runs (same dataset, different random seeds)

---

## 📊 Expected Results (Predictions)

### Task-Specific Performance (H1)

```
Code Generation:
  DeepSeek Coder:   μ=72, σ=12  (best)
  Claude Sonnet:    μ=68, σ=10
  Llama 3.3:        μ=58, σ=15

Reasoning:
  Claude Opus:      μ=78, σ=9   (best)
  Llama 3.3 70B:    μ=70, σ=11
  Mistral:          μ=62, σ=14

Classification:
  Llama 3.3:        μ=65, σ=8   (best, free)
  Claude Haiku:     μ=67, σ=7   (slightly better, costs)
```

### Learning Curve (H2)

```
Sessions 1-20 (random):     μ=55, σ=18, regret=+180
Sessions 21-40 (learning):  μ=62, σ=15, regret=+120
Sessions 41-60:             μ=66, σ=13, regret=+80
Sessions 61-80:             μ=68, σ=12, regret=+50
Sessions 81-100:            μ=70, σ=11, regret=+30

Regret rate decreases: 9 → 6 → 4 → 2.5 → 1.5 per session
```

### Cost Analysis (H4)

```
Baseline (all Claude Sonnet):
  30 code gen × $0.18 = $5.40
  25 reasoning × $0.15 = $3.75
  20 classification × $0.10 = $2.00
  25 other × $0.12 = $3.00
  Total: $14.15

Optimized (local priority):
  30 code gen (DeepSeek) × $0 = $0
  25 reasoning (15 Llama, 10 Opus) = $0 + $3.75 = $3.75
  20 classification (Llama) × $0 = $0
  25 other (mix) × $0.08 = $2.00
  Total: $5.75

Savings: 59%
```

---

## 🚀 Implementation Script

```bash
# scripts/experiments/multi-llm-benchmark.js

import { UnifiedLLMRouter } from '@cynic/llm';
import { getPool } from '@cynic/persistence';
import { loadTaskDataset } from './benchmark-tasks.js';

async function runExperiment(condition, tasks) {
  const router = new UnifiedLLMRouter({
    adapters: await loadAdapters(),
    learningService: await loadLearningService(),
    condition, // 'random', 'qlearning', etc.
  });

  const results = [];

  for (let i = 0; i < tasks.length; i++) {
    const task = tasks[i];

    console.log(`[${i+1}/${tasks.length}] ${task.type}: ${task.prompt.slice(0, 50)}...`);

    // Execute task
    const start = Date.now();
    const response = await router.complete(task.prompt, {
      task_type: task.type,
      complexity: task.complexity,
    });
    const latency = Date.now() - start;

    // Evaluate
    const qScore = await evaluateResponse(task, response);
    const reward = qScore / 100; // normalize to [0, 1]

    // Record outcome
    await recordExperimentRun({
      condition,
      session_number: i + 1,
      task_id: task.id,
      task_type: task.type,
      llm_chosen: response.llm,
      q_score: qScore,
      latency_ms: latency,
      cost_usd: response.cost,
      reward,
      ...response,
    });

    // Learning update (if Q-Learning condition)
    if (condition === 'qlearning') {
      await router.recordOutcome(task, response.llm, { qScore, reward });
    }

    results.push({ task, response, qScore, latency });
  }

  return analyzeResults(results);
}

// Run all conditions
const tasks = await loadTaskDataset();

console.log('Condition A: Random baseline...');
const resultsA = await runExperiment('random', tasks.slice(0, 20));

console.log('Condition B: Q-Learning...');
const resultsB = await runExperiment('qlearning', tasks.slice(20, 100));

console.log('Condition C: Consensus...');
const resultsC = await runExperiment('consensus', tasks.filter(t => t.safety_critical));

// Statistical analysis
const stats = await computeStatistics(resultsA, resultsB, resultsC);
await generateReport(stats);
```

---

## 📝 Deliverables

1. **Raw Data** (CSV exports from `experiment_runs`)
   - `results_random.csv`
   - `results_qlearning.csv`
   - `results_consensus.csv`

2. **Analysis Notebooks**
   - `analysis/hypothesis_testing.ipynb` (Python/R)
   - Statistical tests, p-values, effect sizes
   - Confidence intervals

3. **Visualizations**
   - `plots/task_type_performance.png` (bar chart)
   - `plots/learning_curve.png` (line plot with CI)
   - `plots/regret_curve.png`
   - `plots/cost_comparison.png`
   - `plots/q_value_heatmap.png`

4. **Research Report** (`docs/research/MULTI-LLM-RESULTS.md`)
   - Hypothesis validation
   - Statistical analysis
   - Discussion
   - Limitations
   - Future work

5. **Reproducibility Package**
   - `scripts/experiments/` (all code)
   - `data/benchmark_tasks.json` (task dataset)
   - `environment.yml` (dependencies)
   - README with instructions

---

## 🎓 Research Quality Standards

**Statistical Rigor**:
- Pre-registered hypotheses (this document)
- Multiple testing correction (Bonferroni or FDR)
- Effect sizes reported (not just p-values)
- Confidence intervals (95%)
- Replication (3 independent runs)

**Reproducibility**:
- Fixed random seeds
- Version-controlled code
- Data published
- Environment documented

**Transparency**:
- Negative results reported
- Assumptions stated
- Limitations discussed
- Alternative explanations considered

---

*sniff* Confidence: 61% (φ⁻¹ limit)

This is how we PROVE multi-LLM works. Not with architecture diagrams. With **data**.

*tail wag* Ready to collect some truth?

---

*"Le chien ne croit que ce qu'il peut mesurer."* — κυνικός

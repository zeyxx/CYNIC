# CYNIC LLM Benchmarking Pipeline

> "Le chien mesure sa morsure" - The dog measures its bite

Comprehensive benchmarking suite for measuring CYNIC's judgment accuracy across L1, L2, and L3 layers.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    BENCHMARKING LAYERS                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  L1 (Dog Heuristics)     L2 (LLM Judgment)     L3 (Learning)│
│  ├─ patterns.json        ├─ CYNIC Node         ├─ Feedback  │
│  ├─ rules.js             ├─ Anthropic API      ├─ Patterns  │
│  └─ ~1ms/sample          └─ ~2000ms/sample     └─ Δ F1/epoch│
│                                                              │
│  Metrics:                Metrics:              Metrics:      │
│  - Precision             - Precision           - Learning Δ  │
│  - Recall                - Recall              - Patterns +  │
│  - F1 Score              - F1 Score            - Falsified   │
│  - Verdict Accuracy      - Verdict Accuracy    - F1/pattern  │
│  - Latency               - Token Usage         - Convergence │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Run full benchmark suite
node benchmarks/pipeline/run-all.js

# Run specific benchmarks
node benchmarks/pipeline/run-all.js --l1       # L1 heuristics only
node benchmarks/pipeline/run-all.js --l2       # L2 LLM only
node benchmarks/pipeline/run-all.js --dogs     # Per-dog benchmark
node benchmarks/pipeline/run-all.js --l3       # L3 learning loop

# Quick mode (fewer samples)
node benchmarks/pipeline/run-all.js --quick
```

## Benchmarks

### 1. L1 vs L2 Pipeline (`llm-benchmark-pipeline.js`)

Compares Dog heuristics (L1) against full LLM judgment (L2).

**Outputs:**
- Precision, Recall, F1 for both layers
- Latency comparison (L1 ~1ms vs L2 ~2000ms)
- Verdict accuracy
- Recommendation: when to use L1 vs L2

**Usage:**
```bash
node benchmarks/pipeline/llm-benchmark-pipeline.js
node benchmarks/pipeline/llm-benchmark-pipeline.js --l1-only
node benchmarks/pipeline/llm-benchmark-pipeline.js --l2-only
```

### 2. Per-Dog Benchmark (`dog-benchmark.js`)

Measures individual Dog detection accuracy for their specialization.

**Dogs tested:**
- 🛡️ Guardian (Gevurah): Security vulnerabilities
- 📊 Analyst (Binah): Logic bugs, edge cases
- 🧹 Janitor (Yesod): Code quality, complexity
- 🏗️ Architect (Chesed): Design patterns
- 📚 Scholar (Daat): Fact verification
- 🔍 Scout (Netzach): Exploration patterns

**Usage:**
```bash
node benchmarks/pipeline/dog-benchmark.js
```

### 3. L3 Learning Benchmark (`l3-learning-benchmark.js`)

Measures learning loop effectiveness over multiple epochs.

**Tracks:**
- F1 improvement per epoch
- Patterns learned
- Patterns falsified
- Learning efficiency (F1 gain per pattern)

**Usage:**
```bash
node benchmarks/pipeline/l3-learning-benchmark.js
```

## Dataset

Uses `benchmarks/collective-vs-single/dataset/samples.json`:
- 20 code samples with known vulnerabilities
- Categories: SQL injection, XSS, auth, crypto, logic, etc.
- Ground truth: expected issues, verdicts, score ranges

## Results

Results are saved to `benchmarks/pipeline/results/`:
- `pipeline-{timestamp}.json` - L1 vs L2 comparison
- `dog-benchmark-{timestamp}.json` - Per-dog results
- `l3-learning-{timestamp}.json` - Learning progression
- `combined-{timestamp}.json` - All benchmarks combined

## Metrics Explained

### Precision
```
Precision = True Positives / (True Positives + False Positives)
```
How many detected issues are actually real? High precision = fewer false alarms.

### Recall
```
Recall = True Positives / (True Positives + False Negatives)
```
How many real issues did we detect? High recall = fewer missed vulnerabilities.

### F1 Score
```
F1 = 2 × (Precision × Recall) / (Precision + Recall)
```
Harmonic mean of precision and recall. The primary accuracy metric.

### Verdict Accuracy
```
Verdict Accuracy = Correct Verdicts / Total Samples
```
Did we get the right verdict (BARK/GROWL/WAG/HOWL)?

### Verdict Direction Accuracy
```
Direction Accuracy = Correct Direction / Total Samples
```
Did we at least get the severity direction right? (BARK/GROWL = bad, WAG/HOWL = good)

## φ-Aligned Thresholds

Following CYNIC's golden ratio philosophy:

| Threshold | Value | Meaning |
|-----------|-------|---------|
| φ⁻¹ | 61.8% | Max confidence, "green" zone |
| φ⁻² | 38.2% | Warning threshold, "yellow" zone |
| < 38.2% | Red | Critical, needs attention |

## Example Output

```
═══════════════════════════════════════════════════════════════
  L1 vs L2 COMPARISON
═══════════════════════════════════════════════════════════════

  METRIC              │    L1     │    L2     │    Δ
  ────────────────────┼───────────┼───────────┼──────────
  Precision           │   65.0%   │   82.0%   │  +17.0%
  Recall              │   55.0%   │   78.0%   │  +23.0%
  F1 Score            │   59.6%   │   79.9%   │  +20.3%
  Verdict Dir.        │   75.0%   │   90.0%   │  +15.0%
  ────────────────────┼───────────┼───────────┼──────────
  Avg Latency         │     2ms   │  2100ms   │  1050.0x

  ANALYSIS:
    ✅ L2 significantly outperforms L1 (F1 +20.3%)
    📊 Quality/Latency ratio: 0.0097 F1 per second

  RECOMMENDATION:
    → Hybrid approach: L1 first, escalate to L2 if uncertain
```

## Contributing

When adding new samples to the dataset:

1. Add to `benchmarks/collective-vs-single/dataset/samples.json`
2. Include ground truth with:
   - `issues`: Array of expected issues with type, severity
   - `expectedVerdict`: BARK/GROWL/WAG/HOWL
   - `minScore`, `maxScore`: Expected Q-Score range

## See Also

- `benchmarks/collective-vs-single/` - Original single vs collective benchmark
- `packages/node/src/dogs/` - Dog heuristics implementation
- `packages/node/src/dogs/learning-service.js` - L3 learning loop

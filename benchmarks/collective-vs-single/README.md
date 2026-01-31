# Benchmark: Collective Dogs vs Single Agent

> **Claim to test:** "11 dogs voting in parallel produce better code review judgments than a single Claude call with equivalent context"

## Methodology

### Experimental Design

```
┌─────────────────────────────────────────────────────────────┐
│                    GROUND TRUTH DATASET                      │
│         100 code samples with KNOWN issues                   │
│         (SQL injection, XSS, logic bugs, etc.)               │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────────┐
│   CONDITION A: SINGLE   │     │   CONDITION B: COLLECTIVE   │
│   One Claude call       │     │   11 Dogs + φ-consensus     │
│   Same context window   │     │   DogOrchestrator           │
└─────────────────────────┘     └─────────────────────────────┘
              │                               │
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────────┐
│   Output: Issue list    │     │   Output: Judgment + votes  │
└─────────────────────────┘     └─────────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    EVALUATION                                │
│   Compare outputs to ground truth                            │
│   Metrics: Precision, Recall, F1, Latency                   │
└─────────────────────────────────────────────────────────────┘
```

### Ground Truth Sources

1. **OWASP Top 10 Samples** - Known security vulnerabilities
2. **CWE Examples** - Common Weakness Enumeration test cases
3. **Synthetic Planted Bugs** - Controlled issue insertion
4. **Real Bug Reports** - Code from actual CVEs

### Metrics

| Metric | Formula | What it measures |
|--------|---------|------------------|
| Precision | TP / (TP + FP) | Of issues reported, how many are real? |
| Recall | TP / (TP + FN) | Of real issues, how many were found? |
| F1 Score | 2 * (P * R) / (P + R) | Balance of precision and recall |
| Latency | ms | Time to produce judgment |
| Cost | tokens | API usage |

### Statistical Requirements

- **N ≥ 100** samples for statistical power
- **p < 0.05** for significance claims
- **Effect size** (Cohen's d) to measure practical difference

## Kill Criteria

**CYNIC collective should be abandoned if:**

1. Single agent F1 ≥ Collective F1 (no quality gain)
2. Collective latency > 10x Single with F1 difference < 0.1
3. Collective precision < Single precision (more false positives)

## Running the Benchmark

```bash
# 1. Run single-agent baseline
node benchmarks/collective-vs-single/run-single.mjs

# 2. Run collective
node benchmarks/collective-vs-single/run-collective.mjs

# 3. Evaluate results
node benchmarks/collective-vs-single/evaluate.mjs
```

## Files

```
benchmarks/collective-vs-single/
├── README.md                 # This file
├── dataset/
│   ├── samples.json          # Code samples with ground truth
│   └── categories.json       # Issue categories
├── run-single.mjs            # Single-agent baseline
├── run-collective.mjs        # CYNIC collective
├── evaluate.mjs              # Compare to ground truth
└── results/
    └── *.json                # Run outputs
```

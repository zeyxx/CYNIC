# CYNIC Lab — Versioning & Fast Iteration

This document explains how to develop multiple domains in parallel with proper versioning, measurement, and rollback.

## The Problem We're Solving

Before this infrastructure:
- Heuristics hardcoded in Python (`.py` files)
- No version history
- Changes measured manually or not at all
- Can't roll back without git surgery
- Can't run A/B tests

After this infrastructure:
- Heuristics stored as versioned JSON artifacts
- Before/after measurements mandatory
- Automatic confusion matrix + sensitivity/specificity
- Fast iteration: change → measure → commit or revert
- Multi-domain development in parallel

## Directory Structure

```
cynic-python/
├── heuristics/              # Domain logic (Python)
│   ├── twitter_dog.py       # TwitterDog scorer
│   ├── twitter_signal_extractor.py
│   └── validate_twitter_scorer.py
│
├── lab/
│   ├── config/              # SSOT: domain definitions, axioms
│   │   ├── domains.yaml     # Domain list + targets (D1-D6)
│   │   ├── axioms.yaml      # Axiom definitions + weights
│   │   └── narrative_domains.yaml
│   │
│   ├── versions/
│   │   └── MANIFEST.yaml    # Versioning SSOT (THIS FILE)
│   │
│   ├── measurements/        # Before/after benchmarks
│   │   └── twitter_2026-04-30T14-30-00.json
│   │
│   ├── measure_domain_quality.py  # Measurement framework
│   └── lab.py              # Full analysis pipeline
│
├── artifacts/               # Exported gates (read by kernel)
│   ├── token_gates_v1.3.json       # Token thresholds (conviction-only)
│   ├── twitter_gates_v1.0.json     # Twitter signal extraction rules
│   └── wallet_gates_v1.0.json      # (placeholder, not yet implemented)
│
├── consumers/               # K15 consumers
│   └── k15_observation_consumer.py # Read obs → score → dispatch tasks
│
└── artifacts_kernel/        # Kernel reads these via include_str!()
    ├── token_gates_v1.3.json       # (symlink to artifacts/)
    └── twitter_gates_v1.0.json
```

## Fast Iteration Workflow

### Step 1: Establish Baseline

Before changing ANY heuristic, measure current quality:

```bash
cd cynic-python/lab
python measure_domain_quality.py \
  --domain twitter \
  --dataset ~/.cynic/organs/hermes/x/dataset.jsonl \
  --limit 100 \
  --baseline

# Output: measurements/twitter_2026-04-30T14-30-00.json
# Shows: confusion matrix, sensitivity, specificity, Pearson r
```

Store the output filename for comparison later.

### Step 2: Change Heuristic

1. Edit the heuristic code (`twitter_dog.py`, `twitter_signal_extractor.py`, etc.)
2. **Bump version** in `versions/MANIFEST.yaml` and `artifacts/twitter_gates_v1.X.json`
3. Document the change (what signal/rule changed and why)

Example:
```yaml
# MANIFEST.yaml
domains:
  twitter:
    current_version: "v1.1"  # Was v1.0
    artifacts:
      v1.1:
        date: "2026-04-30"
        description: "Increased LP-discussion weight from 0.8 → 1.0 (signal better discrimination)"
        metrics: null  # To be filled after measurement
```

### Step 3: Measure After Change

```bash
python measure_domain_quality.py \
  --domain twitter \
  --dataset ~/.cynic/organs/hermes/x/dataset.jsonl \
  --limit 100 \
  --after \
  --compare-to measurements/twitter_2026-04-30T14-30-00.json

# Output shows deltas:
#   Sensitivity delta: +0.045
#   Specificity delta: -0.012
#   Pearson r delta: +0.087
```

### Step 4: Decide: Commit or Revert

**Accept change if:**
- Sensitivity ↑ and specificity ↑ (or at least one ↑ with <5% drop in the other)
- Pearson r ↑ (better correlation with ground truth)
- Latency ↓ or negligible change

**Revert if:**
- Sensitivity drops >5% (missing rugs)
- Specificity drops >5% (false alarms)
- No improvement and latency ↑

### Step 5: Commit

When accepting a change:

```bash
# Update MANIFEST.yaml with actual metrics
cat > versions/MANIFEST.yaml << 'EOF'
domains:
  twitter:
    artifacts:
      v1.1:
        metrics:
          before_sensitivity: 0.75
          after_sensitivity: 0.80
          before_specificity: 0.88
          after_specificity: 0.87
          delta_pearson_r: +0.087
          latency_delta_ms: -0.5
EOF

# Commit to git
git add cynic-python/heuristics/ cynic-python/artifacts/ cynic-python/lab/versions/
git commit -m "feat(twitter-dog): increase LP-discussion weight → sensitivity +4.5%"
```

## Measurement Framework API

### `measure_domain_quality.py`

```python
from cynic_python.lab.measure_domain_quality import DomainMeasurement

measurer = DomainMeasurement(domain="twitter")
result = measurer.measure(
    dataset_path=Path("~/.cynic/organs/hermes/x/dataset.jsonl"),
    limit=100
)

# result = {
#   "timestamp": "2026-04-30T14:30:00Z",
#   "sample_size": 100,
#   "confusion_matrix": {"tp": 75, "fp": 12, "tn": 88, "fn": 25},
#   "sensitivity": 0.75,    # TP / (TP + FN)
#   "specificity": 0.88,    # TN / (TN + FP)
#   "pearson_r": 0.82,      # Correlation with ground truth
#   "latency_ms": {...},
#   "q_score_distribution": {...},
# }
```

### `versions/MANIFEST.yaml`

SSOT for all versioned Dogs, thresholds, and baselines. Before changing any heuristic, **update this file first**.

Structure:
```yaml
domains:
  twitter:
    current_version: "v1.0"
    artifacts:
      v1.0:
        date: "2026-04-30"
        commit_hash: "abc123"
        metrics: {sensitivity: 0.75, specificity: 0.88, ...}
```

### Artifact Files

JSON files read by the kernel at compile time:

- `token_gates_v1.3.json` — conviction thresholds, rug detection rules
- `twitter_gates_v1.0.json` — signal extraction rules, axiom weights
- `wallet_gates_v1.0.json` — (placeholder for Option C)

Each artifact includes:
- `version`: semver (v1.X)
- `measurement_baseline`: confusion matrix, sensitivity/specificity from ground truth
- `next_iteration`: what to measure next
- `heuristic_rules`: human-readable rules (for documentation)

## Multi-Domain Development

When working on multiple domains in parallel:

1. **One branch per domain:** `feat/twitter-dog-calibration-2026-04-30`
2. **Independent measurements:** Each domain has its own measurement file
3. **Shared MANIFEST.yaml:** Coordinate via git to avoid conflicts
4. **Async iteration:** Domains don't block each other

Example (parallel branches):

```bash
# Branch 1: Twitter calibration
git checkout -b feat/twitter-calibration-2026-04-30
python measure_domain_quality.py --domain twitter --baseline
# ... change heuristic ...
python measure_domain_quality.py --domain twitter --after --compare-to [baseline]
git commit

# Branch 2: Token accuracy feedback (parallel, independent)
git checkout -b feat/token-feedback-loop-2026-04-30
python measure_domain_quality.py --domain token --baseline
# ... wire conviction-only to kernel ...
python measure_domain_quality.py --domain token --after
git commit
```

## K15 Consumer Integration

The K15 consumer (`k15_observation_consumer.py`) reads versioned gates:

```python
# Loads artifact at runtime
import json
twitter_gates = json.load(open("cynic-python/artifacts/twitter_gates_v1.0.json"))

# Scores observations using current version
signal_thresholds = twitter_gates["high_signal_patterns"]
```

When you bump a version, the consumer automatically uses the new thresholds on the next kernel restart.

## Deployment Pipeline

```
Dev Workflow:
  1. measure_domain_quality.py --baseline
  2. [edit heuristic]
  3. measure_domain_quality.py --after --compare-to baseline
  4. [decide: commit or revert]
  5. git commit (if accepting)
  6. Push branch → PR

Kernel Compilation:
  7. `cargo build` reads artifact versions via include_str!()
  8. Binary locked to specific artifact versions

Deployment:
  9. Copy binary to ~/bin/cynic-kernel
  10. Restart kernel (systemctl restart cynic-kernel)
  11. Consumer picks up new thresholds on next observation poll
```

## Best Practices

1. **Measure before changing.** Always establish a baseline.
2. **One variable at a time.** Change one signal/weight, then measure.
3. **Track commits.** Update `MANIFEST.yaml` with commit_hash for every version.
4. **Document the why.** What signal was missing? What accuracy gap were you fixing?
5. **Test on real data.** Synthetic validation good, but measure on `dataset.jsonl` (real 4,146 tweets).
6. **Delta thresholds:**
   - Sensitivity: accept if ↑ or <5% drop
   - Specificity: accept if ↑ or <5% drop
   - Pearson r: accept if ↑ or >0.85 already
7. **Revert aggressively.** If unsure, revert and discuss before next attempt.
8. **Document the chain.** git blame should show: version → PR → decision (why accepted/reverted).

## Next Domains to Add

After Twitter (v1.0) and Token (v1.3):

1. **Wallet (v1.0)** — Holder concentration, archetype consistency
2. **Behavior (v1.0)** — Interaction patterns, scroll velocity, deliberation time
3. **Chess (v1.0)** — Position evaluation, move legality
4. **Sovereignty (v1.0)** — Self-hosting signals, decentralization claims
5. **Epistemology (v1.0)** — Knowledge claims, falsifiability

Each follows the same workflow:
- Design ground truth (what cases are HOWL/GROWL/BARK?)
- Implement heuristic scorer
- Measure baseline
- Iterate with measurement feedback
- Commit versioned artifact

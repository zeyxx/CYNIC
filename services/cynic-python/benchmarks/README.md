# Benchmarks — Self-Evolving Data-Centric Inference Framework

Framework to measure and optimize judge (Dog) inference quality and performance. Closed-loop architecture: observe → analyze → recommend → measure impact → iterate.

## Philosophy

**Data-centric, not kernel-centric:**
- OBSERVE: Real behavior (latency, consistency, hallucination)
- MEASURE BEFORE deciding: hypothesis without evidence = waste
- VERSION artifacts: all observations timestamped, git-tracked
- ITERATE: measure impact of each change via convergence analysis
- **K15 CONSUMER**: Every observation has an acting consumer (tuner, convergence analyzer)

## Architecture

```
benchmarks/
├── __init__.py
├── judge_axiom_quality.py    # Benchmark runner: sends tokens → /judge → records latency + q_score
├── hardware_profiler.py       # Monitor CPU%, memory, swap, thermal, GPU during inference
├── tuner.py                   # Analyzer: detect bottlenecks, recommend parameter changes
├── convergence.py             # Measure: before/after impact of tuning decisions
├── analysis.py                # Trend detector: stable/degrading/improving
├── observations/              # Timestamped JSON results (all runs, all metrics)
│   ├── gemma-4-e4b-20260427_HHMMSS.json
│   ├── *-hwprofile-20260427_HHMMSS.json
│   ├── convergence-20260427_HHMMSS.md
│   └── analysis.md
└── README.md                  (this file)
```

## Workflow: Observe → Analyze → Recommend → Measure

### 1. Collect Observations

Run the benchmark with real test cases. Data collected:
- **Judgment observations**: token, latency, q_score, verdict_type, success/failure
- **Hardware snapshots**: CPU%, memory, swap, thermal (optional)

```bash
python -m benchmarks.judge_axiom_quality
```

**Output:**
- `observations/gemma-4-e4b-TIMESTAMP.json` (8 samples + summary)
- Logs to stdout (real-time progress)

### 2. Analyze Trends

```bash
python -m benchmarks.analysis
```

**Output:** `observations/analysis.md`
- Per-dog trend: stable|degrading|improving
- Summary stats: latency (mean/median/p95), consistency, success rate
- Compares against previous runs

### 3. Generate Recommendations

```bash
python -m benchmarks.tuner
```

Analyzes hardware bottlenecks + judgment quality → proposes tuning:
- **Swap thrashing** → reduce context size
- **Memory pressure** → reduce parallel requests
- **CPU saturation** → reduce threads
- **Low success rate** → serialize requests
- **High latency** → increase GPU layers

**Output:** Prioritized recommendations (P0=critical, P1=high, P2=medium)

### 4. Apply & Measure Impact

Make a parameter change (e.g., ctx-size 4096→2048), run another benchmark:

```bash
python -m benchmarks.judge_axiom_quality
python -m benchmarks.convergence
```

**Output:** `observations/convergence-TIMESTAMP.md`
- Before/after: latency, q_score, consistency, success rate
- % change in each metric
- Verdict: improved vs degraded

## Metrics Definition

### Per-Observation

- `latency_ms`: Wall-clock time from request to response (includes kernel judgment pipeline)
- `q_score`: Confidence score (0.0-1.0), extracted from kernel verdict
- `verdict_type`: HOWL|BARK|GROWL|WAG (depends on q_score thresholds)
- `input_category`: legit|rug|pump.fun|unknown (for filtering results)
- `error`: null if successful, error message if failed

### Per-Run (Summary)

- **Latency:** mean/median/stdev/p95 (milliseconds)
- **Q-score:** mean/median/stdev (confidence distribution)
- **Consistency:** 1 - (stdev / mean) of q_scores
  - 1.0 = identical q_score across all samples
  - 0.0 = highly variable q_scores
  - Measures whether the judge is deterministic
- **Success rate:** (successful samples) / (total samples)

### Per-Hardware-Snapshot

- `cpu_percent`: Process CPU usage (0-100%)
- `memory_used_gb`: Physical memory consumed
- `swap_percent`: Swap space utilized (0-100%)
- `thermal_celsius`: CPU core temperature
- `gpu_utilization_percent`: GPU utilization (nvidia-smi)
- Hardware snapshots aggregated to: mean/max/min across benchmark

## Example: Tuning Cycle

```
Run 1 (baseline):
  Mean latency: 7.5s, consistency: 0.35

  Observation: High swap %
  Recommendation: Reduce ctx-size 4096→2048

Run 2 (after tuning):
  Mean latency: 4.8s (-36%), consistency: 0.42 (+20%)
  
  Verdict: Parameter change improved latency without sacrificing consistency
  Next step: Tune threads or gpu-layers
```

## Self-Evolving Organ Design

**Phase 1 (current):** Framework + manual iteration
- Human reviews tuner recommendations
- Human applies changes via config update
- Human measures convergence

**Phase 2 (Hermes Agent):** Autonomous tuning agent
- Agent runs full observe→recommend→measure cycle
- Agent updates llama-server.env and restarts service
- Agent logs decisions + rationale to kernel for CCM crystal injection

**Phase 3 (CWO):** Constitutional governance
- Dogs evaluate agent's tuning decisions via FOGC axioms
- Agents tune within sovereign bounds (no kernel compromise)
- Agents crystallize successful parameter patterns

## Data Hygiene

- **Never delete observations:** All runs are versioned in `observations/`. Analysis compares full history.
- **Before/after measurements:** Every tuning claim requires convergence report.
- **Real test cases:** Use actual tokens (Raydium, FTX, pump.fun samples), not synthetic.
- **Hardware context:** Always record hardware profile alongside judgment observations.

## Next Domains

- Token classifier quality (accuracy vs oracle)
- Wallet authenticity scoring (multi-game strategy)
- Chess game outcome prediction
- Embedding correlation (semantic consistency)

## P-Rules (from .claude/rules/python.md)

- P1: Type annotations (mypy --strict)
- P2: Test coverage ≥80%
- P3: Dependencies pinned in pyproject.toml
- P7: Observability (every run logs metrics)
- P12: Test on real data (not synthetic vectors)
- P13: Measure before/after (never hardcode heuristics)

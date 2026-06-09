# Inference Model Testing Plan (DATA-CENTRIC)

**Objective:** Validate X Organ recommendations via measured kernel /judge performance.

**Ground truth source:** 46 inference tweets mentioning real production deployments, 52 model citations, ranked by engagement (views + likes).

---

## Current Baseline (Gemma-4-E4B)

Run ID: `gemma-4-e4b-20260427_063127` (latest)
- Mean latency: 7,184.1 ms
- Success rate: 8/8 (100%)
- Consistency: 35.09%
- Hardware: cynic-core (APU + Vulkan)

**Status:** Acceptable baseline. Consistency (variance) is moderate—indicates some latency jitter.

---

## Phase 1: APU Models (cynic-core)

Target: Replace Gemma with a higher-engagement model that fits APU memory budget (≤9B).

### Test 1.1: Qwen3.5-9B (8GB RAM recommendation from Twitter)

**Hypothesis:** Qwen3.5-9B will maintain <10s latency while improving q_score consistency vs Gemma.

**Setup:**
```bash
# On cynic-core: download + quantize Qwen3.5-9B
llama-cli -hf unsloth/Qwen3.5-9B-GGUF:Q4_K_M -c 4096
```

**Measurement protocol:**
1. Run 8-sample benchmark (same test tokens as Gemma baseline)
2. Collect: latency, q_score distribution, success rate
3. Run convergence analyzer: latency_delta, consistency_delta
4. **Gate:** Accept if latency ≤ 10s AND consistency improves by ≥10% OR success_rate = 100%

**Expected:** Twitter data suggests Qwen3.5-9B on RTX (similar VRAM budget) achieves 64 tok/s ≈ 6.4s on 131K context. Shorter context (4K) should be faster.

### Test 1.2: Qwen2.5-3B (alternative: faster, lower quality)

**Gate:** Only if Qwen3.5-9B fails. Qwen2.5-3B has lower engagement (838 views).

---

## Phase 2: GPU Models (cynic-gpu, RTX 4060 Ti 16GB)

Target: Test highest-engagement model from Twitter (Qwen3.6-27B).

### Test 2.1: Qwen3.6-27B (64 tok/s reported, 2.2M engagement)

**Hypothesis:** Qwen3.6-27B on GPU will achieve >100 tok/s (lower latency than APU), maintain high consistency.

**Setup:**
```bash
# On cynic-gpu (RTX 4060 Ti):
# Download + quantize Qwen3.6-27B
llama-server -hf unsloth/Qwen3.6-27B-GGUF:Q4_K_M -c 4096 --gpu-layers 45
```

**Measurement protocol:**
1. Run 8-sample benchmark
2. Record: latency, throughput (tokens/s), q_score, consistency
3. Run convergence analyzer vs Qwen3.5-9B baseline
4. **Gate:** Accept if latency < 2s AND consistency ≥ 40%

**Expected:** Twitter reports 64 tok/s on mixed hardware. RTX 4060 Ti with GPU layers should push toward 50+ tok/s.

### Test 2.2: Qwen3.5-27B (backup: 18K engagement)

**Gate:** Only if Qwen3.6-27B shows consistency regression >15%.

---

## Phase 3: Cross-Hardware Comparison

Once both APU and GPU models pass their gates, run direct comparison:

```bash
# Same 8-token input set, measure on both
python3 -m benchmarks.judge_axiom_quality --dogs qwen35-9b-apu,qwen36-27b-gpu
python3 -m benchmarks.convergence --before qwen35-9b-baseline --after qwen36-27b
```

**Output:** Convergence report showing latency/consistency/q_score trade-offs.

---

## Metric Definitions

### Per-run summary (from judge_axiom_quality.py)

| Metric | Definition | Good Range | Baseline |
|--------|------------|------------|----------|
| Mean latency | Wall-clock /judge response time | < 10s | 7.18s |
| Consistency | 1 - (stdev / mean) of q_scores | > 0.40 | 0.3509 |
| Success rate | (error=null) / total | = 100% | 100% |
| Mean q_score | Average confidence | Depends on task | varies |

### Falsification conditions

A model **fails** if:
- Latency > 15s (APU) or > 3s (GPU) — too slow for real-time agents
- Success rate < 95% — timeouts or crashes
- Consistency < 0.20 — erratic q_scores indicate model instability

---

## Branching Logic

```
START
├─→ Run Qwen3.5-9B on APU
│   ├─→ PASS → Proceed to GPU testing
│   └─→ FAIL → Run Qwen2.5-3B (lower expectation)
│
└─→ Run Qwen3.6-27B on GPU
    ├─→ PASS → Ship to kernel; update backends.toml
    └─→ FAIL → Run Qwen3.5-27B (lower expectation)
```

---

## After Testing: Commit Decision

**If both phases pass:**
1. Update `.claude/rules/reference.md` with new Dogs
2. Commit models to kernel (recompile if needed)
3. Run `/judge` on production test tokens (from CultScreener or Raydium)
4. Archive observation CSVs: `git add observations/qwen*-*.json`

**If phase 1 passes but phase 2 fails:**
1. Ship APU improvement (Qwen3.5-9B)
2. Defer GPU model (file as TODO: GPU benchmark)
3. Hermes Agent can retry GPU phase in next session

**If both fail:**
1. Revert to Gemma baseline
2. Investigate: hardware limits? Model licensing? Domain mismatch?
3. Spawn new research session (check HF leaderboards for alternatives)

---

## K15 Consumer Rule Check

✓ **Producer:** judge_axiom_quality.py outputs observations (latency, q_score, success_rate)
✓ **Consumer:** tuner.py reads observations → recommends parameter changes
✓ **Consumer:** convergence.py reads before/after → measures impact
✓ **Acting consumer:** Kernel /judge endpoint updates with new Dog model
✓ **Feedback loop:** Next benchmark captures updated latency

All producers feed acting consumers. No dead sinks.

---

## Session Goal

Ship inference improvement from X Organ data (APU + GPU models tuned to real-world deployment patterns) via closed-loop benchmark framework.

# Phase 2 Measurement Plan — Domain Routing Signal Improvement (May 5-6, 2026)

**Hypothesis:** Cortex-generated multi-domain routing improves Dog verdict confidence/precision > 5%.

**Falsification:** Δ ≤ 5% → domain generation doesn't help → delete cortex artifacts, stay with v1 single-domain.

---

## Setup (Complete)

- ✓ Protocol wiring deployed (session hook, cortex manifest, artifact loader)
- ✓ Domain discovery artifacts migrated to external storage
- ✓ Consumer registry updated (kernel_routing_v1 awaiting kernel_health=true)
- ✓ Kernel running and serving verdicts (environment degraded, not critical for measurement)
- ⏳ Cortex bootstrap pending (kernel_health gate must resolve)

---

## Measurement Strategy

### Data Source
Two identical observation batches, run back-to-back:

1. **Baseline (v1):** Dogs evaluate observations WITHOUT domain hints
   - Call `/judge` with standard observation prompt
   - Measure: confidence, json_valid, latency, verdict type distribution

2. **Treatment (v2):** Dogs evaluate observations WITH domain hints from cortex
   - Call `/judge` with observation + domain labels (from discovery artifacts)
   - Measure: same metrics

### Success Criteria

**Signal improvement:**
- Δ(confidence) ≥ 5% → PASS (domain hints help)
- Δ(precision) ≥ 5% → PASS (more correct verdicts per domain)
- Δ(json_valid) ≥ 3% → PASS (models produce better-formed output with context)

**Any of the above → Phase 2 PASSES**

**All below → Phase 2 FAILS**
- Δ(confidence) ≤ 5% AND
- Δ(precision) ≤ 5% AND
- Δ(json_valid) ≤ 3%

### Observation Dataset

**Source:** Real or synthetic, must have:
- ≥ 100 observations (for statistical power)
- Domain distribution matching organ_x capture (current: 62% general, 24% LLM, 9% token, 5% security)
- Known ground truth (for precision measurement) OR LLM consensus (for confidence)

**Recommended:** Use `/observations?domain=session` to fetch recent live observations, or synthetic batch from domain_discovery corpus.

---

## Measurement Script Template

```bash
#!/bin/bash
set -e

# Source env: ~/.cynic-env (loads KERNEL_URL and API_KEY)
source ~/.cynic-env
BATCH_SIZE=100

# Phase 2a: Baseline (v1, no domain hints)
echo "Phase 2a: Baseline measurement (v1, no domain hints)"
curl -s -X POST "${KERNEL_URL}/judge" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d @baseline_batch.json \
  > baseline_results.json

# Phase 2b: Treatment (v2, with domain hints)
echo "Phase 2b: Treatment measurement (v2, with domain hints)"
# Load domain discovery from artifacts
python3 cynic-python/artifact_loader.py claude-code > domains.json

# Inject domain hints into observations and re-judge
curl -s -X POST "${KERNEL_URL}/judge" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d @treatment_batch.json \
  > treatment_results.json

# Phase 2c: Analysis
echo "Phase 2c: Analyzing signal improvement..."
python3 << 'ANALYSIS_SCRIPT'
import json

with open("baseline_results.json") as f:
    baseline = json.load(f)
with open("treatment_results.json") as f:
    treatment = json.load(f)

# Compute Δ
confidence_baseline = sum(v.get("confidence", 0) for v in baseline) / len(baseline)
confidence_treatment = sum(v.get("confidence", 0) for v in treatment) / len(treatment)
delta_confidence = ((confidence_treatment - confidence_baseline) / confidence_baseline) * 100

print(f"Baseline confidence: {confidence_baseline:.3f}")
print(f"Treatment confidence: {confidence_treatment:.3f}")
print(f"Δ confidence: {delta_confidence:+.1f}%")
print()

if delta_confidence >= 5:
    print("✓ PHASE 2 PASSES: Domain routing improves signal > 5%")
else:
    print("✗ PHASE 2 FAILS: Domain routing has no significant impact")
ANALYSIS_SCRIPT
```

---

## Blockers & Dependencies

**Hard blocker:** `kernel_health = true`
- Currently: false (readiness.healthy = degraded due to environment probes)
- Workaround: Comment out GPU backend in backends.toml → kernel transitions to healthy
- Without workaround: Measurement still possible but less clean (environment noise)

**Soft blocker:** Domain discovery artifacts must load without error
- Status: ✓ Loader successfully finds and loads 4/5 artifacts
- Ready for consumption by kernel_routing_v1 consumer

---

## Timeline

| Date | Gate | Dependency | Action |
|------|------|-----------|--------|
| May 3 (today) | Protocol wiring ✓ | None | Complete |
| May 4 | Kernel recovery | Apply workaround (optional) | Monitor kernel health |
| May 5-6 | Phase 2 measurement | Kernel must stay up | Run baseline + treatment |
| May 7 | Phase 2 cleanup | Measurement results | If PASS: activate hermes_framing. If FAIL: document why. |
| May 8+ | Phase 3 design | Phase 2 result = PASS | Begin routing performance measurement |

---

## Decision: Path 1 vs Path 2

This Phase 2 measurement uses **Path 1 (Minimal, Recommended):**
- Effort: 1.5h code
- Test: Signal improvement on SYNTHETIC data (cortex reasoning)
- Blocker: None (can test immediately once kernel recovers)
- Risk: Low (isolated, single mechanism)
- Confidence: Higher

Not Path 2 (Full, Ambitious):
- Effort: 6-8h code
- Test: Signal improvement on REAL observations (full K15 loop)
- Blocker: Hermes must be capturing, kernel must route, observations must reach Dogs
- Risk: High (many integration points)
- Confidence: Lower

**Rationale:** Path 1 validates the mechanism (domain hints help) before scaling to live data. If Phase 2 passes, extend to observations in Phase 3.

---

**Confidence:** φ⁻¹ (0.618) — measurement design is sound, hypothesis is testable, execution depends on kernel stability.

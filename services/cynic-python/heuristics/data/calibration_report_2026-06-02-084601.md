# Phase 2.1: Calibration Report
**Date**: 2026-06-02
**Input**: 45 divergence rows (Phase 2.0) × 45 outcomes

## Finding: Measurement Incomplete

Phase 2.0 data is insufficient for threshold calibration.

### Root Cause: 28/45 judgment failures

| Cohort | Count | Cause |
|--------|-------|-------|
| Valid verdicts (all WAG, q≈0.391) | 17 | deterministic-dog only |
| Failed judgments (enriched_verdict_id=null) | 28 | LLM Dogs circuit=critical during measurement |

The 28 failed judgments represent a kernel degradation event, not token signal.
Infrastructure was repaired 2026-06-01/02 (headscale + llama-server).

### Outcome Distribution

| Outcome Label | Failed judgments | Valid WAG verdicts |
|--------------|-----------------|-------------------|
| flat         | 22              | 12                |
| decline      | 5               | 4                 |
| growth       | 1               | 1                 |
| rug          | 0               | 0                 |

**No rug/severe_decline cases in the 45-token set.** Threshold calibration requires
adversarial examples — a set with only flat/decline/growth cannot distinguish BARK from WAG.

### WAG Accuracy (17 valid tokens)

- 13/17 (76%) correctly WAG — flat or growth outcome
- 4/17 (24%) arguably GROWL — decline -30% to -43%
- WAG price range: min=-43%, max=+244%, mean=-4%

The 4 misclassified tokens suggest the GROWL_MAX threshold (0.382) may be slightly low
for tokens near the boundary. However, 4 samples is insufficient to conclude this.

## Calibration Decision: No Threshold Change

**Falsification condition**: threshold change requires ≥10 adversarial (rug/severe_decline) examples.
Current set has 0. Any threshold change from this data would be noise-fitting.

## Phase 2.1b: Re-run Required

Now that infrastructure is restored (3 Dogs active as of 2026-06-02):

1. **Re-run Phase 2.0 measurement** with all Dogs active → expect BARK/GROWL/WAG/HOWL spread
2. **Collect adversarial examples** — include known-bad tokens from the BARK collection
3. **T+14 outcomes** — collect outcomes for the 45-token set at 2-week mark (2026-06-09)
4. Then: re-run calibration with complete data

## Falsification

**Claim**: re-run with 3 Dogs active will produce BARK/GROWL spread on the same 45 tokens.
**Test**: Run outcome_measurement_t7.sh with kernel at status=healthy (3 Dogs closed).
**Falsify**: If all verdicts still cluster at q≈0.391, deterministic-dog is dominating despite LLM Dogs — investigate why LLM Dogs are not influencing the consensus.

# Phase 2.1: Calibration Report
**Input**: 45 divergence rows × 45 outcomes

## Accuracy per Tier

| Tier | Verdicts | TP | FP | TN | FN | Unknown | Precision | Recall |
|------|---------|----|----|----|----|---------|-----------|--------|
| BARK (q̄=0.000) | 28 | 0 | 28 | 0 | 0 | 0 | 0% | — |
| GROWL (q̄=0.000) | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| WAG (q̄=0.390) | 17 | 0 | 0 | 17 | 0 | 0 | — | — |
| HOWL (q̄=0.000) | 0 | 0 | 0 | 0 | 0 | 0 | — | — |

**Overall accuracy** (labelled tokens): 38%

## Outcome Distribution

- **bad**: 0
- **good**: 45
- **unknown**: 0

## Calibration Signal

⚠️  Note: deterministic-dog dominated this measurement (LLM Dogs were offline during
Phase 2.0 run). Q-scores cluster near 0.391 (deterministic-dog WAG boundary).
LLM Dog calibration requires a fresh measurement run with all Dogs active.

## Proposed Threshold Adjustments

No adjustments needed based on current data.

## Falsification

**Claim**: calibrated thresholds improve prediction accuracy on held-out set.
**Test**: Run measurement on next 45-token batch (collected after calibration).
**Falsify**: If accuracy does not improve by ≥5pp vs baseline, threshold change is noise.

## Next Steps

1. Review threshold proposals with human — apply to `backends.toml` if approved
2. Re-run measurement with LLM Dogs active (all 3 Dogs voting)
3. Collect next outcome batch (T+14) for validation

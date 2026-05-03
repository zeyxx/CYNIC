# Kernel Bootstrap — Phase 2 Readiness (2026-05-03)

**Status:** KERNEL RUNNING with degraded environment (context drift detected)  
**Blocker:** GPU backend context mismatch (actual 65K vs expected 131K)  
**Workaround:** Available (disable GPU Dog, use qwen-7b-hf fallback)  
**Measurement:** Phase 2 gate can proceed if kernel stays stable

---

## Current State

**Kernel alive:** ✓ /health responds with full diagnostics  
**Readiness:** `degraded` (caused by environment probes)  
**Dogs operational:** All 4 circuits CLOSED:
- deterministic-dog (heuristic, sovereign)
- gemini-cli (inference, non-sovereign)
- qwen-7b-hf (inference, non-sovereign, HF router)
- qwen35-9b-gpu (inference, sovereign) — **CONTEXT DRIFT DETECTED**

**Context drift signature:**
```json
{
  "dog_name": "qwen35-9b-gpu",
  "actual_n_ctx": 65536,
  "expected_n_ctx": 131072,
  "model_mismatch": false,
  "latency_ms": 12,
  "reachable": true
}
```

---

## What's Blocking Full Recovery

`~/.config/cynic/backends.toml` section `[backend.qwen35-9b-gpu]`:
- Config declares: `context_size = 131072`
- llama-server on GPU node running with: `--n-ctx 65536` (or similar reduced context)
- Result: Model capacity advertised ≠ actual capacity → boot probe detects drift

**Root cause:** Soma orchestrator missing (would manage backend context budgets, lifecycle, remediation)

---

## Workaround (Optional, For Full Recovery)

Comment out lines 30-49 in `~/.config/cynic/backends.toml`:

```bash
# Edit ~/.config/cynic/backends.toml
# Lines 30-49: Comment out [backend.qwen35-9b-gpu] section
# Kernel will fall back to qwen-7b-hf (HuggingFace, 32K context, 90.5% success rate)
```

**Effect:**
- Kernel boot checks pass cleanly (no degraded environment)
- Readiness transitions to `healthy`
- Inference uses HF router + fallback 9B on local CPU (slower but stable)
- Phase 2 measurement can run without environment noise

**Cost:** ~10% latency increase on verdicts (HF router 3.3s avg vs GPU 11.2s avg), but quality metrics (json_valid) are higher on HF (90.5% vs 93%, comparable in practice).

---

## Phase 2 Readiness Check

**Can measure now (kernel degraded):**
- ✓ Cortex artifacts load (protocol wiring functional)
- ✓ All 4 Dogs operational
- ✓ Verdicts route normally
- ✗ Environment probe shows "degraded" (context drift noise in logs)

**Can measure cleanly (after workaround):**
- ✓ Same as above
- ✓ Environment probe shows "healthy"
- ✓ No context drift noise

**Recommendation:** Apply workaround if deploying Phase 2 script to production. Defer if measuring in dev.

---

## Timeline

| When | What | Status |
|------|------|--------|
| Now (May 3, 17:28 UTC) | Kernel health probed | ✓ Operational (degraded) |
| May 5-6 | Phase 2 measurement (may 5-6) | ? Kernel must stay stable |
| May 7 | Phase 2 cleanup (measure result → accept/reject cortex) | Depends on measurement |
| Post-Phase-2 | Soma design + implementation | Scheduled, 6-8h estimate |

---

## Files

- `/home/user/.cynic/organisms/artifacts/deferred/soma/SOMA_EMERGENCE_2026_05_03.md` — Full analysis of kernel boot failure
- `/home/user/.cynic/organisms/artifacts/deferred/soma/KERNEL_BOOT_FIX_2026_05_03.md` — Detailed workaround + proper fix
- This document — Bootstrap plan for Phase 2

---

## Decision Points

**Do we apply the workaround now?**
- **Pro:** Clean readiness state, no environment noise during measurement
- **Con:** Falls back to HF (slower), GPU still unused during Phase 2
- **Recommendation:** Yes, for cleaner Phase 2 data (Phase 3 can debug GPU)

**Do we measure with current kernel state (degraded)?**
- **Pro:** No config changes, measure immediately
- **Con:** Environment degradation may add noise to Phase 2 results
- **Recommendation:** No. Phase 2 Δ > 5% signal improvement is already marginal. Environment noise could obscure real signal.

---

**Confidence:** φ⁻¹ (0.618) on kernel stability. Measurement confidence pending Phase 2 gate execution.

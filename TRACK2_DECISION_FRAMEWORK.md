# Track 2 Decision Framework — v1 vs v2 Heuristics

## Hypothesis
v2 heuristics (author-tier weighted, engagement-aware, domain-aware) produce systematically higher quality signals than v1 (basic regex + follower count).

**Falsification Criterion:** Δq_mean < 0.05 → no statistically meaningful improvement

## v2 Improvements

| Dimension | v1 | v2 | Rationale |
|-----------|----|----|-----------|
| **Author tier** | Binary check (followers>10K) | Weighted (+2 whale/curated, -1 bot) | Signal from major accounts is qualitatively different |
| **Engagement** | Simple >1% ratio | Tiered: >5% (+2), >1% (+1), + absolute >1000 (+1) | Accounts for both ratio and scale |
| **Retweet penalty** | -1 | -2 | Original analysis more valuable than echo |
| **Text length** | >140 chars (+1) | >200 chars (+1), ≤140 (+0) | Substantive tweets are longer |
| **Cashtags** | Ignored | +1 if present | Token mentions are domain-specific signal |
| **Narratives** | Ignored | +1 if present | Hermes already labeled → use the label |
| **Discussion** | replies OR bookmarks | replies > bookmarks (discussion > saving) | Quality signal from discourse pattern |

## Outcomes

### If Δq_mean ≥ 0.05
**v2 is statistically better.** Immediate actions:
1. Update x_proxy.py signal_score() to v2
2. Restart x-explorer service
3. Dataset will use v2 from now on
4. Phase A continues with improved signals
5. Phase B: measure decision-rhythm with better baseline
6. **Timeline:** Deploy today (2026-04-28)

### If Δq_mean < 0.05
**v2 provides no improvement.** Actions:
1. Revert to v1 (no code change needed)
2. Keep Phase A as-is
3. Focus Phase B on decision-rhythm discovery first
4. Re-evaluate heuristics after understanding Hermes' learning pattern
5. **Timeline:** Continue Phase A completion (midnight 2026-04-29)

## Expected Results

**Preview (first 50 tweets):** Δ=+0.84
- Well above 0.05 threshold
- Suggests v2 will pass falsification test
- If confirmed on full dataset: deploy immediately

## Confidence Bounds

**If v2 wins (high confidence):**
- Mean Δq ≥ 0.05 across 2,287 tweets
- Verdict shifts in coherent direction (bad tweets → more BARK, good → less BARK)
- By-tier breakdown shows curated/whale benefiting, bot penalizing

**If v2 loses (low confidence):**
- Mean Δq < 0.05 (noise, not signal)
- Verdict shift rate ≈ 0% (no systematic change)
- Distribution is symmetric around zero

## Next Steps (after Track 2 results)

**Regardless of outcome:**
- Continue Phase A (T. browsing, dataset growing)
- x-explorer cron runs at 20:55 (Phase A intermediate checkpoint)
- Phase A ends ~midnight 2026-04-29

**If v2 passes:**
- Deploy v2 + restart x-explorer
- Phase B: decision-rhythm discovery (when does Hermes learn?)

**If v2 fails:**
- Keep v1 + continue as-is
- Phase B: same (decision-rhythm is the real blocker)

## Epistemic Status

- **v2 design:** OBSERVED (based on dataset structure, author tier distribution, narrative tags)
- **v2 improvement prediction:** CONJECTURE (preview shows +0.84, but n=50, not 2,287)
- **Falsification threshold:** DEDUCED (0.05 from φ⁻² decay rate; defensible but arbitrary)
- **Confidence in framework:** 0.618 (φ⁻¹ — can state falsification condition, but empirical outcome unknown)

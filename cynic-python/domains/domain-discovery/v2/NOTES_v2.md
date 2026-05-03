# Domain Discovery v2 — Stopword Filtering & Cleanup

**Status**: READY TO EXECUTE (pending v1 silhouette validation)

## Improvements Over v1

### 1. Stopword Filtering
- Removed 60+ common English stopwords (the, and, you, for, all, etc.)
- Removed observed junk tokens (http, tps, amp, href, etc.)
- Reduced vocabulary noise while preserving semantic signals

### 2. URL Cleaning
- Strip URLs before tokenization
- Remove mentions, hashtags (keep content)
- Clean HTML entities (&amp; → and)

### 3. Vocabulary Quality
- Required minimum document frequency (2+ tweets)
- Removed single-character tokens
- Expected vocabulary reduction: 24K → ~18-20K words

## Hypothesis

**v1 → v2 refinement increases semantic coherence**

- v1 silhouette: ? (measuring now)
- v2 silhouette: target > v1 (smaller, purer clusters)
- Falsify if: v2 silhouette < v1 (filtering hurts structure)

## Execution Plan

1. Wait for v1 silhouette result (currently running)
2. Run v2 with same data
3. Compare silhouette: if v2 > v1 + 0.05, accept improvement
4. Otherwise, keep v1 (stopwords not load-bearing)

## Known Caveats

- Vocabulary shrinkage may affect rare-domain signals (e.g., "pump" in token-analysis)
- Stopwords list is generic English; may not fit social media well
- v3 improvement: learn stopword list from data

## Files

- `emergent_clustering_tfidf_v2.py` — implementation
- `results_v2/emergent_clusters_tfidf_v2.json` — results
- `results_v2/metrics_v2.json` — silhouette + vocab size
- `NOTES_v2.md` — this file

---

**Next**: Execute v2 after v1 completes.

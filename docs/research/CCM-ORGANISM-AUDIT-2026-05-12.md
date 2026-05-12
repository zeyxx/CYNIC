# CCM Organism Audit — 2026-05-12

> Data-centric analysis of 120K observations, 13K verdicts, 9 crystals.
> Scripts: `cynic-python/lab/ccm_chaos_clustering.py`, `ccm_deep_analysis.py`

## Core Finding

**The organism converged to 1 bit: "suspect or not."**

6 axioms → 1 effective dimension (PC1=80%). 4 Dogs → 1 direction (cosine 0.985+). 13K verdicts → BARK 73%, GROWL 27%, HOWL/WAG ~0%. The compound loop amplifies this: BARK-dominant crystals → suspicious Dog prompts → more BARK.

## Findings

### Axiom Topology
- PC1 captures 80% of variance across all Dogs — axioms are largely redundant
- Per-Dog: ~4 effective dimensions (not 1.2), but aggregation compresses to ~1
- **VERIFY is the most independent axiom** (r=0.27-0.45 with sovereignty) and the most contested (19.2% of disagreements) — yet 53% abstained by deterministic-dog
- Axiom topology is **domain-dependent**: token-analysis has 4 real dims (PC1=71%), D1 has 1.5 dims (PC1=88%)
- culture×sovereignty (r=0.87) and burn×phi (r=0.85) are the most entangled pairs

### Dog Behavior
- **Dog personalities are quasi-identical** (cosine similarity 0.985-0.997 between centroids)
- **qwen35-9b-gpu**: workhorse (1751 verdicts), most bimodal (46% floor scores), most chaotic (cv=78% on D1)
- **deterministic-dog**: most stable (cv=7-9%), most nuanced (92% mid-range), but abstains on semantic axioms
- **qwen-7b-hf**: least substantive reasoning (21% on fidelity), but stable (cv=11%)
- **gemini-cli**: richest score distribution (28-36 unique values), 100% substantive reasoning, but only 131 scores

### Score Distributions
- deterministic-dog: 8-13 discrete values per axiom (lookup table, not continuous)
- qwen35-9b-gpu: bimodal — 46% at floor (0.05), 13% at ceiling (0.618)
- qwen-7b-hf: 32% of burn scores = 0.618 (max). Binary: {low, φ⁻¹}
- gemini-cli: most continuous, top5 values cover only 33-47%

### Verdict Patterns
- **HOWL = chess** (70% of HOWL contain chess notation). Without chess, HOWL ≈ 0%
- **BARK = keywords** (24% contain pump/rug/scam, 31% contain URL)
- Q-score NOT calibrated: chess p50=0.549, twitter p50=0.249. Same threshold applied to both
- Temporal drift: 41% HOWL (W12) → 0% HOWL (W18) in 7 weeks

### Crystal State
- 4/5 mature crystals are BARK-dominant
- Only dev crystal is WAG-dominant
- Crystal effectiveness unmeasurable (crystals too recent, no before/after data)
- 98% of observations never crystallize (gate-rejected or wrong domain)

### Emergent Taxonomy (UMAP+HDBSCAN)
- 44 natural clusters vs 87 imposed domains
- Clusters emerge from activity×content, not domain labels
- Verdict KIND (Bark vs Growl) is a convergence dimension
- kernel-lifecycle (1 domain) = 10 semantic clusters (under-specified)
- general (1 domain) = 9 clusters (under-specified)

### Aggregation
- Geometric mean ≈ arithmetic mean ≈ median (r > 0.985)
- Changing aggregation method changes nothing
- Agreement (low disagreement) weakly predicts higher Q-score (+0.04)
- More Dogs ≠ more dimensions (5 dogs → PC1=86%, 2 dogs → PC1=83%)

## Implications

1. The CCM compound loop is a **positive feedback loop toward suspicion**. Without diverse positive data, it will keep converging.
2. Adding more Dogs of similar architecture adds volume, not perspective.
3. The 6-axiom framework is structurally sound but **operationally collapsed** to 1 dimension by current Dogs.
4. VERIFY is the most valuable axiom (most independent) and the least served (most abstained, most contested).
5. Domain-specific calibration is needed before Q-score thresholds mean anything.

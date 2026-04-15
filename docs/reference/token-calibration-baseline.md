# Token Domain — Calibration Baseline

> Measured 2026-04-15, WITHOUT `domains/token-analysis.md` domain prompt loaded.
> Kernel: v0.7.7, 5 Dogs registered, 3-5 responding per verdict.
> These are BEFORE measurements for the Scientific Protocol.

## Pipeline Results

| Case | Expected | Actual | Q-Score | Dogs | max_disagreement | Anomaly |
|------|----------|--------|---------|------|------------------|---------|
| Classic Rug (pump.fun, 3 holders, mint active) | BARK | **BARK** | 0.130 | 3/5 | 0.35 | No |
| JUP (governance, 580K holders, revoked) | HOWL | **WAG** | 0.517 | 5/5 | 0.51 | **Yes (CULTURE)** |
| ASDF (pump.fun, 20 holders, revoked, burned LP) | GROWL | **BARK** | 0.183 | 5/5 | 0.35 | No |

## Per-Dog Quality (token domain)

| Dog | BARK score | HOWL score | ASDF score | Δ(HOWL-BARK) | Assessment |
|-----|-----------|-----------|-----------|-------------|-----------|
| gemini-cli | 0.067 | 0.601 | 0.148 | 0.534 | **BEST** — correct reasoning on all 3 |
| qwen35-9b-gpu | — | 0.573 | 0.140 | — | Good, cautious. Not available for BARK test. |
| qwen-7b-hf | — | 0.513 | 0.250 | — | Moderate. Misreads JUP data ("top 1 too high" = 4.2%) |
| deterministic-dog | 0.296 | 0.315 | 0.318 | 0.019 | **Content-blind**. Scores form, Δ≈0 |
| gemma-4b-core | 0.108 | 0.208 | 0.225 | 0.100 | **BACKWARDS** on JUP ("revoked = red flag") |

## Discrimination Analysis

- BARK→HOWL Δ = 0.387 — **strong discrimination**
- BARK→AMBIGUOUS Δ = 0.053 — **weak** (ambiguous treated as BARK)
- AMBIGUOUS→HOWL Δ = 0.334 — **strong**
- **gemini-cli** provides 80%+ of the discrimination signal
- **deterministic-dog** provides <3% — needs token-specific heuristics
- **gemma-4b-core** actively degrades JUP assessment (backwards reasoning)

## Key Observations

1. **The pipeline DOES discriminate.** Δ=0.387 between BARK/HOWL is meaningful.
2. **JUP underscored** (WAG not HOWL). Two Dogs misread the data, one is content-blind.
3. **ASDF treated as rug** — correct given pure metrics. Dogs can't know about team behind it.
4. **gemini-cli carries the verdict** — if it goes down, discrimination collapses.
5. **Domain prompt not loaded** — these scores will change after deploying token-analysis.md.
6. **Anomaly detection works** — caught 0.51 disagreement on CULTURE for JUP.

## Research Grounding (from literature review)

| Finding | Source | Impact on CYNIC |
|---------|--------|----------------|
| Median rug lifecycle: 0.0116 days | SolRugDetector (arXiv 2603.24625) | Token age is strongest retrospective signal |
| 95.69% of rug txs on creation day | SolRugDetector | Day-1 activity concentration = computable at T+24h |
| Bundle detection: 21.4% wash trades | MemeTrans (arXiv 2602.13480) | Earliest signal (block 0), not in current TokenData |
| LP add-to-remove ratio: 97.6% accuracy | SolRPDS (arXiv 2504.07132) | LP flow direction > LP status (burned/locked) |
| High Gini ≠ fraud (UNI=0.85, AAVE=0.78) | OxJournal 2025 | HHI/Gini alone insufficient — need context |
| No multi-axiom scoring system exists | Survey of RugCheck, GoPlus, CertiK | CYNIC's approach is genuinely novel |
| 98.6% = liquidity collapse, not all technical rugs | Solidus Labs methodology | Baseline is correct but nuanced |

## Hypothesis for Domain Prompt Impact

Loading `domains/token-analysis.md` should:
1. Improve JUP score from WAG to HOWL (calibrated score ranges per axiom)
2. Reduce max_disagreement on HOWL cases (concrete examples reduce inter-Dog spread)
3. NOT change BARK scores significantly (Dogs already harsh on clear rugs)
4. Potentially improve ASDF from BARK to GROWL (domain prompt recognizes mixed signals)

**Falsification:** If JUP still gets WAG after domain prompt, the problem is Dog quality, not prompt quality. If max_disagreement > 0.40 persists, Dogs need replacement, not more calibration.

# Meta-Question Benchmark — CYNIC Dogs vs Claude

> "Can a weak local LLM + CYNIC platform outperform a strong LLM alone?"
> — CYNIC-legacy experimental-protocols.md (2026-02-12), experiment never executed

## Context

CYNIC-legacy defined 12 experimental protocols. Zero were executed. The meta-question — whether persistence, enrichment, and accumulated judgment (crystals) can compensate for raw model strength — was never tested.

The current CYNIC kernel now has the infrastructure to answer it:
- Real LLM Dogs (Qwen 2.5 7B, Qwen 3.5 9B) producing verdicts
- CCM crystal compound loop (verdict → crystal → Dog prompt)
- Helius/DexScreener enrichment pipeline with holder context, trajectory, K-Score
- 33 calibration tokens with CultScreener ground truth labels
- Claude Code CLI available programmatically (`claude -p`) via Max subscription

## Hypothesis (falsifiable)

**CYNIC Dogs (Qwen 7B + enrichment + crystals) produce better token verdicts than Claude Sonnet 4.6 without accumulated context.**

Falsified if: ρ(Dogs, conviction) < ρ(Sonnet naive, conviction) OR adjacent_match(Dogs) < adjacent_match(Sonnet enriched).

## Design

### 4 Arms

| Arm | Model | Stimulus | What it measures |
|-----|-------|----------|------------------|
| `cynic_dogs` | Qwen 7B + deterministic | Enriched + crystals + trajectory | The full product |
| `haiku_naive` | Claude Haiku 4.5 | Mint address only | Baseline floor |
| `sonnet_naive` | Claude Sonnet 4.6 | Mint address only | Strong model, no context |
| `sonnet_enriched` | Claude Sonnet 4.6 | Same enriched stimulus as Dogs | Isolates model vs ecosystem |

Key comparisons:
- Dogs vs Sonnet naive → "does the ecosystem compensate for model weakness?"
- Sonnet enriched vs Sonnet naive → "does enrichment add value regardless of model?"
- Dogs vs Sonnet enriched → "does the model matter when both have the same data?"

### Input

`cynic-python/heuristics/data/calibration_results_real.json` — 33 tokens:
- 20 `conviction_tier: "strong"` → `expected_verdict: "Howl"`
- 10 `conviction_tier: "mixed"` → `expected_verdict: "Growl"`
- 3 `conviction_tier: "weak"` → `expected_verdict: "Bark"`

Ground truth: CultScreener `conviction` (0.0–1.0) and `conviction_tier`.

**Known baseline (observed, 2026-05-16):** Current Dogs produce ρ=0.225 (p=0.21, not significant), adjacent_match=48.5%, discrimination gap=0.005. Dogs predict Growl for 30/33 tokens. This benchmark measures whether Claude arms do better, worse, or the same — a ρ<0.4 for all arms means "enrichment as currently implemented does not add value," which is a valid finding.

**Statistical power note:** n=33 with 20/10/3 class split is underpowered. Results are directional (hypothesis-generating), not decisive. Bootstrap CIs on ρ should be computed in `analyze.py`.

### Output Schema

`benchmark_results.jsonl` — one line per (token × arm):

```json
{
  "mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
  "symbol": "JUP",
  "arm": "cynic_dogs",
  "q_score": 0.423,           // from response.q_score.total (Dogs) or geo_mean(axioms) (Claude)
  "verdict": "Wag",
  "axioms": {                  // from response.q_score.{fidelity,...} (Dogs) or parsed JSON (Claude)
    "fidelity": 0.45, "phi": 0.38, "verify": 0.42,
    "culture": 0.40, "burn": 0.44, "sovereignty": 0.35
  },
  "ground_truth_tier": "strong",
  "ground_truth_verdict": "Howl",
  "conviction": 0.876,
  "elapsed_ms": 2340,
  "timestamp": "2026-05-17T21:30:00Z"
}
```

### Metrics

**Primary** (answer the meta-question):

| Metric | Calculation | Decision impact |
|--------|-------------|-----------------|
| Spearman ρ(q_score, conviction) | Rank correlation per arm | Core: does q_score track conviction? |
| Tier accuracy | % verdict == expected_verdict | Exact match rate |
| Adjacent match | % verdict within ±1 ordinal step | Tolerance for WAG gap (4 CYNIC tiers vs 3 CultScreener tiers) |
| Mean absolute tier error | Avg ordinal distance (HOWL=3, WAG=2, GROWL=1, BARK=0) | Magnitude of misses |

**Secondary** (diagnose why):

| Metric | What it reveals |
|--------|-----------------|
| Per-axiom ρ vs conviction | Which axiom predicts conviction best |
| Discrimination | Mean q_score gap between strong and weak tokens |
| Enrichment delta | Sonnet enriched − Sonnet naive metrics |
| Confusion matrix 4×4 per arm | Where each arm errs (over-optimism vs over-pessimism) |

### Verdict Mapping

CYNIC uses 4 tiers (HOWL/WAG/GROWL/BARK), CultScreener uses 3 (strong/mixed/weak). The WAG tier has no direct ground truth equivalent.

Resolution: **ordinal distance** (option C from brainstorming). HOWL=3, WAG=2, GROWL=1, BARK=0. Adjacent match allows ±1 step — WAG is adjacent to both HOWL and GROWL, so it counts as a near-miss rather than a failure.

### Claude Prompt

Identical for all 3 Claude arms:

```
You are evaluating a Solana token. Score it on exactly 6 axioms.
Each score must be between 0.05 and 0.618 (phi-inverse ceiling).

FIDELITY: Is it faithful to its claimed purpose?
PHI: Is the holder distribution proportional and harmonious?
VERIFY: Can the claims be independently verified on-chain?
CULTURE: Does it follow established token standards?
BURN: Is it efficiently structured with minimal waste?
SOVEREIGNTY: Is control distributed, not concentrated?

Return ONLY a JSON object, no explanation:
{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX,
 "culture": 0.XX, "burn": 0.XX, "sovereignty": 0.XX}

TOKEN TO EVALUATE:
{stimulus}
```

For naive arms: `{stimulus}` = mint address + symbol.
For enriched arm: `{stimulus}` = full kernel-generated stimulus (from `/judge` response `stimulus_content`).

### Execution

Arm 1 (CYNIC Dogs) runs first per token — its `stimulus_content` is reused for arm 4. This guarantees Dogs and Sonnet enriched judge the exact same data.

**Preflight:** Before the main loop, run `claude -p "Say hello" --model claude-haiku-4-5 --output-format json` to validate CLI flags and auth.

```
for token in 33_tokens:
    arm1 = POST /judge {content: mint, domain: "token-analysis", crystals: true}
    enriched_stimulus = arm1["stimulus_content"]  # from JudgeResponse.stimulus_content
    if enriched_stimulus is None:
        log(f"SKIP {symbol}: enrichment failed")
        continue  # don't silently downgrade arm 4

    arm2 = claude -p <naive_prompt(mint)> --model claude-haiku-4-5
    sleep(2)

    arm3 = claude -p <naive_prompt(mint)> --model claude-sonnet-4-6
    sleep(2)

    arm4 = claude -p <enriched_prompt(enriched_stimulus)> --model claude-sonnet-4-6
    sleep(2)

    append all 4 results to benchmark_results.jsonl
```

Estimated duration: ~25 minutes (33 × 4 arms, ~10s per Claude call + 2s sleep).

### Success Condition

> ρ(Dogs) > ρ(Sonnet naive) AND adjacent_match(Dogs) ≥ adjacent_match(Sonnet enriched)

If met: the CYNIC thesis holds — weak model + ecosystem ≥ strong model alone.
If ρ(Dogs) < ρ(Sonnet enriched): model strength dominates — thesis refuted at current maturity.

### Pipeline Files

```
cynic-python/heuristics/experiments/meta-question/
├── MANIFEST.yaml              # Tier 1 EXPERIMENTAL
├── collect.py                 # Step 1: run 4 arms, produce JSONL
├── analyze.py                 # Step 2: compute metrics, print report
└── benchmark_results.jsonl    # Artifact (gitignored)
```

### Tier Classification

**Tier 1 EXPERIMENTAL.** Research question with clear success condition.
- Success condition: ρ(Dogs) > ρ(Sonnet naive) AND adjacent_match(Dogs) ≥ adjacent_match(Sonnet enriched)
- Partial support: if only one clause holds, document which and why
- Timeline: run within 7 days of spec approval
- Death date: 2026-06-17 (30 days) — promote to Tier 2 or delete
- Promotion path: if results are decisive, the benchmark becomes a CI gate on PRs touching `pipeline/enrichment.rs` or `dogs/deterministic/token.rs` — fails if ρ drops below the recorded baseline
- n=3 weak tier caveat: treat weak-tier results as directional, not representative

### Dependencies

- Kernel running with Dogs available (`curl ${CYNIC_REST_ADDR}/health`)
- `claude` CLI on PATH (Claude Code Max subscription)
- `calibration_results_real.json` exists (33 tokens)
- Network access to Helius (for enrichment during `/judge`)

### Risks

1. **Claude output parsing**: CLI may not always return clean JSON. Mitigation: regex fallback to extract `{...}` from response, retry once on parse failure.
2. **Kernel Dogs unavailable**: If only deterministic Dog runs (no LLM Dogs), results measure heuristics only. Mitigation: check `/health` for Dog count before starting.
3. **Stale enrichment**: Token on-chain data changes daily. Mitigation: run all arms for a token in the same session (< 1 min per token).
4. **Training data leakage**: Claude may have seen major tokens (JUP, BONK) in training data. Mitigation: measure separately on known vs obscure tokens if results are suspicious.

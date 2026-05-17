# Meta-Question Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compare CYNIC Dogs (Qwen 7B + enrichment + crystals) against Claude models on 33 calibration tokens with CultScreener ground truth.

**Architecture:** Two scripts: `collect.py` runs 4 arms per token and writes JSONL, `analyze.py` reads JSONL and computes metrics. Separation allows re-analysis without re-collection. Reuses existing `spearman_rank()` from conviction-temporal experiment.

**Tech Stack:** Python 3, stdlib only (json, subprocess, urllib, os, time). No external deps. Claude Code CLI (`claude -p`) for Claude arms.

**Spec:** `docs/superpowers/specs/2026-05-17-meta-question-benchmark-design.md`

---

### Task 1: Create experiment directory and MANIFEST

**Files:**
- Create: `cynic-python/heuristics/experiments/meta-question/MANIFEST.yaml`

- [ ] **Step 1: Create MANIFEST.yaml**

```yaml
id: meta-question-2026-05-17
hypothesis: "CYNIC Dogs (Qwen 7B + enrichment + crystals) produce better token verdicts than Claude Sonnet without accumulated context."
domain: token-analysis
created: 2026-05-17
status: ACTIVE

method: |
  4-arm comparison on 33 calibration tokens:
  - cynic_dogs: kernel /judge with enrichment + crystals
  - haiku_naive: Claude Haiku 4.5 with mint address only
  - sonnet_naive: Claude Sonnet 4.6 with mint address only
  - sonnet_enriched: Claude Sonnet 4.6 with same enriched stimulus as Dogs

variables:
  independent: [model, stimulus_richness]
  dependent: [q_score, verdict_tier, axiom_scores]
  controlled: [token_set_n33, prompt_template, scoring_range]

success_condition: |
  rho(Dogs) > rho(Sonnet naive) AND adjacent_match(Dogs) >= adjacent_match(Sonnet enriched)

consumed_by:
  - "Decision: validate or refute CYNIC thesis"
  - "If validated: promote to Tier 2 CI gate on enrichment pipeline changes"

death_date: 2026-06-17

files:
  scripts:
    - collect.py
    - analyze.py
  data:
    - benchmark_results.jsonl
```

- [ ] **Step 2: Commit**

```bash
git add cynic-python/heuristics/experiments/meta-question/MANIFEST.yaml
git commit -m "docs(meta-question): MANIFEST for meta-question benchmark experiment"
```

---

### Task 2: Write collect.py — kernel arm

**Files:**
- Create: `cynic-python/heuristics/experiments/meta-question/collect.py`

- [ ] **Step 1: Write collect.py with kernel arm only (arm 1)**

The script loads calibration data, calls `/judge` for each token, writes results to JSONL. Start with just the kernel arm to validate the pipeline works before adding Claude arms.

```python
#!/usr/bin/env python3
"""Tier 1 EXPERIMENTAL: Meta-question benchmark — CYNIC Dogs vs Claude.

Collects verdicts from 4 arms on 33 calibration tokens, writes JSONL.
Run analyze.py on the output to compute metrics.

Usage: python3 collect.py [--arms all|dogs|claude] [--output FILE]
"""

import json
import os
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

# ── Config ──
CALIBRATION_PATH = Path(__file__).parent.parent.parent / "data" / "calibration_results_real.json"
DEFAULT_OUTPUT = Path(__file__).parent / "benchmark_results.jsonl"

CYNIC_ADDR = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
CYNIC_KEY = os.environ.get("CYNIC_API_KEY", "")
JUDGE_TIMEOUT = 120

TIER_FROM_QSCORE = [
    (0.528, "Howl"),
    (0.382, "Wag"),
    (0.236, "Growl"),
    (0.0,   "Bark"),
]

CLAUDE_PROMPT_TEMPLATE = """You are evaluating a Solana token. Score it on exactly 6 axioms.
Each score must be between 0.05 and 0.618 (phi-inverse ceiling).

FIDELITY: Is it faithful to its claimed purpose?
PHI: Is the holder distribution proportional and harmonious?
VERIFY: Can the claims be independently verified on-chain?
CULTURE: Does it follow established token standards?
BURN: Is it efficiently structured with minimal waste?
SOVEREIGNTY: Is control distributed, not concentrated?

Return ONLY a JSON object, no explanation:
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "culture": 0.XX, "burn": 0.XX, "sovereignty": 0.XX}}

TOKEN TO EVALUATE:
{stimulus}"""

CLAUDE_SLEEP = 3  # seconds between Claude calls


def verdict_from_qscore(q: float) -> str:
    for threshold, tier in TIER_FROM_QSCORE:
        if q >= threshold:
            return tier
    return "Bark"


def geo_mean_6(axioms: Dict[str, float]) -> float:
    """Geometric mean of 6 axiom scores."""
    vals = [axioms.get(k, 0.05) for k in ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]]
    product = 1.0
    for v in vals:
        product *= max(v, 0.001)  # avoid log(0)
    return product ** (1.0 / 6.0)


def load_calibration() -> List[Dict]:
    with open(CALIBRATION_PATH) as f:
        data = json.load(f)
    return data["results"]


# ── Arm 1: CYNIC Dogs ──

def run_cynic_dogs(mint: str) -> Optional[Dict[str, Any]]:
    """POST /judge and extract q_score, verdict, axioms, stimulus_content."""
    payload = json.dumps({
        "content": mint,
        "domain": "token-analysis",
        "crystals": True,
    }).encode()
    req = urllib.request.Request(
        f"http://{CYNIC_ADDR}/judge",
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {CYNIC_KEY}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=JUDGE_TIMEOUT) as resp:
            body = json.loads(resp.read())
        q = body.get("q_score", {})
        return {
            "q_score": q.get("total", 0.0),
            "verdict": body.get("verdict", "Bark"),
            "axioms": {
                "fidelity": q.get("fidelity", 0.0),
                "phi": q.get("phi", 0.0),
                "verify": q.get("verify", 0.0),
                "culture": q.get("culture", 0.0),
                "burn": q.get("burn", 0.0),
                "sovereignty": q.get("sovereignty", 0.0),
            },
            "stimulus_content": body.get("stimulus_content"),
            "voter_count": body.get("voter_count", 0),
        }
    except Exception as e:
        print(f"  [dogs] ERROR: {e}")
        return None


# ── Arms 2-4: Claude Code CLI ──

def run_claude(stimulus: str, model: str) -> Optional[Dict[str, Any]]:
    """Call claude -p with structured prompt, parse 6 axiom scores."""
    prompt = CLAUDE_PROMPT_TEMPLATE.format(stimulus=stimulus)
    try:
        result = subprocess.run(
            ["claude", "-p", prompt, "--model", model, "--output-format", "json"],
            capture_output=True, text=True, timeout=60,
        )
        if result.returncode != 0:
            print(f"  [{model}] CLI error: {result.stderr[:200]}")
            return None

        # Parse Claude Code JSON output — extract the text response
        try:
            cli_output = json.loads(result.stdout)
        except json.JSONDecodeError:
            cli_output = result.stdout

        # Extract text content from Claude Code response
        text = ""
        if isinstance(cli_output, dict):
            # Claude Code JSON format: look for result or content
            if "result" in cli_output:
                text = cli_output["result"]
            elif "content" in cli_output:
                content = cli_output["content"]
                if isinstance(content, list):
                    for block in content:
                        if isinstance(block, dict) and block.get("type") == "text":
                            text += block.get("text", "")
                elif isinstance(content, str):
                    text = content
        elif isinstance(cli_output, str):
            text = cli_output

        # Extract JSON object from text (Claude may wrap it in markdown)
        import re
        json_match = re.search(r'\{[^{}]*"fidelity"[^{}]*\}', text)
        if not json_match:
            print(f"  [{model}] No axiom JSON found in response")
            return None

        axioms = json.loads(json_match.group())
        q = geo_mean_6(axioms)
        return {
            "q_score": q,
            "verdict": verdict_from_qscore(q),
            "axioms": axioms,
        }
    except subprocess.TimeoutExpired:
        print(f"  [{model}] timeout (60s)")
        return None
    except Exception as e:
        print(f"  [{model}] ERROR: {e}")
        return None


def write_result(f, token: Dict, arm: str, result: Dict[str, Any], elapsed_ms: int) -> None:
    """Write one JSONL line."""
    line = {
        "mint": token["mint"],
        "symbol": token["symbol"],
        "arm": arm,
        "q_score": result["q_score"],
        "verdict": result["verdict"],
        "axioms": result["axioms"],
        "ground_truth_tier": token["conviction_tier"],
        "ground_truth_verdict": token["expected_verdict"],
        "conviction": token["conviction"],
        "elapsed_ms": elapsed_ms,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    f.write(json.dumps(line) + "\n")
    f.flush()


def preflight_claude() -> bool:
    """Validate claude CLI works before running 99 calls."""
    print("Preflight: testing claude CLI...")
    try:
        result = subprocess.run(
            ["claude", "-p", "Say hello", "--model", "claude-haiku-4-5", "--output-format", "json"],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode == 0:
            print("  Preflight OK")
            return True
        print(f"  Preflight FAIL: {result.stderr[:200]}")
        return False
    except FileNotFoundError:
        print("  ERROR: 'claude' not found on PATH")
        return False
    except Exception as e:
        print(f"  Preflight ERROR: {e}")
        return False


def main() -> None:
    # Parse args
    arms_filter = "all"
    output_path = DEFAULT_OUTPUT
    for i, arg in enumerate(sys.argv[1:], 1):
        if arg == "--arms" and i < len(sys.argv) - 1:
            arms_filter = sys.argv[i + 1]
        elif arg == "--output" and i < len(sys.argv) - 1:
            output_path = Path(sys.argv[i + 1])

    run_dogs = arms_filter in ("all", "dogs")
    run_claude_arms = arms_filter in ("all", "claude")

    tokens = load_calibration()
    print(f"Loaded {len(tokens)} calibration tokens")

    # Preflight
    if run_claude_arms and not preflight_claude():
        print("Claude preflight failed. Run with --arms dogs to skip Claude arms.")
        sys.exit(1)

    # Check kernel health for Dogs arm
    if run_dogs:
        try:
            req = urllib.request.Request(f"http://{CYNIC_ADDR}/health")
            with urllib.request.urlopen(req, timeout=5) as resp:
                health = json.loads(resp.read())
            print(f"Kernel: {health.get('status', '?')}, dogs: {health.get('active_dogs', '?')}")
        except Exception as e:
            print(f"WARNING: kernel health check failed: {e}")

    print(f"Arms: {'dogs' if run_dogs else ''} {'claude (haiku+sonnet)' if run_claude_arms else ''}")
    print(f"Output: {output_path}")
    print(f"{'='*60}")

    with open(output_path, "w") as f:
        for i, token in enumerate(tokens):
            symbol = token["symbol"].strip()
            mint = token["mint"]
            print(f"\n[{i+1}/{len(tokens)}] {symbol} ({token['conviction_tier']}, conv={token['conviction']:.3f})")

            stimulus_content = None

            # ── Arm 1: CYNIC Dogs ──
            if run_dogs:
                t0 = time.time()
                dogs_result = run_cynic_dogs(mint)
                elapsed = int((time.time() - t0) * 1000)
                if dogs_result:
                    stimulus_content = dogs_result.pop("stimulus_content", None)
                    dogs_result.pop("voter_count", None)
                    write_result(f, token, "cynic_dogs", dogs_result, elapsed)
                    print(f"  [dogs] q={dogs_result['q_score']:.3f} → {dogs_result['verdict']} ({elapsed}ms)")
                else:
                    print(f"  [dogs] FAILED")

            if not run_claude_arms:
                continue

            naive_stimulus = "Solana token: " + symbol + "\n" + "Token mint: " + mint

            # ── Arm 2: Haiku naive ──
            t0 = time.time()
            haiku_result = run_claude(naive_stimulus, "claude-haiku-4-5")
            elapsed = int((time.time() - t0) * 1000)
            if haiku_result:
                write_result(f, token, "haiku_naive", haiku_result, elapsed)
                print(f"  [haiku] q={haiku_result['q_score']:.3f} → {haiku_result['verdict']} ({elapsed}ms)")
            else:
                print(f"  [haiku] FAILED")
            time.sleep(CLAUDE_SLEEP)

            # ── Arm 3: Sonnet naive ──
            t0 = time.time()
            sonnet_result = run_claude(naive_stimulus, "claude-sonnet-4-6")
            elapsed = int((time.time() - t0) * 1000)
            if sonnet_result:
                write_result(f, token, "sonnet_naive", sonnet_result, elapsed)
                print(f"  [sonnet] q={sonnet_result['q_score']:.3f} → {sonnet_result['verdict']} ({elapsed}ms)")
            else:
                print(f"  [sonnet] FAILED")
            time.sleep(CLAUDE_SLEEP)

            # ── Arm 4: Sonnet enriched ──
            if stimulus_content:
                t0 = time.time()
                enriched_result = run_claude(stimulus_content, "claude-sonnet-4-6")
                elapsed = int((time.time() - t0) * 1000)
                if enriched_result:
                    write_result(f, token, "sonnet_enriched", enriched_result, elapsed)
                    print(f"  [sonnet+] q={enriched_result['q_score']:.3f} → {enriched_result['verdict']} ({elapsed}ms)")
                else:
                    print(f"  [sonnet+] FAILED")
            else:
                print(f"  [sonnet+] SKIP — no stimulus_content from Dogs arm")
            time.sleep(CLAUDE_SLEEP)

    print(f"\n{'='*60}")
    print(f"Results written to {output_path}")
    print(f"Run: python3 analyze.py {output_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Test kernel arm only**

```bash
cd cynic-python/heuristics/experiments/meta-question
python3 collect.py --arms dogs --output test_dogs.jsonl
# Expected: 33 lines in test_dogs.jsonl, each with q_score/verdict/axioms
head -2 test_dogs.jsonl | python3 -m json.tool
rm test_dogs.jsonl
```

- [ ] **Step 3: Test Claude preflight**

```bash
python3 -c "
import subprocess, json
r = subprocess.run(['claude', '-p', 'Say hello', '--model', 'claude-haiku-4-5', '--output-format', 'json'], capture_output=True, text=True, timeout=30)
print('returncode:', r.returncode)
print('stdout:', r.stdout[:500])
"
```

Examine the output format to verify JSON parsing logic works. Adjust `run_claude()` if the CLI output structure differs from expected.

- [ ] **Step 4: Commit**

```bash
git add cynic-python/heuristics/experiments/meta-question/collect.py
git commit -m "feat(meta-question): collect.py — 4-arm benchmark collector"
```

---

### Task 3: Write analyze.py

**Files:**
- Create: `cynic-python/heuristics/experiments/meta-question/analyze.py`

- [ ] **Step 1: Write analyze.py**

```python
#!/usr/bin/env python3
"""Tier 1 EXPERIMENTAL: Meta-question benchmark analysis.

Reads benchmark_results.jsonl, computes metrics per arm, prints comparison.

Usage: python3 analyze.py [benchmark_results.jsonl]
"""

import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Tuple

DEFAULT_PATH = Path(__file__).parent / "benchmark_results.jsonl"

TIER_ORDINAL = {"Howl": 3, "Wag": 2, "Growl": 1, "Bark": 0}
ORDINAL_TIER = {v: k for k, v in TIER_ORDINAL.items()}


def spearman_rank(x: List[float], y: List[float]) -> float:
    """Spearman rank correlation (no scipy dependency).
    Reused from conviction-temporal/analyze_correlation.py."""
    n = len(x)
    if n < 3:
        return 0.0

    def ranks(vals: List[float]) -> List[float]:
        indexed = sorted(enumerate(vals), key=lambda t: t[1])
        r = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j < n - 1 and indexed[j + 1][1] == indexed[j][1]:
                j += 1
            avg_rank = (i + j) / 2.0 + 1.0
            for k in range(i, j + 1):
                r[indexed[k][0]] = avg_rank
            i = j + 1
        return r

    rx = ranks(x)
    ry = ranks(y)
    d_sq = sum((a - b) ** 2 for a, b in zip(rx, ry))
    return 1.0 - (6.0 * d_sq) / (n * (n * n - 1))


def bootstrap_ci(x: List[float], y: List[float], n_boot: int = 1000) -> Tuple[float, float]:
    """Bootstrap 95% CI for Spearman rho."""
    import random
    n = len(x)
    rhos = []
    for _ in range(n_boot):
        idx = [random.randint(0, n - 1) for _ in range(n)]
        bx = [x[i] for i in idx]
        by = [y[i] for i in idx]
        rhos.append(spearman_rank(bx, by))
    rhos.sort()
    lo = rhos[int(0.025 * n_boot)]
    hi = rhos[int(0.975 * n_boot)]
    return lo, hi


def load_results(path: Path) -> Dict[str, List[Dict]]:
    """Load JSONL, group by arm."""
    arms: Dict[str, List[Dict]] = defaultdict(list)
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            arms[row["arm"]].append(row)
    return dict(arms)


def compute_metrics(rows: List[Dict]) -> Dict:
    """Compute all metrics for one arm."""
    n = len(rows)
    if n == 0:
        return {}

    q_scores = [r["q_score"] for r in rows]
    convictions = [r["conviction"] for r in rows]

    # Spearman rho
    rho = spearman_rank(q_scores, convictions)
    ci_lo, ci_hi = bootstrap_ci(q_scores, convictions)

    # Tier accuracy
    exact_match = sum(1 for r in rows if r["verdict"] == r["ground_truth_verdict"])

    # Adjacent match (within 1 ordinal step)
    adjacent = 0
    for r in rows:
        pred_ord = TIER_ORDINAL.get(r["verdict"], 1)
        true_ord = TIER_ORDINAL.get(r["ground_truth_verdict"], 1)
        if abs(pred_ord - true_ord) <= 1:
            adjacent += 1

    # Mean absolute tier error
    tier_errors = []
    for r in rows:
        pred_ord = TIER_ORDINAL.get(r["verdict"], 1)
        true_ord = TIER_ORDINAL.get(r["ground_truth_verdict"], 1)
        tier_errors.append(abs(pred_ord - true_ord))
    mae = sum(tier_errors) / n

    # Discrimination: mean q_score for strong vs weak
    strong_q = [r["q_score"] for r in rows if r["ground_truth_tier"] == "strong"]
    weak_q = [r["q_score"] for r in rows if r["ground_truth_tier"] == "weak"]
    discrimination = 0.0
    if strong_q and weak_q:
        discrimination = (sum(strong_q) / len(strong_q)) - (sum(weak_q) / len(weak_q))

    # Confusion matrix
    confusion: Dict[str, Dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for r in rows:
        confusion[r["ground_truth_verdict"]][r["verdict"]] += 1

    # Per-axiom rho
    axiom_rhos = {}
    for axiom in ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]:
        vals = [r["axioms"].get(axiom, 0.0) for r in rows]
        axiom_rhos[axiom] = spearman_rank(vals, convictions)

    return {
        "n": n,
        "rho": rho,
        "rho_ci": (ci_lo, ci_hi),
        "tier_accuracy": exact_match / n,
        "tier_accuracy_raw": f"{exact_match}/{n}",
        "adjacent_match": adjacent / n,
        "adjacent_match_raw": f"{adjacent}/{n}",
        "mae": mae,
        "discrimination": discrimination,
        "axiom_rhos": axiom_rhos,
        "confusion": {k: dict(v) for k, v in confusion.items()},
        "mean_q": sum(q_scores) / n,
        "q_range": (min(q_scores), max(q_scores)),
        "mean_elapsed_ms": sum(r.get("elapsed_ms", 0) for r in rows) / n,
    }


def print_report(all_metrics: Dict[str, Dict]) -> None:
    """Print formatted comparison report."""
    arms_order = ["cynic_dogs", "haiku_naive", "sonnet_naive", "sonnet_enriched"]
    arms_present = [a for a in arms_order if a in all_metrics]

    print(f"\n{'='*70}")
    print("META-QUESTION BENCHMARK — CYNIC Dogs vs Claude")
    print(f"{'='*70}\n")

    # Summary table
    header = f"{'Metric':<25s}"
    for arm in arms_present:
        label = arm.replace("_", " ").title()[:12]
        header += f"  {label:>12s}"
    print(header)
    print("-" * len(header))

    rows_spec = [
        ("Spearman rho", "rho", ".3f"),
        ("  95% CI lo", "rho_ci_lo", ".3f"),
        ("  95% CI hi", "rho_ci_hi", ".3f"),
        ("Tier accuracy", "tier_accuracy", ".1%"),
        ("Adjacent match", "adjacent_match", ".1%"),
        ("Mean abs error", "mae", ".2f"),
        ("Discrimination", "discrimination", ".4f"),
        ("Mean q_score", "mean_q", ".3f"),
        ("Mean latency (ms)", "mean_elapsed_ms", ".0f"),
    ]
    for label, key, fmt in rows_spec:
        line = f"{label:<25s}"
        for arm in arms_present:
            m = all_metrics[arm]
            if key == "rho_ci_lo":
                val = m.get("rho_ci", (0, 0))[0]
            elif key == "rho_ci_hi":
                val = m.get("rho_ci", (0, 0))[1]
            else:
                val = m.get(key, 0)
            line += f"  {val:>12{fmt}}"
        print(line)

    # Enrichment delta
    if "sonnet_naive" in all_metrics and "sonnet_enriched" in all_metrics:
        sn = all_metrics["sonnet_naive"]
        se = all_metrics["sonnet_enriched"]
        print(f"\nEnrichment delta (Sonnet enriched - Sonnet naive):")
        print(f"  rho:            {se['rho'] - sn['rho']:+.3f}")
        print(f"  tier_accuracy:  {se['tier_accuracy'] - sn['tier_accuracy']:+.1%}")
        print(f"  adjacent_match: {se['adjacent_match'] - sn['adjacent_match']:+.1%}")

    # Success condition evaluation
    print(f"\n{'='*70}")
    print("SUCCESS CONDITION EVALUATION")
    print(f"{'='*70}")

    if "cynic_dogs" in all_metrics and "sonnet_naive" in all_metrics:
        dogs = all_metrics["cynic_dogs"]
        sn = all_metrics["sonnet_naive"]
        c1 = dogs["rho"] > sn["rho"]
        print(f"  C1: rho(Dogs)={dogs['rho']:.3f} > rho(Sonnet naive)={sn['rho']:.3f}  → {'PASS' if c1 else 'FAIL'}")

        if "sonnet_enriched" in all_metrics:
            se = all_metrics["sonnet_enriched"]
            c2 = dogs["adjacent_match"] >= se["adjacent_match"]
            print(f"  C2: adj(Dogs)={dogs['adjacent_match']:.1%} >= adj(Sonnet enriched)={se['adjacent_match']:.1%}  → {'PASS' if c2 else 'FAIL'}")
            print(f"\n  THESIS: {'VALIDATED' if c1 and c2 else 'PARTIALLY SUPPORTED' if c1 or c2 else 'REFUTED'}")
        else:
            print(f"\n  THESIS (partial): {'SUPPORTED (C1)' if c1 else 'REFUTED (C1)'}")
    else:
        print("  Insufficient arms for evaluation")

    # Confusion matrices
    for arm in arms_present:
        m = all_metrics[arm]
        confusion = m.get("confusion", {})
        if not confusion:
            continue
        label = arm.replace("_", " ").title()
        print(f"\nConfusion Matrix — {label}:")
        tiers = ["Howl", "Wag", "Growl", "Bark"]
        print(f"  {'True/Pred':<10s}", end="")
        for t in tiers:
            print(f"  {t:>6s}", end="")
        print()
        for true_tier in ["Howl", "Growl", "Bark"]:  # CultScreener only has 3
            print(f"  {true_tier:<10s}", end="")
            for pred_tier in tiers:
                count = confusion.get(true_tier, {}).get(pred_tier, 0)
                print(f"  {count:>6d}", end="")
            print()

    # Per-axiom rho
    print(f"\nPer-axiom Spearman rho (vs conviction):")
    axioms = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]
    header = f"  {'Axiom':<14s}"
    for arm in arms_present:
        label = arm.replace("_", " ").title()[:12]
        header += f"  {label:>12s}"
    print(header)
    for axiom in axioms:
        line = f"  {axiom:<14s}"
        for arm in arms_present:
            r = all_metrics[arm].get("axiom_rhos", {}).get(axiom, 0.0)
            line += f"  {r:>12.3f}"
        print(line)

    # Caveats
    print(f"\nCaveats:")
    print(f"  - n=33, class split 20/10/3 — results are directional, not decisive")
    print(f"  - n=3 weak tier is not representative")
    print(f"  - Bootstrap CIs assume independent samples")


def main() -> None:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PATH
    if not path.exists():
        print(f"ERROR: {path} not found. Run collect.py first.")
        sys.exit(1)

    arms = load_results(path)
    print(f"Loaded {sum(len(v) for v in arms.values())} results across {len(arms)} arms")

    all_metrics = {}
    for arm, rows in arms.items():
        all_metrics[arm] = compute_metrics(rows)

    print_report(all_metrics)

    # Save metrics JSON alongside JSONL
    metrics_path = path.with_suffix(".metrics.json")
    with open(metrics_path, "w") as f:
        # Convert tuples to lists for JSON
        serializable = {}
        for arm, m in all_metrics.items():
            sm = dict(m)
            sm["rho_ci"] = list(sm.get("rho_ci", (0, 0)))
            sm["q_range"] = list(sm.get("q_range", (0, 0)))
            serializable[arm] = sm
        json.dump(serializable, f, indent=2)
    print(f"\nMetrics saved to {metrics_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Test analyze.py with Dogs-only data**

```bash
cd cynic-python/heuristics/experiments/meta-question
python3 collect.py --arms dogs
python3 analyze.py benchmark_results.jsonl
# Expected: table with cynic_dogs arm, rho ~0.225, confusion matrix
```

- [ ] **Step 3: Commit**

```bash
git add cynic-python/heuristics/experiments/meta-question/analyze.py
git commit -m "feat(meta-question): analyze.py — metrics computation and comparison report"
```

---

### Task 4: Run full benchmark

- [ ] **Step 1: Run all 4 arms**

```bash
cd cynic-python/heuristics/experiments/meta-question
python3 collect.py --arms all
# ~25 minutes. Watch for parse failures on Claude arms.
```

- [ ] **Step 2: Analyze results**

```bash
python3 analyze.py benchmark_results.jsonl
# Expected: full comparison table with all 4 arms
```

- [ ] **Step 3: Review results and commit**

If results are clean (no excessive parse failures), commit the analysis:

```bash
git add cynic-python/heuristics/experiments/meta-question/benchmark_results.jsonl
git add cynic-python/heuristics/experiments/meta-question/benchmark_results.metrics.json
git commit -m "data(meta-question): first benchmark run — 33 tokens × 4 arms"
```

- [ ] **Step 4: Update MANIFEST with findings**

Update `MANIFEST.yaml` status to `CONSUMED` or `DEAD` based on results. Add findings section with observed rho values per arm.

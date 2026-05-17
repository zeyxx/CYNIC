"""
Tier 1 EXPERIMENTAL: Analysis script for 4-arm benchmark results.

Research question: Do CYNIC Dogs (Qwen 7B + enrichment + crystals) outperform
  Claude models on token verdict accuracy against CultScreener ground truth?

Arms analysed:
  cynic_dogs      — kernel /judge with enrichment + crystals
  haiku_naive     — Claude Haiku 4.5, mint+symbol only
  sonnet_naive    — Claude Sonnet 4.6, mint+symbol only
  sonnet_enriched — Claude Sonnet 4.6, enriched stimulus from Dogs arm

Success condition:
  C1: rho(Dogs) > rho(Sonnet naive)
  C2: adjacent_match(Dogs) >= adjacent_match(Sonnet enriched)
  THESIS: VALIDATED if both pass, PARTIALLY SUPPORTED if one, REFUTED if none.

Timeline: Active until 2026-06-17 (see MANIFEST.yaml)
Status: ACTIVE (started 2026-05-17)
Owned by: T. / CYNIC meta-question experiment
"""

from __future__ import annotations

import json
import random
import sys
from collections import defaultdict
from pathlib import Path

__version__ = "0.1.0"

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

DEFAULT_INPUT = Path(__file__).resolve().parent / "benchmark_results.jsonl"

# Verdict ordinal map (for adjacent-match and MAE computation)
VERDICT_ORDINAL: dict[str, int] = {
    "Bark": 0,
    "Growl": 1,
    "Wag": 2,
    "Howl": 3,
}

ORDINAL_VERDICT: dict[int, str] = {v: k for k, v in VERDICT_ORDINAL.items()}

ALL_VERDICTS = ["Howl", "Wag", "Growl", "Bark"]  # display order (high→low)

AXIOM_KEYS = ("fidelity", "phi", "verify", "culture", "burn", "sovereignty")

BOOTSTRAP_N = 1000


# ---------------------------------------------------------------------------
# Rank correlation — implemented from scratch (no scipy)
# ---------------------------------------------------------------------------

def _ranks(vals: list[float]) -> list[float]:
    """
    Compute rank vector with average-rank tie handling.

    Input contract: vals is a non-empty list of floats.
    Output: list of rank floats (1-based), same length as vals.
    Failure mode: empty list returns empty list.
    """
    n = len(vals)
    if n == 0:
        return []
    indexed = sorted(enumerate(vals), key=lambda t: t[1])
    r: list[float] = [0.0] * n
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


def spearman_rank(x: list[float], y: list[float]) -> float:
    """
    Spearman rho between two float lists. Returns 0.0 if n < 3.

    Input contract: x and y have the same length.
    Output: float in [-1, 1].
    Failure mode: mismatched lengths raise ValueError.
    """
    n = len(x)
    if len(y) != n:
        raise ValueError("spearman_rank: x and y must have equal length")
    if n < 3:
        return 0.0
    rx, ry = _ranks(x), _ranks(y)
    d_sq = sum((a - b) ** 2 for a, b in zip(rx, ry))
    return 1.0 - (6.0 * d_sq) / (n * (n * n - 1))


def bootstrap_ci_rho(
    x: list[float],
    y: list[float],
    n_boot: int = BOOTSTRAP_N,
    seed: int = 42,
) -> tuple[float, float]:
    """
    Bootstrap 95% CI for Spearman rho via 1000 resamples.

    Input contract: x and y have equal length >= 3.
    Output: (p2_5, p97_5) percentile tuple.
    Failure mode: n < 3 returns (0.0, 0.0).
    """
    n = len(x)
    if n < 3:
        return (0.0, 0.0)
    rng = random.Random(seed)
    boot_rhos: list[float] = []
    for _ in range(n_boot):
        indices = [rng.randint(0, n - 1) for _ in range(n)]
        bx = [x[i] for i in indices]
        by = [y[i] for i in indices]
        boot_rhos.append(spearman_rank(bx, by))
    boot_rhos.sort()
    lo = boot_rhos[int(0.025 * n_boot)]
    hi = boot_rhos[int(0.975 * n_boot)]
    return (lo, hi)


# ---------------------------------------------------------------------------
# Metrics computation
# ---------------------------------------------------------------------------

def _mean(vals: list[float]) -> float:
    """Arithmetic mean. Returns 0.0 for empty list."""
    if not vals:
        return 0.0
    return sum(vals) / len(vals)


def _ordinal(verdict: str) -> int:
    """Convert verdict string to ordinal. Unknown verdicts return -1."""
    return VERDICT_ORDINAL.get(verdict, -1)


def compute_arm_metrics(records: list[dict]) -> dict:
    """
    Compute all primary and secondary metrics for a single arm.

    Input contract: records is a non-empty list of dicts with keys:
      q_score, verdict, axioms, ground_truth_verdict, conviction.
    Output: dict with keys rho, rho_ci, tier_accuracy, adjacent_match,
      mae, per_axiom_rho, discrimination, n, confusion.
    Failure mode: missing keys produce 0 / empty defaults.
    """
    n = len(records)

    q_scores: list[float] = []
    convictions: list[float] = []
    verdict_match: list[bool] = []
    adjacent_matches: list[bool] = []
    ordinal_errors: list[float] = []

    # For discrimination: strong vs weak q_scores
    strong_qs: list[float] = []
    weak_qs: list[float] = []

    # Per-axiom score lists
    axiom_scores: dict[str, list[float]] = {k: [] for k in AXIOM_KEYS}

    # Confusion matrix: [ground_truth_verdict][predicted_verdict] = count
    confusion: dict[str, dict[str, int]] = {
        v: {w: 0 for w in ALL_VERDICTS} for v in ALL_VERDICTS
    }

    for rec in records:
        q = float(rec.get("q_score", 0.0))
        conv = float(rec.get("conviction", 0.0))
        pred = rec.get("verdict", "")
        truth = rec.get("ground_truth_verdict", "")
        tier = rec.get("ground_truth_tier", "")

        q_scores.append(q)
        convictions.append(conv)

        # Exact match
        verdict_match.append(pred == truth)

        # Adjacent match (within ±1 ordinal step)
        pred_ord = _ordinal(pred)
        truth_ord = _ordinal(truth)
        if pred_ord >= 0 and truth_ord >= 0:
            adjacent_matches.append(abs(pred_ord - truth_ord) <= 1)
            ordinal_errors.append(float(abs(pred_ord - truth_ord)))
        else:
            adjacent_matches.append(False)
            ordinal_errors.append(0.0)

        # Discrimination: strong = high conviction (>= 0.7 assumed "strong" tier)
        if tier in ("strong",):
            strong_qs.append(q)
        elif tier in ("weak", "rugged", "rug"):
            weak_qs.append(q)

        # Axioms
        axioms = rec.get("axioms", {})
        for k in AXIOM_KEYS:
            val = axioms.get(k)
            if val is not None:
                axiom_scores[k].append(float(val))
            else:
                axiom_scores[k].append(0.0)

        # Confusion matrix
        if truth in confusion and pred in confusion[truth]:
            confusion[truth][pred] += 1

    # Primary metrics
    rho = spearman_rank(q_scores, convictions)
    rho_ci = bootstrap_ci_rho(q_scores, convictions)
    tier_accuracy = _mean([float(m) for m in verdict_match])
    adjacent_match = _mean([float(m) for m in adjacent_matches])
    mae = _mean(ordinal_errors)

    # Per-axiom rho vs conviction
    per_axiom_rho: dict[str, float] = {}
    for k in AXIOM_KEYS:
        per_axiom_rho[k] = spearman_rank(axiom_scores[k], convictions)

    # Discrimination: mean q_score(strong) - mean q_score(weak)
    discrimination = _mean(strong_qs) - _mean(weak_qs)

    return {
        "n": n,
        "rho": rho,
        "rho_ci": rho_ci,
        "tier_accuracy": tier_accuracy,
        "adjacent_match": adjacent_match,
        "mae": mae,
        "per_axiom_rho": per_axiom_rho,
        "discrimination": discrimination,
        "confusion": confusion,
        "q_scores": q_scores,
        "convictions": convictions,
    }


# ---------------------------------------------------------------------------
# Report formatting
# ---------------------------------------------------------------------------

ARM_DISPLAY: dict[str, str] = {
    "cynic_dogs": "Dogs (enriched)",
    "haiku_naive": "Haiku (naive)",
    "sonnet_naive": "Sonnet (naive)",
    "sonnet_enriched": "Sonnet (enriched)",
}

ARM_ORDER = ["cynic_dogs", "haiku_naive", "sonnet_naive", "sonnet_enriched"]


def _fmt(val: float, decimals: int = 3) -> str:
    """Format a float to fixed decimal places, right-aligned in 7 chars."""
    return format(round(val, decimals), f".{decimals}f").rjust(7)


def _pct(val: float) -> str:
    """Format a float as percentage string, e.g. '76.3%'."""
    return format(round(val * 100, 1), ".1f") + "%"


def print_summary_table(arm_metrics: dict[str, dict]) -> None:
    """Print the primary metrics comparison table across all arms."""
    arms = [a for a in ARM_ORDER if a in arm_metrics]

    col_w = 20
    header = "Metric".ljust(26)
    for arm in arms:
        header += ARM_DISPLAY.get(arm, arm).center(col_w)
    print(header)
    print("-" * (26 + col_w * len(arms)))

    rows = [
        ("n", lambda m: str(m["n"]).rjust(7)),
        ("rho (q_score ~ conviction)", lambda m: _fmt(m["rho"])),
        ("rho 95% CI", lambda m: "[" + _fmt(m["rho_ci"][0], 3) + "," + _fmt(m["rho_ci"][1], 3) + "]"),
        ("tier_accuracy (exact)", lambda m: _pct(m["tier_accuracy"]).rjust(7)),
        ("adjacent_match (±1)", lambda m: _pct(m["adjacent_match"]).rjust(7)),
        ("mean_abs_tier_error", lambda m: _fmt(m["mae"])),
        ("discrimination (str-wk)", lambda m: _fmt(m["discrimination"])),
    ]

    for label, extractor in rows:
        row = label.ljust(26)
        for arm in arms:
            try:
                cell = extractor(arm_metrics[arm])
            except (KeyError, TypeError, ZeroDivisionError):
                cell = "  N/A  "
            row += cell.center(col_w)
        print(row)


def print_enrichment_delta(arm_metrics: dict[str, dict]) -> None:
    """Print enrichment delta: sonnet_enriched minus sonnet_naive."""
    if "sonnet_naive" not in arm_metrics or "sonnet_enriched" not in arm_metrics:
        return

    naive = arm_metrics["sonnet_naive"]
    enriched = arm_metrics["sonnet_enriched"]

    print()
    print("=== Enrichment Delta (sonnet_enriched - sonnet_naive) ===")
    deltas = [
        ("rho", enriched["rho"] - naive["rho"]),
        ("tier_accuracy", enriched["tier_accuracy"] - naive["tier_accuracy"]),
        ("adjacent_match", enriched["adjacent_match"] - naive["adjacent_match"]),
        ("mae (negative = better)", enriched["mae"] - naive["mae"]),
        ("discrimination", enriched["discrimination"] - naive["discrimination"]),
    ]
    for label, delta in deltas:
        sign = "+" if delta >= 0 else ""
        print(f"  {label:<30} {sign}{delta:.3f}")


def print_success_conditions(arm_metrics: dict[str, dict]) -> None:
    """Print C1/C2 evaluation and overall THESIS verdict."""
    print()
    print("=== Success Conditions ===")

    dogs = arm_metrics.get("cynic_dogs")
    sonnet_naive = arm_metrics.get("sonnet_naive")
    sonnet_enriched = arm_metrics.get("sonnet_enriched")

    c1_pass: bool | None = None
    c2_pass: bool | None = None

    # C1: rho(Dogs) > rho(Sonnet naive)
    if dogs is not None and sonnet_naive is not None:
        c1_pass = dogs["rho"] > sonnet_naive["rho"]
        status = "PASS" if c1_pass else "FAIL"
        print(
            f"  C1: rho(Dogs) > rho(Sonnet naive)"
            f"  [{dogs['rho']:.3f} vs {sonnet_naive['rho']:.3f}]  → {status}"
        )
    else:
        print("  C1: insufficient data (need cynic_dogs + sonnet_naive)")

    # C2: adjacent_match(Dogs) >= adjacent_match(Sonnet enriched)
    if dogs is not None and sonnet_enriched is not None:
        c2_pass = dogs["adjacent_match"] >= sonnet_enriched["adjacent_match"]
        status = "PASS" if c2_pass else "FAIL"
        print(
            f"  C2: adjacent_match(Dogs) >= adjacent_match(Sonnet enriched)"
            f"  [{dogs['adjacent_match']:.3f} vs {sonnet_enriched['adjacent_match']:.3f}]  → {status}"
        )
    else:
        print("  C2: insufficient data (need cynic_dogs + sonnet_enriched)")

    print()
    if c1_pass is None or c2_pass is None:
        print("  THESIS: INDETERMINATE (missing arms)")
    elif c1_pass and c2_pass:
        print("  THESIS: VALIDATED  (both conditions pass)")
    elif c1_pass or c2_pass:
        print("  THESIS: PARTIALLY SUPPORTED  (one condition passes)")
    else:
        print("  THESIS: REFUTED  (both conditions fail)")


def print_confusion_matrix(arm: str, confusion: dict[str, dict[str, int]]) -> None:
    """Print a 4x4 confusion matrix for one arm."""
    labels = ALL_VERDICTS  # Howl, Wag, Growl, Bark
    col_w = 8
    print()
    display = ARM_DISPLAY.get(arm, arm)
    print(f"  Confusion matrix — {display}  (row=truth, col=predicted)")
    header = "truth \\ pred".ljust(14)
    for lbl in labels:
        header += lbl.center(col_w)
    print("  " + header)
    print("  " + "-" * (14 + col_w * len(labels)))
    for truth in labels:
        row_counts = confusion.get(truth, {})
        row_total = sum(row_counts.values())
        row = truth.ljust(14)
        for pred in labels:
            count = row_counts.get(pred, 0)
            row += str(count).center(col_w)
        row += f"  (n={row_total})"
        print("  " + row)


def print_per_axiom_table(arm_metrics: dict[str, dict]) -> None:
    """Print per-axiom Spearman rho table across arms."""
    arms = [a for a in ARM_ORDER if a in arm_metrics]
    col_w = 18
    print()
    print("=== Per-Axiom rho (axiom_score ~ conviction) ===")
    header = "Axiom".ljust(14)
    for arm in arms:
        header += ARM_DISPLAY.get(arm, arm).center(col_w)
    print(header)
    print("-" * (14 + col_w * len(arms)))
    for k in AXIOM_KEYS:
        row = k.ljust(14)
        for arm in arms:
            val = arm_metrics[arm]["per_axiom_rho"].get(k, 0.0)
            row += _fmt(val).center(col_w)
        print(row)


def print_caveats(arm_metrics: dict[str, dict]) -> None:
    """Print interpretive caveats."""
    print()
    print("=== Caveats ===")
    n_values = [m["n"] for m in arm_metrics.values()]
    n_repr = n_values[0] if n_values else "?"
    print(f"  - n={n_repr} per arm — directional signal only, not decisive at this sample size.")
    print("  - Class split approximately 20/10/3 (strong/weak/rug) — class imbalance applies.")
    print("  - Bootstrap CI uses 1000 resamples with seed=42 (reproducible).")
    print("  - adjacent_match tolerance ±1 ordinal step; Howl↔Wag is adjacent, Howl↔Bark is not.")
    print("  - Conviction sourced from calibration ground truth, not live CultScreener probe.")
    print("  - φ⁻¹ = 0.618 max confidence per axiom — structural ceiling affects rho range.")


# ---------------------------------------------------------------------------
# JSONL loading
# ---------------------------------------------------------------------------

def load_jsonl(path: Path) -> list[dict]:
    """
    Load records from a JSONL file.

    Input contract: path points to a readable UTF-8 JSONL file.
    Output: list of parsed dicts (skips blank lines and bad JSON with warning).
    Failure mode: file not found → RuntimeError.
    """
    if not path.exists():
        raise RuntimeError("Input file not found: " + str(path))
    records: list[dict] = []
    errors = 0
    with path.open("r", encoding="utf-8") as fh:
        for lineno, raw in enumerate(fh, start=1):
            raw = raw.strip()
            if not raw:
                continue
            try:
                records.append(json.loads(raw))
            except json.JSONDecodeError as exc:
                print(f"[analyze] WARNING: line {lineno} JSON error: {exc}", flush=True)
                errors += 1
    if errors:
        print(f"[analyze] {errors} malformed lines skipped.", flush=True)
    return records


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    """Entry point: analyze benchmark_results.jsonl and print comparison report."""
    print(f"[analyze] version {__version__}", flush=True)

    # Resolve input path
    if len(sys.argv) > 1:
        input_path = Path(sys.argv[1])
    else:
        input_path = DEFAULT_INPUT

    print(f"[analyze] Input: {input_path}", flush=True)

    records = load_jsonl(input_path)
    if not records:
        print("[analyze] ERROR: no records found in input file.", flush=True)
        sys.exit(1)

    print(f"[analyze] Loaded {len(records)} records.", flush=True)

    # Group by arm
    by_arm: dict[str, list[dict]] = defaultdict(list)
    for rec in records:
        arm = rec.get("arm", "unknown")
        by_arm[arm].append(rec)

    print(f"[analyze] Arms found: {sorted(by_arm.keys())}", flush=True)
    for arm, recs in by_arm.items():
        print(f"  {arm}: {len(recs)} records", flush=True)

    # Compute metrics per arm
    arm_metrics: dict[str, dict] = {}
    for arm, recs in by_arm.items():
        arm_metrics[arm] = compute_arm_metrics(recs)

    # --- Report ---
    print()
    print("=" * 80)
    print("  CYNIC Meta-Question — 4-Arm Benchmark Analysis")
    print("  Hypothesis: Dogs > naive Sonnet; Dogs >= enriched Sonnet on adjacent_match")
    print("=" * 80)

    # 1. Summary comparison table
    print()
    print("=== Primary Metrics Comparison ===")
    print_summary_table(arm_metrics)

    # 2. Enrichment delta
    print_enrichment_delta(arm_metrics)

    # 3. Success conditions
    print_success_conditions(arm_metrics)

    # 4. Confusion matrices
    print()
    print("=== Confusion Matrices ===")
    for arm in ARM_ORDER:
        if arm in arm_metrics:
            print_confusion_matrix(arm, arm_metrics[arm]["confusion"])

    # 5. Per-axiom rho table
    print_per_axiom_table(arm_metrics)

    # 6. Caveats
    print_caveats(arm_metrics)

    print()
    print("=" * 80)

    # --- Save metrics JSON ---
    output_json_path = input_path.parent / "benchmark_results.metrics.json"

    # Prepare serialisable metrics (strip raw score lists)
    serialisable: dict[str, dict] = {}
    for arm, m in arm_metrics.items():
        serialisable[arm] = {
            "n": m["n"],
            "rho": m["rho"],
            "rho_ci_lo": m["rho_ci"][0],
            "rho_ci_hi": m["rho_ci"][1],
            "tier_accuracy": m["tier_accuracy"],
            "adjacent_match": m["adjacent_match"],
            "mae": m["mae"],
            "discrimination": m["discrimination"],
            "per_axiom_rho": m["per_axiom_rho"],
            "confusion": m["confusion"],
        }

    with output_json_path.open("w", encoding="utf-8") as fh:
        json.dump(serialisable, fh, indent=2)

    print(f"[analyze] Metrics saved: {output_json_path}", flush=True)


if __name__ == "__main__":
    main()

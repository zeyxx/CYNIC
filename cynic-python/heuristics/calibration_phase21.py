"""
Phase 2.1: Calibration Loop — Outcome-verdict accuracy analysis.

Tier 2 INFRASTRUCTURE: joins Phase 2.0 divergence data with outcome ground-truth
to compute per-tier accuracy and propose threshold adjustments.

K15 Consumer: calibration report → human review → backends.toml threshold update
Input:  cynic-python/heuristics/data/outcomes/*.jsonl
        cynic-python/heuristics/data/divergence_*.csv
Output: calibration_report_<timestamp>.md + calibration_deltas.json
"""

from __future__ import annotations

import json
import csv
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional
from collections import Counter, defaultdict


# ── Thresholds (mirrors cynic-kernel/src/domain/constants.rs) ──────────────
BARK_MAX   = 0.236
GROWL_MAX  = 0.382
WAG_MAX    = 0.528
# HOWL = > WAG_MAX

TIER_BOUNDS = [
    ("BARK",  0.0,       BARK_MAX),
    ("GROWL", BARK_MAX,  GROWL_MAX),
    ("WAG",   GROWL_MAX, WAG_MAX),
    ("HOWL",  WAG_MAX,   1.0),
]


def q_to_tier(q: float) -> str:
    for name, lo, hi in TIER_BOUNDS:
        if lo <= q <= hi:
            return name
    return "HOWL"


# ── Ground-truth labeling ────────────────────────────────────────────────────

def derive_outcome_label(o: dict) -> str:
    """
    Derive binary ground truth from raw outcome fields.
    Conservative: only label as rug when evidence is strong.
    """
    if o.get("no_pool") is True:
        return "rug"
    if o.get("survived") is False:
        return "rug"
    pct = o.get("price_change_pct")
    if pct is None:
        return "unknown"
    if pct <= -0.70:
        return "severe_decline"
    if pct <= -0.30:
        return "decline"
    if pct >= 0.20:
        return "growth"
    return "flat"


def is_bad_outcome(label: str) -> bool:
    return label in ("rug", "severe_decline")


def is_good_outcome(label: str) -> bool:
    return label in ("growth", "flat")


# ── Data loading ─────────────────────────────────────────────────────────────

def load_outcomes(data_dir: Path) -> dict[str, dict]:
    outcomes: dict[str, dict] = {}
    for f in sorted((data_dir / "outcomes").glob("outcomes_*.jsonl")):
        with open(f) as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    o = json.loads(line)
                    mint = o.get("mint", "")
                    if mint:
                        outcomes[mint] = o
                except json.JSONDecodeError:
                    continue
    return outcomes


def load_latest_divergence(data_dir: Path) -> list[dict]:
    csvs = sorted(data_dir.glob("divergence_*.csv"), reverse=True)
    if not csvs:
        print("ERROR: no divergence CSV found", file=sys.stderr)
        sys.exit(1)
    with open(csvs[0]) as f:
        return list(csv.DictReader(f))


# ── Analysis ─────────────────────────────────────────────────────────────────

@dataclass
class TierStats:
    name: str
    verdicts: int = 0
    true_positive: int = 0   # bad verdict → bad outcome
    false_positive: int = 0  # bad verdict → good outcome
    true_negative: int = 0   # good verdict → good outcome
    false_negative: int = 0  # good verdict → bad outcome
    unknown: int = 0
    q_scores: list[float] = field(default_factory=list)


def analyze(divergence_rows: list[dict], outcomes: dict[str, dict]) -> dict[str, TierStats]:
    stats: dict[str, TierStats] = {name: TierStats(name=name) for name, *_ in TIER_BOUNDS}
    unmatched = 0

    for row in divergence_rows:
        mint = row.get("mint", "")
        enriched_q_str = row.get("enriched_q_score", "")
        verdict_kind = row.get("enriched_verdict_kind", "")

        if not mint or enriched_q_str in ("", "ERROR", "null"):
            continue

        # enriched_verdict_id=null + enriched_q=0 = judgment failure, not a real score of 0
        enriched_id = row.get("enriched_verdict_id", "")
        if enriched_id in ("null", "", "N/A") or enriched_q_str in ("", "ERROR", "null"):
            tier = stats.get("BARK")  # track failures separately
            if tier:
                tier.unknown += 1
            continue

        try:
            q = float(enriched_q_str)
        except ValueError:
            continue

        if q == 0.0:
            # Sentinel value — judgment failure recorded as 0, not a real BARK verdict
            stats["BARK"].unknown += 1
            continue

        tier_name = q_to_tier(q)
        tier = stats[tier_name]
        tier.verdicts += 1
        tier.q_scores.append(q)

        outcome = outcomes.get(mint)
        if outcome is None:
            unmatched += 1
            tier.unknown += 1
            continue

        label = derive_outcome_label(outcome)
        if label == "unknown":
            tier.unknown += 1
            continue

        is_bad_v = tier_name in ("BARK", "GROWL")
        is_bad_o = is_bad_outcome(label)

        if is_bad_v and is_bad_o:
            tier.true_positive += 1
        elif is_bad_v and not is_bad_o:
            tier.false_positive += 1
        elif not is_bad_v and not is_bad_o:
            tier.true_negative += 1
        elif not is_bad_v and is_bad_o:
            tier.false_negative += 1

    if unmatched:
        print(f"[calibration] WARN: {unmatched} tokens without outcome match")

    return stats


# ── Threshold proposals ───────────────────────────────────────────────────────

def propose_adjustments(stats: dict[str, TierStats]) -> dict:
    """
    Heuristic: if GROWL has high false-negative rate (bad tokens missed),
    suggest raising BARK_MAX to catch more. If WAG has high false-positive,
    consider tightening GROWL_MAX.
    """
    proposals: dict[str, float] = {}

    growl = stats.get("GROWL")
    if growl and growl.verdicts > 0:
        fn_rate = growl.false_negative / growl.verdicts
        if fn_rate > 0.30:
            proposals["BARK_MAX_delta"] = +0.02
            proposals["rationale_BARK"] = f"GROWL FN rate {fn_rate:.0%} > 30% — raise BARK_MAX to catch more bad tokens"

    wag = stats.get("WAG")
    if wag and wag.verdicts > 0:
        fp_rate = wag.false_positive / wag.verdicts if (wag.false_positive + wag.true_negative) > 0 else 0
        if fp_rate > 0.40:
            proposals["GROWL_MAX_delta"] = +0.02
            proposals["rationale_GROWL"] = f"WAG FP rate {fp_rate:.0%} > 40% — raise GROWL_MAX to be more cautious"

    return proposals


# ── Report generation ─────────────────────────────────────────────────────────

def generate_report(
    stats: dict[str, TierStats],
    proposals: dict,
    divergence_count: int,
    outcomes_count: int,
    output_path: Path,
) -> None:
    lines = [
        "# Phase 2.1: Calibration Report",
        f"**Input**: {divergence_count} divergence rows × {outcomes_count} outcomes",
        "",
        "## Accuracy per Tier",
        "",
        "| Tier | Verdicts | TP | FP | TN | FN | Unknown | Precision | Recall |",
        "|------|---------|----|----|----|----|---------|-----------|--------|",
    ]

    total_tp = total_fp = total_tn = total_fn = 0

    for name, *_ in TIER_BOUNDS:
        t = stats[name]
        precision = t.true_positive / (t.true_positive + t.false_positive) if (t.true_positive + t.false_positive) > 0 else None
        recall    = t.true_positive / (t.true_positive + t.false_negative) if (t.true_positive + t.false_negative) > 0 else None
        prec_str  = f"{precision:.0%}" if precision is not None else "—"
        rec_str   = f"{recall:.0%}"    if recall    is not None else "—"
        q_mean    = sum(t.q_scores) / len(t.q_scores) if t.q_scores else 0
        lines.append(
            f"| {name} (q̄={q_mean:.3f}) | {t.verdicts} | {t.true_positive} | {t.false_positive} | "
            f"{t.true_negative} | {t.false_negative} | {t.unknown} | {prec_str} | {rec_str} |"
        )
        total_tp += t.true_positive
        total_fp += t.false_positive
        total_tn += t.true_negative
        total_fn += t.false_negative

    overall_denom = total_tp + total_fp + total_tn + total_fn
    overall_acc = (total_tp + total_tn) / overall_denom if overall_denom > 0 else 0
    lines += [
        "",
        f"**Overall accuracy** (labelled tokens): {overall_acc:.0%}",
        "",
        "## Outcome Distribution",
        "",
    ]

    # Outcome label breakdown
    label_counts: Counter[str] = Counter()
    for t in stats.values():
        label_counts["bad"] += t.true_positive + t.false_negative
        label_counts["good"] += t.true_negative + t.false_positive
        label_counts["unknown"] += t.unknown

    for label, count in sorted(label_counts.items()):
        lines.append(f"- **{label}**: {count}")

    lines += [
        "",
        "## Calibration Signal",
        "",
        "⚠️  Note: deterministic-dog dominated this measurement (LLM Dogs were offline during",
        "Phase 2.0 run). Q-scores cluster near 0.391 (deterministic-dog WAG boundary).",
        "LLM Dog calibration requires a fresh measurement run with all Dogs active.",
        "",
    ]

    if proposals:
        lines += ["## Proposed Threshold Adjustments", ""]
        for k, v in proposals.items():
            lines.append(f"- `{k}`: {v}")
    else:
        lines += ["## Proposed Threshold Adjustments", "", "No adjustments needed based on current data."]

    lines += [
        "",
        "## Falsification",
        "",
        "**Claim**: calibrated thresholds improve prediction accuracy on held-out set.",
        "**Test**: Run measurement on next 45-token batch (collected after calibration).",
        "**Falsify**: If accuracy does not improve by ≥5pp vs baseline, threshold change is noise.",
        "",
        "## Next Steps",
        "",
        "1. Review threshold proposals with human — apply to `backends.toml` if approved",
        "2. Re-run measurement with LLM Dogs active (all 3 Dogs voting)",
        "3. Collect next outcome batch (T+14) for validation",
    ]

    output_path.write_text("\n".join(lines) + "\n")
    print(f"[calibration] Report written: {output_path}")


# ── Entry point ───────────────────────────────────────────────────────────────

def main() -> None:
    import argparse
    from datetime import datetime, timezone

    parser = argparse.ArgumentParser(description="Phase 2.1 calibration analysis")
    parser.add_argument("--data-dir", default="cynic-python/heuristics/data",
                        help="Path to heuristics data directory")
    parser.add_argument("--output-dir", default="cynic-python/heuristics/data",
                        help="Where to write the report")
    args = parser.parse_args()

    data_dir   = Path(args.data_dir)
    output_dir = Path(args.output_dir)
    ts         = datetime.now(timezone.utc).strftime("%Y-%m-%d-%H%M%S")

    print("[calibration] Loading outcomes...")
    outcomes = load_outcomes(data_dir)
    print(f"[calibration] {len(outcomes)} outcomes loaded")

    print("[calibration] Loading divergence data...")
    divergence_rows = load_latest_divergence(data_dir)
    print(f"[calibration] {len(divergence_rows)} divergence rows loaded")

    print("[calibration] Analysing...")
    stats     = analyze(divergence_rows, outcomes)
    proposals = propose_adjustments(stats)

    report_path = output_dir / f"calibration_report_{ts}.md"
    deltas_path = output_dir / f"calibration_deltas_{ts}.json"

    generate_report(stats, proposals, len(divergence_rows), len(outcomes), report_path)

    with open(deltas_path, "w") as f:
        json.dump(proposals, f, indent=2)
    print(f"[calibration] Deltas written: {deltas_path}")

    # Print summary to stdout
    print("\n── Summary ──────────────────────────────")
    for name, *_ in TIER_BOUNDS:
        t = stats[name]
        q_mean = sum(t.q_scores) / len(t.q_scores) if t.q_scores else 0.0
        print(f"  {name:6s} (q̄={q_mean:.3f}): {t.verdicts:3d} verdicts  "
              f"TP={t.true_positive} FP={t.false_positive} TN={t.true_negative} FN={t.false_negative} ?={t.unknown}")
    if proposals:
        print("\n── Proposals ────────────────────────────")
        for k, v in proposals.items():
            print(f"  {k}: {v}")
    else:
        print("\n  No threshold adjustments proposed.")


if __name__ == "__main__":
    main()

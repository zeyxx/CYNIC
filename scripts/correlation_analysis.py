#!/usr/bin/env python3
"""Tier 1 EXPERIMENTAL: Correlation analysis for token enrichment signals.

Research question: Which enrichment fields predict token survival outcomes?
Success condition: Identify signals with |rho| > 0.3 for weight calibration.
Timeline: 30 days — if not promoted to Tier 2 by 2026-06-25, delete.
Owned by: @T

Computes Spearman rho for each enrichment field vs ground truth category
(DEAD=0, SKETCHY=1, SURVIVOR=2, LEGIT=3). Outputs correlation_matrix.json
for human-gated decision making on deterministic dog weight calibration.

Usage:
    python3 scripts/correlation_analysis.py                    # analyze benchmark data
    python3 scripts/correlation_analysis.py --fallback         # use calibration dataset
    python3 scripts/correlation_analysis.py --output results/  # custom output dir

Input: cynic-python/data/benchmark_daily/*.jsonl (daily benchmark output)
Fallback: cynic-python/heuristics/data/calibration_results_real.json
Output: cynic-python/data/correlation_matrix.json
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# ── Constants ──

CATEGORY_ORDINAL: dict[str, int] = {
    "DEAD": 0,
    "SKETCHY": 1,
    "SURVIVOR": 2,
    "LEGIT": 3,
}

# Fields to correlate from enrichment_snapshot
ENRICHMENT_FIELDS: list[str] = [
    "holders",
    "top_1_pct",
    "herfindahl",
    "age_hours",
    "k_score",
    "diamond_hands",
    "longevity",
    "organic_growth",
    "supply_burned_pct",
    "liquidity_usd",
    "volume_24h_usd",
    "effective_concentration",
    "accumulators",
    "extractors",
]

# Current weight direction in token.rs (for alignment check)
CURRENT_CODE_DIRECTION: dict[str, str] = {
    "holders": "positive",
    "top_1_pct": "negative",
    "herfindahl": "negative",
    "age_hours": "positive",
    "k_score": "removed",       # rho=-0.327, composite removed from scoring
    "diamond_hands": "negative",  # rho=-0.396, inverted in code
    "longevity": "positive",     # rho=+0.632, strongest positive
    "organic_growth": "positive",
    "supply_burned_pct": "positive",
    "liquidity_usd": "removed",  # rho=+0.038, noise, removed from scoring
    "volume_24h_usd": "positive",
    "effective_concentration": "negative",
    "accumulators": "negative",  # rho=-0.622, inverted in code
    "extractors": "positive",
}

# ── Spearman rho (no scipy dependency) ──


def _rank(values: list[float]) -> list[float]:
    """Assign ranks with average tie-breaking."""
    n = len(values)
    indexed = sorted(range(n), key=lambda i: values[i])
    ranks = [0.0] * n

    i = 0
    while i < n:
        j = i
        while j < n - 1 and values[indexed[j]] == values[indexed[j + 1]]:
            j += 1
        avg_rank = (i + j) / 2.0 + 1.0
        for k in range(i, j + 1):
            ranks[indexed[k]] = avg_rank
        i = j + 1

    return ranks


def spearman_rho(x: list[float], y: list[float]) -> tuple[float, float]:
    """Compute Spearman rank correlation and approximate p-value.

    Returns (rho, p_value). p_value uses t-distribution approximation.
    """
    n = len(x)
    if n < 3:
        return 0.0, 1.0

    rx = _rank(x)
    ry = _rank(y)

    mean_rx = sum(rx) / n
    mean_ry = sum(ry) / n

    num = sum((rx[i] - mean_rx) * (ry[i] - mean_ry) for i in range(n))
    den_x = sum((rx[i] - mean_rx) ** 2 for i in range(n)) ** 0.5
    den_y = sum((ry[i] - mean_ry) ** 2 for i in range(n)) ** 0.5

    if den_x == 0 or den_y == 0:
        return 0.0, 1.0

    rho = num / (den_x * den_y)

    # t-approximation for p-value
    import math
    if abs(rho) >= 1.0:
        p_value = 0.0
    else:
        t_stat = rho * math.sqrt((n - 2) / (1 - rho ** 2))
        # Two-tailed p-value approximation using normal for large n
        p_value = 2.0 * (1.0 - _normal_cdf(abs(t_stat))) if n > 30 else 2.0 / (1.0 + abs(t_stat) ** 2) ** ((n - 2) / 2.0)

    return rho, max(p_value, 1e-10)


def _normal_cdf(x: float) -> float:
    """Approximation of standard normal CDF."""
    import math
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


# ── Data loading ──


def load_benchmark_data(benchmark_dir: str) -> list[dict]:
    """Load all JSONL files from benchmark_daily/."""
    rows: list[dict] = []
    bench_path = Path(benchmark_dir)
    if not bench_path.exists():
        return rows

    for jsonl_file in sorted(bench_path.glob("*.jsonl")):
        with open(jsonl_file) as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        row = json.loads(line)
                        if row.get("schema_version") == 1:
                            rows.append(row)
                    except json.JSONDecodeError:
                        continue
    return rows


def load_calibration_fallback(calib_path: str) -> list[dict]:
    """Load calibration_results_real.json as fallback, mapping to benchmark schema."""
    rows: list[dict] = []
    if not os.path.exists(calib_path):
        print(f"WARNING: calibration file not found: {calib_path}", file=sys.stderr)
        return rows

    with open(calib_path) as f:
        calib = json.load(f)

    tier_map = {"strong": "LEGIT", "mixed": "SURVIVOR", "weak": "DEAD"}

    for r in calib.get("results", []):
        category = tier_map.get(r.get("conviction_tier", ""), "UNKNOWN")
        if category == "UNKNOWN":
            continue

        # Map calibration metrics to enrichment_snapshot format
        metrics = r.get("metrics", {})
        rows.append({
            "mint": r["mint"],
            "symbol": r.get("symbol", "?"),
            "category": category,
            "q_score": r.get("q_score", 0),
            "enrichment_snapshot": {
                "holders": metrics.get("holders"),
                "top_1_pct": metrics.get("top1_pct"),
                "herfindahl": None,
                "age_hours": None,
                "k_score": None,
                "diamond_hands": None,
                "longevity": None,
                "organic_growth": None,
                "supply_burned_pct": None,
                "liquidity_usd": None,
                "volume_24h_usd": None,
                "effective_concentration": None,
                "accumulators": None,
                "extractors": None,
            },
        })
    return rows


def deduplicate_by_mint(rows: list[dict]) -> list[dict]:
    """Keep most recent observation per mint (last in list wins)."""
    seen: dict[str, dict] = {}
    for row in rows:
        seen[row["mint"]] = row
    return list(seen.values())


# ── Analysis ──


def compute_correlations(rows: list[dict]) -> dict:
    """Compute Spearman rho for each enrichment field vs category ordinal."""
    results: dict = {}

    for field in ENRICHMENT_FIELDS:
        x_vals: list[float] = []
        y_vals: list[float] = []

        for row in rows:
            category = row.get("category", "UNKNOWN")
            if category not in CATEGORY_ORDINAL:
                continue

            enrichment = row.get("enrichment_snapshot", {})
            value = enrichment.get(field)
            if value is None:
                continue

            try:
                x_vals.append(float(value))
                y_vals.append(float(CATEGORY_ORDINAL[category]))
            except (ValueError, TypeError):
                continue

        if len(x_vals) < 5:
            results[field] = {
                "rho": None,
                "p": None,
                "n": len(x_vals),
                "direction": "insufficient_data",
                "current_code_direction": CURRENT_CODE_DIRECTION.get(field, "unknown"),
                "aligned": None,
                "tier": "insufficient",
            }
            continue

        rho, p_value = spearman_rho(x_vals, y_vals)

        direction = "positive" if rho > 0 else "negative" if rho < 0 else "zero"
        current = CURRENT_CODE_DIRECTION.get(field, "unknown")

        # Check alignment
        if current == "removed":
            aligned = abs(rho) < 0.1  # removed fields should indeed be noise
        elif current == "unknown":
            aligned = None
        else:
            aligned = (direction == current)

        # Weight tier recommendation
        abs_rho = abs(rho)
        if abs_rho >= 0.5:
            tier = "LARGE (0.15)"
        elif abs_rho >= 0.2:
            tier = "MEDIUM (0.10)"
        elif abs_rho >= 0.1:
            tier = "SMALL (0.05)"
        else:
            tier = "REMOVE (noise)"

        results[field] = {
            "rho": round(rho, 4),
            "p": round(p_value, 6),
            "n": len(x_vals),
            "direction": direction,
            "current_code_direction": current,
            "aligned": aligned,
            "tier": tier,
        }

    return results


# ── Main ──


def main() -> None:
    project_dir = Path(__file__).resolve().parent.parent
    benchmark_dir = project_dir / "cynic-python" / "data" / "benchmark_daily"
    calib_path = project_dir / "cynic-python" / "heuristics" / "data" / "calibration_results_real.json"
    output_path = project_dir / "cynic-python" / "data" / "correlation_matrix.json"

    use_fallback = "--fallback" in sys.argv

    # Custom output
    for i, arg in enumerate(sys.argv):
        if arg == "--output" and i + 1 < len(sys.argv):
            output_path = Path(sys.argv[i + 1]) / "correlation_matrix.json"

    # Load data
    rows: list[dict] = []
    if not use_fallback:
        rows = load_benchmark_data(str(benchmark_dir))
        if len(rows) < 10:
            print(f"Only {len(rows)} benchmark rows — falling back to calibration dataset")
            use_fallback = True

    if use_fallback:
        fallback_rows = load_calibration_fallback(str(calib_path))
        rows.extend(fallback_rows)

    rows = deduplicate_by_mint(rows)
    print(f"Analyzing {len(rows)} tokens (deduplicated by mint)")

    if len(rows) < 5:
        print("ERROR: insufficient data for correlation analysis (<5 tokens)", file=sys.stderr)
        sys.exit(1)

    # Compute
    correlations = compute_correlations(rows)

    # Build output
    output: dict = {
        "computed_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "n_tokens": len(rows),
        "source": "benchmark" if not use_fallback else "calibration_fallback",
        "correlations": correlations,
    }

    # Print summary
    print(f"\n{'Field':<25} {'rho':>8} {'p':>10} {'n':>4} {'Direction':<10} {'Aligned':>8} {'Tier':<16}")
    print("-" * 90)
    for field, data in sorted(correlations.items(), key=lambda kv: abs(kv[1].get("rho") or 0), reverse=True):
        rho = data.get("rho")
        rho_str = f"{rho:+.4f}" if rho is not None else "   n/a"
        p_str = f"{data['p']:.6f}" if data.get("p") is not None else "      n/a"
        aligned_str = "YES" if data.get("aligned") is True else "NO" if data.get("aligned") is False else "?"
        print(f"{field:<25} {rho_str:>8} {p_str:>10} {data['n']:>4} {data['direction']:<10} {aligned_str:>8} {data['tier']:<16}")

    # Misalignment warnings
    misaligned = [f for f, d in correlations.items() if d.get("aligned") is False]
    if misaligned:
        print(f"\nWARNING: {len(misaligned)} signals misaligned with current code: {misaligned}")
        print("Review token.rs weight assignments for these fields.")

    # Write output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nOutput: {output_path}")


if __name__ == "__main__":
    main()

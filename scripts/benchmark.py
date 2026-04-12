#!/usr/bin/env python3
"""CYNIC Empirical Benchmark — Multi-domain A/B evaluation with metrics.

Reads stimuli from tests/fixtures/benchmark.json, runs each through
POST /judge with crystals=true and crystals=false, computes:
  - Tier match rate (ordinal accuracy)
  - Adjacent match rate (within 1 tier)
  - Score discrimination (mean HOWL - mean BARK)
  - Crystal delta (mean score change from crystal injection)
  - Dog concordance (1 - mean max_disagreement)

Usage: python3 scripts/benchmark.py [--save]
  --save  Write results to results/benchmark_YYYYMMDD_HHMMSS.jsonl
"""

import json, os, sys, time, urllib.request, datetime
from pathlib import Path

ADDR = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
API_KEY = os.environ.get("CYNIC_API_KEY", "")
BASE = f"http://{ADDR}"
TIMEOUT = 120

TIER_ORDER = {"Howl": 0, "Wag": 1, "Growl": 2, "Bark": 3}
TIER_THRESHOLDS = {
    "Howl":  (0.528, 1.0),
    "Wag":   (0.382, 0.528),
    "Growl": (0.236, 0.382),
    "Bark":  (0.0,   0.236),
}

def judge(content, context, domain, crystals):
    payload = json.dumps({
        "content": content,
        "context": context,
        "domain": domain,
        "crystals": crystals,
    }).encode()
    req = urllib.request.Request(
        f"{BASE}/judge",
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {API_KEY}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            return json.loads(resp.read())
    except Exception as e:
        return {"verdict": "Error", "q_score": {"total": 0.0},
                "dogs_used": "", "max_disagreement": 0.0, "error": str(e)}


def verdict_to_tier(verdict_str):
    return TIER_ORDER.get(verdict_str, -1)


def tier_match(expected, actual):
    e = TIER_ORDER.get(expected, -1)
    a = verdict_to_tier(actual)
    return e == a


def adjacent_match(expected, actual):
    e = TIER_ORDER.get(expected, -1)
    a = verdict_to_tier(actual)
    return abs(e - a) <= 1


def score_in_range(q, smin, smax):
    return smin <= q <= smax + 0.02  # small tolerance


def main():
    save = "--save" in sys.argv

    # Load dataset
    root = Path(__file__).resolve().parent.parent
    fixture = root / "cynic-kernel" / "tests" / "fixtures" / "benchmark.json"
    with open(fixture) as f:
        dataset = json.load(f)

    stimuli = dataset["stimuli"]
    n = len(stimuli)

    print(f"\u25b6 CYNIC Empirical Benchmark v{dataset['version']} ({n} stimuli \u00d7 2 modes = {n*2} calls)")
    print()

    results = []
    tier_matches = 0
    adj_matches = 0
    range_hits = 0
    scores_by_tier = {"Howl": [], "Wag": [], "Growl": [], "Bark": []}
    deltas = []
    disagreements = []
    t0 = time.time()

    for i, s in enumerate(stimuli):
        sid = s["id"]
        expected = s["expected_verdict"]
        label = s["content"][:45]
        sys.stdout.write(f"  {i+1:2d}/{n} {sid:20s} ")
        sys.stdout.flush()

        v_on = judge(s["content"], s.get("context", ""), s["domain"], True)
        v_off = judge(s["content"], s.get("context", ""), s["domain"], False)

        q_on = v_on["q_score"]["total"]
        q_off = v_off["q_score"]["total"]
        vd_on = v_on["verdict"]
        vd_off = v_off["verdict"]
        delta = q_on - q_off
        disagree = v_on.get("max_disagreement", 0.0)

        tm = tier_match(expected, vd_on)
        am = adjacent_match(expected, vd_on)
        rng = score_in_range(q_on, s.get("score_min", 0), s.get("score_max", 1))

        if tm: tier_matches += 1
        if am: adj_matches += 1
        if rng: range_hits += 1
        deltas.append(delta)
        disagreements.append(disagree)

        # Collect by expected tier for discrimination
        if expected in scores_by_tier:
            scores_by_tier[expected].append(q_on)

        tag = "\u2713" if tm else ("\u2248" if am else "\u2717")
        print(f"{expected:5s}\u2192{vd_on:5s} Q={q_on:.3f} \u0394={delta:+.3f} [{tag}]")

        # Per-dog scores: {dog_id: {axiom: score}}
        dog_scores = {}
        for ds in v_on.get("dog_scores", []):
            dog_id = ds.get("dog_id", "?")
            dog_scores[dog_id] = {
                a: ds.get(a, 0.0)
                for a in ["fidelity","phi","verify","culture","burn","sovereignty"]
            }
            dog_scores[dog_id]["latency_ms"] = ds.get("latency_ms", 0)

        results.append({
            "id": sid, "domain": s["domain"], "expected": expected,
            "verdict_on": vd_on, "q_on": q_on, "verdict_off": vd_off, "q_off": q_off,
            "delta": delta, "tier_match": tm, "adjacent_match": am,
            "max_disagreement": disagree, "dogs_on": v_on.get("dogs_used", ""),
            "dog_scores": dog_scores,
            "q_score_axioms": v_on.get("q_score", {}),
        })

    elapsed = time.time() - t0

    # ── Metrics ──
    mean_delta = sum(deltas) / n if n else 0
    mean_disagree = sum(disagreements) / n if n else 0
    concordance = 1 - mean_disagree

    howl_mean = sum(scores_by_tier["Howl"]) / len(scores_by_tier["Howl"]) if scores_by_tier["Howl"] else 0
    bark_mean = sum(scores_by_tier["Bark"]) / len(scores_by_tier["Bark"]) if scores_by_tier["Bark"] else 0
    discrimination = howl_mean - bark_mean

    tier_rate = tier_matches / n if n else 0
    adj_rate = adj_matches / n if n else 0
    range_rate = range_hits / n if n else 0

    # QBR = tier_match × discrimination × concordance (0-1 composite)
    qbr = tier_rate * min(discrimination / 0.5, 1.0) * concordance

    print()
    print(f"{'='*60}")
    print(f"  CYNIC Empirical Benchmark Results ({elapsed:.0f}s)")
    print(f"{'='*60}")
    print(f"  Stimuli:        {n}")
    print(f"  Tier Match:     {tier_matches}/{n} ({tier_rate:.1%})")
    print(f"  Adjacent Match: {adj_matches}/{n} ({adj_rate:.1%})")
    print(f"  Score in Range: {range_hits}/{n} ({range_rate:.1%})")
    print()
    print(f"  Mean Q by tier:")
    for tier in ["Howl", "Wag", "Growl", "Bark"]:
        scores = scores_by_tier[tier]
        if scores:
            print(f"    {tier:5s}: {sum(scores)/len(scores):.4f} (n={len(scores)})")
    print()
    print(f"  Discrimination (HOWL-BARK): {discrimination:.4f}")
    print(f"  Crystal Delta (mean):       {mean_delta:+.4f}")
    print(f"  Dog Concordance:            {concordance:.4f}")
    print(f"  QBR (composite):            {qbr:.4f}")
    print()

    # Per-dog accuracy breakdown
    all_dogs = set()
    for r in results:
        all_dogs.update(r.get("dog_scores", {}).keys())
    if all_dogs:
        print(f"  Per-Dog mean scores (across all stimuli):")
        for dog in sorted(all_dogs):
            axiom_sums = {a: [] for a in ["fidelity","phi","verify","culture","burn","sovereignty"]}
            latencies = []
            for r in results:
                ds = r.get("dog_scores", {}).get(dog)
                if ds:
                    for a in axiom_sums:
                        axiom_sums[a].append(ds.get(a, 0.0))
                    latencies.append(ds.get("latency_ms", 0))
            if latencies:
                means = " ".join(f"{a[:3]}={sum(v)/len(v):.2f}" for a, v in axiom_sums.items())
                lat = sum(latencies) / len(latencies)
                print(f"    {dog:22s}: {means}  lat={lat:.0f}ms (n={len(latencies)})")
        print()

    # Per-domain breakdown
    domains = sorted(set(s["domain"] for s in stimuli))
    for d in domains:
        d_results = [r for r in results if r["domain"] == d]
        d_tm = sum(1 for r in d_results if r["tier_match"])
        d_n = len(d_results)
        print(f"  {d:12s}: {d_tm}/{d_n} tier match ({d_tm/d_n:.0%})")

    # Misses detail
    misses = [r for r in results if not r["tier_match"]]
    if misses:
        print()
        print(f"  Misclassifications ({len(misses)}):")
        for r in misses:
            adj = " (adjacent)" if r["adjacent_match"] else ""
            print(f"    {r['id']:20s} expected={r['expected']:5s} got={r['verdict_on']:5s} Q={r['q_on']:.3f}{adj}")

    # Save
    if save:
        out_dir = root / "results"
        out_dir.mkdir(exist_ok=True)
        ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        out_file = out_dir / f"benchmark_{ts}.jsonl"
        with open(out_file, "w") as f:
            for r in results:
                f.write(json.dumps(r) + "\n")
            # Summary line
            f.write(json.dumps({
                "_summary": True, "timestamp": ts, "stimuli": n,
                "tier_match": tier_matches, "adj_match": adj_matches,
                "discrimination": discrimination, "crystal_delta": mean_delta,
                "concordance": concordance, "qbr": qbr, "elapsed_s": elapsed,
            }) + "\n")
        print(f"\n  Results saved: {out_file}")

    print()
    # Exit code: fail if tier match < 50%
    sys.exit(0 if tier_rate >= 0.5 else 1)


if __name__ == "__main__":
    main()

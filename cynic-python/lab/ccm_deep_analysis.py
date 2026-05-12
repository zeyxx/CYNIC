"""
Tier 1 EXPERIMENTAL: CCM deep analysis — what the Dogs actually see.

Research question: What patterns in 13K verdicts should inform crystal design?
"""

import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import httpx
import numpy as np

SURREAL_URL = "http://localhost:8000/sql"
SURREAL_HEADERS = {"Accept": "application/json", "surreal-ns": "cynic", "surreal-db": "main"}
SURREAL_AUTH = ("root", "root")
OUTPUT_PATH = Path(__file__).parent / "ccm_deep_analysis.json"


def query_surreal(sql: str) -> list:
    resp = httpx.post(SURREAL_URL, headers=SURREAL_HEADERS, auth=SURREAL_AUTH, content=sql, timeout=60.0)
    data = resp.json()
    if isinstance(data, list) and data and data[0].get("status") == "OK":
        return data[0].get("result", [])
    return []


def main():
    start = time.time()
    print("=== CCM Deep Analysis ===\n")

    # 1. Q-score by domain
    print("--- Q-score by domain ---")
    q_domain = query_surreal(
        "SELECT domain, math::mean(total) AS avg_q, math::stddev(total) AS std_q, "
        "count() AS n FROM verdict GROUP BY domain ORDER BY n DESC LIMIT 15;"
    )
    for d in q_domain:
        avg = float(d.get("avg_q") or 0)
        std = float(d.get("std_q") or 0)
        domain_name = d.get('domain') or '(none)'
        print(f"  {domain_name:>20}: avg={avg:.3f} std={std:.3f} n={d['n']}")

    # 2. Voter count distribution
    print("\n--- Voter count ---")
    voter = query_surreal("SELECT voter_count, count() AS c FROM verdict GROUP BY voter_count ORDER BY voter_count;")
    for v in voter:
        vc = v.get("voter_count") or 0
        print(f"  {vc} dogs: {v['c']}")

    # 3. Agreement distribution
    print("\n--- Agreement (max_disagreement) ---")
    agr = query_surreal(
        "SELECT "
        "count() FILTER (WHERE max_disagreement < 0.382) AS agreed, "
        "count() FILTER (WHERE max_disagreement >= 0.382 AND max_disagreement < 0.618) AS disputed, "
        "count() FILTER (WHERE max_disagreement >= 0.618) AS contested "
        "FROM verdict GROUP ALL;"
    )
    if agr:
        a = agr[0]
        total_v = sum(int(a.get(k) or 0) for k in ["agreed", "disputed", "contested"])
        for k in ["agreed", "disputed", "contested"]:
            val = int(a.get(k) or 0)
            pct = val / total_v * 100 if total_v else 0
            print(f"  {k}: {val} ({pct:.1f}%)")

    # 4. Per-axiom disagreement from sampled dog_scores_json
    print("\n--- Per-axiom Dog disagreement (sampled) ---")
    sample = query_surreal(
        "SELECT domain, total, max_disagreement, kind, voter_count, dog_scores_json "
        "FROM verdict WHERE voter_count >= 2 ORDER BY RAND() LIMIT 1000;"
    )

    axioms = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]
    axiom_spreads: dict[str, list] = {a: [] for a in axioms}
    dog_pair_deltas: dict[str, list] = defaultdict(list)
    domain_disagree: dict[str, list] = defaultdict(list)
    kind_by_domain: dict[str, Counter] = defaultdict(Counter)

    parsed = 0
    for v in sample:
        raw = v.get("dog_scores_json")
        if not raw:
            continue
        try:
            scores = json.loads(raw) if isinstance(raw, str) else raw
        except (json.JSONDecodeError, TypeError):
            continue
        if not isinstance(scores, list) or len(scores) < 2:
            continue

        parsed += 1
        domain = v.get("domain", "?")
        kind = v.get("kind", "?")
        domain_disagree[domain].append(float(v.get("max_disagreement") or 0))
        kind_by_domain[domain][kind] += 1

        for axiom in axioms:
            vals = [float(s.get(axiom, 0) or 0) for s in scores]
            if len(vals) >= 2:
                axiom_spreads[axiom].append(max(vals) - min(vals))

        dog_ids = [s.get("dog_id", "?") for s in scores]
        for i in range(len(dog_ids)):
            for j in range(i + 1, len(dog_ids)):
                pair = " x ".join(sorted([dog_ids[i], dog_ids[j]]))
                qi = float(scores[i].get("fidelity", 0) or 0)
                qj = float(scores[j].get("fidelity", 0) or 0)
                dog_pair_deltas[pair].append(abs(qi - qj))

    print(f"  (parsed {parsed}/{len(sample)} verdicts with dog_scores)")
    for axiom in sorted(axiom_spreads, key=lambda a: -np.mean(axiom_spreads[a]) if axiom_spreads[a] else 0):
        vals = axiom_spreads[axiom]
        if vals:
            print(f"  {axiom:>12}: mean_spread={np.mean(vals):.3f} median={np.median(vals):.3f} p90={np.percentile(vals,90):.3f}")

    # 5. Dog pair agreement
    print("\n--- Dog pair agreement (fidelity delta) ---")
    for pair in sorted(dog_pair_deltas, key=lambda p: -len(dog_pair_deltas[p])):
        vals = dog_pair_deltas[pair]
        if len(vals) >= 10:
            print(f"  {pair:>45}: mean_delta={np.mean(vals):.3f} n={len(vals)}")

    # 6. Domain disagreement ranking
    print("\n--- Domain disagreement (lower = Dogs agree more) ---")
    for domain in sorted(domain_disagree, key=lambda d: -len(domain_disagree[d]))[:12]:
        vals = domain_disagree[domain]
        print(f"  {domain:>20}: mean={np.mean(vals):.3f} std={np.std(vals):.3f} n={len(vals)}")

    # 7. Verdict kind distribution per domain
    print("\n--- Verdict kind by domain ---")
    for domain in sorted(kind_by_domain, key=lambda d: -sum(kind_by_domain[d].values()))[:10]:
        kinds = kind_by_domain[domain]
        total_k = sum(kinds.values())
        dist = " ".join(f"{k}={v}({v/total_k*100:.0f}%)" for k, v in kinds.most_common())
        print(f"  {domain:>20}: {dist}")

    # 8. Crystal quality
    print("\n--- Crystal quality ---")
    crystals = query_surreal(
        "SELECT id, domain, state, observations, confidence, certainty, "
        "variance_m2, mean_quorum, howl_count, wag_count, growl_count, bark_count "
        "FROM crystal ORDER BY observations DESC;"
    )
    for c in crystals:
        obs = int(c.get("observations") or 0)
        howl = int(c.get("howl_count") or 0)
        wag = int(c.get("wag_count") or 0)
        growl = int(c.get("growl_count") or 0)
        bark = int(c.get("bark_count") or 0)
        total_p = howl + wag + growl + bark
        dominant = max({"H": howl, "W": wag, "G": growl, "B": bark}, key=lambda k: {"H": howl, "W": wag, "G": growl, "B": bark}[k])
        var = float(c.get("variance_m2") or 0)
        quorum = float(c.get("mean_quorum") or 0)

        print(
            f"  {c.get('domain','?'):>10} [{c.get('state','?')}] obs={obs:>4} "
            f"conf={float(c.get('confidence') or 0):.3f} cert={float(c.get('certainty') or 0):.3f} "
            f"var_m2={var:.2f} quorum={quorum:.1f} "
            f"H={howl} W={wag} G={growl} B={bark} dom={dominant}"
        )

    elapsed = time.time() - start
    print(f"\n({elapsed:.1f}s)")


if __name__ == "__main__":
    main()

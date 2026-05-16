#!/usr/bin/env python3
"""Calibration test: inject sovereign conviction into TokenScorer.

Before: TokenScorer uses only static metrics → 30% accuracy
After: TokenScorer uses conviction (7d all_holders retention) → measure accuracy

Uses cached data from conviction_by_tranche.py (0 API calls).

Approach: conviction directly maps to the CULTURE + VERIFY axioms.
  - High conviction (>0.9) → strong boost to culture + verify + sovereignty
  - Low conviction (<0.5) → penalty
  - This is the signal the Dogs were missing.
"""

import json
import os
import sys
from dataclasses import dataclass
from typing import Optional, Dict, List

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


@dataclass
class AxiomScores:
    fidelity: float
    phi: float
    verify: float
    culture: float
    burn: float
    sovereignty: float
    q_score: float


def verdict_from_qscore(q: float) -> str:
    if q > 0.528: return "Howl"
    elif q > 0.382: return "Wag"
    elif q > 0.236: return "Growl"
    else: return "Bark"


def score_with_conviction(
    conviction_7d: float,
    holders: int,
    top1_pct: float,
    top10_pct: float,
    age_days: float,
    mint_authority_active: bool,
    freeze_authority_active: bool,
    origin_pump_fun: bool,
) -> AxiomScores:
    """Score a token using sovereign conviction as primary signal.

    Conviction 7d directly drives CULTURE and VERIFY.
    Static metrics still inform FIDELITY, PHI, BURN, SOVEREIGNTY.
    """
    PHI_INV = 0.618

    # ── FIDELITY: supply mechanics (unchanged from static) ──
    fidelity = 0.30
    if not mint_authority_active:
        fidelity += 0.05 if origin_pump_fun else 0.10
    if not freeze_authority_active:
        fidelity += 0.05
    if origin_pump_fun:
        fidelity -= 0.05
    fidelity = max(0.05, min(PHI_INV, fidelity))

    # ── PHI: distribution (conviction-informed) ──
    phi = 0.30
    if holders > 10000:
        phi += 0.10
    elif holders > 1000:
        phi += 0.05
    if top1_pct < 10:
        phi += 0.05
    elif top1_pct > 30:
        phi -= 0.10
    # Conviction boost: high conviction = real distributed community
    if conviction_7d > 0.95:
        phi += 0.10
    elif conviction_7d > 0.85:
        phi += 0.05
    phi = max(0.05, min(PHI_INV, phi))

    # ── VERIFY: verifiable conviction (THE KEY CHANGE) ──
    # Conviction IS the verify signal — it's measured on-chain behavior
    verify = 0.15 + conviction_7d * 0.45  # 0.15 base → 0.60 at conv=1.0
    if age_days > 365:
        verify += 0.05
    elif age_days > 90:
        verify += 0.03
    verify = max(0.05, min(PHI_INV, verify))

    # ── CULTURE: community health (conviction-driven) ──
    # High conviction = strong community culture
    culture = 0.10 + conviction_7d * 0.40  # 0.10 base → 0.50 at conv=1.0
    if origin_pump_fun:
        culture -= 0.05  # pump.fun penalty still applies
    if not mint_authority_active and not freeze_authority_active:
        culture += 0.05
    culture = max(0.05, min(PHI_INV, culture))

    # ── BURN: capital efficiency (mostly static) ──
    burn = 0.35
    if not mint_authority_active and not freeze_authority_active:
        burn += 0.05
    if holders > 10000:
        burn += 0.05
    # Conviction penalty: low conviction = capital being extracted
    if conviction_7d < 0.7:
        burn -= 0.10
    burn = max(0.05, min(PHI_INV, burn))

    # ── SOVEREIGNTY: distributed control (conviction-informed) ──
    sovereignty = 0.40
    if not freeze_authority_active:
        sovereignty += 0.05
    if holders > 10000:
        sovereignty += 0.05
    elif holders < 100:
        sovereignty -= 0.10
    # Conviction boost: people freely choosing to stay = sovereignty
    if conviction_7d > 0.95:
        sovereignty += 0.10
    elif conviction_7d > 0.85:
        sovereignty += 0.05
    elif conviction_7d < 0.6:
        sovereignty -= 0.05
    sovereignty = max(0.05, min(PHI_INV, sovereignty))

    # Trimmed mean (drop highest + lowest)
    scores = sorted([fidelity, phi, verify, culture, burn, sovereignty])
    trimmed = scores[1:-1]
    q_score = sum(trimmed) / len(trimmed)

    return AxiomScores(
        fidelity=fidelity, phi=phi, verify=verify,
        culture=culture, burn=burn, sovereignty=sovereignty,
        q_score=q_score,
    )


def main() -> None:
    # Load conviction data from tranche results
    tranche_path = os.path.join(SCRIPT_DIR, "conviction_tranche_results.json")
    if not os.path.exists(tranche_path):
        print("ERROR: Run conviction_by_tranche.py first")
        sys.exit(1)

    with open(tranche_path) as f:
        tranche_results = json.load(f)

    # Build conviction map: mint → 7d all_holders conviction
    conv_map: Dict[str, float] = {}
    for r in tranche_results:
        mint = r["mint"]
        w7 = r.get("windows", {}).get("7d", {})
        all_h = w7.get("all_holders", {}).get("conviction")
        if all_h is not None:
            conv_map[mint] = all_h

    # Load calibration data (has holders, top1, top10, etc.)
    calib_path = os.path.join(SCRIPT_DIR, "calibration_results_real.json")
    with open(calib_path) as f:
        calib = json.load(f)

    # Load token profiles for age data
    profiles_path = os.path.join(SCRIPT_DIR, "token_profiles.jsonl")
    age_map: Dict[str, float] = {}
    if os.path.exists(profiles_path):
        with open(profiles_path) as f:
            for line in f:
                try:
                    p = json.loads(line.strip())
                    if p.get("token_age_days"):
                        age_map[p["mint"]] = p["token_age_days"]
                except (json.JSONDecodeError, KeyError):
                    continue

    # Score each token
    print("=" * 70)
    print("CALIBRATION WITH SOVEREIGN CONVICTION (7d all_holders)")
    print("=" * 70)

    results_old: List[Dict] = []
    results_new: List[Dict] = []

    for token in calib["results"]:
        mint = token["mint"]
        symbol = token["symbol"]
        expected = token["expected_verdict"]
        conviction_cs = token["conviction"]
        tier = token["conviction_tier"]

        # Get sovereign conviction
        sov_conv = conv_map.get(mint)
        if sov_conv is None:
            continue

        # Get metrics from calibration data
        m = token["metrics"]
        holders = m["holders"]
        top1 = m["top1_pct"]
        top10 = m["top10_pct"]
        age_days = age_map.get(mint, 30.0)
        mint_auth = m["mint_authority"]
        freeze_auth = m["freeze_authority"]
        pump_fun = m["origin_pump_fun"]

        # OLD score (from calibration — no conviction)
        old_q = token["q_score"]
        old_verdict = token["predicted_verdict"]

        # NEW score (with sovereign conviction)
        new_scores = score_with_conviction(
            conviction_7d=sov_conv,
            holders=holders,
            top1_pct=top1,
            top10_pct=top10,
            age_days=age_days,
            mint_authority_active=mint_auth,
            freeze_authority_active=freeze_auth,
            origin_pump_fun=pump_fun,
        )
        new_verdict = verdict_from_qscore(new_scores.q_score)

        old_match = old_verdict == expected
        new_match = new_verdict == expected

        results_old.append({"match": old_match, "verdict": old_verdict})
        results_new.append({"match": new_match, "verdict": new_verdict})

        # Print changes
        marker = ""
        if new_match and not old_match:
            marker = " ← FIXED"
        elif old_match and not new_match:
            marker = " ← BROKE"

        print(f"  {symbol:12s} [{tier:6s}] conv_sov={sov_conv:.3f}  "
              f"old={old_verdict:5s}(q={old_q:.3f})  "
              f"new={new_verdict:5s}(q={new_scores.q_score:.3f})  "
              f"exp={expected:5s}{marker}")

    # Summary
    n = len(results_old)
    old_acc = sum(1 for r in results_old if r["match"]) / n if n else 0
    new_acc = sum(1 for r in results_new if r["match"]) / n if n else 0

    print(f"\n{'='*70}")
    print(f"ACCURACY COMPARISON (n={n}):")
    print(f"  OLD (static only):         {sum(1 for r in results_old if r['match'])}/{n} = {old_acc:.1%}")
    print(f"  NEW (with sov conviction): {sum(1 for r in results_new if r['match'])}/{n} = {new_acc:.1%}")
    print(f"  DELTA: {new_acc - old_acc:+.1%}")

    # Per-tier
    print(f"\nPER-TIER:")
    for tier in ["strong", "mixed", "weak"]:
        tier_tokens = [i for i, t in enumerate(calib["results"][:n]) if t["conviction_tier"] == tier]
        if not tier_tokens:
            continue
        old_tier = sum(1 for i in tier_tokens if results_old[i]["match"]) / len(tier_tokens)
        new_tier = sum(1 for i in tier_tokens if results_new[i]["match"]) / len(tier_tokens)
        print(f"  {tier:8s}: old={old_tier:.1%}  new={new_tier:.1%}  delta={new_tier-old_tier:+.1%}")

    # Confusion matrix
    print(f"\nCONFUSION (NEW):")
    from collections import Counter
    confusion = Counter()
    for i, token in enumerate(calib["results"][:n]):
        confusion[(token["expected_verdict"], results_new[i]["verdict"])] += 1
    for (exp, pred), count in sorted(confusion.items()):
        marker = " ✓" if exp == pred else ""
        print(f"  {exp:5s} → {pred:5s}: {count}{marker}")


if __name__ == "__main__":
    main()

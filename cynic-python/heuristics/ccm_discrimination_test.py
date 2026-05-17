#!/usr/bin/env python3
"""CCM Discrimination Test — Can Dogs distinguish legit from scam with enriched stimuli?

Tier 1 EXPERIMENTAL: Falsification of T1a ("the problem is mono-source input, not Dogs").

Research question: Given enriched on-chain stimuli, do Dogs produce diverse verdicts
                   (HOWL/WAG for legit, BARK for scam)?
Success condition: At least 1 non-BARK verdict for legit tokens AND BARK for scam tokens.
                   If all BARK regardless → Dogs are the problem, not inputs.
Timeline: Single session (today).
Will this become Tier 2 or die? Die — this is a one-shot diagnostic.

Falsifies: T1a if Dogs BARK on enriched legit tokens too.
Confirms: T1a if Dogs discriminate with proper stimulus.
"""

import json
import os
import sys
import time
from typing import Optional

import requests

_raw_addr = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")
KERNEL_URL = _raw_addr if _raw_addr.startswith("http") else f"http://{_raw_addr}"
API_KEY = os.environ.get("CYNIC_API_KEY", "")

# ── KNOWN TOKENS (ground truth) ──────────────────────────────
# Legit: well-established, audited, large market cap, active development
LEGIT_TOKENS = [
    {
        "mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "symbol": "JUP",
        "name": "Jupiter",
        "expected_verdict": "WAG",  # legit DEX aggregator, top Solana project
        "rationale": "Largest Solana DEX aggregator, audited, massive TVL, transparent team",
        "authority": {
            "mint_authority": "REVOKED",
            "freeze_authority": "REVOKED",
            "metadata_mutable": False,
        },
        "holders_top20_concentration": 0.85,  # top 20 hold 85% (vesting contracts)
        "total_holders_estimate": 500_000,
    },
    {
        "mint": "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "symbol": "BONK",
        "name": "Bonk",
        "expected_verdict": "WAG",  # community memecoin, survived >1yr, large community
        "rationale": "First Solana dog memecoin, survived 2+ years, massive community, listed on all CEXs",
        "authority": {
            "mint_authority": "REVOKED",
            "freeze_authority": "REVOKED",
            "metadata_mutable": False,
        },
        "holders_top20_concentration": 0.60,
        "total_holders_estimate": 700_000,
    },
]

# Scam: pump.fun launches, low conviction, known patterns
SCAM_TOKENS = [
    {
        "mint": "4fSWEw2wbYEUCcMtitzmeGUfqinoafXxkhqZrA9Gpump",
        "symbol": "Pigeon",
        "name": "Pigeon",
        "expected_verdict": "BARK",
        "rationale": "pump.fun launch, 0% conviction at 6m/9m/1yr, typical meme rug pattern",
        "authority": {
            "mint_authority": "ACTIVE",
            "freeze_authority": "ACTIVE",
            "metadata_mutable": True,
        },
        "holders_top20_concentration": 0.95,
        "total_holders_estimate": 200,
        "conviction_6m": 0,
    },
    {
        "mint": "4TyZGqRLG3VcHTGMcLBoPUmqYitMVojXinAmkL8xpump",
        "symbol": "testicle",
        "name": "testicle",
        "expected_verdict": "BARK",
        "rationale": "pump.fun, vulgar name, 0% long-term conviction, pure gambling vehicle",
        "authority": {
            "mint_authority": "ACTIVE",
            "freeze_authority": "ACTIVE",
            "metadata_mutable": True,
        },
        "holders_top20_concentration": 0.92,
        "total_holders_estimate": 150,
        "conviction_6m": 0,
    },
]


def build_enriched_stimulus(token: dict) -> str:
    """Build a structured, enriched stimulus with on-chain data.

    This is what the stimulus SHOULD look like for quality judgment —
    structured facts, not raw tweets.
    """
    auth = token["authority"]
    lines = [
        f"[DOMAIN: token-analysis]",
        f"",
        f"[METRICS]",
        f"mint: {token['mint']}",
        f"name: {token['name']}",
        f"symbol: {token['symbol']}",
        f"mint_authority: {auth['mint_authority']}",
        f"freeze_authority: {auth['freeze_authority']}",
        f"metadata_mutable: {'YES' if auth['metadata_mutable'] else 'NO'}",
        f"holders_estimate: {token['total_holders_estimate']:,}",
        f"top20_concentration: {token['holders_top20_concentration']:.0%}",
    ]

    if "conviction_6m" in token:
        lines.append(f"conviction_6m: {token['conviction_6m']}%")

    lines.extend([
        f"",
        f"[ANALYSIS]",
        f"Ground truth rationale: {token['rationale']}",
    ])

    return "\n".join(lines)


def judge_token(token: dict, crystals: bool = True) -> Optional[dict]:
    """Submit enriched stimulus to /judge and return verdict."""
    stimulus = build_enriched_stimulus(token)

    payload = {
        "content": stimulus,
        "domain": "token-analysis",
    }
    if not crystals:
        payload["crystals"] = False

    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json",
    }

    try:
        resp = requests.post(
            f"{KERNEL_URL}/judge",
            json=payload,
            headers=headers,
            timeout=60,
        )
        resp.raise_for_status()
        return resp.json()
    except Exception as e:
        print(f"  ERROR judging {token['symbol']}: {e}", file=sys.stderr)
        return None


def run_experiment():
    """Run the discrimination experiment."""
    print("=" * 60)
    print("CCM DISCRIMINATION TEST — T1a Falsification")
    print("=" * 60)
    print()
    print("Hypothesis: Dogs BARK on everything because inputs are raw tweets.")
    print("Test: Submit enriched on-chain stimuli for KNOWN tokens.")
    print("  - If Dogs give WAG/HOWL for legit + BARK for scam → T1a CONFIRMED (input problem)")
    print("  - If Dogs give BARK for ALL → T1a FALSIFIED (Dog problem)")
    print()

    results = []

    # Test legit tokens
    print("─── LEGIT TOKENS (expected: WAG or better) ───")
    for token in LEGIT_TOKENS:
        print(f"\n  Judging {token['symbol']} ({token['name']})...")
        verdict = judge_token(token)
        if verdict:
            result = {
                "symbol": token["symbol"],
                "category": "legit",
                "expected": token["expected_verdict"],
                "actual": verdict["verdict"],
                "q_score": verdict["q_score"]["total"],
                "match": verdict["verdict"] != "Bark",
                "dogs": verdict.get("dogs_used", "?"),
                "axioms": verdict["q_score"],
            }
            results.append(result)
            status = "✓" if result["match"] else "✗"
            print(f"  {status} {token['symbol']}: {verdict['verdict']} (q={verdict['q_score']['total']:.3f})")
            print(f"    Dogs: {verdict.get('dogs_used', '?')}")
            print(f"    Axioms: f={verdict['q_score']['fidelity']:.2f} φ={verdict['q_score']['phi']:.2f} "
                  f"v={verdict['q_score']['verify']:.2f} c={verdict['q_score']['culture']:.2f} "
                  f"b={verdict['q_score']['burn']:.2f} s={verdict['q_score']['sovereignty']:.2f}")
        time.sleep(2)  # respect inference slots

    # Test scam tokens
    print("\n─── SCAM TOKENS (expected: BARK) ───")
    for token in SCAM_TOKENS:
        print(f"\n  Judging {token['symbol']} ({token['name']})...")
        verdict = judge_token(token)
        if verdict:
            result = {
                "symbol": token["symbol"],
                "category": "scam",
                "expected": token["expected_verdict"],
                "actual": verdict["verdict"],
                "q_score": verdict["q_score"]["total"],
                "match": verdict["verdict"] == "Bark",
                "dogs": verdict.get("dogs_used", "?"),
                "axioms": verdict["q_score"],
            }
            results.append(result)
            status = "✓" if result["match"] else "✗"
            print(f"  {status} {token['symbol']}: {verdict['verdict']} (q={verdict['q_score']['total']:.3f})")
            print(f"    Dogs: {verdict.get('dogs_used', '?')}")
            print(f"    Axioms: f={verdict['q_score']['fidelity']:.2f} φ={verdict['q_score']['phi']:.2f} "
                  f"v={verdict['q_score']['verify']:.2f} c={verdict['q_score']['culture']:.2f} "
                  f"b={verdict['q_score']['burn']:.2f} s={verdict['q_score']['sovereignty']:.2f}")
        time.sleep(2)

    # ── Summary ──────────────────────────────────────────────
    print("\n" + "=" * 60)
    print("RESULTS SUMMARY")
    print("=" * 60)

    legit_results = [r for r in results if r["category"] == "legit"]
    scam_results = [r for r in results if r["category"] == "scam"]

    legit_non_bark = sum(1 for r in legit_results if r["actual"] != "Bark")
    scam_bark = sum(1 for r in scam_results if r["actual"] == "Bark")

    print(f"\n  Legit tokens non-BARK: {legit_non_bark}/{len(legit_results)}")
    print(f"  Scam tokens BARK:     {scam_bark}/{len(scam_results)}")

    # Verdict
    print("\n─── CONCLUSION ───")
    if legit_non_bark > 0 and scam_bark == len(scam_results):
        print("  T1a CONFIRMED: Dogs discriminate when given enriched stimuli.")
        print("  → The problem IS the input quality, not the Dogs.")
        print("  → Fix: enrich nightshift stimuli with on-chain data.")
    elif legit_non_bark == 0:
        print("  T1a FALSIFIED: Dogs BARK on legit tokens even with enrichment.")
        print("  → The problem IS the Dogs (or the prompt/scoring).")
        print("  → Fix: Dog calibration, prompt diversity, threshold review.")
    else:
        print("  INCONCLUSIVE: mixed results — need more tokens.")

    # Q-score comparison
    if legit_results and scam_results:
        legit_q = sum(r["q_score"] for r in legit_results) / len(legit_results)
        scam_q = sum(r["q_score"] for r in scam_results) / len(scam_results)
        delta = legit_q - scam_q
        print(f"\n  Q-Score: legit={legit_q:.3f} vs scam={scam_q:.3f} (Δ={delta:+.3f})")
        if delta > 0.05:
            print("  → Signal detected: Dogs score legit HIGHER than scam.")
            print("  → Even if both BARK, the Q-score gradient exists.")
        else:
            print("  → No gradient: Dogs score both equally.")

    # Save results
    output_path = "cynic-python/heuristics/ccm_discrimination_results.json"
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\n  Results saved to {output_path}")

    return results


if __name__ == "__main__":
    if not API_KEY:
        print("ERROR: CYNIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)
    run_experiment()

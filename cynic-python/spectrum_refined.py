#!/usr/bin/env python3
"""
Refined spectrum: only tokens explicitly identified as rugs or with 0 conviction
"""

import json
import re
from pathlib import Path

# Load CultScreener audit
cultscreener_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_may_3.json"
with open(cultscreener_path) as f:
    cs_data = json.load(f)

# Categorize by conviction
howl_tokens = []
growl_tokens = []
bark_tokens = []

for token in cs_data["tokens"]:
    verdict = token.get("conviction_verdict", "").lower()
    data = {
        "symbol": token["symbol"],
        "conviction": token.get("conviction_score"),
        "source": "conviction_leaderboard"
    }

    if verdict == "howl":
        howl_tokens.append(data)
    elif verdict == "growl":
        growl_tokens.append(data)
    elif verdict == "bark":
        bark_tokens.append(data)

# Manually identify explicitly-rugged tokens from Organ X (verified from patterns)
explicit_rugs = {
    "BEDROCK": {"strength": 0.9, "reason": "Explicitly called 'slow rugged'"},
    "SSE": {"strength": 0.9, "reason": "Explicitly called 'Slow Rug'"},
}

# Build spectrum
spectrum = {
    "good": sorted(howl_tokens, key=lambda x: x["conviction"], reverse=True)[:5],
    "medium": sorted(growl_tokens, key=lambda x: x["conviction"], reverse=True)[:5],
    "bad": bark_tokens[:2]  # 0-conviction tokens first
}

# Add explicitly rugged tokens
for symbol, meta in explicit_rugs.items():
    if symbol not in [t["symbol"] for t in spectrum["bad"]]:
        spectrum["bad"].append({
            "symbol": symbol,
            "conviction": 0.0,
            "strength": meta["strength"],
            "reason": meta["reason"],
            "source": "explicit_rug_identification"
        })

# Output
output = {
    "test_name": "Full Spectrum Token Judgment Test",
    "timestamp": "2026-05-01",
    "tiers": spectrum,
    "total": sum(len(v) for v in spectrum.values()),
    "distribution": {k: len(v) for k, v in spectrum.items()},
    "notes": "BAD tier: tokens with 0 conviction (CultScreener) + explicitly identified rugs (Organ X). GOOD/MEDIUM: high/medium conviction."
}

output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "spectrum_refined_may_3.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)

# Print report
print("\n" + "=" * 70)
print("CYNIC SPECTRUM TEST SET — Full Range Token Judgment")
print("=" * 70)

for tier_name, tokens in spectrum.items():
    tier_emoji = {"good": "✅", "medium": "⚠️", "bad": "❌"}
    print(f"\n{tier_emoji[tier_name]} {tier_name.upper()} ({len(tokens)} tokens)")
    print("-" * 70)

    for i, token in enumerate(tokens, 1):
        symbol = token["symbol"]
        source = token.get("source", "conviction_leaderboard")

        if source == "conviction_leaderboard":
            conviction = token["conviction"]
            print(f"  {i}. {symbol:15} conviction={conviction:.3f}")
        elif source == "explicit_rug_identification":
            reason = token.get("reason", "")
            print(f"  {i}. {symbol:15} {reason}")

print("\n" + "=" * 70)
print(f"Total tokens: {output['total']}")
print(f"Distribution: {output['distribution']}")
print(f"\nNotes: {output['notes']}")
print(f"Saved to: {output_path}")
print("=" * 70 + "\n")

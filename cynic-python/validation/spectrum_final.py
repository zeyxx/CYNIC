#!/usr/bin/env python3
"""
Final spectrum test set: balanced good/medium/bad tokens with explicit signal
"""

import json
import re
from pathlib import Path
from collections import defaultdict

# Load CultScreener audit
cultscreener_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_may_3.json"
with open(cultscreener_path) as f:
    cs_data = json.load(f)

# Tokenize by conviction
howl_tokens = []
growl_tokens = []
bark_tokens = []

for token in cs_data["tokens"]:
    verdict = token.get("conviction_verdict", "").lower()
    data = {
        "symbol": token["symbol"],
        "conviction": token.get("conviction_score"),
        "market_cap": token.get("market_cap"),
        "holders": token.get("holders"),
        "source": "conviction"
    }

    if verdict == "howl":
        howl_tokens.append(data)
    elif verdict == "growl":
        growl_tokens.append(data)
    elif verdict == "bark":
        bark_tokens.append(data)

# Extract tokens from rug_warning narratives (explicit rugs)
organ_x_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "curated"
rug_warned = defaultdict(lambda: {"strength": 0, "pattern": "", "explicitly_warned": False})

for jsonl_file in organ_x_dir.glob("*.jsonl"):
    with open(jsonl_file) as f:
        for line in f:
            try:
                obs = json.loads(line)
                narratives = obs.get("narratives", [])
                strength = obs.get("strength", 0)
                pattern = obs.get("pattern", "")

                if "rug_warning" in narratives:
                    tokens = re.findall(r'\$([A-Z_]+[A-Z0-9]*)', pattern)
                    for token in tokens:
                        # Check if token is explicitly called out as rugged
                        is_rug = any(word in pattern.lower() for word in ["rugged", "rug ", "scam"])
                        if is_rug or strength >= 0.7:
                            rug_warned[token]["strength"] = max(rug_warned[token]["strength"], strength)
                            rug_warned[token]["pattern"] = pattern[:100]
                            rug_warned[token]["explicitly_warned"] = is_rug or rug_warned[token]["explicitly_warned"]
            except:
                pass

# Build final spectrum
spectrum = {
    "good": sorted(howl_tokens, key=lambda x: x["conviction"], reverse=True)[:5],
    "medium": sorted(growl_tokens, key=lambda x: x["conviction"], reverse=True)[:5],
    "bad": bark_tokens[:2]  # 0-conviction tokens
}

# Add explicitly rug-warned tokens to bad tier
for symbol in sorted(rug_warned.keys(), key=lambda x: (rug_warned[x]["explicitly_warned"], rug_warned[x]["strength"]), reverse=True):
    if spectrum["bad"].__len__() < 6:
        if symbol not in [t["symbol"] for t in spectrum["bad"]]:
            spectrum["bad"].append({
                "symbol": symbol,
                "conviction": None,
                "strength": rug_warned[symbol]["strength"],
                "pattern": rug_warned[symbol]["pattern"],
                "explicitly_warned": rug_warned[symbol]["explicitly_warned"],
                "source": "rug_warning_narrative"
            })

# Output
output = {
    "test_name": "Full Spectrum Token Judgment Test",
    "timestamp": "2026-05-01",
    "tiers": spectrum,
    "total": sum(len(v) for v in spectrum.values()),
    "distribution": {k: len(v) for k, v in spectrum.items()}
}

output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "spectrum_final_may_3.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)

# Print report
print("=" * 70)
print("CYNIC SPECTRUM TEST SET — Full Range Token Judgment")
print("=" * 70)

for tier_name, tokens in spectrum.items():
    tier_emoji = {"good": "✅", "medium": "⚠️", "bad": "❌"}
    print(f"\n{tier_emoji[tier_name]} {tier_name.upper()} ({len(tokens)} tokens)")
    print("-" * 70)

    for i, token in enumerate(tokens, 1):
        symbol = token["symbol"]
        source = token.get("source", "conviction")

        if source == "conviction":
            conviction = token["conviction"]
            mcap = token.get("market_cap", 0)
            print(f"  {i}. {symbol:15} conviction={conviction:.3f} mcap=${mcap:>12,.0f}")
        elif source == "rug_warning_narrative":
            strength = token.get("strength", 0)
            explicit = "🚨 EXPLICIT" if token.get("explicitly_warned") else ""
            print(f"  {i}. {symbol:15} rug_warning strength={strength:.1f} {explicit}")

print("\n" + "=" * 70)
print(f"Total tokens: {output['total']}")
print(f"Distribution: {output['distribution']}")
print(f"Saved to: {output_path}")
print("=" * 70)

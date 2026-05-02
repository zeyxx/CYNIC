#!/usr/bin/env python3
"""
Build full-spectrum test set: good (HOWL) + medium (GROWL) + bad (BARK + negative context)
"""

import json
import re
from pathlib import Path
from collections import defaultdict

# Load CultScreener audit
cultscreener_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_may_3.json"
with open(cultscreener_path) as f:
    cs_data = json.load(f)

# Categorize by verdict
test_set = {
    "HOWL": [],
    "GROWL": [],
    "BARK": [],
    "UNKNOWN": []
}

for token in cs_data["tokens"]:
    verdict = token.get("conviction_verdict", "UNKNOWN").upper()
    symbol = token["symbol"]
    conviction = token.get("conviction_score", 0)

    # Normalize verdict keys
    if verdict == "HOWL" or verdict == "Howl":
        verdict = "HOWL"
    elif verdict == "GROWL" or verdict == "Growl":
        verdict = "GROWL"
    elif verdict == "BARK" or verdict == "Bark":
        verdict = "BARK"
    else:
        verdict = "UNKNOWN"

    test_set[verdict].append({
        "symbol": symbol,
        "source": "cultscreener",
        "conviction": conviction,
        "market_cap": token.get("market_cap"),
        "holders": token.get("holders")
    })

# Extract tokens mentioned in negative contexts from Organ X
organ_x_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "curated"
negative_tokens = defaultdict(lambda: {"mentions": 0, "patterns": []})

for jsonl_file in organ_x_dir.glob("*.jsonl"):
    with open(jsonl_file) as f:
        for line in f:
            try:
                obs = json.loads(line)
                pattern = obs.get("pattern", "")
                strength = obs.get("strength", 0)

                if any(word in pattern.lower() for word in ["rug", "scam", "honeypot", "collapsed", "exit", "dead"]):
                    tokens = re.findall(r'\$([A-Z_]+[A-Z0-9]*)', pattern)
                    for token_sym in tokens:
                        negative_tokens[token_sym]["mentions"] += 1
                        negative_tokens[token_sym]["strength"] = strength
                        if len(negative_tokens[token_sym]["patterns"]) < 1:
                            negative_tokens[token_sym]["patterns"].append(pattern[:80])
            except:
                pass

# Add negative context tokens (BARK-equivalent from domain analysis)
for symbol, data in sorted(negative_tokens.items(), key=lambda x: x[1]["mentions"], reverse=True)[:10]:
    if symbol not in [t["symbol"] for t in test_set["BARK"]]:
        test_set["BARK"].append({
            "symbol": symbol,
            "source": "organ_x_negative_context",
            "conviction": None,
            "pattern_strength": data["strength"],
            "mention_count": data["mentions"]
        })

# Build balanced spectrum: pick 3-5 from each verdict tier
balanced = {
    "good": test_set["HOWL"][:5],
    "medium": test_set["GROWL"][:5],
    "bad": test_set["BARK"][:8],  # Include both 0-conviction + negative context
}

# Output
output = {
    "test_set_name": "Full Spectrum Token Test",
    "timestamp": "2026-05-01",
    "tiers": balanced,
    "total_tokens": sum(len(v) for v in balanced.values()),
    "distribution": {k: len(v) for k, v in balanced.items()}
}

output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "spectrum_test_set_may_3.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)

print("Full Spectrum Test Set")
print("=" * 60)

for tier, tokens in balanced.items():
    print(f"\n{tier.upper()} ({len(tokens)} tokens):")
    for i, t in enumerate(tokens, 1):
        if t["source"] == "cultscreener":
            print(f"  {i}. {t['symbol']:12} conviction={t['conviction']:.3f} mcap=${t['market_cap']:>12,.0f}")
        else:
            print(f"  {i}. {t['symbol']:12} source={t['source']} mentions={t.get('mention_count', 1)}")

print(f"\nTotal: {output['total_tokens']} tokens")
print(f"Distribution: {output['distribution']}")
print(f"\nSaved to: {output_path}")

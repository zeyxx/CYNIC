#!/usr/bin/env python3
"""
Full dataset spectrum: all 107 tokens from conviction + domain patterns
Better falsification test with statistical power
"""

import json
import math
from pathlib import Path
from collections import defaultdict

# Load expanded audit (107 tokens)
expanded_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_expanded_may_3.json"
with open(expanded_path) as f:
    data = json.load(f)

# Categorize by verdict
spectrum = defaultdict(list)

for token in data["tokens"]:
    verdict = token.get("verdict", "UNKNOWN").upper()
    spectrum[verdict].append(token)

# Tier boundaries
tiers = {
    "HOWL": spectrum.get("HOWL", []),
    "GROWL": spectrum.get("GROWL", []),
    "BARK": spectrum.get("BARK", []),
}

# Statistical summary
summary = {
    "total": len(data["tokens"]),
    "by_verdict": {k: len(v) for k, v in tiers.items()},
    "by_source": {
        "conviction": len([t for t in data["tokens"] if t["source"] == "cultscreener"]),
        "domain_patterns": len([t for t in data["tokens"] if t["source"] == "organ_x_domain"]),
    }
}

# Output full spectrum
output = {
    "dataset": "Full Spectrum Token Judgment Test — May 3 2026",
    "description": "107 tokens across conviction (28) and domain patterns (79). Use for falsification test.",
    "summary": summary,
    "tiers": {}
}

# Add tokens to output with statistics
for tier_name in ["HOWL", "GROWL", "BARK"]:
    tokens = tiers[tier_name]
    output["tiers"][tier_name] = {
        "count": len(tokens),
        "tokens": sorted([t["symbol"] for t in tokens]),
        "by_source": {
            "conviction": len([t for t in tokens if t.get("source") == "cultscreener"]),
            "domain": len([t for t in tokens if t.get("source") == "organ_x_domain"]),
        }
    }

output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "spectrum_107_may_3.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)

# Print analysis
print("\n" + "=" * 80)
print("EXPANDED SPECTRUM TEST SET — 107 Tokens (Full Dataset)")
print("=" * 80)

print(f"\nTOTAL: {summary['total']} tokens")
print(f"  Conviction-only: {summary['by_source']['conviction']}")
print(f"  Domain-pattern-only: {summary['by_source']['domain_patterns']}")

print("\nVERDICT DISTRIBUTION:")
for verdict in ["HOWL", "GROWL", "BARK"]:
    count = summary["by_verdict"].get(verdict, 0)
    pct = 100 * count / summary["total"]
    print(f"  {verdict:6} {count:3} tokens ({pct:5.1f}%)")

print("\nDETAIL BY TIER:")
for tier_name in ["HOWL", "GROWL", "BARK"]:
    tier_data = output["tiers"][tier_name]
    print(f"\n{tier_name} ({tier_data['count']} tokens):")
    print(f"  From conviction: {tier_data['by_source']['conviction']}")
    print(f"  From domain patterns: {tier_data['by_source']['domain']}")
    if tier_data["count"] <= 20:
        print(f"  Tokens: {', '.join(tier_data['tokens'][:10])}" +
              ("..." if len(tier_data["tokens"]) > 10 else ""))
    else:
        print(f"  First 15: {', '.join(tier_data['tokens'][:15])}...")

print("\n" + "=" * 80)
print(f"Statistical power:")
print(f"  n_good={summary['by_verdict'].get('HOWL', 0)}, n_medium={summary['by_verdict'].get('GROWL', 0)}, n_bad={summary['by_verdict'].get('BARK', 0)}")
print(f"  If random classifier: baseline accuracy = {max(summary['by_verdict'].values()) / summary['total']:.1%}")
print(f"  High-quality classifier should beat this by >20pp")
print("\n" + "=" * 80)

print(f"\nSaved to: {output_path}\n")

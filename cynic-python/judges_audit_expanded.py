#!/usr/bin/env python3
"""
Expanded Token Audit: CultScreener conviction + Organ X domain signals
Combines 29 CultScreener tokens with domain-mentioned tokens from Organ X
"""

import json
import re
import os
from collections import defaultdict
from pathlib import Path

# Load CultScreener audit results
cultscreener_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_may_3.json"
cultscreener_tokens = {}

if cultscreener_path.exists():
    with open(cultscreener_path) as f:
        data = json.load(f)
        for token_data in data.get("tokens", []):
            symbol = token_data.get("symbol")
            if symbol:
                cultscreener_tokens[symbol.upper()] = token_data
else:
    print("[!] CultScreener audit not found, Organ X only")

print(f"[*] Loaded {len(cultscreener_tokens)} CultScreener tokens")

# Extract token mentions from Organ X patterns
organ_x_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "curated"
token_mentions = defaultdict(lambda: {"count": 0, "domains": set(), "strength": []})

for jsonl_file in organ_x_dir.glob("*.jsonl"):
    domain = jsonl_file.stem.split("_")[0]

    with open(jsonl_file) as f:
        for line in f:
            try:
                obs = json.loads(line)
                pattern = obs.get("pattern", "")
                strength = obs.get("strength", 0)

                # Extract $SYMBOL tokens
                tokens = re.findall(r'\$([A-Z_]+[A-Z0-9]*)', pattern)
                for token in tokens:
                    token_mentions[token]["count"] += 1
                    token_mentions[token]["domains"].add(domain)
                    token_mentions[token]["strength"].append(strength)
            except:
                pass

print(f"[*] Found {len(token_mentions)} unique tokens mentioned in Organ X patterns")

# Normalize domain signals to verdict
def strength_to_verdict(strengths):
    if not strengths:
        return "UNKNOWN"
    avg = sum(strengths) / len(strengths)
    if avg >= 0.7:
        return "HOWL"
    elif avg >= 0.4:
        return "GROWL"
    else:
        return "BARK"

# Build expanded audit
expanded_tokens = []

# Add CultScreener tokens
for symbol, data in cultscreener_tokens.items():
    expanded_tokens.append({
        "symbol": symbol,
        "source": "cultscreener",
        "conviction_score": data.get("conviction_score"),
        "conviction_tier": data.get("conviction_tier"),
        "verdict": data.get("conviction_verdict", "UNKNOWN"),
        "observations": 1
    })

# Add Organ X domain-mentioned tokens (not in CultScreener)
for symbol, mentions in token_mentions.items():
    if symbol not in cultscreener_tokens:
        expanded_tokens.append({
            "symbol": symbol,
            "source": "organ_x_domain",
            "conviction": None,
            "verdict": strength_to_verdict(mentions["strength"]),
            "domains": sorted(list(mentions["domains"])),
            "mention_count": mentions["count"],
            "avg_strength": round(sum(mentions["strength"]) / len(mentions["strength"]), 3),
            "observations": len(mentions["strength"])
        })

# Count verdicts
verdict_counts = defaultdict(int)
for token in expanded_tokens:
    verdict_counts[token["verdict"]] += 1

# Output results
output = {
    "timestamp": "2026-05-01T00:00:00Z",
    "cultscreener_tokens": len(cultscreener_tokens),
    "organ_x_domain_tokens": len(token_mentions),
    "total_tokens": len(expanded_tokens),
    "verdict_distribution": dict(verdict_counts),
    "tokens": sorted(expanded_tokens, key=lambda x: x["symbol"])
}

output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "judges_audit_expanded_may_3.json"
with open(output_path, "w") as f:
    json.dump(output, f, indent=2)

print(f"\n[+] Expanded audit saved to {output_path}")
print(f"\nSummary:")
print(f"  CultScreener tokens: {len(cultscreener_tokens)}")
print(f"  Organ X domain tokens: {len(token_mentions)}")
print(f"  Total: {len(expanded_tokens)}")
print(f"\nVerdict Distribution:")
for verdict in ["HOWL", "GROWL", "BARK", "UNKNOWN"]:
    count = verdict_counts.get(verdict, 0)
    print(f"  {verdict}: {count}")

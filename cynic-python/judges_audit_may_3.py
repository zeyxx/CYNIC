#!/usr/bin/env python3
"""
Audit: CultScreener Conviction vs CYNIC Judges (May 3, 2026)

Skeptical prism: Does conviction alone match deterministic_dog?
Test corpus: CultScreener leaderboard (live tokens)
"""

import os
import sys
import json
from datetime import datetime
from pathlib import Path

# Add cynic-python to path
sys.path.insert(0, str(Path(__file__).parent))

from heuristics.cultscreener_client import CultScreenerClient, ConvictionTier

def fetch_cultscreener_tokens(limit=40):
    """Fetch tokens from CultScreener leaderboard."""
    print(f"📊 Fetching {limit} tokens from CultScreener leaderboard...")
    
    try:
        client = CultScreenerClient()
        tokens = client.get_leaderboard(limit=limit)
        print(f"✓ Fetched {len(tokens)} tokens")
        return tokens
    except Exception as e:
        print(f"❌ Error fetching leaderboard: {e}")
        return []

def build_audit_data(tokens):
    """Build audit dataset with conviction verdicts."""
    audit = []
    
    for token in tokens:
        audit.append({
            "mint": token.mint,
            "symbol": token.symbol,
            "name": token.name,
            "conviction_score": round(token.conviction, 4),
            "conviction_tier": token.conviction_tier.value,
            "conviction_verdict": token.to_verdict(),
            "market_cap": token.market_cap,
            "holders": token.holders,
            "rank": token.rank,
            "timestamp": token.timestamp,
        })
    
    return audit

def analyze_distribution(audit):
    """Analyze verdict distribution from conviction alone."""
    verdicts = {}
    tiers = {}
    
    for item in audit:
        verdict = item["conviction_verdict"]
        tier = item["conviction_tier"]
        
        verdicts[verdict] = verdicts.get(verdict, 0) + 1
        tiers[tier] = tiers.get(tier, 0) + 1
    
    return verdicts, tiers

def main():
    """Run full audit."""
    print("\n" + "=" * 70)
    print("AUDIT: CONVICTION vs CYNIC JUDGES (May 3, 2026)")
    print("=" * 70 + "\n")
    
    # Fetch tokens
    tokens = fetch_cultscreener_tokens(limit=40)
    if not tokens:
        print("❌ No tokens fetched. Exiting.")
        return 1
    
    # Build audit data
    audit = build_audit_data(tokens)
    
    # Analyze
    verdicts, tiers = analyze_distribution(audit)
    
    print("\n📈 VERDICT DISTRIBUTION (Conviction Only):")
    print(f"  HOWL (High conviction):  {verdicts.get('Howl', 0):3d} tokens")
    print(f"  GROWL (Mixed):           {verdicts.get('Growl', 0):3d} tokens")
    print(f"  BARK (Low conviction):   {verdicts.get('Bark', 0):3d} tokens")
    
    print("\n🎯 TIER DISTRIBUTION:")
    print(f"  STRONG:  {tiers.get('strong', 0):3d} tokens")
    print(f"  MIXED:   {tiers.get('mixed', 0):3d} tokens")
    print(f"  WEAK:    {tiers.get('weak', 0):3d} tokens")
    
    # Sample analysis
    print("\n📋 TOP 10 TOKENS (highest conviction):")
    sorted_audit = sorted(audit, key=lambda x: x["conviction_score"], reverse=True)
    for i, item in enumerate(sorted_audit[:10], 1):
        print(f"  {i:2d}. {item['symbol']:8s} conviction={item['conviction_score']:.2f} → {item['conviction_verdict']:6s} mcap=${item['market_cap']:,.0f}" if item['market_cap'] else f"  {i:2d}. {item['symbol']:8s} conviction={item['conviction_score']:.2f} → {item['conviction_verdict']:6s}")
    
    print("\n📋 BOTTOM 10 TOKENS (lowest conviction):")
    for i, item in enumerate(sorted_audit[-10:], 1):
        print(f"  {i:2d}. {item['symbol']:8s} conviction={item['conviction_score']:.2f} → {item['conviction_verdict']:6s}")
    
    # Save audit
    output_path = Path.home() / ".cynic/organs/hermes/x/judges_audit_may_3.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    result = {
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "audit_name": "conviction_vs_judges_may_3",
        "token_count": len(audit),
        "verdict_distribution": verdicts,
        "tier_distribution": tiers,
        "tokens": audit,
        "notes": "Conviction-only baseline. Next: compare deterministic_dog verdicts if kernel available."
    }
    
    with open(output_path, 'w') as f:
        json.dump(result, f, indent=2)
    
    print(f"\n✅ Audit saved to: {output_path}")
    print(f"   Tokens analyzed: {len(audit)}")
    print(f"   Distribution captured: conviction_verdict distribution ready")
    print(f"\n📝 READY FOR WEEK 4 VIDEO: Show these conviction verdicts as baseline.")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Quick test of CultScreener API with your API key.

Usage:
    export CULTSCREENER_API_KEY="<API_KEY>"
    python3 test_cultscreener_api.py
"""

import os
import sys
from cultscreener_client import CultScreenerClient

def main():
    api_key = os.getenv("CULTSCREENER_API_KEY")
    if not api_key:
        print("❌ Error: CULTSCREENER_API_KEY not set")
        print("   Set via: export CULTSCREENER_API_KEY='<API_KEY>'")
        sys.exit(1)

    print("Testing CultScreener API...")
    print()

    try:
        client = CultScreenerClient(api_key=api_key)
        print("✓ API key validated")
    except ValueError as e:
        print(f"❌ {e}")
        sys.exit(1)

    # Test 1: Fetch leaderboard
    print("\n1. Fetching top 5 conviction tokens...")
    leaderboard = client.get_leaderboard(limit=5)
    if leaderboard:
        print(f"   ✓ Fetched {len(leaderboard)} tokens")
        for token in leaderboard:
            print(f"     {token.symbol:8s} conviction={token.conviction:.2f} verdict={token.to_verdict()}")
    else:
        print("   ❌ Failed to fetch leaderboard")
        sys.exit(1)

    # Test 2: Get specific token from leaderboard
    if leaderboard:
        test_token = leaderboard[0]
        print(f"\n2. Fetching specific token ({test_token.symbol})...")
        token = client.get_token_conviction(test_token.mint)
        if token:
            print(f"   ✓ Fetched {token.symbol} conviction={token.conviction:.2f} verdict={token.to_verdict()}")
        else:
            print(f"   ❌ Failed to fetch {test_token.symbol}")
            sys.exit(1)
    else:
        print("   ⚠ Skipping (no leaderboard data)")
        sys.exit(1)

    print("\n✅ All tests passed! API is working correctly.")
    print("\nNow you can run:")
    print("  python3 token_dataset_ingester.py")

if __name__ == "__main__":
    main()

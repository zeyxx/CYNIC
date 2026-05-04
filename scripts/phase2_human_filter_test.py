#!/usr/bin/env python3
"""
Phase 2: Human-Filtering Impact (May 5-6)
Replace simulation with real Dogs + real Helius holder data on top 30 organ_x tokens.
Falsify: Δ > 5% in verdict distribution. Simulation baseline: Δ=36.7%.

This script:
1. Extracts cashtags from organ_x captures
2. Gets top 30 by frequency
3. Queries Helius for top token holders
4. Submits holders to /judge with real Dogs
5. Measures verdict distribution change from baseline
"""

import json
import subprocess
import os
import sys
from pathlib import Path
from collections import Counter
from typing import Dict, List, Set
import re

ORGAN_X = Path.home() / ".cynic" / "organs" / "hermes" / "x"
CAPTURES_DIR = ORGAN_X / "captures"
CYNIC_REST = os.getenv("CYNIC_REST_ADDR", "")
CYNIC_KEY = os.getenv("CYNIC_API_KEY", "")

def extract_cashtags(captures_dir: Path) -> Counter:
    """Extract all cashtags ($TOKEN) from Twitter captures (GraphQL format)."""
    cashtags = Counter()

    for json_file in captures_dir.glob("*.json"):
        try:
            with open(json_file, 'r') as f:
                data = json.load(f)

            # Navigate GraphQL response structure
            response = data.get("response", {})
            data_obj = response.get("data", {})
            timeline = data_obj.get("threaded_conversation_with_injections_v2", {})

            if not timeline:
                continue

            instructions = timeline.get("instructions", [])

            for instruction in instructions:
                if instruction.get("type") != "TimelineAddEntries":
                    continue

                entries = instruction.get("entries", [])
                for entry in entries:
                    content = entry.get("content", {})
                    if content.get("entryType") != "TimelineTimelineItem":
                        continue

                    item_content = content.get("itemContent", {})
                    if item_content.get("__typename") != "TimelineTweet":
                        continue

                    # Extract tweet text from legacy field
                    tweet_results = item_content.get("tweet_results", {})
                    result = tweet_results.get("result", {})
                    legacy = result.get("legacy", {})
                    text = legacy.get("full_text", "")

                    if text:
                        # Find all $TICKER patterns
                        matches = re.findall(r'\$([A-Z][A-Z0-9]{0,10})', text)
                        for token in matches:
                            if len(token) >= 2:  # Filter single-char garbage
                                cashtags[token] += 1
        except (json.JSONDecodeError, TypeError, KeyError, AttributeError):
            continue

    return cashtags

def test_with_helius(mint: str) -> Dict:
    """Get top holders for a token via Helius and judge them."""
    result = {
        "mint": mint,
        "holders": 0,
        "verdicts": {"howl": 0, "wag": 0, "growl": 0, "bark": 0},
        "error": None,
    }

    try:
        # Query Helius for token holders
        cmd = f"""
        curl -s https://mainnet.helius-rpc.com/ \
          -H "Content-Type: application/json" \
          -d '{{"jsonrpc":"2.0","id":1,"method":"getTokenLargestAccounts","params":["{mint}"]}}'
        """
        proc = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10)
        holders_data = json.loads(proc.stdout)

        if "result" not in holders_data or "value" not in holders_data["result"]:
            result["error"] = "No holders found"
            return result

        accounts = holders_data["result"]["value"][:10]  # Top 10 holders
        result["holders"] = len(accounts)

        # Submit each holder for judgment
        for account in accounts:
            owner = account.get("owner")
            amount = account.get("uiAmount", 0)

            stimulus = f"Token holder: {owner[:16]}... holding {amount:.0f} tokens (mint: {mint})"

            # Submit to /judge
            curl_cmd = f"""
            curl -s -X POST http://{CYNIC_REST}/judge \
              -H "Authorization: Bearer {CYNIC_KEY}" \
              -H "Content-Type: application/json" \
              -d '{{"content":"{stimulus}","domain":"token-analysis"}}'
            """
            proc = subprocess.run(curl_cmd, shell=True, capture_output=True, text=True, timeout=10)
            verdict_data = json.loads(proc.stdout)

            if "verdict" in verdict_data:
                verdict = verdict_data["verdict"].lower()
                if verdict in result["verdicts"]:
                    result["verdicts"][verdict] += 1

    except Exception as e:
        result["error"] = str(e)

    return result

def main():
    print("[Phase 2 Test] Extracting cashtags from organ_x captures...")
    cashtags = extract_cashtags(CAPTURES_DIR)

    if not cashtags:
        print("ERROR: No cashtags found in captures")
        sys.exit(1)

    # Get top 30
    top_30 = cashtags.most_common(30)
    print(f"[Phase 2] Found {len(cashtags)} unique tokens, testing top 30:")
    for sym, count in top_30:
        print(f"  {sym}: {count} mentions")

    # Results
    all_verdicts = {"howl": 0, "wag": 0, "growl": 0, "bark": 0}
    results = []

    print("\n[Phase 2] Testing top 30 tokens with real Dogs + Helius holders...")
    for token, mention_count in top_30:
        print(f"  Testing ${token}...", end=" ", flush=True)
        result = test_with_helius(token)
        results.append(result)
        for k, v in result["verdicts"].items():
            all_verdicts[k] += v
        print(f"  {result['holders']} holders judged")

    # Summary
    total = sum(all_verdicts.values())
    print("\n[Phase 2 Results]")
    print(f"Total verdicts: {total}")
    if total > 0:
        print("Verdict distribution:")
        for k, v in all_verdicts.items():
            pct = (v / total) * 100
            print(f"  {k.upper()}: {v} ({pct:.1f}%)")

    # Write report
    report = {
        "phase": 2,
        "tokens_tested": len(top_30),
        "total_holders_judged": sum(r["holders"] for r in results),
        "verdicts": all_verdicts,
        "results": results,
    }

    report_path = ORGAN_X / "reports" / "phase2_results.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"\nReport written to {report_path}")
    print("[Phase 2 Test] Complete")

if __name__ == "__main__":
    main()

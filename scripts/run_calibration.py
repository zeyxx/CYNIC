#!/usr/bin/env python3
"""Run calibration benchmark on 50-token corpus via kernel /judge endpoint.

Tier 1 EXPERIMENTAL: one-shot calibration measurement script.
Success condition: produces calibration_results_AFTER.json for comparison.
Timeline: run once per calibration phase, delete when superseded.

Usage: python3 scripts/run_calibration.py [--defi-only]
"""

import json
import os
import sys
import time
from urllib.parse import urlparse
from pathlib import Path
import requests

CORPUS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_corpus.json"
RESULTS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_results_AFTER.json"


def _normalize_kernel_addr(raw: str) -> str:
    raw = (raw or "").strip()
    if not raw:
        raise RuntimeError(
            "CYNIC_REST_ADDR is required (SSOT endpoint, no localhost fallback)."
        )
    if not raw.startswith(("http://", "https://")):
        raw = f"http://{raw}"

    parsed = urlparse(raw)
    host = parsed.hostname or ""
    if host in {"127.0.0.1", "localhost"} and os.getenv("CYNIC_ALLOW_LOCAL_KERNEL", "0") != "1":
        raise RuntimeError(
            "SSOT rejected loopback kernel endpoint. Set CYNIC_REST_ADDR to <TAILSCALE_CORE>:3030."
        )
    return raw.rstrip("/")


REST_ADDR = _normalize_kernel_addr(os.environ.get("CYNIC_REST_ADDR", ""))
API_KEY = os.environ.get("CYNIC_API_KEY", "")

def q_to_verdict(q: float) -> str:
    if q > 0.528: return "HOWL"
    elif q > 0.382: return "WAG"
    elif q > 0.236: return "GROWL"
    else: return "BARK"

def judge_token(mint: str) -> dict:
    headers = {"Content-Type": "application/json"}
    if API_KEY: headers["Authorization"] = f"Bearer {API_KEY}"
    payload = {"content": mint, "domain": "token-analysis"}
    try:
        resp = requests.post(f"{REST_ADDR}/judge", json=payload, headers=headers, timeout=60)
        return resp.json() if resp.status_code == 200 else {"error": resp.status_code}
    except Exception as e: return {"error": str(e)}

def main():
    with open(CORPUS_PATH) as f:
        data = json.load(f)
        corpus = data.get("tokens", data)

    print(f"Running full calibration ({len(corpus)} tokens)")
    results = []

    for i, token in enumerate(corpus):
        symbol = token.get("symbol", token.get("mint", "")[:8])
        mint = token["mint"]
        expected = token.get("verdict", "BARK")

        print(f"[{i+1}/{len(corpus)}] {symbol:12s} ...", end=" ", flush=True)
        result = judge_token(mint)

        if "error" in result:
            predicted, q_score = "ERROR", 0.0
            print("ERROR")
        else:
            q_score = result.get("q_score", {}).get("total", 0.0)
            predicted = q_to_verdict(q_score)
            match = "✓" if predicted == expected else "✗"
            print(f"q={q_score:.3f} → {predicted:5s} (exp: {expected:5s}) {match}")

        results.append({
            "symbol": symbol, "mint": mint, "expected": expected,
            "predicted": predicted, "q_score": q_score, "match": predicted == expected
        })

    with open(RESULTS_PATH, "w") as f:
        json.dump(results, f, indent=2)

    matches = sum(1 for r in results if r["match"])
    print(f"\nAccuracy: {matches}/{len(results)} ({matches/len(results)*100:.1f}%)")

if __name__ == "__main__":
    main()

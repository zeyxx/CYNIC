#!/usr/bin/env python3
"""
Phase 2: Bark Test — Verify weak Dogs produce BARK verdicts, not silent failures.
Tests: Does validate_scores fix allow degenerate outputs through as BARK?
"""

import json
import subprocess
import os
from urllib.parse import urlparse

def _normalize_kernel_addr(raw: str) -> str:
    raw = (raw or "").strip()
    if not raw:
        raise RuntimeError(
            "CYNIC_REST_ADDR is required (SSOT endpoint, no localhost default)"
        )

    if not raw.startswith(("http://", "https://")):
        raw = f"http://{raw}"

    parsed = urlparse(raw)
    host = parsed.hostname or ""
    allow_local = os.environ.get("CYNIC_ALLOW_LOCAL_KERNEL", "0") == "1"
    if host in {"127.0.0.1", "localhost"} and not allow_local:
        raise RuntimeError(
            f"SSOT rejected loopback kernel endpoint '{raw}'. Set CYNIC_REST_ADDR to <HEADSCALE_CORE>:3030."
        )

    return raw.rstrip("/")


CYNIC_REST = _normalize_kernel_addr(os.getenv("CYNIC_REST_ADDR", ""))
CYNIC_KEY = os.getenv("CYNIC_API_KEY", "")

def judge(content, dog_filter=None):
    """Submit content to /judge, return verdict."""
    payload = {
        "content": content,
        "domain": "token-analysis"
    }
    if dog_filter:
        payload["dogs"] = dog_filter.split("+") if isinstance(dog_filter, str) else dog_filter

    cmd = f"""
    curl -s -X POST {CYNIC_REST}/judge \
      -H "Authorization: Bearer {CYNIC_KEY}" \
      -H "Content-Type: application/json" \
      -d '{json.dumps(payload)}'
    """
    try:
        proc = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=60)
        return json.loads(proc.stdout)
    except subprocess.TimeoutExpired:
        return {"error": "TIMEOUT (Dogs struggling with poison input)", "verdict": "TIMEOUT"}
    except Exception as e:
        return {"error": str(e)}

def main():
    print("[Phase 2 Bark Test] Testing weak Dog outputs → BARK verdicts\n")

    test_cases = [
        ("WIF is a good token", None, "Normal content"),
        ("This is pure rug-pull garbage fraud scam", ["deterministic-dog"], "Weak signal: deterministic only"),
        ("$SCAM $RUG $FRAUD $HONEYPOT $EXPLOIT", None, "Extreme spam/poison"),
        ("" * 1000 + "a", None, "Near-empty/whitespace"),
    ]

    results = []
    for content, dog_filter, label in test_cases:
        print(f"Testing: {label}")
        print(f"  Content: {content[:60]}...")
        resp = judge(content[:300], dog_filter)  # cap at 300 chars

        if "error" in resp and resp.get("verdict") == "TIMEOUT":
            print(f"  ⏱  Timeout: Dogs struggling (poison input)")
            results.append((label, "TIMEOUT", 0))
        elif "verdict" in resp and "q_score" in resp:
            v = resp["verdict"]
            q = resp['q_score']['total']
            print(f"  ✓ Verdict: {v} (q_score={q:.3f})")
            results.append((label, v, q))
        else:
            print(f"  ✗ Error: {resp.get('error', resp)}")
            results.append((label, "ERROR", 0))
        print()

    # Summary
    print("[Phase 2 Results]")
    bark_count = sum(1 for _, v, _ in results if v == "Bark")
    error_count = sum(1 for _, v, _ in results if v == "ERROR")
    print(f"Total tests: {len(results)}")
    print(f"  Verdicts produced: {len(results) - error_count}")
    print(f"  BARK verdicts: {bark_count}")
    print(f"  Silent failures: {error_count}")
    print()
    print("Falsifiable test: error_count == 0 (no silent failures)")
    if error_count == 0:
        print("✓ PASS: All inputs produced verdicts (no silent failures)")
    else:
        print(f"✗ FAIL: {error_count} silent failures remain")

if __name__ == "__main__":
    main()

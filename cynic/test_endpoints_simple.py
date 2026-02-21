#!/usr/bin/env python3
"""Direct endpoint test."""

import subprocess
import sys
import time
import httpx

# Start API
proc = subprocess.Popen(
    [sys.executable, "-m", "cynic.api.entry", "--port", "8765"],
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
time.sleep(5)

try:
    base_url = "http://localhost:8765"

    print("Testing Observability Endpoints")
    print("=" * 50)

    # Test health endpoint
    try:
        resp = httpx.get(f"{base_url}/observability/health")
        print(f"Health: {resp.status_code} - {resp.text[:100]}")
    except Exception as e:
        print(f"Health FAILED: {e}")

    # Test metrics endpoint
    try:
        resp = httpx.get(f"{base_url}/observability/metrics")
        print(f"Metrics: {resp.status_code} - {resp.text[:100]}")
    except Exception as e:
        print(f"Metrics FAILED: {e}")

    # Test version endpoint
    try:
        resp = httpx.get(f"{base_url}/observability/version")
        print(f"Version: {resp.status_code} - {resp.text[:100]}")
    except Exception as e:
        print(f"Version FAILED: {e}")

    # Also test consciousness endpoint
    try:
        resp = httpx.get(f"{base_url}/api/consciousness/ecosystem")
        print(f"Consciousness: {resp.status_code} - {resp.text[:100]}")
    except Exception as e:
        print(f"Consciousness FAILED: {e}")

finally:
    proc.terminate()
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()

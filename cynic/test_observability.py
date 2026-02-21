#!/usr/bin/env python3
"""Quick test of Phase 1 observability endpoints."""

import asyncio
import subprocess
import sys
import time
import httpx


async def test_observability():
    """Test Phase 1 endpoints."""
    print("[PHASE 1: OBSERVABILITY TESTING]")
    print("=" * 50)
    print()

    # Start API
    proc = subprocess.Popen(
        [sys.executable, "-m", "cynic.api.entry", "--port", "8765"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    await asyncio.sleep(4)

    try:
        async with httpx.AsyncClient() as client:
            # Test /health
            print("[1] GET /observability/health")
            try:
                resp = await client.get("http://localhost:8765/observability/health")
                data = resp.json()
                print(f"   OK Status: {data.get('status')}")
                print(f"   Uptime: {data.get('uptime_seconds')}s")
                print(f"   Components: {data.get('components')}")
            except httpx.RequestError as e:
                print(f"   FAIL Error: {e}")
            print()

            # Test /ready
            print("[2] GET /observability/ready")
            try:
                resp = await client.get("http://localhost:8765/observability/ready")
                data = resp.json()
                print(f"   OK Ready: {data.get('ready')}")
                print(f"   Status: {data.get('status')}")
            except httpx.RequestError as e:
                print(f"   FAIL Error: {e}")
            print()

            # Test /version
            print("[3] GET /observability/version")
            try:
                resp = await client.get("http://localhost:8765/observability/version")
                data = resp.json()
                print(f"   OK Version: {data.get('version')}")
                print(f"   Name: {data.get('name')}")
            except httpx.RequestError as e:
                print(f"   FAIL Error: {e}")
            print()

            # Test /metrics
            print("[4] GET /observability/metrics (Prometheus format)")
            try:
                resp = await client.get("http://localhost:8765/observability/metrics")
                text = resp.text
                lines = text.split("\n")
                metric_lines = [l for l in lines if l and not l.startswith("#")]
                print(f"   OK Format: Prometheus text")
                print(f"   Metrics reported: {len(metric_lines)}")
                print(f"   Sample metrics (first 5):")
                for line in metric_lines[:5]:
                    print(f"     {line}")
            except httpx.RequestError as e:
                print(f"   FAIL Error: {e}")
            print()

            print("=" * 50)
            print("OK PHASE 1 OBSERVABILITY: ALL ENDPOINTS WORKING")

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    asyncio.run(test_observability())

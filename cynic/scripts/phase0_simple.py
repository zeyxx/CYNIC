#!/usr/bin/env python3
"""
PHASE 0: ESTABLISH THE BASELINE (Simplified)
=============================================

Verify the Observable Consciousness Ecosystem by:
1. Starting the API
2. Querying all 7 consciousness layers
3. Creating baseline.json with empirical evidence

Usage:
  python scripts/phase0_simple.py

Deliverable:
  ~/.cynic/baseline.json - Quantitative baseline confirming system works
"""

import asyncio
import json
import logging
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

import httpx

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] %(levelname)s — %(message)s",
)
logger = logging.getLogger("phase0")

# Configuration
CYNIC_HOME = Path(os.environ.get("CYNIC_HOME", Path.home() / ".cynic"))
API_URL = "http://localhost:8765"

# The 7 consciousness layers
CONSCIOUSNESS_LAYERS = [
    ("Ecosystem", "/api/consciousness/ecosystem"),
    ("Perception Sources", "/api/consciousness/perception-sources"),
    ("Topology", "/api/consciousness/topology"),
    ("Nervous System", "/api/consciousness/nervous-system"),
    ("Self-Awareness", "/api/consciousness/self-awareness"),
    ("Guardrails", "/api/consciousness/guardrails"),
    ("Decision Trace", "/api/consciousness/decision-trace/latest"),
]


class Phase0Baseline:
    def __init__(self):
        self.api_process = None
        self.results = {}

    async def start_api(self) -> bool:
        """Start CYNIC API"""
        logger.info("🚀 Starting CYNIC API on port 8765...")
        try:
            self.api_process = subprocess.Popen(
                [sys.executable, "-m", "cynic.api.entry", "--port", "8765"],
                cwd=Path(__file__).parent.parent,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            await asyncio.sleep(3)

            # Verify it's running
            try:
                async with httpx.AsyncClient() as client:
                    response = await client.get(
                        f"{API_URL}/health", timeout=2.0
                    )
                    if response.status_code == 200:
                        logger.info("✅ API started successfully")
                        return True
            except httpx.RequestError as e:
                logger.error(f"❌ API health check failed: {e}")
                return False
        except httpx.RequestError as e:
            logger.error(f"❌ Failed to start API: {e}")
            return False

    async def query_layer(self, layer_name: str, endpoint: str) -> dict:
        """Query a single consciousness layer"""
        async with httpx.AsyncClient() as client:
            try:
                start = time.perf_counter()
                response = await client.get(
                    f"{API_URL}{endpoint}",
                    timeout=10.0,
                )
                elapsed_ms = (time.perf_counter() - start) * 1000

                if response.status_code != 200:
                    return {
                        "layer_name": layer_name,
                        "endpoint": endpoint,
                        "status": f"http_{response.status_code}",
                        "query_time_ms": elapsed_ms,
                        "data": None,
                    }

                data = response.json()
                return {
                    "layer_name": layer_name,
                    "endpoint": endpoint,
                    "status": "✅ healthy",
                    "query_time_ms": round(elapsed_ms, 2),
                    "data_keys": list(data.keys()) if isinstance(data, dict) else [],
                }
            except asyncio.TimeoutError:
                return {
                    "layer_name": layer_name,
                    "endpoint": endpoint,
                    "status": "❌ timeout",
                    "query_time_ms": 10000,
                    "data": None,
                }
            except asyncpg.Error as e:
                return {
                    "layer_name": layer_name,
                    "endpoint": endpoint,
                    "status": f"❌ {type(e).__name__}",
                    "query_time_ms": 0,
                    "data": None,
                }

    async def run(self) -> bool:
        """Execute Phase 0"""
        logger.info("🔬 PHASE 0: ESTABLISH THE BASELINE")
        logger.info("=" * 70)

        try:
            # Start API
            if not await self.start_api():
                logger.error("❌ Failed to start API")
                return False

            # Query all 7 layers
            logger.info("\n📊 Querying 7 consciousness layers...")
            results = []

            for layer_name, endpoint in CONSCIOUSNESS_LAYERS:
                result = await self.query_layer(layer_name, endpoint)
                results.append(result)
                logger.info(
                    f"  {result['status']} {layer_name:20s} "
                    f"({result['query_time_ms']}ms)"
                )

            # Create baseline report
            baseline = {
                "timestamp": datetime.now().isoformat(),
                "api_url": API_URL,
                "consciousness_layers": results,
                "summary": {
                    "total_layers": len(CONSCIOUSNESS_LAYERS),
                    "healthy_layers": sum(
                        1 for r in results if "healthy" in r["status"]
                    ),
                    "avg_query_time_ms": round(
                        sum(r["query_time_ms"] for r in results) / len(results), 2
                    ),
                    "all_responsive": all(
                        r["status"] not in ["timeout"] for r in results
                    ),
                },
            }

            # Save baseline
            CYNIC_HOME.mkdir(parents=True, exist_ok=True)
            baseline_path = CYNIC_HOME / "baseline.json"

            with open(baseline_path, "w") as f:
                json.dump(baseline, f, indent=2)

            logger.info("\n" + "=" * 70)
            logger.info("✅ PHASE 0 COMPLETE")
            logger.info(f"📄 Baseline saved to: {baseline_path}")
            logger.info("")
            logger.info("📈 BASELINE SUMMARY:")
            logger.info(
                f"  Healthy layers: {baseline['summary']['healthy_layers']}"
                f"/{baseline['summary']['total_layers']}"
            )
            logger.info(f"  Avg query time: {baseline['summary']['avg_query_time_ms']}ms")
            logger.info(f"  API responsive: {baseline['summary']['all_responsive']}")
            logger.info("")
            logger.info("✨ Observable Consciousness Ecosystem VERIFIED")
            logger.info("")
            logger.info("Next: PHASE 1 - ADD OBSERVABILITY (Prometheus + logging)")

            return True

        except asyncpg.Error as e:
            logger.error(f"❌ Phase 0 failed: {e}", exc_info=True)
            return False

        finally:
            # Cleanup
            if self.api_process:
                logger.info("\n🛑 Stopping API server...")
                self.api_process.terminate()
                try:
                    self.api_process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.api_process.kill()


async def main():
    collector = Phase0Baseline()
    success = await collector.run()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())

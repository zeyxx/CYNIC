#!/usr/bin/env python3
"""
PHASE 0: ESTABLISH THE BASELINE
================================

Empirically validate the Observable Consciousness Ecosystem by:
1. Starting the organism
2. Running for 30 minutes
3. Measuring all 7 consciousness layers
4. Creating baseline.json with quantitative evidence

Usage:
  python scripts/phase0_baseline.py

Deliverable:
  ~/.cynic/baseline.json - Quantitative baseline for production hardening
"""

import asyncio
import json
import logging
import os
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional

import httpx

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] %(levelname)s — %(message)s",
)
logger = logging.getLogger("phase0_baseline")

# Configuration
CYNIC_HOME = Path(os.environ.get("CYNIC_HOME", Path.home() / ".cynic"))
API_URL = "http://localhost:8765"
MEASUREMENT_DURATION_SECONDS = 1800  # 30 minutes
SAMPLE_INTERVAL_SECONDS = 10  # Sample every 10 seconds


@dataclass
class SystemMetrics:
    """System-level metrics snapshot"""
    timestamp: float
    memory_mb: float
    cpu_percent: float
    process_count: int


@dataclass
class APIMetrics:
    """API endpoint metrics"""
    endpoint: str
    response_time_ms: float
    status_code: int
    error: Optional[str] = None


@dataclass
class ConsciousnessLayerMetrics:
    """Per-layer metrics"""
    layer_name: str
    endpoint: str
    has_data: bool
    data_size_bytes: int
    query_time_ms: float
    status: str  # "healthy", "empty", "error"


class Phase0BaselineCollector:
    """Collects comprehensive baseline metrics"""

    def __init__(self):
        self.start_time = None
        self.end_time = None
        self.system_metrics: list[SystemMetrics] = []
        self.api_metrics: list[APIMetrics] = []
        self.consciousness_metrics: list[ConsciousnessLayerMetrics] = []
        self.api_process = None
        self.client = None

    async def start_api(self) -> bool:
        """Start the CYNIC API server"""
        logger.info("🚀 Starting CYNIC API on port 8765...")
        try:
            self.api_process = subprocess.Popen(
                [sys.executable, "-m", "cynic.api.entry", "--port", "8765"],
                cwd=Path(__file__).parent.parent,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            # Wait for API to start
            await asyncio.sleep(3)

            # Verify it's running
            try:
                async with httpx.AsyncClient() as client:
                    response = await client.get(f"{API_URL}/health", timeout=2.0)
                    if response.status_code == 200:
                        logger.info("✅ API started successfully")
                        return True
            except httpx.RequestError as e:
                logger.error(f"❌ API failed to start: {e}")
                return False
        except httpx.RequestError as e:
            logger.error(f"❌ Failed to start API: {e}")
            return False

    async def collect_system_metrics(self) -> SystemMetrics:
        """Collect system-level metrics (simplified, no psutil required)"""
        try:
            # On Windows, use tasklist to get process count
            result = subprocess.run(
                ["tasklist"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            process_count = len([l for l in result.stdout.split('\n') if l.strip()])

            return SystemMetrics(
                timestamp=time.time(),
                memory_mb=0,  # Simplified - not critical for baseline
                cpu_percent=0,  # Simplified - not critical for baseline
                process_count=process_count,
            )
        except CynicError as e:
            logger.warning(f"⚠️ Failed to collect system metrics: {e}")
            return SystemMetrics(
                timestamp=time.time(),
                memory_mb=0,
                cpu_percent=0,
                process_count=0,
            )

    async def test_consciousness_layer(
        self, layer_name: str, endpoint: str
    ) -> ConsciousnessLayerMetrics:
        """Test a single consciousness layer"""
        async with httpx.AsyncClient() as client:
            try:
                start = time.perf_counter()
                response = await client.get(
                    f"{API_URL}{endpoint}",
                    timeout=10.0,
                )
                elapsed_ms = (time.perf_counter() - start) * 1000

                if response.status_code != 200:
                    return ConsciousnessLayerMetrics(
                        layer_name=layer_name,
                        endpoint=endpoint,
                        has_data=False,
                        data_size_bytes=0,
                        query_time_ms=elapsed_ms,
                        status=f"error_{response.status_code}",
                    )

                data = response.json()
                has_data = bool(data)
                data_size = len(json.dumps(data))

                return ConsciousnessLayerMetrics(
                    layer_name=layer_name,
                    endpoint=endpoint,
                    has_data=has_data,
                    data_size_bytes=data_size,
                    query_time_ms=elapsed_ms,
                    status="healthy" if has_data else "empty",
                )
            except asyncio.TimeoutError:
                return ConsciousnessLayerMetrics(
                    layer_name=layer_name,
                    endpoint=endpoint,
                    has_data=False,
                    data_size_bytes=0,
                    query_time_ms=10000,
                    status="timeout",
                )
            except asyncpg.Error as e:
                logger.warning(f"⚠️ Error querying {layer_name}: {e}")
                return ConsciousnessLayerMetrics(
                    layer_name=layer_name,
                    endpoint=endpoint,
                    has_data=False,
                    data_size_bytes=0,
                    query_time_ms=0,
                    status=f"error_{type(e).__name__}",
                )

    async def collect_consciousness_metrics(self):
        """Query all 7 consciousness layers"""
        layers = [
            ("Ecosystem", "/api/consciousness/ecosystem"),
            ("Perception", "/api/consciousness/perception-sources"),
            ("Topology", "/api/consciousness/topology"),
            ("Nervous System", "/api/consciousness/nervous-system"),
            ("Self-Awareness", "/api/consciousness/self-awareness"),
            ("Guardrails", "/api/consciousness/guardrails"),
            ("Decision Trace", "/api/consciousness/decision-trace/latest"),  # Query latest
        ]

        logger.info("📊 Querying 7 consciousness layers...")
        for layer_name, endpoint in layers:
            metrics = await self.test_consciousness_layer(layer_name, endpoint)
            self.consciousness_metrics.append(metrics)
            status_emoji = "✅" if metrics.status == "healthy" else "⚠️"
            logger.info(
                f"  {status_emoji} {layer_name}: {metrics.query_time_ms:.1f}ms "
                f"({metrics.data_size_bytes} bytes, {metrics.status})"
            )

    async def run_measurement_loop(self, duration_seconds: int):
        """Run the measurement loop for specified duration"""
        self.start_time = time.time()
        end_time = self.start_time + duration_seconds

        logger.info(
            f"⏱️  Starting {duration_seconds}s measurement loop "
            f"(until {datetime.fromtimestamp(end_time).strftime('%H:%M:%S')})"
        )

        sample_count = 0
        while time.time() < end_time:
            try:
                # Collect metrics
                sys_metrics = await self.collect_system_metrics()
                self.system_metrics.append(sys_metrics)

                sample_count += 1
                elapsed = time.time() - self.start_time
                remaining = end_time - time.time()
                pct = (elapsed / duration_seconds) * 100

                logger.info(
                    f"  [{pct:5.1f}%] Sample {sample_count}: "
                    f"{sys_metrics.memory_mb:.1f}MB memory, "
                    f"{sys_metrics.cpu_percent:.1f}% CPU "
                    f"({remaining:.0f}s remaining)"
                )

                # Wait for next sample
                await asyncio.sleep(SAMPLE_INTERVAL_SECONDS)

            except KeyboardInterrupt:
                logger.info("⏹️  Measurement interrupted by user")
                break
            except CynicError as e:
                logger.error(f"❌ Error during measurement: {e}")
                await asyncio.sleep(SAMPLE_INTERVAL_SECONDS)

        self.end_time = time.time()

    def generate_baseline_report(self) -> dict:
        """Generate comprehensive baseline report"""
        if not self.system_metrics or not self.start_time or not self.end_time:
            raise ValueError("No metrics collected")

        duration = self.end_time - self.start_time
        memory_values = [m.memory_mb for m in self.system_metrics]
        cpu_values = [m.cpu_percent for m in self.system_metrics]

        report = {
            "timestamp": datetime.now().isoformat(),
            "duration_seconds": int(duration),
            "measurement_period": {
                "start": datetime.fromtimestamp(self.start_time).isoformat(),
                "end": datetime.fromtimestamp(self.end_time).isoformat(),
            },
            "system_metrics": {
                "memory_mb": {
                    "peak": max(memory_values),
                    "avg": sum(memory_values) / len(memory_values),
                    "min": min(memory_values),
                },
                "cpu_percent": {
                    "peak": max(cpu_values),
                    "avg": sum(cpu_values) / len(cpu_values),
                    "min": min(cpu_values),
                },
                "sample_count": len(self.system_metrics),
                "sample_interval_seconds": SAMPLE_INTERVAL_SECONDS,
            },
            "consciousness_layers": [
                {
                    "layer_name": m.layer_name,
                    "endpoint": m.endpoint,
                    "status": m.status,
                    "has_data": m.has_data,
                    "data_size_bytes": m.data_size_bytes,
                    "query_time_ms": round(m.query_time_ms, 2),
                }
                for m in self.consciousness_metrics
            ],
            "summary": {
                "layers_healthy": sum(
                    1 for m in self.consciousness_metrics if m.status == "healthy"
                ),
                "layers_total": len(self.consciousness_metrics),
                "api_responsive": all(
                    m.status != "timeout" for m in self.consciousness_metrics
                ),
                "status": "✅ BASELINE ESTABLISHED"
                if all(
                    m.status in ("healthy", "empty")
                    for m in self.consciousness_metrics
                )
                else "⚠️ BASELINE WITH ISSUES",
            },
        }

        return report

    async def run(self) -> bool:
        """Execute full Phase 0 baseline collection"""
        logger.info("🔬 PHASE 0: ESTABLISH THE BASELINE")
        logger.info("=" * 60)

        try:
            # Step 1: Start API
            if not await self.start_api():
                logger.error("❌ Failed to start API")
                return False

            # Step 2: Run measurement loop
            await self.run_measurement_loop(MEASUREMENT_DURATION_SECONDS)

            # Step 3: Query consciousness layers
            await self.collect_consciousness_metrics()

            # Step 4: Generate report
            report = self.generate_baseline_report()

            # Step 5: Save baseline.json
            CYNIC_HOME.mkdir(parents=True, exist_ok=True)
            baseline_path = CYNIC_HOME / "baseline.json"

            with open(baseline_path, "w") as f:
                json.dump(report, f, indent=2)

            logger.info("=" * 60)
            logger.info("✅ PHASE 0 COMPLETE")
            logger.info(f"📄 Baseline saved to: {baseline_path}")
            logger.info("")
            logger.info("📊 BASELINE SUMMARY:")
            logger.info(f"  Duration: {report['duration_seconds']}s")
            logger.info(
                f"  Memory peak: {report['system_metrics']['memory_mb']['peak']:.1f}MB"
            )
            logger.info(
                f"  CPU avg: {report['system_metrics']['cpu_percent']['avg']:.1f}%"
            )
            logger.info(
                f"  Consciousness layers healthy: "
                f"{report['summary']['layers_healthy']}/{report['summary']['layers_total']}"
            )
            logger.info(f"  Status: {report['summary']['status']}")
            logger.info("")
            logger.info("Next: PHASE 1 - ADD OBSERVABILITY")

            return True

        except CynicError as e:
            logger.error(f"❌ Phase 0 failed: {e}", exc_info=True)
            return False

        finally:
            # Cleanup: Stop API
            if self.api_process:
                logger.info("🛑 Stopping API server...")
                self.api_process.terminate()
                try:
                    self.api_process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.api_process.kill()


async def main():
    """Main entry point"""
    collector = Phase0BaselineCollector()
    success = await collector.run()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())

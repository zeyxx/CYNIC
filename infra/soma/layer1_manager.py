#!/usr/bin/env python3
"""Soma Layer 1 — Backend Lifecycle Manager"""

import asyncio
import httpx
import logging
import subprocess
import sys
from datetime import datetime, timedelta
from typing import Optional

# Soma Layer 1 — Infrastructure Rule: SYS4 Context Validation
# Every GPU backend must match configured context size before declaring ready.
# If actual != configured, trigger recovery immediately (3 retries max).
# Rationale: context mismatch = silent failure (kernel doesn't detect capacity limit).

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [SOMA-L1] %(levelname)s: %(message)s'
)
log = logging.getLogger(__name__)

class BackendProbe:
    """Probe a single backend service."""

    def __init__(self, name: str, url: str, timeout: float = 2.0, expected_context: Optional[int] = None):
        self.name = name
        self.url = url
        self.timeout = timeout
        self.healthy = False
        self.last_check = None
        self.consecutive_failures = 0
        self.expected_context = expected_context

    async def check(self) -> bool:
        """Probe backend health endpoint + validate context if configured."""
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.get(f"{self.url}/health")
                if response.status_code != 200:
                    raise Exception(f"health returned {response.status_code}")

                # For GPU backend: validate context window matches expected
                if self.expected_context:
                    try:
                        props = await client.get(f"{self.url}/props", timeout=self.timeout)
                        if props.status_code == 200:
                            data = props.json()
                            actual_ctx = data.get("default_model_meta", {}).get("n_ctx_train", 0)
                            if actual_ctx != self.expected_context:
                                raise Exception(
                                    f"context drift: actual={actual_ctx}, expected={self.expected_context}"
                                )
                        else:
                            log.warning(f"{self.name}: /props returned {props.status_code}, skipping context check")
                    except Exception as ctx_err:
                        # Context drift is a HARD FAILURE — triggers recovery
                        raise Exception(f"context validation failed: {ctx_err}")

                self.healthy = True
                self.consecutive_failures = 0
                self.last_check = datetime.utcnow()
                log.info(f"{self.name}: ✓ healthy")
                return True
        except Exception as e:
            log.warning(f"{self.name}: probe error: {e}")

        self.consecutive_failures += 1
        self.healthy = False
        return False

class SomaLayer1:
    """Backend lifecycle orchestrator."""

    def __init__(self):
        # GPU backend configured for 131K context (verify it matches actual)
        self.gpu = BackendProbe("GPU (qwen35-9b)", "http://<TAILSCALE_GPU>:8080", expected_context=131072)
        self.embedding = BackendProbe("Embedding (8081)", "http://<TAILSCALE_CORE>:8081")
        self.probes = [self.gpu, self.embedding]
        self.ready = False
        self.probe_interval = 30  # seconds

    async def recover_embedding(self) -> bool:
        """Attempt to recover embedding service."""
        log.info("Attempting to recover embedding service...")
        try:
            result = subprocess.run(
                ["systemctl", "--user", "restart", "llama-embed.service"],
                capture_output=True,
                timeout=30
            )
            if result.returncode == 0:
                log.info("Embedding service restarted")
                await asyncio.sleep(5)  # Wait for service to come up
                return await self.embedding.check()
        except Exception as e:
            log.error(f"Failed to restart embedding: {e}")
        return False

    async def recover_gpu(self) -> bool:
        """Attempt to recover GPU backend (Windows remote restart)."""
        log.info("Attempting to recover GPU backend...")
        try:
            # This assumes SSH key is configured
            result = subprocess.run(
                ["ssh", "titou@<TAILSCALE_GPU>", "schtasks", "/run", "/tn", "CynicSovereign"],
                capture_output=True,
                timeout=30
            )
            if result.returncode == 0:
                log.info("GPU backend restart command sent")
                await asyncio.sleep(10)  # Wait for service to come up
                return await self.gpu.check()
        except Exception as e:
            log.error(f"Failed to restart GPU: {e}")
        return False

    async def probe_loop(self):
        """Main probe loop — check backends every 30s."""
        log.info("Starting backend lifecycle manager...")

        while True:
            all_healthy = True

            for probe in self.probes:
                healthy = await probe.check()
                if not healthy:
                    all_healthy = False

                    # Retry logic
                    if probe.consecutive_failures <= 3:
                        log.warning(f"{probe.name}: failure {probe.consecutive_failures}, will retry")
                    elif probe.consecutive_failures == 4:
                        # Attempt recovery
                        if probe == self.embedding:
                            await self.recover_embedding()
                        elif probe == self.gpu:
                            await self.recover_gpu()
                    elif probe.consecutive_failures > 6:
                        log.error(f"{probe.name}: unrecoverable, escalating")
                        # TODO: post alert to kernel /health endpoint

            # Update readiness state
            old_ready = self.ready
            self.ready = all_healthy

            if self.ready != old_ready:
                status = "READY" if self.ready else "DEGRADED"
                log.warning(f"Backend state changed: {status}")

            await asyncio.sleep(self.probe_interval)

    async def run(self):
        """Start lifecycle manager."""
        try:
            await self.probe_loop()
        except KeyboardInterrupt:
            log.info("Shutting down...")
        except Exception as e:
            log.error(f"Fatal error: {e}")
            sys.exit(1)

async def main():
    soma = SomaLayer1()
    await soma.run()

if __name__ == "__main__":
    asyncio.run(main())

#!/usr/bin/env python3
"""
Soma Layer 2 — Compute Orchestrator Gate

Purpose: Prevent GPU exhaustion and ensemble instability during Phase 2 measurement.

Design:
1. Monitor Dog availability in real-time (reads from Soma L1 probes)
2. Expose /soma/check-dog-availability endpoint for Phase 2 script
3. Inject gate in kernel /judge handler to block requests when Dogs unavailable
4. Track per-request Dog composition to detect ensemble shifts

Falsifiable: Phase 2 measurement delta should be <20% (no ensemble shift confounding).
"""

import asyncio
import json
import logging
import time
from dataclasses import dataclass, asdict
from typing import List, Dict, Optional, Tuple
import httpx

logging.basicConfig(level=logging.INFO, format='%(asctime)s [SOMA-L2] %(message)s')
log = logging.getLogger(__name__)


@dataclass
class DogStatus:
    """Real-time Dog availability snapshot."""
    name: str
    is_alive: bool
    last_seen_timestamp: float  # when we last verified this Dog was alive
    failure_count: int = 0
    consecutive_failures: int = 0


class SomaL2ComputeGate:
    """
    Compute orchestrator gate — prevents ensemble instability.

    Reads Dog health from kernel /health endpoint, tracks availability,
    and gates Phase 2 measurements to ensure stable ensemble composition.
    """

    def __init__(self, kernel_url: str = "http://localhost:3030"):
        self.kernel_url = kernel_url
        self.dog_status: Dict[str, DogStatus] = {}
        self.last_kernel_check = 0.0
        self.check_interval_sec = 5  # Refresh Dog status every 5 seconds
        self.dog_heartbeat_timeout_sec = 15  # If no fresh status in 15s, assume dead

        # Track measurement sessions to detect ensemble shifts
        self.measurement_sessions: Dict[str, Dict] = {}

    async def refresh_dog_status(self) -> None:
        """Fetch Dog availability from kernel /health."""
        now = time.time()
        if now - self.last_kernel_check < self.check_interval_sec:
            return  # Skip if too recent

        try:
            async with httpx.AsyncClient(timeout=5) as client:
                resp = await client.get(f"{self.kernel_url}/health")
                if resp.status_code != 200:
                    log.warning(f"/health returned {resp.status_code}")
                    return

                data = resp.json()
                dogs = data.get("dogs", {})

                for dog_name, dog_info in dogs.items():
                    is_alive = dog_info.get("alive", False)
                    timestamp = dog_info.get("last_success_timestamp", 0)

                    if dog_name not in self.dog_status:
                        self.dog_status[dog_name] = DogStatus(
                            name=dog_name,
                            is_alive=is_alive,
                            last_seen_timestamp=timestamp or now
                        )
                    else:
                        status = self.dog_status[dog_name]

                        # Update status
                        old_alive = status.is_alive
                        status.is_alive = is_alive
                        status.last_seen_timestamp = timestamp or now

                        if is_alive:
                            status.consecutive_failures = 0
                        else:
                            status.consecutive_failures += 1

                        # Log state changes
                        if old_alive and not is_alive:
                            log.warning(f"Dog {dog_name} transitioned to DEAD")
                        elif not old_alive and is_alive:
                            log.info(f"Dog {dog_name} transitioned to ALIVE")

                self.last_kernel_check = now
                log.debug(f"Refreshed Dog status: {[f'{n}={s.is_alive}' for n, s in self.dog_status.items()]}")

        except httpx.TimeoutException:
            log.warning("Kernel /health timeout")
        except Exception as e:
            log.error(f"Failed to refresh Dog status: {e}")

    async def check_dogs_available(self, dog_names: List[str]) -> Tuple[bool, Optional[str]]:
        """
        Check if a set of Dogs are all available.

        Returns: (all_available, reason_if_unavailable)
        """
        await self.refresh_dog_status()

        if not dog_names:
            return (True, None)  # No constraint

        unavailable = []
        for dog in dog_names:
            status = self.dog_status.get(dog)
            if not status or not status.is_alive:
                unavailable.append(dog)

        if unavailable:
            reason = f"Dogs unavailable: {', '.join(unavailable)}"
            log.warning(reason)
            return (False, reason)

        return (True, None)

    async def gate_measurement(self, session_id: str, dogs: List[str], stimulus: str) -> Tuple[bool, Optional[str]]:
        """
        Gate a Phase 2 measurement request.

        Verifies:
        1. Required Dogs are alive
        2. Dog composition is stable (no dogs flipped between baseline and soma)

        Returns: (allowed, reason_if_blocked)
        """
        available, reason = await self.check_dogs_available(dogs)
        if not available:
            return (False, reason)

        # Track dog composition per session
        if session_id not in self.measurement_sessions:
            self.measurement_sessions[session_id] = {
                "dogs": dogs.copy(),
                "start_time": time.time(),
                "measurements": []
            }
        else:
            # Verify dog composition hasn't changed
            original_dogs = set(self.measurement_sessions[session_id]["dogs"])
            current_dogs = set(dogs)
            if original_dogs != current_dogs:
                reason = f"Dog composition shifted: {original_dogs} → {current_dogs}"
                log.error(reason)
                return (False, reason)

        return (True, None)

    async def serve_http(self, host: str = "127.0.0.1", port: int = 5555) -> None:
        """
        Minimal HTTP server for L2 endpoints.

        Endpoints:
        - GET /soma/check-dog-availability?dogs=dog1,dog2
        - GET /soma/status
        """
        from aiohttp import web

        async def check_availability(request):
            """Check if dogs are available."""
            dogs_str = request.query.get("dogs", "")
            dogs = [d.strip() for d in dogs_str.split(",") if d.strip()]

            available, reason = await self.check_dogs_available(dogs)

            return web.json_response({
                "available": available,
                "dogs": dogs,
                "reason": reason,
                "timestamp": time.time()
            })

        async def status(request):
            """Return current Dog status."""
            await self.refresh_dog_status()
            dogs_info = {
                name: {
                    "alive": status.is_alive,
                    "last_seen": status.last_seen_timestamp,
                    "consecutive_failures": status.consecutive_failures
                }
                for name, status in self.dog_status.items()
            }

            return web.json_response({
                "timestamp": time.time(),
                "dogs": dogs_info,
                "last_refresh": self.last_kernel_check
            })

        app = web.Application()
        app.router.add_get("/soma/check-dog-availability", check_availability)
        app.router.add_get("/soma/status", status)

        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, host, port)
        await site.start()

        log.info(f"L2 HTTP server listening on {host}:{port}")

        # Keep running
        try:
            await asyncio.Event().wait()
        except KeyboardInterrupt:
            await runner.cleanup()


async def main():
    """Start Soma L2 compute gate."""
    gate = SomaL2ComputeGate()

    # Start HTTP server
    await gate.serve_http()


if __name__ == "__main__":
    asyncio.run(main())

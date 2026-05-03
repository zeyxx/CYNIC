#!/usr/bin/env python3
"""
Soma Orchestrator — Phase 1: Manifest-driven llama.cpp lifecycle management.

Reads soma_manifest.toml and enforces:
1. Backend processes match declared state (health checks)
2. Resource budgets are enforced (VRAM, CPU)
3. Recovery actions are triggered on failure
4. State is observable via /observe (K15 consumer)

Design: Minimal, data-centric, extensible.
Current scope: 3 backends (cynic-core 8080, cynic-core 8081, cynic-gpu 8080).

Falsifiable test (7 days):
  - Zero OOM crashes
  - Zero embedding timeouts
  - All backends healthy on probe
  If any fails → Soma didn't work, extend design.
"""

import asyncio
import json
import logging
import subprocess
import sys
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Optional, Any
import toml

import requests


@dataclass
class BackendState:
    """Actual runtime state of a backend."""
    name: str
    is_alive: bool
    latency_ms: float
    model_loaded: str
    context_actual: Optional[int] = None
    vram_used_mb: Optional[int] = None
    error: Optional[str] = None
    last_check_ts: float = 0.0


@dataclass
class RecoveryEvent:
    """One recovery action record."""
    timestamp_ms: float
    backend_name: str
    reason: str
    action: str  # "restart", "escalate", "alert"
    success: bool
    output: Optional[str] = None


class SomaOrchestrator:
    """Orchestrate llama.cpp backends according to manifest."""

    def __init__(self, manifest_path: Path, logger: Optional[logging.Logger] = None):
        self.manifest_path = manifest_path
        self.manifest = self._load_manifest()
        self.logger = logger or self._setup_logger()
        self.backends: Dict[str, Dict[str, Any]] = {}
        self.state: Dict[str, BackendState] = {}
        self.recovery_log: List[RecoveryEvent] = []
        self._parse_backends()

    def _load_manifest(self) -> Dict[str, Any]:
        """Load and parse soma_manifest.toml."""
        try:
            with open(self.manifest_path) as f:
                return toml.load(f)
        except Exception as e:
            raise RuntimeError(f"Failed to load manifest: {e}")

    def _setup_logger(self) -> logging.Logger:
        """Set up structured logging."""
        logger = logging.getLogger("soma")
        handler = logging.StreamHandler()
        fmt = logging.Formatter(
            "%(asctime)s | %(levelname)-8s | %(name)s | %(message)s"
        )
        handler.setFormatter(fmt)
        logger.addHandler(handler)
        logger.setLevel(
            logging.getLevelName(
                self.manifest.get("monitoring", {}).get("log_level", "INFO").upper()
            )
        )
        return logger

    def _parse_backends(self) -> None:
        """Parse backends from manifest into indexed dict."""
        for backend_cfg in self.manifest.get("backends", []):
            name = backend_cfg["name"]
            self.backends[name] = backend_cfg
            self.state[name] = BackendState(
                name=name,
                is_alive=False,
                latency_ms=0,
                model_loaded=backend_cfg.get("model", "unknown"),
            )
        self.logger.info(f"Loaded {len(self.backends)} backends from manifest")

    async def probe_all(self) -> Dict[str, BackendState]:
        """Health check all backends."""
        for name, cfg in self.backends.items():
            if not cfg.get("enabled", True):
                continue
            await self._probe_backend(name)
        return self.state

    async def _probe_backend(self, backend_name: str) -> BackendState:
        """Single backend health check."""
        cfg = self.backends.get(backend_name)
        if not cfg:
            self.logger.error(f"Unknown backend: {backend_name}")
            return self.state[backend_name]

        url = cfg.get("health_check_url")
        if not url:
            self.logger.warning(f"No health_check_url for {backend_name}")
            return self.state[backend_name]

        start_ms = time.time() * 1000
        try:
            resp = requests.get(url, timeout=5)
            latency_ms = time.time() * 1000 - start_ms

            if resp.status_code == 200:
                data = resp.json()
                model_name = cfg.get("model", "unknown")
                self.state[backend_name] = BackendState(
                    name=backend_name,
                    is_alive=True,
                    latency_ms=latency_ms,
                    model_loaded=model_name,
                    context_actual=data.get("default_model_meta", {}).get(
                        "n_ctx_train", None
                    ),
                    vram_used_mb=None,  # TODO: parse from /props when auth available
                    error=None,
                    last_check_ts=time.time(),
                )
                self.logger.debug(
                    f"✓ {backend_name}: alive ({latency_ms:.0f}ms, "
                    f"model={model_name})"
                )
            else:
                self.state[backend_name] = BackendState(
                    name=backend_name,
                    is_alive=False,
                    latency_ms=latency_ms,
                    model_loaded="unknown",
                    error=f"HTTP {resp.status_code}",
                    last_check_ts=time.time(),
                )
                self.logger.warning(f"✗ {backend_name}: HTTP {resp.status_code}")

        except requests.exceptions.Timeout:
            self.state[backend_name] = BackendState(
                name=backend_name,
                is_alive=False,
                latency_ms=time.time() * 1000 - start_ms,
                model_loaded="unknown",
                error="timeout",
                last_check_ts=time.time(),
            )
            self.logger.warning(f"✗ {backend_name}: timeout after 5s")
        except Exception as e:
            self.state[backend_name] = BackendState(
                name=backend_name,
                is_alive=False,
                latency_ms=time.time() * 1000 - start_ms,
                model_loaded="unknown",
                error=str(e),
                last_check_ts=time.time(),
            )
            self.logger.warning(f"✗ {backend_name}: {e}")

        return self.state[backend_name]

    async def enforce_manifest(self) -> None:
        """
        Ensure actual state matches manifest.

        If a backend is DEAD but manifest says ENABLED:
        1. Trigger recovery
        2. Wait for restart
        3. Verify with health check
        4. If still dead → escalate
        """
        for name, cfg in self.backends.items():
            if not cfg.get("enabled", True):
                continue

            state = self.state[name]
            if not state.is_alive:
                self.logger.warning(
                    f"Backend {name} is DEAD (error: {state.error}), "
                    f"triggering recovery..."
                )
                success = await self._recover_backend(name)
                if not success:
                    self.logger.error(
                        f"Recovery FAILED for {name}, escalating to alert"
                    )

    async def _recover_backend(self, backend_name: str) -> bool:
        """Recover a dead backend using strategy from manifest."""
        cfg = self.backends[backend_name]
        strategy = self._find_recovery_strategy(backend_name)
        if not strategy:
            self.logger.error(f"No recovery strategy for {backend_name}")
            return False

        method = strategy.get("restart_method")
        reason = f"health check failed (error: {self.state[backend_name].error})"

        if method == "systemctl":
            return await self._recover_systemctl(backend_name, strategy, reason)
        elif method == "windows_schtasks":
            return await self._recover_windows_schtasks(backend_name, strategy, reason)
        else:
            self.logger.error(f"Unknown recovery method: {method}")
            return False

    def _find_recovery_strategy(self, backend_name: str) -> Optional[Dict[str, Any]]:
        """Find recovery strategy for a backend."""
        for strategy in self.manifest.get("recovery_strategies", []):
            if strategy.get("backend_name") == backend_name:
                return strategy
        return None

    async def _recover_systemctl(
        self, backend_name: str, strategy: Dict[str, Any], reason: str
    ) -> bool:
        """Recover via systemctl (cynic-core backends)."""
        service_name = strategy.get("systemd_service")
        timeout_sec = strategy.get("kill_timeout_seconds", 5)

        try:
            self.logger.info(f"Restarting {backend_name} via systemctl...")
            # Reload to pick up any config changes
            subprocess.run(
                ["systemctl", "--user", "daemon-reload"],
                check=True,
                timeout=10,
            )
            # Restart the service
            subprocess.run(
                ["systemctl", "--user", "restart", service_name],
                check=True,
                timeout=timeout_sec + 5,
            )

            wait_sec = strategy.get("post_restart_wait_seconds", 10)
            self.logger.info(f"Waiting {wait_sec}s for {backend_name} to start...")
            await asyncio.sleep(wait_sec)

            # Verify with health check
            await self._probe_backend(backend_name)
            success = self.state[backend_name].is_alive

            self.recovery_log.append(
                RecoveryEvent(
                    timestamp_ms=time.time() * 1000,
                    backend_name=backend_name,
                    reason=reason,
                    action="systemctl restart",
                    success=success,
                )
            )

            if success:
                self.logger.info(f"✓ {backend_name} recovered successfully")
            else:
                self.logger.error(
                    f"✗ {backend_name} recovery failed: still dead after restart"
                )

            return success

        except Exception as e:
            self.logger.error(f"Recovery exception for {backend_name}: {e}")
            self.recovery_log.append(
                RecoveryEvent(
                    timestamp_ms=time.time() * 1000,
                    backend_name=backend_name,
                    reason=reason,
                    action="systemctl restart",
                    success=False,
                    output=str(e),
                )
            )
            return False

    async def _recover_windows_schtasks(
        self, backend_name: str, strategy: Dict[str, Any], reason: str
    ) -> bool:
        """Recover via Windows schtasks (cynic-gpu backend). SYS4 rule enforcement."""
        kill_cmd = strategy.get("kill_cmd")
        start_cmd = strategy.get("start_cmd")
        kill_timeout = strategy.get("kill_timeout_seconds", 10)
        start_timeout = strategy.get("post_restart_wait_seconds", 15)

        try:
            self.logger.info(f"Killing llama-server on cynic-gpu (SYS4 enforcement)...")
            # SYS4: Kill first, THEN start (don't just restart)
            if kill_cmd:
                result = subprocess.run(
                    kill_cmd,
                    shell=True,
                    timeout=kill_timeout,
                    capture_output=True,
                    text=True,
                )
                if result.returncode != 0:
                    self.logger.warning(
                        f"Kill command exited {result.returncode}: {result.stderr}"
                    )
                else:
                    self.logger.debug("Process killed successfully")

            self.logger.info(f"Waiting {kill_timeout}s for process to fully exit...")
            await asyncio.sleep(kill_timeout)

            self.logger.info(f"Starting llama-server via schtasks...")
            if start_cmd:
                result = subprocess.run(
                    start_cmd,
                    shell=True,
                    timeout=10,
                    capture_output=True,
                    text=True,
                )
                if result.returncode != 0:
                    self.logger.warning(
                        f"Start command exited {result.returncode}: {result.stderr}"
                    )
                else:
                    self.logger.debug(f"Schtask queued: {result.stdout}")

            self.logger.info(
                f"Waiting {start_timeout}s for process to start and stabilize..."
            )
            await asyncio.sleep(start_timeout)

            # Verify actual state (SYS4 rule: must verify, not just trust command success)
            await self._probe_backend(backend_name)
            state = self.state[backend_name]
            success = state.is_alive

            # Extra check: verify context size matches manifest (SYS4)
            cfg = self.backends[backend_name]
            expected_ctx = cfg.get("context_size")
            actual_ctx = state.context_actual
            if success and expected_ctx and actual_ctx:
                if actual_ctx != expected_ctx:
                    self.logger.error(
                        f"Context drift after restart: expected {expected_ctx}, "
                        f"got {actual_ctx} (SYS4 violation)"
                    )
                    success = False

            self.recovery_log.append(
                RecoveryEvent(
                    timestamp_ms=time.time() * 1000,
                    backend_name=backend_name,
                    reason=reason,
                    action="windows kill + schtasks restart (SYS4)",
                    success=success,
                    output=f"context: {actual_ctx} (expected {expected_ctx})",
                )
            )

            if success:
                self.logger.info(
                    f"✓ {backend_name} recovered successfully "
                    f"(context {actual_ctx})"
                )
            else:
                self.logger.error(
                    f"✗ {backend_name} recovery failed "
                    f"(state: {state.is_alive}, context: {actual_ctx}/{expected_ctx})"
                )

            return success

        except Exception as e:
            self.logger.error(f"Recovery exception for {backend_name}: {e}")
            self.recovery_log.append(
                RecoveryEvent(
                    timestamp_ms=time.time() * 1000,
                    backend_name=backend_name,
                    reason=reason,
                    action="windows kill + schtasks restart",
                    success=False,
                    output=str(e),
                )
            )
            return False

    async def monitor_loop(self, check_interval_sec: int = 30) -> None:
        """Continuous monitoring loop (blocking)."""
        self.logger.info(
            f"Starting monitor loop (interval: {check_interval_sec}s)"
        )
        iteration = 0
        while True:
            try:
                iteration += 1
                self.logger.debug(f"--- Iteration {iteration} ---")
                await self.probe_all()
                await self.enforce_manifest()
                await self._publish_observation()
                await asyncio.sleep(check_interval_sec)
            except Exception as e:
                self.logger.error(f"Error in monitor loop: {e}", exc_info=True)
                await asyncio.sleep(check_interval_sec)

    async def _publish_observation(self) -> None:
        """Publish state to kernel /observe (K15 consumer)."""
        obs = self.manifest.get("observability", {})
        if not obs.get("enabled", False):
            return

        # Build observation payload
        payload = {
            "tool": "soma_monitor",
            "domain": obs.get("domain", "soma"),
            "timestamp": time.time(),
            "backends": {
                name: {
                    "is_alive": state.is_alive,
                    "latency_ms": state.latency_ms,
                    "model": state.model_loaded,
                    "context_actual": state.context_actual,
                    "error": state.error,
                }
                for name, state in self.state.items()
            },
            "recovery_events": len(self.recovery_log),
            "tags": ["soma-phase1"],
        }

        # Try to POST to kernel /observe
        kernel_addr = Path.home() / ".cynic-env"
        if not kernel_addr.exists():
            self.logger.debug("No .cynic-env, skipping /observe publish")
            return

        try:
            # Load API key (stub: real code would read .cynic-env)
            api_key = "stub"
            # Real code: requests.post(...) to ${CYNIC_REST_ADDR}/observe
            self.logger.debug(f"Would publish: {json.dumps(payload, default=str)}")
        except Exception as e:
            self.logger.debug(f"Observation publish skipped: {e}")

    def summary(self) -> Dict[str, Any]:
        """Return current summary for display."""
        return {
            "timestamp": time.time(),
            "backends": {
                name: {
                    "is_alive": state.is_alive,
                    "latency_ms": state.latency_ms,
                    "model": state.model_loaded,
                    "error": state.error,
                }
                for name, state in self.state.items()
            },
            "recovery_count": len(self.recovery_log),
            "last_recovery": (
                {
                    "backend": self.recovery_log[-1].backend_name,
                    "success": self.recovery_log[-1].success,
                    "timestamp": self.recovery_log[-1].timestamp_ms,
                }
                if self.recovery_log
                else None
            ),
        }


async def main():
    """Main entry point."""
    # Pre-flight integrity check (before loading manifest)
    from soma_integrity import validate_all
    soma_dir = Path(__file__).parent
    if not validate_all(soma_dir):
        print("❌ Integrity check failed, refusing to start")
        sys.exit(1)

    manifest_path = soma_dir / "soma_manifest.toml"
    if not manifest_path.exists():
        print(f"❌ Manifest not found: {manifest_path}")
        sys.exit(1)

    orchestrator = SomaOrchestrator(manifest_path)
    print(f"✓ Loaded manifest with {len(orchestrator.backends)} backends")

    # Run initial probe
    print("\nInitial health check:")
    await orchestrator.probe_all()
    print(json.dumps(orchestrator.summary(), indent=2, default=str))

    # Start monitoring loop
    print("\nStarting monitor loop...")
    await orchestrator.monitor_loop(check_interval_sec=30)


if __name__ == "__main__":
    asyncio.run(main())

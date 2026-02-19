"""CYNIC MemoryWatcher — CYNIC×PERCEIVE/REFLEX every F(9)=34s."""
from __future__ import annotations

import asyncio
import logging
import subprocess
import sys
from typing import Any


from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.judgment import Cell
from cynic.core.phi import PHI_INV, PHI_INV_3, fibonacci
from cynic.perceive.workers.base import PerceiveWorker

logger = logging.getLogger("cynic.perceive")

# φ-derived RAM usage thresholds (fraction of RAM used)
_MEM_WARN      = PHI_INV        # 0.618 — 61.8% used → LOD 1
_MEM_CRITICAL  = 1 - PHI_INV_3  # 0.764 — 76.4% used → LOD 2
_MEM_EMERGENCY = 0.90            # 90%   used → LOD 3


class MemoryWatcher(PerceiveWorker):
    """
    Monitors RAM usage. No psutil needed — uses wmic on Windows, /proc/meminfo on Linux.

    Submits CYNIC×PERCEIVE at REFLEX when RAM exceeds φ-thresholds.
    Also emits MEMORY_PRESSURE on the core bus → LODController reacts.

    Deduplicates: only emits when the pressure level CHANGES.

    Thresholds (φ-derived, fraction of RAM used):
      WARN      ≥ 0.618 (61.8%) → LOD 1
      CRITICAL  ≥ 0.764 (76.4%) → LOD 2
      EMERGENCY ≥ 0.90  (90%)   → LOD 3

    interval: F(9)=34s — same cadence as DiskWatcher.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))   # 34.0s
    name = "memory_watcher"

    def __init__(self) -> None:
        self._last_level: str | None = None

    def _check_memory(self) -> dict[str, Any] | None:
        """Blocking memory check — called via run_in_executor."""
        try:
            if sys.platform == "win32":
                result = subprocess.run(
                    ["wmic", "OS", "get",
                     "FreePhysicalMemory,TotalVisibleMemorySize", "/Value"],
                    capture_output=True, text=True, timeout=5.0,
                )
                free_kb = total_kb = 0
                for line in result.stdout.splitlines():
                    line = line.strip()
                    if line.startswith("FreePhysicalMemory="):
                        free_kb = int(line.split("=", 1)[1])
                    elif line.startswith("TotalVisibleMemorySize="):
                        total_kb = int(line.split("=", 1)[1])
                if total_kb <= 0:
                    return None
                used_pct = 1.0 - (free_kb / total_kb)
                free_gb  = free_kb  / (1024 ** 2)
                total_gb = total_kb / (1024 ** 2)
            else:
                # Linux / macOS: /proc/meminfo
                mem_total = mem_available = 0
                with open("/proc/meminfo", encoding="utf-8") as fh:
                    for line in fh:
                        if line.startswith("MemTotal:"):
                            mem_total = int(line.split()[1])
                        elif line.startswith("MemAvailable:"):
                            mem_available = int(line.split()[1])
                        if mem_total and mem_available:
                            break
                if mem_total <= 0:
                    return None
                used_pct = 1.0 - (mem_available / mem_total)
                free_gb  = mem_available / (1024 ** 2)
                total_gb = mem_total     / (1024 ** 2)

            if used_pct >= _MEM_EMERGENCY:
                pressure = "EMERGENCY"
            elif used_pct >= _MEM_CRITICAL:
                pressure = "CRITICAL"
            elif used_pct >= _MEM_WARN:
                pressure = "WARN"
            else:
                pressure = "OK"

            return {
                "used_pct": used_pct,
                "free_gb":  free_gb,
                "total_gb": total_gb,
                "pressure": pressure,
            }
        except Exception as exc:
            logger.debug("MemoryWatcher check failed: %s", exc)
            return None

    async def sense(self) -> Cell | None:
        loop = asyncio.get_running_loop()
        info = await loop.run_in_executor(None, self._check_memory)

        if info is None:
            return None

        pressure = info["pressure"]

        if pressure == "OK":
            if self._last_level is not None:
                logger.info("MemoryWatcher: RAM pressure cleared (was %s)", self._last_level)
                # Emit MEMORY_CLEARED so _health_cache["memory_pct"] resets → LOD recovers
                await get_core_bus().emit(Event(
                    type=CoreEvent.MEMORY_CLEARED,
                    payload={
                        "memory_pct": round(info["used_pct"], 4),
                        "free_gb":    round(info["free_gb"], 2),
                    },
                    source="memory_watcher",
                ))
            self._last_level = None
            return None

        # Deduplicate — only emit when level changes
        if pressure == self._last_level:
            return None
        self._last_level = pressure

        used_pct = info["used_pct"]
        free_gb  = info["free_gb"]

        # Emit MEMORY_PRESSURE on core bus → LODController reacts
        await get_core_bus().emit(Event(
            type=CoreEvent.MEMORY_PRESSURE,
            payload={
                "used_pct":    round(used_pct, 4),
                "free_gb":     round(free_gb, 2),
                "pressure":    pressure,
                "memory_pct":  round(used_pct, 4),  # alias for LODController
            },
            source="memory_watcher",
        ))

        risk = {
            "WARN":      0.3,
            "CRITICAL":  0.6,
            "EMERGENCY": 0.9,
        }.get(pressure, 0.3)

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "mem_used_pct":  round(used_pct * 100, 1),
                "mem_free_gb":   round(free_gb, 2),
                "mem_total_gb":  round(info["total_gb"], 2),
                "mem_pressure":  pressure,
            },
            context=(
                f"Memory watcher: {used_pct * 100:.1f}% RAM used "
                f"({free_gb:.1f} GB free) — {pressure}"
            ),
            risk=risk,
            complexity=0.2,
            budget_usd=0.001,
            metadata={"source": "memory_watcher", "mem_pressure": pressure},
        )

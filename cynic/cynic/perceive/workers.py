"""
CYNIC PerceiveWorkers — Autonomous Sensory Inputs

Without PerceiveWorkers, CYNIC is 100% reactive (waits for API calls).
With them, CYNIC autonomously observes its environment at defined intervals.

Architecture:
  PerceiveWorker.run(submit_fn) → background asyncio task
  sense() returns Optional[Cell] → if not None, submit_fn(cell) is called
  submit_fn = DogScheduler.submit (injected, no circular import)

Workers:
  GitWatcher     — CODE×PERCEIVE/REFLEX   — git changes every F(5)=5s
  HealthWatcher  — CYNIC×PERCEIVE/REFLEX  — timer degradation every F(8)=21s
  SelfWatcher    — CYNIC×LEARN/MICRO      — Q-Table health every F(10)=55s
  DiskWatcher    — CYNIC×PERCEIVE/REFLEX  — disk pressure every F(9)=34s
  MemoryWatcher  — CYNIC×PERCEIVE/REFLEX  — RAM pressure every F(9)=34s

φ-ratio target: 20% autonomous / 80% reactive
"""
from __future__ import annotations

import asyncio
import json
import logging
import subprocess
import sys
import time
import urllib.request
from abc import ABC, abstractmethod
from typing import Any, Callable, Dict, Optional

import shutil

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.judgment import Cell
from cynic.core.phi import PHI_INV, PHI_INV_3, fibonacci

logger = logging.getLogger("cynic.perceive")


# ════════════════════════════════════════════════════════════════════════════
# ABSTRACT BASE
# ════════════════════════════════════════════════════════════════════════════

class PerceiveWorker(ABC):
    """
    Autonomous sensor that generates PerceptionEvents for the Scheduler.

    CYNIC starts these as background asyncio tasks in DogScheduler.start().
    Each worker:
      1. Calls sense() to observe its domain
      2. If sense() returns a Cell → calls submit_fn(cell)
      3. Sleeps interval_s seconds
      4. Repeats until task is cancelled

    Usage:
        worker = GitWatcher()
        task = asyncio.ensure_future(worker.run(scheduler.submit))
    """

    #: Target queue level for submitted cells (override in subclasses)
    level: ConsciousnessLevel = ConsciousnessLevel.REFLEX

    #: Seconds between sense() calls (Fibonacci-derived)
    interval_s: float = float(fibonacci(5))  # 5.0s default

    #: Short name for logging and source tagging
    name: str = "perceive_worker"

    @abstractmethod
    async def sense(self) -> Optional[Cell]:
        """
        Observe the worker's domain.

        Returns a Cell if something worth judging was detected, else None.
        Must be non-blocking (use run_in_executor for blocking calls).
        CancelledError must propagate (do not catch it here).
        """
        ...

    async def run(self, submit_fn: Callable) -> None:
        """
        Main loop: sense → submit → sleep → repeat.

        submit_fn: DogScheduler.submit (or any callable matching its signature)
        Runs as a background asyncio task; exits cleanly on CancelledError.
        """
        logger.info(
            "PerceiveWorker %s started (interval=%.1fs, level=%s)",
            self.name, self.interval_s, self.level.name,
        )
        while True:
            try:
                cell = await self.sense()
                if cell is not None:
                    submitted = submit_fn(
                        cell,
                        level=self.level,
                        budget_usd=cell.budget_usd,
                        source=self.name,
                    )
                    if submitted:
                        logger.debug(
                            "%s: submitted %s to %s",
                            self.name, cell.cell_id[:8], self.level.name,
                        )
                    else:
                        logger.debug("%s: queue full, cell dropped", self.name)
            except asyncio.CancelledError:
                logger.info("PerceiveWorker %s cancelled", self.name)
                return
            except Exception as exc:
                logger.warning("PerceiveWorker %s sense() error: %s", self.name, exc)

            # CancelledError raised here exits the loop cleanly
            await asyncio.sleep(self.interval_s)

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(interval={self.interval_s}s, level={self.level.name})"


# ════════════════════════════════════════════════════════════════════════════
# GIT WATCHER — CODE×PERCEIVE/REFLEX every 5s
# ════════════════════════════════════════════════════════════════════════════

class GitWatcher(PerceiveWorker):
    """
    Monitors git working tree for uncommitted changes.

    Submits CODE×PERCEIVE at REFLEX level when new changes are detected.
    Deduplicates: only submits when the change set actually differs from last check.

    interval: F(5)=5s — git status is fast (<50ms), harmless to run often.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(5))   # 5.0s
    name = "git_watcher"

    def __init__(self, cwd: Optional[str] = None) -> None:
        self._cwd = cwd
        self._last_hash: Optional[int] = None

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        try:
            result = await loop.run_in_executor(
                None,
                lambda: subprocess.run(
                    ["git", "status", "--porcelain"],
                    capture_output=True, text=True, timeout=3.0,
                    cwd=self._cwd,
                ),
            )
        except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
            return None

        if result.returncode != 0 or not result.stdout.strip():
            self._last_hash = None
            return None

        changes = result.stdout.strip()
        change_hash = hash(changes)
        if change_hash == self._last_hash:
            return None     # No new changes since last check

        self._last_hash = change_hash
        lines = changes.splitlines()

        return Cell(
            reality="CODE",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={"git_status": changes[:1500], "changed_files": len(lines)},
            context=f"Git watcher: {len(lines)} changed file(s) detected",
            risk=0.0,
            complexity=min(len(lines) / 50.0, 1.0),
            budget_usd=0.001,
            metadata={"source": "git_watcher", "file_count": len(lines)},
        )


# ════════════════════════════════════════════════════════════════════════════
# HEALTH WATCHER — CYNIC×PERCEIVE/REFLEX every 21s
# ════════════════════════════════════════════════════════════════════════════

class HealthWatcher(PerceiveWorker):
    """
    Monitors CycleTimer health across all 4 consciousness levels.

    Submits CYNIC×PERCEIVE at REFLEX level only when a timer is
    DEGRADED or CRITICAL (not on every tick — only on bad news).
    This closes the self-monitoring loop: "CYNIC sees its own slowness."

    interval: F(8)=21s — enough resolution to detect degradation early.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(8))   # 21.0s
    name = "health_watcher"

    def __init__(self, get_consciousness_fn: Optional[Callable] = None) -> None:
        self._get_consciousness = get_consciousness_fn or get_consciousness

    async def sense(self) -> Optional[Cell]:
        consciousness = self._get_consciousness()
        degraded: Dict[str, Any] = {
            name: timer.to_dict()
            for name, timer in consciousness.timers.items()
            if timer.health in ("DEGRADED", "CRITICAL")
        }

        if not degraded:
            return None

        severity_rank = {"EXCELLENT": 0, "GOOD": 1, "UNKNOWN": 1, "DEGRADED": 2, "CRITICAL": 3}
        worst = max(
            consciousness.timers.values(),
            key=lambda t: severity_rank.get(t.health, 0),
        )

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "degraded_levels": degraded,
                "worst_health": worst.health,
                "worst_p95_ms": worst.p95_ms,
                "total_cycles": consciousness.total_cycles,
            },
            context=(
                f"Health watcher: {len(degraded)} level(s) degraded "
                f"— worst={worst.health} p95={worst.p95_ms:.0f}ms"
            ),
            risk=0.2 if worst.health == "DEGRADED" else 0.5,
            complexity=0.3,
            budget_usd=0.001,
            metadata={"source": "health_watcher", "degraded_count": len(degraded)},
        )


# ════════════════════════════════════════════════════════════════════════════
# SELF WATCHER — CYNIC×LEARN/MICRO every 55s
# ════════════════════════════════════════════════════════════════════════════

class SelfWatcher(PerceiveWorker):
    """
    CYNIC observes its own Q-Table learning health.

    Submits CYNIC×LEARN at MICRO level every ~55s.
    Creates a self-judgment loop: "How well am I learning?"
    The judgment system judges its own learning state → feeds more Q-Learning.

    interval: F(10)=55s — ~1 minute is the right granularity for learning checks.
    """

    level = ConsciousnessLevel.MICRO
    interval_s = float(fibonacci(10))  # 55.0s
    name = "self_watcher"

    def __init__(self, qtable_getter: Optional[Callable] = None) -> None:
        self._qtable_getter = qtable_getter

    async def sense(self) -> Optional[Cell]:
        if self._qtable_getter is None:
            return None

        try:
            qtable = self._qtable_getter()
            stats = qtable.stats()
        except Exception:
            return None

        return Cell(
            reality="CYNIC",
            analysis="LEARN",
            time_dim="PRESENT",
            content={
                "states": stats.get("states", 0),
                "total_updates": stats.get("total_updates", 0),
                "pending_flush": stats.get("pending_flush", 0),
                "max_confidence": stats.get("max_confidence", 0.0),
                "unique_states": stats.get("unique_states", 0),
            },
            context=(
                f"Self-watcher: {stats.get('states', 0)} states learned, "
                f"{stats.get('total_updates', 0)} total updates"
            ),
            risk=0.0,
            complexity=0.2,
            budget_usd=0.003,
            metadata={"source": "self_watcher"},
        )


# ════════════════════════════════════════════════════════════════════════════
# MARKET WATCHER — MARKET×PERCEIVE/REFLEX every 34s
# ════════════════════════════════════════════════════════════════════════════

_COINGECKO_URL = (
    "https://api.coingecko.com/api/v3/simple/price"
    "?ids=solana&vs_currencies=usd&include_24hr_change=true"
)
_HTTP_TIMEOUT = 5.0         # seconds
_MARKET_MOVE_THRESHOLD = 0.02  # 2% price move triggers perception


class MarketWatcher(PerceiveWorker):
    """
    Monitors SOL/USD price via CoinGecko public API (no key needed).

    Submits MARKET×PERCEIVE at REFLEX level only on significant moves
    (>2% from last observed price) — not on every tick.

    Graceful degradation: returns None on network errors or rate limits.
    interval: F(9)=34s — respectful of CoinGecko free tier rate limits.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))   # 34.0s
    name = "market_watcher"

    def __init__(self) -> None:
        self._last_price: Optional[float] = None
        self._consecutive_errors: int = 0

    def _fetch_price(self) -> Optional[Dict[str, Any]]:
        """Blocking fetch — called via run_in_executor."""
        try:
            req = urllib.request.Request(
                _COINGECKO_URL,
                headers={"User-Agent": "CYNIC-MarketWatcher/2.0"},
            )
            with urllib.request.urlopen(req, timeout=_HTTP_TIMEOUT) as resp:
                data = json.loads(resp.read().decode())
            sol = data.get("solana", {})
            return {
                "price_usd": float(sol.get("usd", 0)),
                "change_24h": float(sol.get("usd_24h_change", 0)),
            }
        except Exception:
            return None

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        result = await loop.run_in_executor(None, self._fetch_price)

        if result is None:
            self._consecutive_errors += 1
            return None

        self._consecutive_errors = 0
        price = result["price_usd"]
        change_24h = result["change_24h"]

        if price <= 0:
            return None

        # Only emit when price moved significantly from last check
        if self._last_price is not None:
            move = abs(price - self._last_price) / self._last_price
            if move < _MARKET_MOVE_THRESHOLD and abs(change_24h) < 5.0:
                self._last_price = price
                return None

        self._last_price = price
        volatility = min(abs(change_24h) / 20.0, 1.0)  # normalize to [0,1]

        return Cell(
            reality="MARKET",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "sol_usd": price,
                "change_24h_pct": round(change_24h, 4),
            },
            context=(
                f"Market watcher: SOL=${price:.2f} "
                f"({change_24h:+.2f}% 24h)"
            ),
            risk=volatility,
            complexity=0.2,
            budget_usd=0.001,
            metadata={"source": "market_watcher", "price_usd": price},
        )


# ════════════════════════════════════════════════════════════════════════════
# SOLANA WATCHER — SOLANA×PERCEIVE/REFLEX every 34s
# ════════════════════════════════════════════════════════════════════════════

# ════════════════════════════════════════════════════════════════════════════
# SOCIAL WATCHER — SOCIAL×PERCEIVE/MICRO every 89s
# ════════════════════════════════════════════════════════════════════════════

import os as _os

_SOCIAL_SIGNAL_PATH = _os.path.join(_os.path.expanduser("~"), ".cynic", "social.json")


class SocialWatcher(PerceiveWorker):
    """
    Monitors social signals from the ~/.cynic/social.json feed.

    JS hooks (observe.js) or external scripts write social signals here.
    SocialWatcher reads them and submits SOCIAL×PERCEIVE at MICRO level.

    signal.json schema:
      {"ts": 1234567890, "source": "twitter|discord|reddit", "sentiment": -1.0..1.0,
       "volume": 0..100, "topic": "...", "signal_type": "...", "read": false}

    Only processes signals where read=false (marks them as read after submission).

    interval: F(11)=89s — social signals are slow-moving.
    API-key-free: reads a local file written by any JS/Python hook.
    """

    level = ConsciousnessLevel.MICRO
    interval_s = float(fibonacci(11))  # 89.0s
    name = "social_watcher"

    def __init__(self, signal_path: Optional[str] = None) -> None:
        self._path = signal_path or _SOCIAL_SIGNAL_PATH
        self._last_ts: float = 0.0

    def _read_signal(self) -> Optional[Dict[str, Any]]:
        """Blocking read — called via run_in_executor."""
        try:
            if not _os.path.exists(self._path):
                return None
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            # Accept both single signal and list; take first unread
            signals = data if isinstance(data, list) else [data]
            for sig in signals:
                if not sig.get("read", False) and sig.get("ts", 0) > self._last_ts:
                    return sig
            return None
        except Exception:
            return None

    def _mark_read(self, ts: float) -> None:
        """Mark the signal as read so we don't re-submit it."""
        try:
            if not _os.path.exists(self._path):
                return
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            signals = data if isinstance(data, list) else [data]
            for sig in signals:
                if sig.get("ts") == ts:
                    sig["read"] = True
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump(signals if isinstance(data, list) else signals[0], fh)
        except Exception:
            pass

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        sig = await loop.run_in_executor(None, self._read_signal)
        if sig is None:
            return None

        ts = float(sig.get("ts", time.time()))
        self._last_ts = ts
        await loop.run_in_executor(None, self._mark_read, ts)

        sentiment = float(sig.get("sentiment", 0.0))
        volume = float(sig.get("volume", 0.0))
        source = sig.get("source", "unknown")

        # Positive sentiment → low risk; negative → higher risk
        risk = max(0.0, min(1.0, (0.5 - sentiment * 0.5)))

        return Cell(
            reality="SOCIAL",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "source": source,
                "sentiment": round(sentiment, 3),
                "volume": round(volume, 1),
                "topic": sig.get("topic", ""),
                "signal_type": sig.get("signal_type", "mention"),
            },
            context=(
                f"Social watcher: {source} sentiment={sentiment:+.2f} "
                f"volume={volume:.0f} topic={sig.get('topic', '')}"
            ),
            risk=risk,
            complexity=0.3,
            budget_usd=0.002,
            metadata={"source": "social_watcher", "social_source": source, "ts": ts},
        )


_SOLANA_RPC_URL = "https://api.mainnet-beta.solana.com"
_SOLANA_RPC_HEADERS = {"Content-Type": "application/json"}
_TPS_WARNING_THRESHOLD = 1000   # below this → slow network
_SLOT_LAG_WARNING = 10          # >10 behind tip → lagging


def _rpc_call(method: str, params: list) -> Optional[Dict[str, Any]]:
    """Blocking Solana JSON-RPC call — called via run_in_executor."""
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params})
    try:
        req = urllib.request.Request(
            _SOLANA_RPC_URL,
            data=body.encode(),
            headers=_SOLANA_RPC_HEADERS,
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=_HTTP_TIMEOUT) as resp:
            return json.loads(resp.read().decode()).get("result")
    except Exception:
        return None


class SolanaWatcher(PerceiveWorker):
    """
    Monitors Solana mainnet health via public RPC (no API key needed).

    Checks:
    - Current slot (liveness)
    - Recent performance samples (TPS)

    Submits SOLANA×PERCEIVE at REFLEX level on anomalies:
    - TPS < 1000 (slow network)
    - Slot appears stuck (same as last check)

    Graceful degradation: returns None on RPC errors.
    interval: F(9)=34s — same cadence as MarketWatcher.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))   # 34.0s
    name = "solana_watcher"

    def __init__(self, rpc_url: Optional[str] = None) -> None:
        global _SOLANA_RPC_URL
        if rpc_url:
            _SOLANA_RPC_URL = rpc_url
        self._last_slot: Optional[int] = None

    def _fetch_chain_state(self) -> Optional[Dict[str, Any]]:
        """Blocking — calls getSlot + getRecentPerformanceSamples."""
        slot = _rpc_call("getSlot", [])
        if slot is None:
            return None

        samples = _rpc_call("getRecentPerformanceSamples", [1]) or []
        tps = 0.0
        if samples:
            s = samples[0]
            elapsed = s.get("samplePeriodSecs", 1) or 1
            tps = s.get("numTransactions", 0) / elapsed

        return {"slot": slot, "tps": round(tps, 1)}

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        state = await loop.run_in_executor(None, self._fetch_chain_state)

        if state is None:
            return None

        slot = state["slot"]
        tps = state["tps"]

        slot_stuck = self._last_slot is not None and slot == self._last_slot
        low_tps = tps > 0 and tps < _TPS_WARNING_THRESHOLD
        self._last_slot = slot

        # Only emit on anomalies
        if not slot_stuck and not low_tps:
            return None

        issues = []
        if slot_stuck:
            issues.append(f"slot stuck at {slot}")
        if low_tps:
            issues.append(f"low TPS={tps:.0f}")
        risk = 0.4 if slot_stuck else 0.2

        return Cell(
            reality="SOLANA",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "slot": slot,
                "tps": tps,
                "slot_stuck": slot_stuck,
                "low_tps": low_tps,
            },
            context=f"Solana watcher anomaly: {', '.join(issues)}",
            risk=risk,
            complexity=0.3,
            budget_usd=0.001,
            metadata={"source": "solana_watcher", "slot": slot, "tps": tps},
        )


# ════════════════════════════════════════════════════════════════════════════
# DISK WATCHER — CYNIC×PERCEIVE/REFLEX every 34s
# ════════════════════════════════════════════════════════════════════════════

# φ-derived disk usage thresholds (fraction of disk used)
_DISK_WARN      = PHI_INV        # 0.618 — 61.8% full → GROWL / LOD 1
_DISK_CRITICAL  = 1 - PHI_INV_3  # 0.764 — 76.4% full → LOD 2
_DISK_EMERGENCY = 0.90            # 90%   full → BARK  / LOD 3


class DiskWatcher(PerceiveWorker):
    """
    Monitors disk usage via shutil.disk_usage() (stdlib, no psutil needed).

    Submits CYNIC×PERCEIVE at REFLEX when disk exceeds φ-thresholds.
    Also emits DISK_PRESSURE on the core bus → triggers StorageGC.

    Deduplicates: only emits when the pressure level CHANGES
    (WARN → CRITICAL → EMERGENCY, or back to OK).

    Thresholds (φ-derived, fraction of disk used):
      WARN      ≥ 0.618 (61.8%) → LOD 1, GC pre-warm
      CRITICAL  ≥ 0.764 (76.4%) → LOD 2, GC aggressive
      EMERGENCY ≥ 0.90  (90%)   → LOD 3, BARK

    interval: F(9)=34s — same cadence as MarketWatcher/SolanaWatcher.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))   # 34.0s
    name = "disk_watcher"

    def __init__(self, path: Optional[str] = None) -> None:
        # Monitor the drive where ~/.cynic lives (covers DB data dir too)
        self._path = path or _os.path.expanduser("~")
        self._last_level: Optional[str] = None   # deduplicate emissions

    def _check_disk(self) -> Dict[str, Any]:
        """Blocking disk check — called via run_in_executor."""
        usage = shutil.disk_usage(self._path)
        used_pct = usage.used / usage.total
        free_gb  = usage.free  / (1024 ** 3)
        total_gb = usage.total / (1024 ** 3)

        if used_pct >= _DISK_EMERGENCY:
            pressure = "EMERGENCY"
        elif used_pct >= _DISK_CRITICAL:
            pressure = "CRITICAL"
        elif used_pct >= _DISK_WARN:
            pressure = "WARN"
        else:
            pressure = "OK"

        return {
            "used_pct": used_pct,
            "free_gb":  free_gb,
            "total_gb": total_gb,
            "pressure": pressure,
        }

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        try:
            info = await loop.run_in_executor(None, self._check_disk)
        except Exception as exc:
            logger.debug("DiskWatcher check failed: %s", exc)
            return None

        pressure = info["pressure"]

        if pressure == "OK":
            if self._last_level is not None:
                logger.info("DiskWatcher: disk pressure cleared (was %s)", self._last_level)
            self._last_level = None
            return None

        # Deduplicate — only emit when level changes
        if pressure == self._last_level:
            return None
        self._last_level = pressure

        used_pct = info["used_pct"]
        free_gb  = info["free_gb"]

        # Emit DISK_PRESSURE on core bus → StorageGC reacts
        await get_core_bus().emit(Event(
            type=CoreEvent.DISK_PRESSURE,
            payload={
                "used_pct":  round(used_pct, 4),
                "free_gb":   round(free_gb, 2),
                "pressure":  pressure,
                "disk_pct":  round(used_pct, 4),  # alias for LODController
            },
            source="disk_watcher",
        ))

        risk = {
            "WARN":      0.4,
            "CRITICAL":  0.7,
            "EMERGENCY": 1.0,
        }.get(pressure, 0.4)

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "disk_used_pct":  round(used_pct * 100, 1),
                "disk_free_gb":   round(free_gb, 2),
                "disk_total_gb":  round(info["total_gb"], 2),
                "disk_pressure":  pressure,
            },
            context=(
                f"Disk watcher: {used_pct * 100:.1f}% full "
                f"({free_gb:.1f} GB free) — {pressure}"
            ),
            risk=risk,
            complexity=0.2,
            budget_usd=0.001,
            metadata={"source": "disk_watcher", "disk_pressure": pressure},
        )


# ════════════════════════════════════════════════════════════════════════════
# MEMORY WATCHER — CYNIC×PERCEIVE/REFLEX every 34s
# ════════════════════════════════════════════════════════════════════════════

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
        self._last_level: Optional[str] = None

    def _check_memory(self) -> Optional[Dict[str, Any]]:
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

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        info = await loop.run_in_executor(None, self._check_memory)

        if info is None:
            return None

        pressure = info["pressure"]

        if pressure == "OK":
            if self._last_level is not None:
                logger.info("MemoryWatcher: RAM pressure cleared (was %s)", self._last_level)
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

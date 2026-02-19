"""CYNIC SolanaWatcher — SOLANA×PERCEIVE/REFLEX every F(9)=34s."""
from __future__ import annotations

import asyncio
import json
import urllib.request
from typing import Any


from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.perceive.workers.base import PerceiveWorker

_SOLANA_RPC_URL = "https://api.mainnet-beta.solana.com"
_SOLANA_RPC_HEADERS = {"Content-Type": "application/json"}
_TPS_WARNING_THRESHOLD = 1000   # below this → slow network
_SLOT_LAG_WARNING = 10          # >10 behind tip → lagging
_HTTP_TIMEOUT = 5.0


def _rpc_call(method: str, params: list) -> dict[str, Any] | None:
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

    def __init__(self, rpc_url: str | None = None) -> None:
        global _SOLANA_RPC_URL
        if rpc_url:
            _SOLANA_RPC_URL = rpc_url
        self._last_slot: int | None = None

    def _fetch_chain_state(self) -> dict[str, Any] | None:
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

    async def sense(self) -> Cell | None:
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

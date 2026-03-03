"""CYNIC SolanaWatcher " SOLANA-PERCEIVE/REFLEX every F(9)=34s."""

from __future__ import annotations

import asyncio
import json
import logging
import urllib.request
from typing import Any

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import fibonacci
from cynic.kernel.organism.perception.senses.workers.base import PerceiveWorker

logger = logging.getLogger(__name__)


def _get_helius_api_key() -> str:
    """Get Helius API key from unified config (Rule 3: single source of truth)."""
    try:
        from cynic.kernel.core.config import CynicConfig
        config = CynicConfig.from_env()
        return config.helius_api_key or ""
    except Exception:
        return ""


def _build_solana_rpc_urls() -> list[str]:
    """Build Solana RPC URL list with optional Helius priority."""
    helius_key = _get_helius_api_key()
    if helius_key:
        return [f"https://mainnet.helius-rpc.com/?api-key={helius_key}"]
    else:
        # Fallback to public RPCs (rate limited)
        return [
            "https://api.mainnet-beta.solana.com",
            "https://solana-api.projectserum.com",
            "https://rpc.ankr.com/solana",
        ]


_SOLANA_RPC_URLS = _build_solana_rpc_urls()
_SOLANA_RPC_HEADERS = {
    "Content-Type": "application/json",
    "User-Agent": "CYNIC/1.0 SolanaWatcher",
}
_TPS_WARNING_THRESHOLD = 1000  # below this ' slow network
_SLOT_LAG_WARNING = 10  # >10 behind tip ' lagging
_HTTP_TIMEOUT = 5.0

# Track current RPC index for round-robin fallback
_rpc_index = 0


def _get_rpc_url() -> str:
    """Get current RPC URL with round-robin fallback."""
    global _rpc_index
    return _SOLANA_RPC_URLS[_rpc_index % len(_SOLANA_RPC_URLS)]


def _rotate_rpc() -> None:
    """Rotate to next RPC endpoint on failure."""
    global _rpc_index
    _rpc_index = (_rpc_index + 1) % len(_SOLANA_RPC_URLS)
    logger.debug("Rotated to RPC index %d: %s", _rpc_index, _get_rpc_url())


def _rpc_call(method: str, params: list) -> dict[str, Any] | None:
    """Blocking Solana JSON-RPC call " called via run_in_executor."""
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params})

    # Try each RPC endpoint until one works
    for _ in range(len(_SOLANA_RPC_URLS)):
        rpc_url = _get_rpc_url()
        try:
            req = urllib.request.Request(
                rpc_url,
                data=body.encode(),
                headers=_SOLANA_RPC_HEADERS,
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=_HTTP_TIMEOUT) as resp:
                return json.loads(resp.read().decode()).get("result")
        except urllib.error.HTTPError as e:
            if e.code == 403:
                # 403 Forbidden - likely rate limited, try next RPC
                logger.warning(
                    "Solana RPC 403 Forbidden at %s, trying next endpoint",
                    rpc_url,
                )
                _rotate_rpc()
                continue
            else:
                logger.warning("Solana RPC HTTP error %d at %s: %s", e.code, rpc_url, e)
                _rotate_rpc()
                continue
        except urllib.error.URLError as e:
            logger.warning("Solana RPC connection error at %s: %s", rpc_url, e)
            _rotate_rpc()
            continue
        except json.JSONDecodeError as e:
            logger.warning("Solana RPC JSON decode error at %s: %s", rpc_url, e)
            _rotate_rpc()
            continue

    # All RPCs failed
    logger.error("All Solana RPC endpoints failed after %d attempts", len(_SOLANA_RPC_URLS))
    return None


class SolanaWatcher(PerceiveWorker):
    """
    Monitors Solana mainnet health via public RPC (no API key needed).

    Checks:
    - Current slot (liveness)
    - Recent performance samples (TPS)

    Submits SOLANA-PERCEIVE at REFLEX level on anomalies:
    - TPS < 1000 (slow network)
    - Slot appears stuck (same as last check)

    Graceful degradation: returns None on RPC errors.
    interval: F(9)=34s " same cadence as MarketWatcher.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))  # 34.0s
    name = "solana_watcher"

    def __init__(self, rpc_url: str | None = None) -> None:
        # rpc_url param kept for API compatibility but now appends to fallback list
        global _SOLANA_RPC_URLS
        if rpc_url and rpc_url not in _SOLANA_RPC_URLS:
            _SOLANA_RPC_URLS.insert(0, rpc_url)  # User-provided URL takes priority
        self._last_slot: int | None = None

    def _fetch_chain_state(self) -> dict[str, Any] | None:
        """Blocking " calls getSlot + getRecentPerformanceSamples."""
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

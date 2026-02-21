"""CYNIC MarketWatcher — MARKET×PERCEIVE/REFLEX every F(9)=34s."""
from __future__ import annotations

import asyncio
import json
import urllib.request
from typing import Any


from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.senses.workers.base import PerceiveWorker

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

    def _fetch_price(self) -> Optional[dict[str, Any]]:
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
        except json.JSONDecodeError:
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

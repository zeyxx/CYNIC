"""
Cashtag → Mint resolver. File cache + Helius searchAssets fallback.

Tier 2 INFRASTRUCTURE: consumed by convergence detector.
K15 consumer: convergence summaries use resolved_mint for kernel enrichment cross-reference.
"""
import json
import logging
from pathlib import Path
from typing import Optional

logger = logging.getLogger("cashtag-resolver")

# Well-known mappings that never change
WELL_KNOWN = {
    "SOL": "So11111111111111111111111111111111",
    "USDC": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "USDT": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
}

AMBIGUOUS = "AMBIGUOUS"


class CashtagResolver:
    def __init__(self, cache: dict[str, str] | None = None, cache_path: Path | None = None):
        self._cache: dict[str, str] = {**WELL_KNOWN}
        if cache:
            self._cache.update(cache)
        self._cache_path = cache_path

    def resolve(self, symbol: str) -> Optional[str]:
        """Resolve cashtag to mint. Returns None if unknown or ambiguous."""
        symbol = symbol.lstrip("$").upper()
        mint = self._cache.get(symbol)
        if mint == AMBIGUOUS:
            return None
        return mint

    def add(self, symbol: str, mint: str):
        """Add a resolved mapping to cache."""
        symbol = symbol.lstrip("$").upper()
        self._cache[symbol] = mint

    def mark_ambiguous(self, symbol: str):
        """Mark a symbol as ambiguous (multiple mints)."""
        symbol = symbol.lstrip("$").upper()
        self._cache[symbol] = AMBIGUOUS

    def save(self):
        """Persist cache to disk (if cache_path set)."""
        if self._cache_path:
            self._cache_path.write_text(json.dumps(self._cache, indent=2))

    @classmethod
    def load(cls, cache_path: Path) -> "CashtagResolver":
        """Load from file, or create empty."""
        cache = {}
        if cache_path.exists():
            try:
                cache = json.loads(cache_path.read_text())
            except (json.JSONDecodeError, OSError) as e:
                logger.warning("cache load failed (%s), starting fresh", e)
        return cls(cache=cache, cache_path=cache_path)

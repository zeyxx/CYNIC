"""
Cashtag → Mint resolver. File cache + Helius searchAssets fallback.

Tier 2 INFRASTRUCTURE: consumed by convergence detector + ingest daemon.
K15 consumer: convergence summaries use resolved_mint for kernel enrichment cross-reference.
"""
import json
import logging
import os
import urllib.request
import urllib.error
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

# Jupiter verified token list — public API, no auth
_JUPITER_TOKEN_URL = "https://tokens.jup.ag/token/{mint}"
_JUPITER_STRICT_URL = "https://tokens.jup.ag/tokens?tags=verified"

# In-memory index built on first lookup (lazy)
_jup_index: dict[str, str] | None = None


def _build_jupiter_index() -> dict[str, str]:
    """Fetch Jupiter verified token list, build symbol→mint index."""
    global _jup_index
    if _jup_index is not None:
        return _jup_index

    _jup_index = {}
    req = urllib.request.Request(_JUPITER_STRICT_URL, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            tokens = json.loads(resp.read())
            for token in tokens:
                sym = token.get("symbol", "").upper()
                mint = token.get("address", "")
                if sym and mint:
                    if sym in _jup_index:
                        # Multiple mints for same symbol — mark ambiguous
                        _jup_index[sym] = AMBIGUOUS
                    else:
                        _jup_index[sym] = mint
            logger.info("Jupiter index: %d symbols loaded", len(_jup_index))
    except (urllib.error.URLError, OSError, json.JSONDecodeError) as e:
        logger.warning("Jupiter token list fetch failed: %s", e)
        _jup_index = {}
    return _jup_index


def _resolve_via_jupiter(symbol: str) -> Optional[str]:
    """Resolve symbol to mint via Jupiter verified token list."""
    index = _build_jupiter_index()
    mint = index.get(symbol)
    if mint == AMBIGUOUS:
        return None
    return mint


class CashtagResolver:
    def __init__(self, cache: dict[str, str] | None = None, cache_path: Path | None = None):
        self._cache: dict[str, str] = {**WELL_KNOWN}
        if cache:
            self._cache.update(cache)
        self._cache_path = cache_path

    def resolve(self, symbol: str) -> Optional[str]:
        """Resolve cashtag to mint. Cache first, then Jupiter verified list fallback."""
        symbol = symbol.lstrip("$").upper()
        mint = self._cache.get(symbol)
        if mint == AMBIGUOUS:
            return None
        if mint:
            return mint

        # Jupiter verified token list fallback
        mint = _resolve_via_jupiter(symbol)
        if mint:
            self.add(symbol, mint)
            self.save()
            logger.info("resolved %s → %s via Jupiter", symbol, mint)
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

"""Compatibility facade for the Helius wallet collector."""

# Tier 3
from __future__ import annotations

import os

from heuristics.wallet_behavior_helius import *  # noqa: F401,F403
from heuristics.wallet_behavior_helius import HeliusWalletCollector as _BaseHeliusWalletCollector


class HeliusWalletCollector(_BaseHeliusWalletCollector):
    """Compatibility wrapper that tolerates missing API keys during offline tests."""

    def __init__(self, api_key: str | None = None, timeout: int = 30):
        self.api_key = api_key or os.getenv("HELIUS_API_KEY") or ""
        self.rpc_url = "https://mainnet.helius-rpc.com"
        self.rest_url = "https://api.helius.xyz/v1"
        self.timeout = timeout

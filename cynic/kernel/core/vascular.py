"""
CYNIC Vascular System — Optimized HTTP/IO Connection Pooling.

Centralizes all network IO to prevent socket exhaustion and handshake overhead.
Lentilles : Backend (Pooling), SRE (Resilience), Security (Centralized TLS).
"""

from __future__ import annotations

import logging
import httpx
import asyncio
from typing import Optional

logger = logging.getLogger("cynic.kernel.vascular")

class VascularSystem:
    """
    The 'Vascular System' of CYNIC. 
    Manages a persistent, optimized HTTP connection pool for the entire Organism.
    """

    def __init__(self, instance_id: str, timeout: float = 30.0):
        self.instance_id = instance_id
        self._timeout = timeout
        self._client: Optional[httpx.AsyncClient] = None
        self._lock = asyncio.Lock()

    async def get_client(self) -> httpx.AsyncClient:
        """Get or initialize the shared persistent client."""
        async with self._lock:
            if self._client is None or self._client.is_closed:
                # Optimized pool for high-concurrency LLM/Blockchain calls
                limits = httpx.Limits(
                    max_connections=100, 
                    max_keepalive_connections=20,
                    keepalive_expiry=30.0
                )
                self._client = httpx.AsyncClient(
                    timeout=self._timeout,
                    limits=limits,
                    headers={"X-Cynic-Instance": self.instance_id},
                    follow_redirects=True
                )
                logger.info(f"[{self.instance_id}] Vascular System: Shared IO client initialized.")
            return self._client

    async def close(self):
        """Gracefully close all connections in the pool."""
        async with self._lock:
            if self._client and not self._client.is_closed:
                await self._client.aclose()
                logger.info(f"[{self.instance_id}] Vascular System: IO pool closed.")

"""
CYNIC Vascular System — Optimized Resource Pooling (HTTP & Redis).

Centralizes all network IO to prevent resource exhaustion.
Acts as the physical transport layer for the nervous system.

Lentilles : Backend (Pooling), SRE (Resilience), Security (Centralized Connection).
"""

from __future__ import annotations

import logging
import httpx
import asyncio
import redis.asyncio as redis
from typing import Optional

logger = logging.getLogger("cynic.kernel.vascular")

class VascularSystem:
    """
    The 'Vascular System' of CYNIC. 
    Manages persistent connection pools for HTTP and Redis.
    """

    def __init__(self, instance_id: str, redis_url: str = "redis://localhost:6379/0", timeout: float = 30.0):
        self.instance_id = instance_id
        self._redis_url = redis_url
        self._timeout = timeout
        
        # Connection Pools
        self._http_client: Optional[httpx.AsyncClient] = None
        self._redis_client: Optional[redis.Redis] = None
        
        self._lock = asyncio.Lock()

    async def get_client(self) -> httpx.AsyncClient:
        """Get or initialize the shared persistent HTTP client."""
        async with self._lock:
            if self._http_client is None or self._http_client.is_closed:
                limits = httpx.Limits(
                    max_connections=100, 
                    max_keepalive_connections=20,
                    keepalive_expiry=30.0
                )
                self._http_client = httpx.AsyncClient(
                    timeout=self._timeout,
                    limits=limits,
                    headers={"X-Cynic-Instance": self.instance_id},
                    follow_redirects=True
                )
                logger.info(f"[{self.instance_id}] Vascular: HTTP pool initialized.")
            return self._http_client

    async def get_redis(self) -> redis.Redis:
        """Get or initialize the shared Redis client."""
        async with self._lock:
            if self._redis_client is None:
                self._redis_client = redis.from_url(
                    self._redis_url,
                    decode_responses=True,
                    socket_timeout=self._timeout,
                    retry_on_timeout=True
                )
                # Verify connection
                await self._redis_client.ping()
                logger.info(f"[{self.instance_id}] Vascular: Redis conduit active on {self._redis_url}")
            return self._redis_client

    async def close(self):
        """Gracefully close all connection pools."""
        async with self._lock:
            if self._http_client and not self._http_client.is_closed:
                await self._http_client.aclose()
                logger.info(f"[{self.instance_id}] Vascular: HTTP pool closed.")
            
            if self._redis_client:
                await self._redis_client.close()
                logger.info(f"[{self.instance_id}] Vascular: Redis conduit closed.")

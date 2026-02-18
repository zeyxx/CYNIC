"""
CYNIC Embedding Provider — Dense Vector Representations for ScholarDog

Provides a unified interface for generating text embeddings used in
PostgreSQL pgvector semantic search (β1 implementation).

Providers:
  OllamaEmbedder  — Real embeddings via Ollama /api/embeddings endpoint
                    (requires Ollama running with an embedding model)
                    Models: nomic-embed-text (768d), all-minilm (384d)

  DummyEmbedder   — Returns zero vectors (for testing/no-Ollama mode)
                    Useful for DB schema testing without real Ollama

Embedding dimension: 384 (all-minilm) — pgvector vector(384) compatible
Fallback dimension:  768 (nomic-embed-text) — also supported

Usage:
    embedder = OllamaEmbedder(base_url="http://localhost:11434", model="nomic-embed-text")
    vector = await embedder.embed("def phi_aggregate(scores): ...")
    # vector: List[float], len=768

    # In ScholarDog:
    scholar.set_embedder(embedder)  # enables vector search path
"""
from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import struct
import time
from abc import ABC, abstractmethod
from typing import List, Optional

import aiohttp

logger = logging.getLogger("cynic.embeddings")

# Default embedding dimensions per model
_MODEL_DIMS = {
    "nomic-embed-text":           768,
    "nomic-embed-text-v1.5":      768,
    "all-minilm":                 384,
    "all-minilm:l6-v2":           384,
    "mxbai-embed-large":          1024,
    "snowflake-arctic-embed":     1024,
}

# Default model used when none specified
DEFAULT_EMBEDDING_MODEL = "nomic-embed-text"
DEFAULT_EMBEDDING_DIM   = 768  # nomic-embed-text dimension


# ── Abstract base ─────────────────────────────────────────────────────────

class EmbeddingProvider(ABC):
    """
    Abstract embedding provider.
    All implementors must return a float list of consistent dimension.
    """

    @abstractmethod
    async def embed(self, text: str) -> List[float]:
        """Generate embedding vector for text. Returns List[float]."""

    @property
    @abstractmethod
    def dimension(self) -> int:
        """Embedding dimension (e.g. 768 for nomic-embed-text)."""

    @abstractmethod
    def is_available(self) -> bool:
        """Whether the provider is currently usable."""


# ── OllamaEmbedder ────────────────────────────────────────────────────────

class OllamaEmbedder(EmbeddingProvider):
    """
    Dense embeddings via Ollama /api/embeddings endpoint.

    Requires Ollama running with an embedding-capable model:
      ollama pull nomic-embed-text
      ollama pull all-minilm

    Timeout: 5s (embedding should be fast for small texts)
    Auto-detects dimension from first successful call.
    """

    def __init__(
        self,
        base_url: str = "http://localhost:11434",
        model: str = DEFAULT_EMBEDDING_MODEL,
        timeout_s: float = 5.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._model = model
        self._timeout = aiohttp.ClientTimeout(total=timeout_s)
        self._dim: int = _MODEL_DIMS.get(model, DEFAULT_EMBEDDING_DIM)
        self._available: bool = True   # Optimistic — flipped on connection error
        self._error_count: int = 0
        self._MAX_ERRORS = 3           # Stop trying after 3 consecutive failures

    @property
    def dimension(self) -> int:
        return self._dim

    def is_available(self) -> bool:
        return self._available and self._error_count < self._MAX_ERRORS

    async def embed(self, text: str) -> List[float]:
        """
        Call Ollama /api/embeddings.
        Returns zero vector on failure (graceful degradation).
        """
        if not self.is_available():
            return [0.0] * self._dim

        url = f"{self._base_url}/api/embeddings"
        payload = {"model": self._model, "prompt": text[:4096]}  # Cap input

        try:
            async with aiohttp.ClientSession(timeout=self._timeout) as session:
                async with session.post(url, json=payload) as resp:
                    if resp.status != 200:
                        self._error_count += 1
                        logger.warning(
                            "OllamaEmbedder: HTTP %d for model=%s",
                            resp.status, self._model,
                        )
                        return [0.0] * self._dim

                    data = await resp.json()
                    vector = data.get("embedding", [])
                    if not vector:
                        self._error_count += 1
                        return [0.0] * self._dim

                    # Auto-detect dimension from first successful response
                    if len(vector) != self._dim:
                        self._dim = len(vector)
                        logger.info(
                            "OllamaEmbedder: auto-detected dim=%d for model=%s",
                            self._dim, self._model,
                        )

                    self._error_count = 0  # Reset on success
                    self._available = True
                    return [float(v) for v in vector]

        except (aiohttp.ClientConnectorError, asyncio.TimeoutError) as e:
            self._error_count += 1
            if self._error_count >= self._MAX_ERRORS:
                self._available = False
                logger.warning(
                    "OllamaEmbedder: disabled after %d errors (last: %s)",
                    self._MAX_ERRORS, e,
                )
            return [0.0] * self._dim
        except Exception as e:
            self._error_count += 1
            logger.debug("OllamaEmbedder error: %s", e)
            return [0.0] * self._dim


# ── DummyEmbedder ────────────────────────────────────────────────────────

class DummyEmbedder(EmbeddingProvider):
    """
    Deterministic pseudo-embeddings via SHA256 hashing.

    Not real semantic embeddings — but reproducible and non-zero.
    Useful for:
      - DB schema testing without a running Ollama
      - CI/CD test pipelines
      - Offline mode

    The hash-derived vector has some locality properties (similar short
    texts with common prefixes have more correlated bytes), but similarity
    is NOT semantically meaningful. For real use, prefer OllamaEmbedder.
    """

    def __init__(self, dim: int = DEFAULT_EMBEDDING_DIM) -> None:
        self._dim = dim

    @property
    def dimension(self) -> int:
        return self._dim

    def is_available(self) -> bool:
        return True

    async def embed(self, text: str) -> List[float]:
        """
        Generate deterministic pseudo-embedding from SHA256 hash.
        Normalized to unit vector.
        """
        # Use SHA256 as seed, then derive `dim` floats
        digest = hashlib.sha256(text.encode("utf-8")).digest()
        # Repeat digest to fill dim floats (4 bytes each = 32 floats per digest)
        raw_bytes = (digest * ((self._dim * 4 // 32) + 1))[: self._dim * 4]
        floats = [
            struct.unpack_from(">f", raw_bytes, i * 4)[0]
            for i in range(self._dim)
        ]
        # L2-normalize to unit vector
        norm = sum(f * f for f in floats) ** 0.5
        if norm < 1e-9:
            return [0.0] * self._dim
        return [f / norm for f in floats]

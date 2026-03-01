"""
CYNIC Universal LLM Adapter â€” OS-level LLM routing.

Unified format for all AI interactions (Local, CLI, Cloud).
Implements hardware-aware discovery and EMA-based benchmarking.
"""

from __future__ import annotations

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

from cynic.kernel.core.formulas import LLM_TIMEOUT_SEC
from cynic.kernel.core.phi import MAX_Q_SCORE, PHI, PHI_INV, weighted_geometric_mean

logger = logging.getLogger("cynic.kernel.organism.brain.llm.adapter")


@dataclass
class LLMRequest:
    """A request to any LLM."""

    prompt: str
    system: str = ""
    max_tokens: int = 2048
    temperature: float = 0.0
    stream: bool = False
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class LLMResponse:
    """Unified response from any LLM."""

    content: str
    model: str
    provider: str
    prompt_tokens: int = 0
    completion_tokens: int = 0
    cost_usd: float = 0.0
    latency_ms: float = 0.0
    error: str | None = None

    @property
    def is_success(self) -> bool:
        return self.error is None and bool(self.content)


class LLMAdapter(ABC):
    """Base for all LLM muscles."""

    def __init__(self, model: str, provider: str) -> None:
        self.model = model
        self.provider = provider

    @property
    def adapter_id(self) -> str:
        return f"{self.provider}:{self.model}"

    @abstractmethod
    async def complete(self, request: LLMRequest) -> LLMResponse: ...

    @abstractmethod
    async def check_available(self) -> bool: ...

    async def complete_safe(self, request: LLMRequest) -> LLMResponse:
        """Contain errors and log call."""
        try:
            return await asyncio.wait_for(self.complete(request), timeout=LLM_TIMEOUT_SEC)
        except Exception as e:
            return LLMResponse(content="", model=self.model, provider=self.provider, error=str(e))


@dataclass
class BenchmarkResult:
    llm_id: str
    dog_id: str
    task_type: str
    quality_score: float
    speed_score: float
    cost_score: float
    timestamp: float = field(default_factory=time.time)

    @property
    def composite_score(self) -> float:
        return weighted_geometric_mean(
            [self.quality_score / MAX_Q_SCORE, self.speed_score, self.cost_score],
            [PHI, 1.0, PHI_INV],
        )


class LLMRegistry:
    """The intelligence warehouse of the organism."""

    def __init__(self) -> None:
        self._adapters: dict[str, LLMAdapter] = {}
        self._available: dict[str, bool] = {}
        self._benchmarks: dict[tuple[str, str, str], BenchmarkResult] = {}
        self._manifest: dict[str, Any] = {}

    def register(self, adapter: LLMAdapter, available: bool = True) -> None:
        self._adapters[adapter.adapter_id] = adapter
        self._available[adapter.adapter_id] = available

    def get_available(self) -> list[LLMAdapter]:
        return [a for aid, a in self._adapters.items() if self._available.get(aid, False)]

    def _is_generation_adapter(self, adapter: LLMAdapter) -> bool:
        name = adapter.model.lower()
        if "embed" in name or "nomic" in name:
            return False
        return True

    def get_available_for_generation(self) -> list[LLMAdapter]:
        return [a for a in self.get_available() if self._is_generation_adapter(a)]

    async def discover(
        self,
        ollama_url: str = "http://localhost:11434",
        claude_api_key: str | None = None,
        google_api_key: str | None = None,
        models_dir: str | None = None,
    ) -> dict[str, Any]:
        """Hardware-aware discovery of all muscles (V3.5)."""
        from cynic.kernel.organism.metabolism.model_profiler import ModelProfiler

        profiler = ModelProfiler()

        self._manifest = {
            "timestamp": time.time(),
            "hardware": profiler.announce_limits(),
            "available": [],
            "rejected": [],
        }

        # 1. Local GGUF (Level 0: Pure Sovereignty)
        if models_dir:
            try:
                from cynic.kernel.organism.brain.llm.adapters.local_gguf import LlamaCppAdapter
                # Placeholder for listing models, in real it scans models_dir
                # ...
            except ImportError:
                pass

        # 2. Local Service (Level 1: Local Network)
        from cynic.kernel.organism.brain.llm.adapters.local_service import OllamaAdapter

        probe = OllamaAdapter(model="probe", base_url=ollama_url)
        if await probe.check_available():
            self.register(probe)
            self._manifest["available"].append("ollama:local_service")

        # 3. CLI Bridges (Level 2: Binary control)
        from cynic.kernel.organism.brain.llm.adapters.cli_bridge import CLIAdapter

        for binary in ["claude", "gemini"]:
            adapter = CLIAdapter(binary=binary, model_alias=f"{binary}-cli")
            if await adapter.check_available():
                self.register(adapter)
                self._manifest["available"].append(adapter.adapter_id)

        # 4. Cloud API (Level 3: External)
        # ...

        return self._manifest

    def get_best_for(self, dog_id: str, task_type: str) -> LLMAdapter | None:
        """Sovereignty-first routing."""
        avail = self.get_available_for_generation()
        if not avail:
            return None

        # Priority: llama_cpp > ollama > cli > cloud
        prio = {
            "llama_cpp": 0,
            "ollama": 1,
            "claude_cli": 2,
            "gemini_cli": 2,
            "anthropic": 3,
            "gemini": 3,
        }
        return sorted(avail, key=lambda a: prio.get(a.provider, 99))[0]


# --- REGISTRY SINGLETON REMOVED ---
# Each Organism instance must manage its own LLMRegistry.

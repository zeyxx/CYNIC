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
from typing import Any, Optional

from cynic.kernel.core.formulas import LLM_TIMEOUT_SEC
from cynic.kernel.core.phi import MAX_Q_SCORE, PHI, PHI_INV, PHI_INV_2, weighted_geometric_mean
from cynic.kernel.core.vascular import VascularSystem
import httpx

logger = logging.getLogger("cynic.kernel.organism.brain.llm.adapter")


@dataclass
class LLMRequest:
    """A request to any LLM."""

    prompt: str
    system: str = ""
    max_tokens: int = 2048
    temperature: float = 0.0
    stream: bool = False
    multimodal_data: list[Any] = field(default_factory=list) # List of MultimodalPacket
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

    def __init__(self, model: str, provider: str, vascular: Optional[VascularSystem] = None) -> None:
        self.model = model
        self.provider = provider
        self.vascular = vascular

    async def _get_client(self) -> httpx.AsyncClient:
        """Helper to get shared client or local fallback."""
        if self.vascular:
            return await self.vascular.get_client()
        # Fallback for tests/unmanaged runs
        return httpx.AsyncClient(timeout=30.0)

    @property
    def adapter_id(self) -> str:
        return f"{self.provider}:{self.model}"

    @property
    def llm_id(self) -> str:
        """Alias for adapter_id to maintain consistency across the organism."""
        return self.adapter_id

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
    error_rate: float = 0.0 # [0, 1] where 1 is total failure
    timestamp: float = field(default_factory=time.time)

    @property
    def composite_score(self) -> float:
        # Quality, Speed, and Cost are rewarded. Error rate is heavily punished.
        base_score = weighted_geometric_mean(
            [max(0.001, self.quality_score / MAX_Q_SCORE), max(0.001, self.speed_score), max(0.001, self.cost_score)],
            [PHI, 1.0, PHI_INV],
        )
        return base_score * (1.0 - self.error_rate)


class LLMRegistry:
    """The intelligence warehouse of the organism."""

    def __init__(self, vascular: Optional[VascularSystem] = None) -> None:
        self.vascular = vascular
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

        # 2. Local Service (Level 1: Local Network - Ollama)
        from cynic.kernel.organism.brain.llm.adapters.local_service import OllamaAdapter
        import httpx

        try:
            # Query Ollama for the list of actually installed models
            async with httpx.AsyncClient(timeout=2.0) as client:
                resp = await client.get(f"{ollama_url}/api/tags")
                if resp.status_code == 200:
                    models_data = resp.json().get("models", [])
                    for m in models_data:
                        m_name = m["name"]
                        adapter = OllamaAdapter(model=m_name, base_url=ollama_url, vascular=self.vascular)
                        self.register(adapter)
                        self._manifest["available"].append(f"ollama:{m_name}")
                    logger.info(f"Discovered {len(models_data)} local models via Ollama.")
        except Exception as e:
            logger.debug(f"Ollama not found at {ollama_url}: {e}")

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
        """
        Dynamic routing based on PHI-weighted performance scores.
        Favors Local/Open-Source models if they meet the quality threshold.
        """
        avail = self.get_available_for_generation()
        if not avail:
            return None

        # 1. Gather all adapters and their latest benchmarks
        scored_adapters = []
        for adapter in avail:
            # Get the last benchmark for this specific dog/task
            bench = self._benchmarks.get((adapter.adapter_id, dog_id, task_type))
            
            if bench:
                score = bench.composite_score
            else:
                # Default scores if no benchmark exists yet
                # We default to high priority for Local/OS to encourage discovery
                if adapter.provider in ["llama_cpp", "ollama"]:
                    score = PHI_INV  # 0.618 (Good starting point)
                else:
                    score = PHI_INV_2 # 0.382 (Conservative for Cloud)
            
            scored_adapters.append((score, adapter))

        # 2. Sort by highest composite score
        # In case of tie, prefer the one with the lowest cost (inherent in composite_score)
        scored_adapters.sort(key=lambda x: x[0], reverse=True)
        
        return scored_adapters[0][1]

    def update_benchmark(self, dog_id: str, task_type: str, llm_id: str, result: BenchmarkResult) -> None:
        """Update the performance record for a specific model+dog+task combination."""
        key = (llm_id, dog_id, task_type)
        
        if key in self._benchmarks:
            # PHI-weighted EMA update: PHI_INV (0.618) old + PHI_INV_2 (0.382) new
            old = self._benchmarks[key]
            self._benchmarks[key] = BenchmarkResult(
                llm_id=llm_id,
                dog_id=dog_id,
                task_type=task_type,
                quality_score=(old.quality_score * PHI_INV) + (result.quality_score * PHI_INV_2),
                speed_score=(old.speed_score * PHI_INV) + (result.speed_score * PHI_INV_2),
                cost_score=(old.cost_score * PHI_INV) + (result.cost_score * PHI_INV_2),
                error_rate=(old.error_rate * PHI_INV) + (result.error_rate * PHI_INV_2),
                timestamp=time.time()
            )
        else:
            self._benchmarks[key] = result
        
        logger.debug(f"Registry: Updated benchmark for {llm_id} ({dog_id}:{task_type})")


# --- REGISTRY SINGLETON REMOVED ---
# Each Organism instance must manage its own LLMRegistry.


def get_registry() -> LLMRegistry:
    """Get the LLMRegistry from the current organism (via container)."""
    from cynic.interfaces.api.state import get_app_container
    container = get_app_container()
    if container is None:
        raise RuntimeError("No app container available (organism not awake)")
    return container.organism.archive.llm_registry

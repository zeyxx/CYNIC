"""
CYNIC Universal LLM Adapter — OS-level LLM routing

CYNIC is an OS that routes to the BEST LLM for each Dog × Task.

4 Consciousness Levels (cycle speeds):
    L3 REFLEX (<10ms):  Non-LLM Dogs only (GUARDIAN/ANALYST/JANITOR/CYNIC)
    L2 MICRO  (~500ms): Fast LLM calls (Ollama local, small models)
    L1 MACRO  (~2.85s): Full LLM reasoning (Claude, Gemini, large Ollama)
    L4 META   (daily):  Meta-learning, weight updates

LLM Selection Architecture:
    - NOT hardcoded — benchmarking determines optimal routing
    - Each Dog × Task type has a benchmark-determined best LLM
    - CYNIC benchmarks quality (Q-Score), speed, cost
    - Results stored → router improves continuously

Adapters:
    OllamaAdapter   — Local open source (primary, free, private)
    ClaudeAdapter   — Anthropic API (claude-sonnet-4.5, etc.)
    GeminiAdapter   — Google Generative AI (free tier)
"""
from __future__ import annotations

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import PHI, PHI_INV, MAX_Q_SCORE, weighted_geometric_mean

logger = logging.getLogger("cynic.llm.adapter")


# ════════════════════════════════════════════════════════════════════════════
# UNIVERSAL REQUEST / RESPONSE
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class LLMRequest:
    """A request to any LLM (unified format)."""
    prompt: str
    system: str = ""
    max_tokens: int = 2048
    temperature: float = 0.0        # Default: deterministic
    stream: bool = False
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class LLMResponse:
    """Response from any LLM (unified format)."""
    content: str
    model: str
    provider: str                   # ollama / claude / gemini
    prompt_tokens: int = 0
    completion_tokens: int = 0
    cost_usd: float = 0.0
    latency_ms: float = 0.0
    error: Optional[str] = None

    @property
    def total_tokens(self) -> int:
        return self.prompt_tokens + self.completion_tokens

    @property
    def is_success(self) -> bool:
        return self.error is None and bool(self.content)

    @property
    def tokens_per_second(self) -> float:
        if self.latency_ms <= 0:
            return 0.0
        return self.completion_tokens / (self.latency_ms / 1000.0)


# ════════════════════════════════════════════════════════════════════════════
# ABSTRACT ADAPTER
# ════════════════════════════════════════════════════════════════════════════

class LLMAdapter(ABC):
    """Abstract adapter — same interface for all LLMs."""

    def __init__(self, model: str, provider: str) -> None:
        self.model = model
        self.provider = provider

    @property
    def adapter_id(self) -> str:
        return f"{self.provider}:{self.model}"

    @abstractmethod
    async def complete(self, request: LLMRequest) -> LLMResponse:
        """Send request, return response."""
        ...

    @abstractmethod
    async def check_available(self) -> bool:
        """Ping check — returns True if this LLM is reachable."""
        ...

    async def complete_safe(self, request: LLMRequest) -> LLMResponse:
        """Complete with error containment — never raises."""
        try:
            return await asyncio.wait_for(self.complete(request), timeout=120.0)
        except asyncio.TimeoutError:
            return LLMResponse(
                content="", model=self.model, provider=self.provider,
                error="timeout after 120s",
            )
        except Exception as exc:
            return LLMResponse(
                content="", model=self.model, provider=self.provider,
                error=str(exc),
            )


# ════════════════════════════════════════════════════════════════════════════
# OLLAMA ADAPTER (Local — PRIMARY for privacy + cost)
# ════════════════════════════════════════════════════════════════════════════


# Module-level singleton ollama clients — one per base_url (shared across all
# OllamaAdapter instances for the same URL — avoids 7 TCP connections per MCTS call)
_OLLAMA_CLIENTS: Dict[str, Any] = {}


def _get_ollama_client(base_url: str) -> Any:
    """Return a cached ollama.AsyncClient for base_url, creating one if needed."""
    import ollama as _ollama  # type: ignore
    if base_url not in _OLLAMA_CLIENTS:
        _OLLAMA_CLIENTS[base_url] = _ollama.AsyncClient(host=base_url)
    return _OLLAMA_CLIENTS[base_url]


class OllamaAdapter(LLMAdapter):
    """
    Ollama local models adapter via ollama.AsyncClient (singleton per URL).

    CYNIC prefers Ollama by default:
    - Free (no API cost)
    - Private (data stays local)
    - Fast for small models

    Models auto-discovered at startup via list_models().
    Best model per Dog × Task determined by benchmarking.

    Uses a module-level AsyncClient singleton to avoid creating 7 TCP connections
    per MCTS asyncio.gather() call.
    """

    DEFAULT_URL = "http://localhost:11434"

    def __init__(self, model: str = "llama3.2", base_url: str = DEFAULT_URL) -> None:
        super().__init__(model=model, provider="ollama")
        self.base_url = base_url.rstrip("/")

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()
        messages: List[Dict[str, str]] = []
        if request.system:
            messages.append({"role": "system", "content": request.system})
        messages.append({"role": "user", "content": request.prompt})

        client = _get_ollama_client(self.base_url)
        response = await client.chat(
            model=self.model,
            messages=messages,
            options={"temperature": request.temperature, "num_predict": request.max_tokens},
        )
        # ollama 0.1.x returns a dict; 0.2+ returns a ChatResponse object
        if isinstance(response, dict):
            content = response.get("message", {}).get("content", "")
            p_tokens = response.get("prompt_eval_count", 0)
            c_tokens = response.get("eval_count", 0)
        else:
            msg = getattr(response, "message", None)
            content = getattr(msg, "content", "") if msg else ""
            p_tokens = getattr(response, "prompt_eval_count", 0)
            c_tokens = getattr(response, "eval_count", 0)

        latency_ms = (time.time() - start) * 1000
        return LLMResponse(
            content=content,
            model=self.model,
            provider="ollama",
            prompt_tokens=p_tokens,
            completion_tokens=c_tokens,
            cost_usd=0.0,
            latency_ms=latency_ms,
        )

    async def check_available(self) -> bool:
        try:
            client = _get_ollama_client(self.base_url)
            await asyncio.wait_for(client.list(), timeout=5.0)
            return True
        except Exception:
            return False

    async def list_models(self) -> List[str]:
        """Return names of all installed Ollama models."""
        try:
            client = _get_ollama_client(self.base_url)
            resp = await client.list()
            if isinstance(resp, dict):
                models = resp.get("models", [])
            else:
                models = getattr(resp, "models", []) or []
            result = []
            for m in models:
                if isinstance(m, dict):
                    result.append(m.get("name", ""))
                else:
                    result.append(getattr(m, "name", str(m)))
            return [n for n in result if n]
        except Exception:
            return []


# ════════════════════════════════════════════════════════════════════════════
# CLAUDE ADAPTER (Anthropic API)
# ════════════════════════════════════════════════════════════════════════════

class ClaudeAdapter(LLMAdapter):
    """
    Claude adapter via Anthropic Python SDK.

    CYNIC controls Claude as one organ among many — not as master.
    Claude = language cortex (L1 Macro cycle reasoning).
    Non-LLM Dogs (GUARDIAN, ANALYST, JANITOR, CYNIC-Dog) are L3 Reflex.
    """

    # Token pricing (USD per million tokens, approximate 2025)
    PRICING: Dict[str, Tuple[float, float]] = {
        "claude-sonnet-4-5-20250929":  (3.0, 15.0),
        "claude-haiku-4-5-20251001":   (0.8,  4.0),
        "claude-opus-4-6":             (15.0, 75.0),
    }

    def __init__(
        self,
        model: str = "claude-sonnet-4-5-20250929",
        api_key: Optional[str] = None,
    ) -> None:
        super().__init__(model=model, provider="claude")
        self._api_key = api_key

    def _estimate_cost(self, prompt_tokens: int, completion_tokens: int) -> float:
        input_price, output_price = self.PRICING.get(self.model, (3.0, 15.0))
        return (prompt_tokens * input_price + completion_tokens * output_price) / 1_000_000

    async def complete(self, request: LLMRequest) -> LLMResponse:
        import anthropic

        start = time.time()
        client = anthropic.AsyncAnthropic(api_key=self._api_key)

        kwargs: Dict[str, Any] = {
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": [{"role": "user", "content": request.prompt}],
        }
        if request.system:
            kwargs["system"] = request.system

        response = await client.messages.create(**kwargs)
        latency_ms = (time.time() - start) * 1000

        content = response.content[0].text if response.content else ""
        usage = response.usage
        cost = self._estimate_cost(usage.input_tokens, usage.output_tokens)

        return LLMResponse(
            content=content,
            model=self.model,
            provider="claude",
            prompt_tokens=usage.input_tokens,
            completion_tokens=usage.output_tokens,
            cost_usd=cost,
            latency_ms=latency_ms,
        )

    async def check_available(self) -> bool:
        try:
            import anthropic
            client = anthropic.AsyncAnthropic(api_key=self._api_key)
            await client.messages.create(
                model=self.model, max_tokens=1,
                messages=[{"role": "user", "content": "ping"}],
            )
            return True
        except Exception:
            return False


# ════════════════════════════════════════════════════════════════════════════
# GEMINI ADAPTER (Google Generative AI)
# ════════════════════════════════════════════════════════════════════════════

class GeminiAdapter(LLMAdapter):
    """
    Google Gemini adapter via google-generativeai SDK.

    Models:
    - gemini-1.5-flash: fast, generous free tier
    - gemini-1.5-pro: smarter, limited free tier
    - gemini-2.0-flash: latest fast model
    """

    def __init__(
        self,
        model: str = "gemini-1.5-flash",
        api_key: Optional[str] = None,
    ) -> None:
        super().__init__(model=model, provider="gemini")
        self._api_key = api_key

    async def complete(self, request: LLMRequest) -> LLMResponse:
        import google.generativeai as genai  # type: ignore

        if self._api_key:
            genai.configure(api_key=self._api_key)

        start = time.time()
        config = genai.types.GenerationConfig(
            max_output_tokens=request.max_tokens,
            temperature=request.temperature,
        )
        llm = genai.GenerativeModel(
            model_name=self.model,
            system_instruction=request.system or None,
            generation_config=config,
        )

        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None, lambda: llm.generate_content(request.prompt)
        )
        latency_ms = (time.time() - start) * 1000
        content = response.text if hasattr(response, "text") else ""

        usage = getattr(response, "usage_metadata", None)
        p_tokens = getattr(usage, "prompt_token_count", 0) if usage else 0
        c_tokens = getattr(usage, "candidates_token_count", 0) if usage else 0

        return LLMResponse(
            content=content,
            model=self.model,
            provider="gemini",
            prompt_tokens=p_tokens,
            completion_tokens=c_tokens,
            cost_usd=0.0,
            latency_ms=latency_ms,
        )

    async def check_available(self) -> bool:
        try:
            import google.generativeai as genai  # type: ignore
            if self._api_key:
                genai.configure(api_key=self._api_key)
            list(genai.list_models())
            return True
        except Exception:
            return False


# ════════════════════════════════════════════════════════════════════════════
# BENCHMARK RESULT
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class BenchmarkResult:
    """Benchmark of one LLM for one Dog × Task combination."""
    llm_id: str
    dog_id: str
    task_type: str
    quality_score: float    # Q-Score from CYNIC Judge [0, 61.8]
    speed_score: float      # tokens/s (normalized 0-1)
    cost_score: float       # 1/cost (normalized 0-1, higher=cheaper)
    error_rate: float = 0.0
    sample_count: int = 1
    timestamp: float = field(default_factory=time.time)

    @property
    def composite_score(self) -> float:
        """
        φ-weighted composite: Quality (φ) > Speed (1.0) > Cost (φ⁻¹)

        Quality dominates because CYNIC's purpose is truth, not speed.
        """
        if self.quality_score <= 0:
            return 0.0
        return weighted_geometric_mean(
            [self.quality_score / 61.8, self.speed_score, self.cost_score],
            [PHI, 1.0, PHI_INV],
        )

    def ema_update(self, new: "BenchmarkResult", alpha: float = 0.3) -> "BenchmarkResult":
        """Exponential moving average update (continuous learning)."""
        return BenchmarkResult(
            llm_id=self.llm_id,
            dog_id=self.dog_id,
            task_type=self.task_type,
            quality_score=alpha * new.quality_score + (1 - alpha) * self.quality_score,
            speed_score=alpha * new.speed_score + (1 - alpha) * self.speed_score,
            cost_score=alpha * new.cost_score + (1 - alpha) * self.cost_score,
            error_rate=alpha * new.error_rate + (1 - alpha) * self.error_rate,
            sample_count=self.sample_count + 1,
        )


# ════════════════════════════════════════════════════════════════════════════
# ROUTING CONSTANTS (empirical — benchmarked 2026-02-17)
# ════════════════════════════════════════════════════════════════════════════

# Models that only do embeddings — never used for text generation
EMBEDDING_ONLY_MODELS: set = {"nomic-embed-text", "nomic-embed-text:latest"}

# task_type → preferred Ollama model (empirical benchmark results)
# gemma2:2b: 7-parallel MCTS in 35s, diff=+40.3, 0 failures → FAST judgment
# mistral:7b: single deep call only (88s/call → timeouts in parallel 7-call MCTS)
PREFERRED_MODELS: Dict[str, str] = {
    "temporal_mcts": "gemma2:2b",               # 7-parallel MCTS — must be fast
    "wisdom":        "gemma2:2b",               # SageDog temporal path
    "vector_rag":    "gemma2:2b",               # ScholarDog generation (not embeddings)
    "topology":      "gemma2:2b",               # CartographerDog
    "deployment":    "gemma2:2b",               # DeployerDog
    "deep_analysis": "mistral:7b-instruct-q4_0", # Single-call deep review
    "default":       "gemma2:2b",               # Safe fallback
}


# ════════════════════════════════════════════════════════════════════════════
# LLM REGISTRY (Dynamic discovery + routing)
# ════════════════════════════════════════════════════════════════════════════

class LLMRegistry:
    """
    OS-level LLM registry.

    1. Discovers available LLMs on startup (Ollama models + Claude + Gemini)
    2. Routes each Dog × Task request to best-performing LLM
    3. Continuously updates routing table from benchmark results
    4. CYNIC surpasses single LLMs because it uses THE BEST LLM per task
    """

    def __init__(self) -> None:
        self._adapters: Dict[str, LLMAdapter] = {}
        self._available: Dict[str, bool] = {}
        self._benchmarks: Dict[Tuple[str, str, str], BenchmarkResult] = {}
        self._db_pool: Optional[Any] = None

    def set_db_pool(self, pool: Any) -> None:
        """Wire a DB pool so benchmark updates are persisted automatically."""
        self._db_pool = pool

    def register(self, adapter: LLMAdapter, available: bool = True) -> None:
        self._adapters[adapter.adapter_id] = adapter
        self._available[adapter.adapter_id] = available

    async def discover(
        self,
        ollama_url: str = "http://localhost:11434",
        claude_api_key: Optional[str] = None,
        gemini_api_key: Optional[str] = None,
        models_dir: Optional[str] = None,
        llama_gpu_layers: int = -1,
        llama_threads: int = 8,
    ) -> List[str]:
        """
        Auto-discover all available LLMs.

        Returns list of available adapter IDs.
        CYNIC benchmarks these at startup to build routing table.

        Args:
            models_dir: Optional path to a directory containing .gguf model files.
                        If set and llama-cpp-python is installed, LlamaCppAdapter
                        instances are registered for each .gguf found.
            llama_gpu_layers: Layers to offload to GPU (-1 = all, 0 = CPU only).
            llama_threads: CPU threads for llama-cpp-python inference.
        """
        available: List[str] = []

        # Ollama: discover all installed models
        async def _discover_ollama() -> None:
            probe = OllamaAdapter(model="probe", base_url=ollama_url)
            if await probe.check_available():
                models = await probe.list_models()
                for model_name in models:
                    adapter = OllamaAdapter(model=model_name, base_url=ollama_url)
                    self.register(adapter, available=True)
                    available.append(adapter.adapter_id)

        # Claude
        async def _discover_claude() -> None:
            if claude_api_key:
                adapter = ClaudeAdapter(api_key=claude_api_key)
                if await adapter.check_available():
                    self.register(adapter, available=True)
                    available.append(adapter.adapter_id)

        # Gemini
        async def _discover_gemini() -> None:
            if gemini_api_key:
                adapter = GeminiAdapter(api_key=gemini_api_key)
                if await adapter.check_available():
                    self.register(adapter, available=True)
                    available.append(adapter.adapter_id)

        # LlamaCpp: scan models_dir for .gguf files (silent if not installed)
        async def _discover_llama_cpp() -> None:
            if models_dir is None:
                return
            try:
                from cynic.llm.llama_cpp import LlamaCppAdapter, list_local_models  # type: ignore
                paths = list_local_models(models_dir)
                for path in paths:
                    adapter = LlamaCppAdapter(
                        model_path=path,
                        n_gpu_layers=llama_gpu_layers,
                        n_threads=llama_threads,
                    )
                    if await adapter.check_available():
                        self.register(adapter, available=True)
                        available.append(adapter.adapter_id)
            except ImportError:
                pass  # llama-cpp-python not installed — skip silently

        await asyncio.gather(
            _discover_ollama(),
            _discover_claude(),
            _discover_gemini(),
            _discover_llama_cpp(),
        )
        return available

    def get_available(self) -> List[LLMAdapter]:
        return [a for aid, a in self._adapters.items() if self._available.get(aid, False)]

    def _is_generation_adapter(self, adapter: LLMAdapter) -> bool:
        """Return True if adapter can generate text (excludes embedding-only models).
        Non-string model attributes (e.g. MagicMock in tests) are assumed capable."""
        model = getattr(adapter, "model", None)
        if not isinstance(model, str):
            return True  # Unknown/mock model — assume generation capable
        name = model.lower()
        if any(name.startswith(e.lower()) for e in EMBEDDING_ONLY_MODELS):
            return False
        # Also exclude llama_cpp embedding models (contain "embed" in name)
        if getattr(adapter, "provider", None) == "llama_cpp" and "embed" in name:
            return False
        return True

    def get_available_for_generation(self) -> List[LLMAdapter]:
        """Available adapters that support text generation (not embeddings-only)."""
        return [a for a in self.get_available() if self._is_generation_adapter(a)]

    def get_best_for(self, dog_id: str, task_type: str) -> Optional[LLMAdapter]:
        """
        Return best LLM for this Dog × Task.

        Priority:
          1. Benchmark history (composite_score — learned from real judgments)
          2. PREFERRED_MODELS[task_type] (empirical defaults)
          3. PREFERRED_MODELS["default"] (gemma2:2b)
          4. Any available generation adapter

        Embedding-only models (nomic-embed-text) are never returned here.
        """
        best_score = -1.0
        best: Optional[LLMAdapter] = None

        for aid, adapter in self._adapters.items():
            if not self._available.get(aid, False):
                continue
            if not self._is_generation_adapter(adapter):
                continue
            key = (dog_id, task_type, aid)
            bench = self._benchmarks.get(key)
            if bench and bench.composite_score > best_score:
                best_score = bench.composite_score
                best = adapter

        if best is not None:
            return best

        # No benchmark data — use empirical preferred model for this task type
        preferred_name = PREFERRED_MODELS.get(task_type, PREFERRED_MODELS["default"])
        for adapter in self._adapters.values():
            model = getattr(adapter, "model", None)
            if self._available.get(adapter.adapter_id, False) and model == preferred_name:
                return adapter

        # Preferred not installed — any available generation adapter
        avail = self.get_available_for_generation()
        return avail[0] if avail else None

    def get_for_temporal_mcts(self) -> Optional[LLMAdapter]:
        """
        Return best adapter for 7-parallel temporal MCTS calls.

        Must be fast (<10s/call): gemma2:2b validated at ~5s/call.
        mistral:7b excluded — 88s/call causes timeouts in asyncio.gather(7).
        """
        return self.get_best_for("__temporal_mcts__", "temporal_mcts")

    def update_benchmark(
        self, dog_id: str, task_type: str, llm_id: str, result: BenchmarkResult
    ) -> None:
        key = (dog_id, task_type, llm_id)
        existing = self._benchmarks.get(key)
        updated = existing.ema_update(result) if existing else result
        self._benchmarks[key] = updated

        if self._db_pool is not None:
            try:
                loop = asyncio.get_running_loop()
                loop.create_task(self._save_benchmark_to_db(updated))
            except RuntimeError:
                pass  # No running loop — skip (sync test context)

    async def _save_benchmark_to_db(self, result: BenchmarkResult) -> None:
        """Fire-and-forget: persist benchmark to llm_benchmarks table."""
        import uuid
        try:
            async with self._db_pool.acquire() as conn:
                await conn.execute("""
                    INSERT INTO llm_benchmarks
                        (benchmark_id, dog_id, task_type, llm_id,
                         quality_score, speed_score, cost_score, composite_score,
                         latency_ms, cost_usd)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                """,
                    str(uuid.uuid4()),
                    result.dog_id,
                    result.task_type,
                    result.llm_id,
                    min(result.quality_score / MAX_Q_SCORE, 1.0),  # Normalize [0,61.8] → [0,1]
                    result.speed_score,
                    result.cost_score,
                    result.composite_score,
                    0.0,   # latency_ms not tracked in BenchmarkResult
                    0.0,   # cost_usd not tracked in BenchmarkResult
                )
        except Exception as exc:
            logger.warning("Benchmark persist failed: %s", exc)

    async def load_benchmarks_from_db(self, pool: Any) -> int:
        """
        Warm-start _benchmarks from DB on boot.

        DB quality_score is [0,1] (normalized). BenchmarkResult expects [0, MAX_Q_SCORE].
        We keep only the most recent result per (dog_id, task_type, llm_id).
        Returns count of entries loaded.
        """
        try:
            async with pool.acquire() as conn:
                rows = await conn.fetch("""
                    SELECT dog_id, task_type, llm_id,
                           quality_score, speed_score, cost_score
                    FROM llm_benchmarks
                    ORDER BY created_at DESC
                """)
        except Exception as exc:
            logger.warning("Benchmark warm-start failed: %s", exc)
            return 0

        loaded = 0
        for row in rows:
            key = (row["dog_id"], row["task_type"], row["llm_id"])
            if key in self._benchmarks:
                continue  # Keep most recent (already ordered DESC)
            self._benchmarks[key] = BenchmarkResult(
                llm_id=row["llm_id"],
                dog_id=row["dog_id"],
                task_type=row["task_type"],
                quality_score=float(row["quality_score"]) * MAX_Q_SCORE,
                speed_score=float(row["speed_score"]),
                cost_score=float(row["cost_score"]),
            )
            loaded += 1
        return loaded

    def benchmark_matrix(self, dog_id: str, task_type: str) -> Dict[str, BenchmarkResult]:
        return {
            llm_id: r
            for (d, t, llm_id), r in self._benchmarks.items()
            if d == dog_id and t == task_type
        }


# Singleton
_registry: Optional[LLMRegistry] = None


def get_registry() -> LLMRegistry:
    global _registry
    if _registry is None:
        _registry = LLMRegistry()
    return _registry

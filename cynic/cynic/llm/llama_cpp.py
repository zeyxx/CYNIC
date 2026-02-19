"""
CYNIC LlamaCpp Adapter — Direct GGUF inference via llama-cpp-python.

Two modes:
  CPU    : n_gpu_layers=0  → BLAS multi-thread, ~25 tok/s for gemma2:2b
  Vulkan : n_gpu_layers=-1 → iGPU Vega 8 (AMD 5700G APU), ~50 tok/s estimated

Advantage vs OllamaAdapter:
  - Zero HTTP overhead (no socket, no TCP round-trip)
  - Same .gguf weights, identical outputs
  - BenchmarkResult composite_score will naturally exceed OllamaAdapter
    after first measurement (lower latency + cost_usd=0)

Thread-safety:
  llama-cpp-python Llama is synchronous and not thread-safe.
  We use asyncio.Semaphore(1) + run_in_executor(None) to serialise calls
  to the same model instance while remaining non-blocking for the rest of
  the async kernel.

  For 7-parallel MCTS (asyncio.gather): the 7 calls to the SAME adapter
  execute sequentially through the semaphore, but without HTTP overhead
  → real gain of ~20-30% vs OllamaAdapter on CPU.

  Two DISTINCT LlamaCppAdapter instances (e.g. gemma2:2b on iGPU +
  mistral:7b on CPU) each have their own semaphore and CAN run
  simultaneously.

Installation:
  CPU   : pip install cynic[local-inference]
  Vulkan: CMAKE_ARGS="-DGGML_VULKAN=on" pip install llama-cpp-python \\
              --upgrade --force-reinstall --no-cache-dir
"""
from __future__ import annotations

import asyncio
import glob
import logging
import os
import time
from typing import Any, List, Optional

from cynic.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.llm.llama_cpp")


def list_local_models(models_dir: str) -> list[str]:
    """
    Return absolute paths to .gguf files found under models_dir (recursive).
    Sorted by file size ascending so smaller/faster models are registered first.
    """
    pattern = os.path.join(os.path.expanduser(models_dir), "**", "*.gguf")
    paths = glob.glob(pattern, recursive=True)
    return sorted(paths, key=os.path.getsize)


class LlamaCppAdapter(LLMAdapter):
    """
    Local GGUF inference via llama-cpp-python.

    n_gpu_layers=-1  → offload all layers to GPU (Vulkan/CUDA/Metal)
    n_gpu_layers=0   → CPU-only inference
    n_gpu_layers=N   → partial GPU offload (N layers)
    """

    def __init__(
        self,
        model_path: str,
        n_gpu_layers: int = 0,
        n_threads: int = 8,
        n_ctx: int = 2048,
        verbose: bool = False,
    ) -> None:
        model_name = os.path.basename(model_path).replace(".gguf", "")
        super().__init__(model=model_name, provider="llama_cpp")
        self._model_path = model_path
        self._n_gpu_layers = n_gpu_layers
        self._n_threads = n_threads
        self._n_ctx = n_ctx
        self._verbose = verbose
        self._llm: Any | None = None         # lazy-loaded on first inference
        self._lock: asyncio.Semaphore | None = None  # lazy-init inside running loop

    # ── Lazy load ────────────────────────────────────────────────────────────

    def _load(self) -> None:
        """Load the GGUF model (blocking, called once from ThreadPoolExecutor)."""
        if self._llm is not None:
            return
        from llama_cpp import Llama  # type: ignore
        self._llm = Llama(
            model_path=self._model_path,
            n_gpu_layers=self._n_gpu_layers,
            n_threads=self._n_threads,
            n_ctx=self._n_ctx,
            verbose=self._verbose,
        )
        logger.info(
            "LlamaCpp loaded: %s (gpu_layers=%d, threads=%d, ctx=%d)",
            self._model_path,
            self._n_gpu_layers,
            self._n_threads,
            self._n_ctx,
        )

    # ── Synchronous inference (runs in ThreadPoolExecutor) ───────────────────

    def _sync_complete(
        self, messages: list[dict], max_tokens: int, temperature: float
    ) -> str:
        """Blocking chat completion — called via run_in_executor."""
        self._load()
        output = self._llm.create_chat_completion(  # type: ignore[union-attr]
            messages=messages,
            max_tokens=max_tokens,
            temperature=temperature,
        )
        choices = output.get("choices", []) if isinstance(output, dict) else []
        if choices:
            msg = choices[0].get("message", {})
            return msg.get("content", "")
        return ""

    # ── Public async interface ───────────────────────────────────────────────

    async def complete(self, request: LLMRequest) -> LLMResponse:
        messages: list[dict] = []
        if request.system:
            messages.append({"role": "system", "content": request.system})
        messages.append({"role": "user", "content": request.prompt})

        if self._lock is None:
            self._lock = asyncio.Semaphore(1)

        start = time.time()
        async with self._lock:
            loop = asyncio.get_running_loop()
            content = await loop.run_in_executor(
                None,
                self._sync_complete,
                messages,
                request.max_tokens,
                request.temperature,
            )
        latency_ms = (time.time() - start) * 1000

        return LLMResponse(
            content=content,
            model=self.model,
            provider="llama_cpp",
            cost_usd=0.0,
            latency_ms=latency_ms,
        )

    async def check_available(self) -> bool:
        """
        Returns True iff:
        1. llama-cpp-python package is installed
        2. The .gguf file exists at model_path
        """
        try:
            import llama_cpp  # type: ignore  # noqa: F401
            return os.path.isfile(self._model_path)
        except ImportError:
            return False

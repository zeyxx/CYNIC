"""
CYNIC Local GGUF Adapter " Pure Hardware Inference.

Directly loads GGUF models on CPU/iGPU via llama-cpp-python.
The most sovereign muscle: Zero network, Zero cost, 100% Private.
"""

from __future__ import annotations

import logging
import os
import time

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.kernel.organism.brain.llm.local_gguf")


class LlamaCppAdapter(LLMAdapter):
    def __init__(self, model_path: str, n_threads: int = 8, n_gpu_layers: int = -1):
        model_name = os.path.basename(model_path)
        super().__init__(model=model_name, provider="llama_cpp")
        self._model_path = model_path
        self._n_threads = n_threads
        self._n_gpu_layers = n_gpu_layers
        self._llm = None

    async def _ensure_loaded(self):
        if self._llm is None:
            import llama_cpp

            self._llm = llama_cpp.Llama(
                model_path=self._model_path,
                n_threads=self._n_threads,
                n_gpu_layers=self._n_gpu_layers,
                verbose=False,
            )

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()
        try:
            await self._ensure_loaded()

            # Simple synchronous call wrapped in executor for async
            import asyncio

            loop = asyncio.get_running_loop()

            def _run():
                return self._llm.create_chat_completion(
                    messages=[
                        {"role": "system", "content": request.system},
                        {"role": "user", "content": request.prompt},
                    ],
                    max_tokens=request.max_tokens,
                    temperature=request.temperature,
                )

            result = await loop.run_in_executor(None, _run)
            content = result["choices"][0]["message"]["content"]

            return LLMResponse(
                content=content,
                model=self.model,
                provider="llama_cpp",
                latency_ms=(time.time() - start) * 1000,
            )
        except Exception as e:
            return LLMResponse(
                content="", model=self.model, provider="llama_cpp", error=str(e)
            )

    async def check_available(self) -> bool:
        return os.path.exists(self._model_path)

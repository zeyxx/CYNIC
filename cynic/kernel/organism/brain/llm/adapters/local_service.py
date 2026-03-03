"""
CYNIC Local Service Adapter — Ollama / LocalAI.

Connects to locally running LLM servers via HTTP API.
Now using VascularSystem for persistent connection pooling.
"""

from __future__ import annotations

import logging
import time
from typing import Optional

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse
from cynic.kernel.core.vascular import VascularSystem

logger = logging.getLogger("cynic.kernel.organism.brain.llm.local_service")


import ollama
from ollama import AsyncClient

class OllamaAdapter(LLMAdapter):
    def __init__(self, model: str, base_url: str = "http://localhost:11434", vascular: Optional[VascularSystem] = None):
        super().__init__(model=model, provider="ollama", vascular=vascular)
        self._client = AsyncClient(host=base_url)

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()
        # Metadata can carry a keep_alive instruction
        # If not specified, we default to 5m (Ollama default) or 0 for metabolic saving
        keep_alive = request.metadata.get("keep_alive", "5m")
        
        try:
            # Using the official SDK
            response = await self._client.generate(
                model=self.model,
                prompt=request.prompt,
                system=request.system,
                options={"temperature": request.temperature},
                keep_alive=keep_alive
            )
            
            latency_ms = (time.time() - start) * 1000
            
            return LLMResponse(
                content=response['response'],
                model=self.model,
                provider="ollama",
                prompt_tokens=response.get('prompt_eval_count', 0),
                completion_tokens=response.get('eval_count', 0),
                latency_ms=latency_ms,
            )
        except Exception as e:
            logger.error(f"Ollama SDK failure: {e}")
            return LLMResponse(content="", model=self.model, provider="ollama", error=str(e))

    async def unload(self) -> bool:
        """Forcefully unload the model from VRAM/RAM."""
        try:
            # In Ollama, sending a request with keep_alive=0 unloads the model
            await self._client.generate(model=self.model, keep_alive=0)
            logger.info(f"HAL: Metaphorical 'breathing out' - Model {self.model} unloaded.")
            return True
        except Exception as e:
            logger.error(f"HAL: Failed to unload {self.model}: {e}")
            return False

    async def check_available(self) -> bool:
        try:
            await self._client.ps()
            return True
        except Exception:
            return False

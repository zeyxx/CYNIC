"""
CYNIC Local Service Adapter â€” Ollama / LocalAI.

Connects to locally running LLM servers via HTTP API.
Sovereign but service-based.
"""

from __future__ import annotations

import logging
import time

import httpx

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.kernel.organism.brain.llm.local_service")


class OllamaAdapter(LLMAdapter):
    def __init__(self, model: str, base_url: str = "http://localhost:11434"):
        super().__init__(model=model, provider="ollama")
        self._url = base_url.rstrip("/")

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()
        payload = {
            "model": self.model,
            "prompt": f"{request.system}\n\n{request.prompt}" if request.system else request.prompt,
            "stream": False,
            "options": {"temperature": request.temperature},
        }
        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                resp = await client.post(f"{self._url}/api/generate", json=payload)
                resp.raise_for_status()
                data = resp.json()
                return LLMResponse(
                    content=data["response"],
                    model=self.model,
                    provider="ollama",
                    latency_ms=(time.time() - start) * 1000,
                )
        except Exception as e:
            return LLMResponse(content="", model=self.model, provider="ollama", error=str(e))

    async def check_available(self) -> bool:
        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                resp = await client.get(f"{self._url}/api/tags")
                return resp.status_code == 200
        except:
            return False

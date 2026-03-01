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


class OllamaAdapter(LLMAdapter):
    def __init__(self, model: str, base_url: str = "http://localhost:11434", vascular: Optional[VascularSystem] = None):
        super().__init__(model=model, provider="ollama", vascular=vascular)
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
            client = await self._get_client()
            resp = await client.post(f"{self._url}/api/generate", json=payload, timeout=60.0)
            resp.raise_for_status()
            data = resp.json()
            return LLMResponse(
                content=data["response"],
                model=self.model,
                provider="ollama",
                latency_ms=(time.time() - start) * 1000,
            )
        except Exception as e:
            logger.error(f"Ollama failure: {e}")
            return LLMResponse(content="", model=self.model, provider="ollama", error=str(e))

    async def check_available(self) -> bool:
        try:
            client = await self._get_client()
            resp = await client.get(f"{self._url}/api/tags", timeout=2.0)
            return resp.status_code == 200
        except Exception:
            return False

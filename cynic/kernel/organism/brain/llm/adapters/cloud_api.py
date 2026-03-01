"""
CYNIC Cloud API Adapter — Anthropic & Google.

Production-grade adapters using VascularSystem for pooling.
Handles real network IO with retry and error boundary.
"""

from __future__ import annotations

import logging
import time
from typing import Optional

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse
from cynic.kernel.core.vascular import VascularSystem

logger = logging.getLogger("cynic.kernel.organism.brain.llm.cloud_api")


class AnthropicAdapter(LLMAdapter):
    def __init__(self, model: str = "claude-3-sonnet-20240229", api_key: str | None = None, vascular: Optional[VascularSystem] = None):
        super().__init__(model=model, provider="anthropic", vascular=vascular)
        self._key = api_key

    async def complete(self, request: LLMRequest) -> LLMResponse:
        if not self._key:
            return LLMResponse(content="", model=self.model, provider="anthropic", error="No API Key")

        t0 = time.perf_counter()
        try:
            client = await self._get_client()
            # Real Anthropic API Call
            resp = await client.post(
                "https://api.anthropic.com/v1/messages",
                headers={
                    "x-api-key": self._key,
                    "anthropic-version": "2023-06-01",
                    "content-type": "application/json"
                },
                json={
                    "model": self.model,
                    "max_tokens": request.max_tokens,
                    "messages": [{"role": "user", "content": request.prompt}],
                    "system": request.system,
                    "temperature": request.temperature
                },
                timeout=60.0
            )
            resp.raise_for_status()
            data = resp.json()
            
            return LLMResponse(
                content=data["content"][0]["text"],
                model=self.model,
                provider="anthropic",
                prompt_tokens=data.get("usage", {}).get("input_tokens", 0),
                completion_tokens=data.get("usage", {}).get("output_tokens", 0),
                latency_ms=(time.perf_counter() - t0) * 1000
            )
        except Exception as e:
            logger.error(f"Anthropic API failure: {e}")
            return LLMResponse(content="", model=self.model, provider="anthropic", error=str(e))

    async def check_available(self) -> bool:
        return bool(self._key)


class GeminiAdapter(LLMAdapter):
    def __init__(self, model: str = "gemini-1.5-pro", api_key: str | None = None, vascular: Optional[VascularSystem] = None):
        super().__init__(model=model, provider="gemini", vascular=vascular)
        self._key = api_key

    async def complete(self, request: LLMRequest) -> LLMResponse:
        if not self._key:
            return LLMResponse(content="", model=self.model, provider="gemini", error="No API Key")

        t0 = time.perf_counter()
        try:
            client = await self._get_client()
            # Real Google Gemini API Call
            url = f"https://generativelanguage.googleapis.com/v1beta/models/{self.model}:generateContent?key={self._key}"
            resp = await client.post(
                url,
                json={
                    "contents": [{"parts": [{"text": request.prompt}]}],
                    "generationConfig": {
                        "temperature": request.temperature,
                        "maxOutputTokens": request.max_tokens
                    }
                },
                timeout=60.0
            )
            resp.raise_for_status()
            data = resp.json()
            
            content = data["candidates"][0]["content"]["parts"][0]["text"]
            
            return LLMResponse(
                content=content,
                model=self.model,
                provider="gemini",
                latency_ms=(time.perf_counter() - t0) * 1000
            )
        except Exception as e:
            logger.error(f"Gemini API failure: {e}")
            return LLMResponse(content="", model=self.model, provider="gemini", error=str(e))

    async def check_available(self) -> bool:
        return bool(self._key)

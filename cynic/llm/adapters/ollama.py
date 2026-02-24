"""
Ollama Adapter — local model inference via ollama.AsyncClient.

CYNIC prefers Ollama by default:
  - Free (no API cost)
  - Private (data stays local)
  - Fast for small models

Models auto-discovered at startup via list_models().
Best model per Dog × Task determined by benchmarking.
"""
import asyncio
import httpx
import logging
import time
from typing import Any, Optional

from cynic.core.formulas import LLM_TIMEOUT_SEC, LLM_DISCOVERY_TIMEOUT_SEC
from cynic.llm.adapters.interface import LLMAdapterInterface
from cynic.llm.adapter import LLMRequest, LLMResponse

logger = logging.getLogger("cynic.llm.adapters.ollama")


class OllamaConnectionPool:
    """
    Manages cached ollama.AsyncClient instances per base_url.

    Instead of module-level singleton, use instance-level pool.
    Multiple pools can coexist (enables parallel testing, multi-instance deployment).
    """

    def __init__(self):
        """Initialize empty client cache."""
        self._clients: dict[str, Any] = {}

    def get_client(self, base_url: str) -> Any:
        """Return cached ollama.AsyncClient for base_url, creating if needed."""
        import ollama as _ollama  # type: ignore
        if base_url not in self._clients:
            self._clients[base_url] = _ollama.AsyncClient(host=base_url)
        return self._clients[base_url]

    async def close_all(self) -> None:
        """Close all cached clients."""
        for client in self._clients.values():
            if hasattr(client, "close"):
                try:
                    await client.close()
                except asyncio.TimeoutError as e:
                    logger.debug(f"Error closing client: {e}")
        self._clients.clear()


class OllamaAdapter(LLMAdapterInterface):
    """
    Ollama local models adapter via ollama.AsyncClient.

    Uses OllamaConnectionPool to cache AsyncClient instances per URL.
    This avoids creating 7 TCP connections per MCTS asyncio.gather() call,
    while enabling multi-instance deployment (no global state).
    """

    DEFAULT_URL = "http://localhost:11434"
    _default_pool: Optional[OllamaConnectionPool] = None  # Lazy-initialized for backward compat

    def __init__(
        self,
        model: str = "llama3.2",
        base_url: str = DEFAULT_URL,
        pool: Optional[OllamaConnectionPool] = None,
    ) -> None:
        self._model = model
        self._provider = "ollama"
        self.base_url = base_url.rstrip("/")
        # Use provided pool, or create a default one (backward compat)
        if pool is None:
            if OllamaAdapter._default_pool is None:
                OllamaAdapter._default_pool = OllamaConnectionPool()
            pool = OllamaAdapter._default_pool
        self.pool = pool

    @property
    def adapter_id(self) -> str:
        return f"{self._provider}:{self._model}"

    @property
    def model(self) -> str:
        return self._model

    @property
    def provider(self) -> str:
        return self._provider

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()

        # Build messages: use explicit messages if provided, else prompt-based
        if request.messages is not None:
            messages = list(request.messages)
            # Prepend system if not already present
            if request.system and (not messages or messages[0].get("role") != "system"):
                messages.insert(0, {"role": "system", "content": request.system})
        else:
            messages = []
            if request.system:
                messages.append({"role": "system", "content": request.system})
            messages.append({"role": "user", "content": request.prompt})

        client = self.pool.get_client(self.base_url)
        kwargs: dict[str, Any] = {
            "model": self._model,
            "messages": messages,
            "options": {"temperature": request.temperature, "num_predict": request.max_tokens},
        }
        if request.tools:
            kwargs["tools"] = request.tools

        response = await client.chat(**kwargs)

        # ollama 0.1.x returns a dict; 0.2+ returns a ChatResponse object
        tool_calls = None
        if isinstance(response, dict):
            msg_data = response.get("message", {})
            content = msg_data.get("content", "")
            p_tokens = response.get("prompt_eval_count", 0)
            c_tokens = response.get("eval_count", 0)
            raw_tc = msg_data.get("tool_calls")
            if raw_tc:
                tool_calls = [
                    {"name": tc["function"]["name"], "arguments": tc["function"]["arguments"]}
                    for tc in raw_tc if isinstance(tc, dict) and "function" in tc
                ]
        else:
            msg = getattr(response, "message", None)
            content = getattr(msg, "content", "") if msg else ""
            p_tokens = getattr(response, "prompt_eval_count", 0)
            c_tokens = getattr(response, "eval_count", 0)
            raw_tc = getattr(msg, "tool_calls", None)
            if raw_tc:
                tool_calls = []
                for tc in raw_tc:
                    fn = getattr(tc, "function", None) or (tc.get("function") if isinstance(tc, dict) else None)
                    if fn:
                        name = getattr(fn, "name", None) or (fn.get("name") if isinstance(fn, dict) else "")
                        args = getattr(fn, "arguments", None) or (fn.get("arguments") if isinstance(fn, dict) else {})
                        tool_calls.append({"name": name, "arguments": args})

        latency_ms = (time.time() - start) * 1000
        return LLMResponse(
            content=content,
            model=self._model,
            provider="ollama",
            prompt_tokens=p_tokens,
            completion_tokens=c_tokens,
            cost_usd=0.0,
            latency_ms=latency_ms,
            tool_calls=tool_calls or None,
            raw_message=response,
        )

    async def check_available(self) -> bool:
        try:
            client = self.pool.get_client(self.base_url)
            await asyncio.wait_for(client.list(), timeout=LLM_DISCOVERY_TIMEOUT_SEC)
            return True
        except httpx.RequestError:
            return False

    async def list_models(self) -> list[str]:
        """Return names of all installed Ollama models."""
        try:
            client = self.pool.get_client(self.base_url)
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
        except httpx.RequestError:
            return []

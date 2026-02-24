"""Base LLM Provider Protocol."""
from typing import Protocol, AsyncIterator
from dataclasses import dataclass


@dataclass
class LLMResponse:
    """Response from any LLM (unified format)."""
    content: str
    model: str
    provider: str
    prompt_tokens: int = 0
    completion_tokens: int = 0
    cost_usd: float = 0.0
    latency_ms: float = 0.0
    error: str | None = None
    tool_calls: list[dict] | None = None
    raw_message: object | None = None

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


class LLMProvider(Protocol):
    """Protocol defining the interface for all LLM providers."""

    async def complete(self, prompt: str) -> LLMResponse:
        """Send prompt, return response."""
        ...

    async def stream(self, prompt: str) -> AsyncIterator[str]:
        """Stream response token by token."""
        ...

    async def check_available(self) -> bool:
        """Check if provider is available."""
        ...

"""
LLM Adapter Interface — abstract contract for all LLM backends.

This interface lets Track C create test adapters (MockLLMAdapter, EmpiricalLLMAdapter)
that return deterministic or replayed responses without calling real LLMs.
"""
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.llm.adapter import LLMRequest, LLMResponse


class LLMAdapterInterface(ABC):
    """Abstract adapter interface — all LLMs must implement this."""

    @abstractmethod
    async def complete(self, request: "LLMRequest") -> "LLMResponse":
        """Send a completion request, return response.

        Args:
            request: LLMRequest with prompt, system, temperature, etc.

        Returns:
            LLMResponse with content, model, provider, and metrics.
        """
        ...

    @abstractmethod
    async def check_available(self) -> bool:
        """Ping check — returns True if this LLM is reachable/available.

        Used at startup for discovery and runtime health checks.
        """
        ...

    @property
    @abstractmethod
    def adapter_id(self) -> str:
        """Unique identifier for this adapter (e.g., 'ollama:llama3.2')."""
        ...

    @property
    @abstractmethod
    def model(self) -> str:
        """Model name (e.g., 'llama3.2', 'claude-sonnet-4-5-20250929')."""
        ...

    @property
    @abstractmethod
    def provider(self) -> str:
        """Provider name (e.g., 'ollama', 'claude', 'gemini')."""
        ...

    async def complete_safe(self, request: "LLMRequest") -> "LLMResponse":
        """Complete with error containment — default implementation.

        Subclasses can override for custom error handling/logging.
        By default, just calls complete() (see adapter.py for full impl).
        """
        return await self.complete(request)

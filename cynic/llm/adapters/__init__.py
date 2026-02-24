"""
LLM Adapter module — pluggable interface for any LLM backend.

Exports:
  - LLMAdapterInterface: Abstract base for all adapters
  - OllamaAdapter: Local model inference via ollama
  - ClaudeAdapter: Anthropic Claude API
  - GeminiAdapter: Google Generative AI
  - LLMRequest/LLMResponse: Unified message format
  - BenchmarkResult: Performance metrics
"""
from cynic.llm.adapters.interface import LLMAdapterInterface
from cynic.llm.adapters.ollama import OllamaAdapter, OllamaConnectionPool
from cynic.llm.adapter import (
    LLMRequest,
    LLMResponse,
    ClaudeAdapter,
    GeminiAdapter,
    BenchmarkResult,
    LLMRegistry,
    get_registry,
)

__all__ = [
    "LLMAdapterInterface",
    "OllamaAdapter",
    "OllamaConnectionPool",
    "ClaudeAdapter",
    "GeminiAdapter",
    "LLMRequest",
    "LLMResponse",
    "BenchmarkResult",
    "LLMRegistry",
    "get_registry",
]

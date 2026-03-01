"""CYNIC LLM Module â€” Universal LLM integration layer.

Provides:
- Adapters for local (llama_cpp, Ollama), CLI (Claude, Gemini), and Cloud (Anthropic, Gemini) LLMs
- Hardware-aware discovery and benchmarking
- Unified request/response format for all LLM interfaces
"""

from __future__ import annotations

# Adapters are loaded dynamically via LLMRegistry.discover()
# to avoid circular import chains

__all__ = []

"""CYNIC LLM Module — Universal LLM integration layer.

Provides:
- Adapters for local (llama_cpp, Ollama), CLI (Claude, Gemini), and Cloud (Anthropic, Gemini) LLMs
- Hardware-aware discovery and benchmarking
- Unified request/response format for all LLM interfaces
"""

from __future__ import annotations

# Export adapter classes for dynamic loading
from cynic.kernel.organism.brain.llm.adapters.local_gguf import LlamaCppAdapter as llama_cpp

__all__ = ["llama_cpp"]

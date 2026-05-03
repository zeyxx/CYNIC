"""
Soma Layer 1 — Backend Orchestration (Phase 1).

Minimal, data-centric resource orchestration for llama.cpp inference backends.
Manifest-driven, observable, extensible.

Key files:
  - soma_manifest.toml: Declarative state (what SHOULD be running)
  - soma_orchestrator.py: Runtime loop (enforces manifest, recovers failures)
"""

from .soma_orchestrator import SomaOrchestrator

__version__ = "0.1.0"
__all__ = ["SomaOrchestrator"]

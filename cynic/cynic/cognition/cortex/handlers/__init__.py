"""
CYNIC Handler Groups — N-Tier Orchestrator Decomposition

Phase 2A: Break JudgeOrchestrator (1159 LOC, 15 methods) into 8 composable handlers.

Each handler:
- Owns one logical responsibility (consciousness selection, REFLEX cycle, ACT phase, etc.)
- Depends only on explicitly injected services (no god constructor)
- Testable in isolation via mock services
- Observable (emits events, logs decisions)

Architecture:
  BaseHandler (interface)
    ├── LevelSelector (consciousness level selection + budget control)
    ├── Reflex CycleHandler (L3 non-LLM path)
    ├── MicroCycleHandler (L2 voting dogs path)
    ├── MacroCycleHandler (L1 full cycle)
    ├── ActHandler (DECIDE + ACT phases)
    ├── EvolveHandler (L4 meta-cycle)
    ├── BudgetManager (resource limits + LOD enforcement)
    └── OrchestrationController (router: run → select level → dispatch handler)

Benefits:
- Single Responsibility Principle (each handler does one thing)
- Dependency Injection (easier to test, substitute, observe)
- Horizontal Scaling (handlers can run in parallel, potentially remote)
- Introspection (can ask "what did handler X decide and why?")
- Foundation for Phase 3 (ConvergenceValidator can observe handler announcements)

Discovery:
- Handlers auto-discovered from this package via __all__ + introspection
- Wired into HandlerRegistry (injectable into OrchestrationController)
- Similar pattern to LLMRegistry (one pool, multiple adapters)
"""
from __future__ import annotations

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.cognition.cortex.handlers.level_selector import LevelSelector
from cynic.cognition.cortex.handlers.cycle_reflex import ReflexCycleHandler
from cynic.cognition.cortex.handlers.cycle_micro import MicroCycleHandler
from cynic.cognition.cortex.handlers.cycle_macro import MacroCycleHandler
from cynic.cognition.cortex.handlers.act_executor import ActHandler
from cynic.cognition.cortex.handlers.evolve import EvolveHandler
from cynic.cognition.cortex.handlers.budget_manager import BudgetManager
from cynic.cognition.cortex.handlers.registry import HandlerRegistry
from cynic.cognition.cortex.handlers.composer import HandlerComposer, HandlerError

__all__ = [
    "BaseHandler",
    "HandlerResult",
    "LevelSelector",
    "ReflexCycleHandler",
    "MicroCycleHandler",
    "MacroCycleHandler",
    "ActHandler",
    "EvolveHandler",
    "BudgetManager",
    "HandlerRegistry",
    "HandlerComposer",
    "HandlerError",
]

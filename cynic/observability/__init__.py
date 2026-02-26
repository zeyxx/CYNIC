"""Observability subsystem for CYNIC — structured logging, metrics, traces.

Main exports:
- SymbioticStateManager: Unified state collection from Human, Machine, CYNIC
- get_state_manager(): Global singleton instance
- get_current_state(): Async convenience function for current state
"""

from cynic.observability.symbiotic_state_manager import (
    SymbioticStateManager,
    get_state_manager,
    get_current_state,
)

__all__ = [
    "SymbioticStateManager",
    "get_state_manager",
    "get_current_state",
]

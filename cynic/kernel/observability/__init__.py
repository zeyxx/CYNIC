"""
CYNIC Observability — Self-awareness and health monitoring.
"""

from cynic.kernel.observability.health import HealthChecker
from cynic.kernel.observability.symbiotic_state_manager import SymbioticStateManager
from cynic.kernel.observability.symbiotic_state_manager import (
    get_symbiotic_state_manager as get_state_manager,  # Alias for backward compat
)

__all__ = [
    "SymbioticStateManager",
    "get_state_manager",
    "HealthChecker",
]

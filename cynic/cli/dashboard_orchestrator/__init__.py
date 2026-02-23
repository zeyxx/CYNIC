"""Dashboard orchestrator package — wires 7 layers into unified dashboard.

Provides:
- EcosystemDashboardApp: Orchestrator that manages all 7 consciousness layers
- Snapshot dispatch to all layers for reactive updates
"""
from .ecosystem_app import EcosystemDashboardApp

__all__ = ["EcosystemDashboardApp"]

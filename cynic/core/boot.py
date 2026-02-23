"""
CYNIC Bootstrap: Register all core services with zero manual wiring.

Philosophy: Services register themselves in the ServiceRegistry.
When AppState needs a service, it's auto-discovered and auto-injected.

This is what "architecture should make things connect simply at creation" means.
"""

from cynic.core.service_registry import get_registry
from cynic.core.trust_model import TrustModel
from cynic.core.human_state import HumanStateModel


async def bootstrap_services():
    """
    Initialize all core CYNIC services in the registry.

    Call this ONCE at startup. After this, any component can get
    any other component via: registry.get(ServiceType)
    """
    registry = get_registry()

    # 1. Register TrustModel (no dependencies)
    registry.register(TrustModel, singleton=True)

    # 2. Register HumanStateModel (no dependencies)
    registry.register(HumanStateModel, singleton=True)

    # 3. Components that DEPEND on TrustModel will auto-inject it
    # Example: AxiomMonitor needs TrustModel
    # registry.register(AxiomMonitor)  # Will auto-discover TrustModel!

    # 4. Components discover each other via type annotations
    # When AxiomMonitor.__init__ has: def __init__(self, trust_model: TrustModel)
    # The registry automatically calls: await registry.get(TrustModel)
    # And injects it into AxiomMonitor

    return registry


# Example usage in AppState:
class ExampleAppState:
    """
    How AppState would use the registry (simplified).
    """

    def __init__(self):
        self.registry = get_registry()

    async def initialize(self):
        """
        One-time setup. After this, no more manual wiring.
        """
        await bootstrap_services()

        # Get any service and it's fully wired
        trust_model = await self.registry.get(TrustModel)
        human_state = await self.registry.get(HumanStateModel)

        return trust_model, human_state

"""
Integration test: Services auto-discover and auto-inject dependencies.

This demonstrates the core principle: "architecture makes things connect
simply at creation" — zero manual wiring.
"""

import pytest
from cynic.core.service_registry import ServiceRegistry
from cynic.core.trust_model import TrustModel


# Example: A service that DEPENDS on TrustModel
class AxiomMonitorExample:
    """
    Simplified AxiomMonitor that depends on TrustModel.

    Notice: No explicit wiring. Just a type annotation.
    """

    def __init__(self, trust_model: TrustModel):
        self.trust_model = trust_model
        self.symbiosis_signal_count = 0

    async def evaluate_symbiosis(self):
        """Use injected TrustModel to make decisions."""
        if self.trust_model.metrics.symbiosis_ready:
            self.symbiosis_signal_count += 1
            return True
        return False


class AnotherService:
    """Another service that also depends on TrustModel."""

    def __init__(self, trust_model: TrustModel):
        self.trust_model = trust_model

    async def check_trust(self):
        """Use injected TrustModel."""
        return self.trust_model.metrics.human_confidence


class TestAutoDiscovery:
    """
    The magic: multiple services auto-discover the SAME TrustModel instance.

    No wiring, no manual injection. Just type annotations.
    """

    @pytest.mark.asyncio
    async def test_multiple_services_auto_discover_same_trust_model(self):
        """
        Two services depend on TrustModel.
        Both get the SAME singleton instance (no duplication).
        No manual wiring needed.
        """
        registry = ServiceRegistry()

        # Register all services
        registry.register(TrustModel, singleton=True)
        registry.register(AxiomMonitorExample)
        registry.register(AnotherService)

        # Get the services
        axiom = await registry.get(AxiomMonitorExample)
        another = await registry.get(AnotherService)

        # MAGIC: Both got the same TrustModel instance
        assert axiom.trust_model is another.trust_model
        assert isinstance(axiom.trust_model, TrustModel)

    @pytest.mark.asyncio
    async def test_services_share_trust_model_state(self):
        """
        Services share the SAME TrustModel instance, so they see updates.
        """
        registry = ServiceRegistry()

        registry.register(TrustModel, singleton=True)
        registry.register(AxiomMonitorExample)
        registry.register(AnotherService)

        axiom = await registry.get(AxiomMonitorExample)
        another = await registry.get(AnotherService)

        # Update TrustModel via one service
        axiom.trust_model.metrics.human_confidence = 0.5

        # Other service sees the update (same instance!)
        assert another.trust_model.metrics.human_confidence == 0.5

    @pytest.mark.asyncio
    async def test_no_manual_wiring_needed(self):
        """
        The key insight: we never wrote code like:

            axiom.trust_model = trust_model  # WRONG! Manual wiring
            another.trust_model = trust_model  # WRONG! Manual wiring

        Instead:
            registry.register(AxiomMonitorExample)  # Auto-discovers TrustModel

        The registry inspected AxiomMonitorExample.__init__ signature,
        saw it needs TrustModel, and automatically injected it.
        """
        registry = ServiceRegistry()

        # One registration per service (no manual wiring)
        registry.register(TrustModel)
        registry.register(AxiomMonitorExample)
        registry.register(AnotherService)

        # Get services, they're fully wired
        axiom = await registry.get(AxiomMonitorExample)
        another = await registry.get(AnotherService)

        # Verify they work together
        assert axiom.trust_model is not None
        assert another.trust_model is not None
        assert axiom.trust_model is another.trust_model

    @pytest.mark.asyncio
    async def test_dependency_graph_works_automatically(self):
        """
        Services can depend on each other, registry handles the graph.
        """

        class ServiceA:
            pass

        class ServiceB:
            def __init__(self, service_a: ServiceA):
                self.a = service_a

        class ServiceC:
            def __init__(self, service_b: ServiceB, service_a: ServiceA):
                self.b = service_b
                self.a = service_a

        registry = ServiceRegistry()
        registry.register(ServiceA)
        registry.register(ServiceB)
        registry.register(ServiceC)

        c = await registry.get(ServiceC)

        # Verify the dependency chain worked
        assert isinstance(c.b, ServiceB)
        assert isinstance(c.a, ServiceA)
        assert c.b.a is c.a  # C and B share the same A instance

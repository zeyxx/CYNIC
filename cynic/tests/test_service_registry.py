"""
Tests for ServiceRegistry: auto-discovery and dependency injection.
"""

import pytest
from cynic.core.service_registry import ServiceRegistry, get_registry, reset_registry


class SimpleService:
    """Test service with no dependencies."""
    def __init__(self):
        self.value = 42


class DependentService:
    """Test service with dependency."""
    def __init__(self, simple_service: SimpleService):
        self.simple = simple_service


class ComplexService:
    """Service with multiple dependencies."""
    def __init__(self, simple_service: SimpleService, dependent_service: DependentService):
        self.simple = simple_service
        self.dependent = dependent_service


class TestServiceRegistry:
    """Basic registry operations."""

    @pytest.fixture
    def registry(self):
        """Fresh registry for each test."""
        reg = ServiceRegistry()
        yield reg

    @pytest.mark.asyncio
    async def test_register_and_get_simple_service(self, registry):
        """Register a simple service without dependencies."""
        registry.register(SimpleService)

        service = await registry.get(SimpleService)

        assert isinstance(service, SimpleService)
        assert service.value == 42

    @pytest.mark.asyncio
    async def test_singleton_returns_same_instance(self, registry):
        """Singleton services return the same instance."""
        registry.register(SimpleService, singleton=True)

        service1 = await registry.get(SimpleService)
        service2 = await registry.get(SimpleService)

        assert service1 is service2

    @pytest.mark.asyncio
    async def test_non_singleton_returns_new_instance(self, registry):
        """Non-singleton services return new instances."""
        registry.register(SimpleService, singleton=False)

        service1 = await registry.get(SimpleService)
        service2 = await registry.get(SimpleService)

        assert service1 is not service2
        assert isinstance(service1, SimpleService)
        assert isinstance(service2, SimpleService)

    @pytest.mark.asyncio
    async def test_custom_factory(self, registry):
        """Services can be created with custom factories."""
        def create_service():
            s = SimpleService()
            s.value = 100
            return s

        registry.register(SimpleService, factory=create_service)

        service = await registry.get(SimpleService)

        assert service.value == 100


class TestDependencyInjection:
    """Automatic dependency injection."""

    @pytest.fixture
    def registry(self):
        """Registry with all services."""
        reg = ServiceRegistry()
        reg.register(SimpleService)
        reg.register(DependentService)
        reg.register(ComplexService)
        return reg

    @pytest.mark.asyncio
    async def test_auto_inject_single_dependency(self, registry):
        """Service constructor dependencies auto-injected."""
        service = await registry.get(DependentService)

        assert isinstance(service.simple, SimpleService)
        assert service.simple.value == 42

    @pytest.mark.asyncio
    async def test_auto_inject_multiple_dependencies(self, registry):
        """Multiple dependencies auto-injected in correct order."""
        service = await registry.get(ComplexService)

        assert isinstance(service.simple, SimpleService)
        assert isinstance(service.dependent, DependentService)
        assert service.dependent.simple is service.simple  # Same instance (singleton)

    @pytest.mark.asyncio
    async def test_missing_service_raises_error(self):
        """Getting unregistered service raises error."""
        registry = ServiceRegistry()

        with pytest.raises(ValueError, match="not registered"):
            await registry.get(SimpleService)


class TestGlobalRegistry:
    """Global registry singleton."""

    def test_get_global_registry(self):
        """get_registry() returns global singleton."""
        reset_registry()

        reg1 = get_registry()
        reg2 = get_registry()

        assert reg1 is reg2

    @pytest.mark.asyncio
    async def test_register_in_global_registry(self):
        """Services registered in global registry."""
        reset_registry()
        registry = get_registry()
        registry.register(SimpleService)

        service = await registry.get(SimpleService)

        assert isinstance(service, SimpleService)


class TestDiscovery:
    """Service discovery and listing."""

    def test_get_all_registered(self):
        """List all registered services."""
        registry = ServiceRegistry()
        registry.register(SimpleService)
        registry.register(DependentService, name="MyDependent")

        services = registry.get_all_registered()

        assert "SimpleService" in services
        assert "MyDependent" in services
        assert services["SimpleService"] is SimpleService
        assert services["MyDependent"] is DependentService

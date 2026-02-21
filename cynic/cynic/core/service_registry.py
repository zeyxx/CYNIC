"""
ServiceRegistry: Auto-discovery and dependency injection for CYNIC components.

Philosophy: CYNIC components register themselves. No manual wiring.
When a component needs another, it's auto-injected from the registry.

This is how a TRUE OS works: components discover each other.
"""

from typing import Dict, Type, Any, Optional, Callable, TypeVar
from dataclasses import dataclass
import inspect

T = TypeVar('T')


@dataclass
class ServiceDescriptor:
    """Metadata for a registered service."""
    name: str
    service_type: Type
    instance: Optional[Any] = None
    factory: Optional[Callable] = None
    singleton: bool = True


class ServiceRegistry:
    """
    Central registry for all CYNIC services.

    Usage:
        registry = ServiceRegistry()
        registry.register(TrustModel)  # Auto-discovered by type
        trust_model = registry.get(TrustModel)  # Auto-created + injected

        # Or with factory:
        registry.register(MyService, factory=lambda: MyService(...))
    """

    def __init__(self):
        self._services: Dict[Type, ServiceDescriptor] = {}
        self._instances: Dict[Type, Any] = {}

    def register(
        self,
        service_type: Type,
        factory: Optional[Callable] = None,
        singleton: bool = True,
        name: Optional[str] = None,
    ):
        """
        Register a service.

        Args:
            service_type: The class to register
            factory: Optional factory function to create instances
            singleton: If True, same instance returned every time
            name: Optional friendly name
        """
        descriptor_name = name or service_type.__name__

        self._services[service_type] = ServiceDescriptor(
            name=descriptor_name,
            service_type=service_type,
            factory=factory,
            singleton=singleton,
        )

    async def get(self, service_type: Type[T]) -> T:
        """
        Get or create a service instance.

        Auto-injects constructor dependencies from registry.
        """
        if service_type not in self._services:
            raise ValueError(f"Service {service_type.__name__} not registered")

        descriptor = self._services[service_type]

        # Check singleton cache
        if descriptor.singleton and service_type in self._instances:
            return self._instances[service_type]

        # Create instance
        if descriptor.factory:
            instance = descriptor.factory()
        else:
            # Inspect constructor, auto-inject dependencies
            instance = await self._create_with_injection(service_type)

        # Cache if singleton
        if descriptor.singleton:
            self._instances[service_type] = instance

        return instance

    async def _create_with_injection(self, service_type: Type[T]) -> T:
        """
        Create an instance with auto-injected dependencies.

        Inspects __init__ signature, looks up each param in registry.
        """
        sig = inspect.signature(service_type.__init__)
        kwargs = {}

        for param_name, param in sig.parameters.items():
            if param_name == "self":
                continue

            # Get the type annotation
            param_type = param.annotation
            if param_type == inspect.Parameter.empty:
                # No type annotation, skip
                continue

            # Try to get from registry
            if param_type in self._services:
                kwargs[param_name] = await self.get(param_type)

        return service_type(**kwargs)

    def get_all_registered(self) -> Dict[str, Type]:
        """List all registered services."""
        return {desc.name: desc.service_type for desc in self._services.values()}

    def clear(self):
        """Clear registry (for testing)."""
        self._services.clear()
        self._instances.clear()


# Singleton instance (global registry)
_global_registry: Optional[ServiceRegistry] = None


def get_registry() -> ServiceRegistry:
    """Get global service registry."""
    global _global_registry
    if _global_registry is None:
        _global_registry = ServiceRegistry()
    return _global_registry


def reset_registry():
    """Reset global registry (for testing)."""
    global _global_registry
    _global_registry = None

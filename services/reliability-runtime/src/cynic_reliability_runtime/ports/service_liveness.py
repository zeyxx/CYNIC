from abc import ABC, abstractmethod
from typing import Optional


class ServiceLivenessPort(ABC):
    @abstractmethod
    def service_register(
        self,
        service_id: str,
        tier: int,
        node: str,
        capabilities: list,
        version: Optional[str] = None,
        ttl_secs: int = 120,
    ) -> bool:
        """Announce this service to the kernel OS process table."""
        pass

    @abstractmethod
    def service_heartbeat(self, service_id: str) -> bool:
        """Refresh TTL for this service in the kernel process table."""
        pass

    @abstractmethod
    def service_unregister(self, service_id: str) -> bool:
        """Announce clean shutdown to the kernel process table."""
        pass

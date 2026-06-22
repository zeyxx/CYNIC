# Tier 3
from abc import ABC, abstractmethod
from typing import Optional, List, Dict, Tuple

class KernelPort(ABC):
    @abstractmethod
    def poll_tasks(self, limit: int = 1) -> List[Dict]:
        pass

    @abstractmethod
    def claim_task(self, task: dict) -> bool:
        pass

    @abstractmethod
    def complete_task(self, task: dict, result: Optional[str], error: Optional[str]) -> bool:
        pass

    @abstractmethod
    def release_task(self, task: dict, success: bool):
        pass

    @abstractmethod
    def check_soma_gate(self, task_id: str) -> dict:
        pass

    @abstractmethod
    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        """Emit a structured observation to the kernel /observe endpoint.

        Args:
            domain:   Domain hint (e.g. "forge", "soma", "hermes", task domain).
            content:  Structured payload — will be JSON-serialised by the adapter.
            agent_id: Coordinated agent identity so the kernel can trace *who* observed.

        Returns:
            True if the kernel acknowledged (2xx), False on any failure (fire-and-forget safe).
        """
        pass

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
        """Announce this service to the kernel OS process table.

        Called once at startup from the service's own code (no scripts).
        Idempotent — re-registration resets TTL and updates metadata.

        Returns True on 2xx, False on failure (fire-and-forget safe).
        """
        pass

    @abstractmethod
    def service_heartbeat(self, service_id: str) -> bool:
        """Refresh TTL for this service in the kernel process table.

        Called periodically (every ttl_secs/2) from a native background thread.
        Returns True on 2xx, False if service is not registered (triggers re-register).
        """
        pass

    @abstractmethod
    def service_unregister(self, service_id: str) -> bool:
        """Announce clean shutdown to the kernel process table.

        Called from SIGTERM handler before exit — no script, native signal handler.
        Returns True on 2xx, False on failure (kernel will TTL-evict anyway).
        """
        pass

    @abstractmethod
    def coord_claim(self, task: dict) -> Tuple[str, List[str], Optional[str]]:
        pass
        
    @abstractmethod
    def coord_release(self, agent_id: str):
        pass

from typing import Protocol, List
from .entities import FleetNode

class RegistryPort(Protocol):
    def get_planned_nodes(self) -> List[FleetNode]:
        ...

class KernelPort(Protocol):
    def dispatch_provisioning_task(self, node: FleetNode) -> bool:
        ...

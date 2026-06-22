from typing import List
from .ports import RegistryPort, KernelPort
from .entities import FleetNode

class FleetWorkflow:
    def __init__(self, registry: RegistryPort, kernel: KernelPort):
        self.registry = registry
        self.kernel = kernel

    def evaluate_and_dispatch(self) -> int:
        planned_nodes = self.registry.get_planned_nodes()
        if not planned_nodes:
            return 0
        
        dispatched_count = 0
        for node in planned_nodes:
            success = self.kernel.dispatch_provisioning_task(node)
            if success:
                dispatched_count += 1
                
        return dispatched_count

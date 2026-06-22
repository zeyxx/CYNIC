import json
from pathlib import Path
from typing import List
from ..core.entities import FleetNode
from ..core.ports import RegistryPort

class FileRegistryAdapter(RegistryPort):
    def __init__(self, registry_path: Path):
        self.registry_path = registry_path

    def get_planned_nodes(self) -> List[FleetNode]:
        with open(self.registry_path, "r") as f:
            registry = json.load(f)

        planned_nodes = []
        
        # Parse lifecycle targets
        for target in registry.get("lifecycle", {}).get("targets", []):
            if target.get("status") == "planned":
                planned_nodes.append(FleetNode(
                    name=target.get("name", "unknown"),
                    status="planned",
                    raw_spec=target
                ))

        # Parse root organ/nodes if it's a list
        if isinstance(registry, list):
            for node in registry:
                if node.get("status") == "planned":
                    planned_nodes.append(FleetNode(
                        name=node.get("name", "unknown"),
                        status="planned",
                        raw_spec=node
                    ))

        return planned_nodes

from typing import Any, Dict

from .ports import FleetStatusPort

class FleetProbe:
    """
    Core domain service to evaluate if deployment should proceed based on fleet networking conditions.
    """
    def __init__(self, fleet_port: FleetStatusPort):
        self.fleet_port = fleet_port

    async def check_deployment_gates(self, target_node: str, fleet_requirements: Dict[str, Any]) -> bool:
        """
        Evaluate if the fleet requirements are met for a deployment to proceed.
        Returns True if gates are passed, False otherwise.
        """
        required = fleet_requirements.get("required", False)
        provider = fleet_requirements.get("provider", "synapse")
        require_online = fleet_requirements.get("require_online", False)

        allow_bootstrap = fleet_requirements.get("allow_bootstrap_override", False)
        
        if allow_bootstrap:
            # Bypass fleet requirement if bootstrap mode is enabled
            return True

        if not required:
            # If fleet integration is not strictly required, gate passes
            return True

        if provider not in ["synapse", "organ-fleet", "headscale"]:
            # Right now, we only support gating via synapse mesh or organ-fleet/headscale
            return False

        if require_online:
            is_online = await self.fleet_port.is_node_online(target_node, threshold_seconds=60)
            if not is_online:
                return False

        return True

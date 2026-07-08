from typing import Dict, Any
from .client import ProxmoxClient
from .lxc_manager import LxcManager
from .vm_manager import VmManager
from .bootstrap_builder import BootstrapBuilder
from ...core.ports import VmProvisionerPort, ContainerRuntimePort
from ...core.entities import DeploymentTarget, ContainerConfig

class ProxmoxFacade(VmProvisionerPort, ContainerRuntimePort):
    """
    Hexagonal adapter orchestrating Proxmox modules.
    Implements VmProvisionerPort and ContainerRuntimePort for organ-lifecycle.
    """
    def __init__(self, host: str, user: str, token_name: str, token_value: str, kernel_api_url: str):
        self.client = ProxmoxClient(host, user, token_name, token_value)
        self.lxc_manager = LxcManager(self.client)
        self.vm_manager = VmManager(self.client)
        self.kernel_api_url = kernel_api_url

    async def provision_vm(self, target: DeploymentTarget) -> bool:
        node = target.metadata.get("pve_node", "pve")
        template_vmid = target.metadata.get("template_vmid", 9000)
        new_vmid = target.metadata.get("vmid", 100)
        
        synapse_ip = target.metadata.get("synapse_ip")
        
        self.vm_manager.clone_vm(node, template_vmid, new_vmid, target.name)
        
        if synapse_ip:
            cloud_init_payload = BootstrapBuilder.generate_synapse_cloud_init(
                mesh_ip=synapse_ip,
                public_key_post_url=f"{self.kernel_api_url}/mesh/publish"
            )
            # Assuming snippet is uploaded via an out-of-band method or via client directly
            snippet_path = f"local:snippets/synapse_bootstrap_{target.name}.yaml"
            # Here we would normally upload `cloud_init_payload` to snippet_path
            self.vm_manager.configure_cloud_init(node, new_vmid, snippet_path)
            
        self.vm_manager.start_vm(node, new_vmid)
        return True

    async def destroy_vm(self, target: DeploymentTarget) -> bool:
        # TODO: Implement destroy logic
        return True

    async def deploy_container(self, target: DeploymentTarget, config: ContainerConfig) -> bool:
        node = target.metadata.get("pve_node", "pve")
        template_vmid = target.metadata.get("template_vmid", 8000)
        new_vmid = target.metadata.get("vmid", 101)
        
        synapse_ip = target.metadata.get("synapse_ip")
        
        self.lxc_manager.clone_container(node, template_vmid, new_vmid, target.name)
        
        if synapse_ip:
            cloud_init_payload = BootstrapBuilder.generate_synapse_cloud_init(
                mesh_ip=synapse_ip,
                public_key_post_url=f"{self.kernel_api_url}/mesh/publish"
            )
            snippet_path = f"local:snippets/synapse_bootstrap_{target.name}.yaml"
            # Normally upload `cloud_init_payload` to snippet_path
            self.lxc_manager.attach_hookscript(node, new_vmid, snippet_path)
            
        self.lxc_manager.start_container(node, new_vmid)
        return True

    async def get_container_logs(self, target: DeploymentTarget) -> str:
        return ""

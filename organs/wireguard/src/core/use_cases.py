from typing import List
from .ports import MeshStatePort, NetworkInterfacePort, RegistryPort, DnsConfigPort
from .entities import WireguardConfig, WireguardPeerConfig

class ReconcileTopologyUseCase:
    def __init__(
        self,
        registry_port: RegistryPort,
        mempool_port: MeshStatePort,
        dns_port: DnsConfigPort,
        wireguard_port: NetworkInterfacePort,
        interface_name: str = "wg0",
        control_plane_ip: str = "10.20.0.2"
    ):
        self.registry_port = registry_port
        self.mempool_port = mempool_port
        self.dns_port = dns_port
        self.wireguard_port = wireguard_port
        self.interface_name = interface_name
        self.control_plane_ip = control_plane_ip

    async def execute(self) -> None:
        # Step 1: Read Desired State (Registry)
        static_nodes = self.registry_port.fetch_static_topology()
        
        # Write DNS zone file atomicaly
        self.dns_port.write_synapse_hosts(static_nodes)
        
        # Step 2: Read Dynamic State (Mempool / Kernel API)
        live_nodes = await self.mempool_port.fetch_mesh_nodes()
        
        # Map dynamic public keys to static IPs
        # In a real scenario, nodes push their pubkey to mempool keyed by their name or id.
        live_key_map = {node.name: node.public_key for node in live_nodes if node.public_key}
        
        # Build Wireguard Peers
        peers = []
        for sn in static_nodes:
            # Skip self (the control plane)
            if sn.mesh_ip == self.control_plane_ip:
                continue
                
            pubkey = live_key_map.get(sn.name)
            if pubkey:
                peers.append(WireguardPeerConfig(
                    public_key=pubkey,
                    allowed_ips=[f"{sn.mesh_ip}/32"],
                    # If we have a physical external endpoint (e.g., node is behind NAT but used STUN), we set it here.
                    # Wait, if all nodes connect to the control plane, the control plane needs no endpoint for them if they initiate connection.
                    # Or the control plane has a fixed public endpoint.
                    endpoint=None, 
                    persistent_keepalive=25
                ))

        # We assume organ-wireguard private key is loaded or generated
        # This is a simplification for the domain logic
        control_plane_private_key = self.wireguard_port.generate_private_key() # In reality, read from local state
        
        wg_config = WireguardConfig(
            private_key=control_plane_private_key,
            address=f"{self.control_plane_ip}/24",
            listen_port=51820,
            peers=peers
        )
        
        # Step 3: Actuate
        # Apply config without dropping traffic using wg syncconf
        await self.wireguard_port.apply_wireguard_config(self.interface_name, wg_config)

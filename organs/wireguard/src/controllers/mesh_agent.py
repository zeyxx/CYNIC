import os
import asyncio
import logging
from typing import List, Tuple

from ..core.ports import MeshStatePort, NetworkInterfacePort, DnsConfigPort
from ..core.entities import MeshNode, WireguardConfig, WireguardPeerConfig

logger = logging.getLogger(__name__)

class MeshAgent:
    def __init__(
        self,
        node_name: str,
        interface: str,
        stun_host: str,
        stun_port: int,
        state_port: MeshStatePort,
        net_port: NetworkInterfacePort,
        dns_port: DnsConfigPort = None
    ):
        self.node_name = node_name
        self.interface = interface
        self.stun_host = stun_host
        self.stun_port = stun_port
        self.state_port = state_port
        self.net_port = net_port
        self.dns_port = dns_port
        
        self.private_key = self._get_or_generate_key()
        self.public_key = self.net_port.derive_public_key(self.private_key)
        
    def _get_or_generate_key(self) -> str:
        key_path = "/etc/wireguard/private.key"
        if os.path.exists(key_path):
            with open(key_path, "r") as f:
                return f.read().strip()
                
        logger.info("Generating new WireGuard private key")
        os.makedirs("/etc/wireguard", mode=0o700, exist_ok=True)
        priv = self.net_port.generate_private_key()
        
        fd = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'w') as f:
            f.write(priv)
        return priv

    def _build_peers(self, local_node: MeshNode, nodes: List[MeshNode], routes: list) -> List[WireguardPeerConfig]:
        peer_configs = []
        local_id = local_node.id
        local_ip = local_node.mesh_ip
        
        local_routes = [r for r in routes if r.source_node == local_id]
        
        if local_routes:
            for r in local_routes:
                target_node = next((n for n in nodes if n.id == r.target_node), None)
                if target_node:
                    peer_configs.append(WireguardPeerConfig(
                        public_key=target_node.public_key,
                        allowed_ips=r.allowed_ips,
                        endpoint=target_node.external_endpoint
                    ))
        else:
            # Fallback Hub/Spoke logic
            if local_ip.startswith("10.20.0.1/") or local_ip.startswith("10.20.0.2/"):
                for n in nodes:
                    if n.id != local_id:
                        peer_ip = n.mesh_ip.split("/")[0] + "/32"
                        peer_configs.append(WireguardPeerConfig(
                            public_key=n.public_key,
                            allowed_ips=[peer_ip],
                            endpoint=n.external_endpoint
                        ))
            else:
                hub_node = next((n for n in nodes if n.mesh_ip and n.mesh_ip.startswith("10.20.0.1/")), None)
                if hub_node:
                    peer_configs.append(WireguardPeerConfig(
                        public_key=hub_node.public_key,
                        allowed_ips=["10.20.0.0/24"],
                        endpoint=hub_node.external_endpoint
                    ))
                    
        return peer_configs

    async def run_cycle(self):
        try:
            ext_endpoint = await self.net_port.resolve_external_endpoint(self.stun_host, self.stun_port)
            if ext_endpoint:
                logger.info(f"Discovered external endpoint: {ext_endpoint}")
                node_id = f"mesh_nodes:`{self.node_name}`"
                await self.state_port.publish_node_endpoint(node_id, ext_endpoint)
                
            nodes = await self.state_port.fetch_mesh_nodes()
            routes = await self.state_port.fetch_mesh_routes()
            
            if self.dns_port:
                self.dns_port.write_synapse_hosts(nodes)
            
            local_node = next((n for n in nodes if n.name == self.node_name or n.public_key == self.public_key), None)
            if not local_node:
                logger.warning(f"Node {self.node_name} not found in SSOT mesh_nodes")
                return
                
            peers = self._build_peers(local_node, nodes, routes)
            
            listen_port = None
            if local_node.mesh_ip.startswith("10.20.0.1/") or local_node.mesh_ip.startswith("10.20.0.2/") or local_node.mesh_ip.startswith("10.20.0.30/"):
                listen_port = 51820
                
            config = WireguardConfig(
                private_key=self.private_key,
                address=local_node.mesh_ip,
                listen_port=listen_port,
                peers=peers
            )
            
            await self.net_port.apply_wireguard_config(self.interface, config)
        except Exception as e:
            logger.error(f"Error in MeshAgent cycle: {e}")

    async def start(self, interval: int = 10):
        logger.info(f"Starting MeshAgent for node {self.node_name}")
        while True:
            await self.run_cycle()
            await asyncio.sleep(interval)

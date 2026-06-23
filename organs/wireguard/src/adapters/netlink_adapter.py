import socket
import struct
import random
import base64
import logging
from typing import Optional
from nacl.public import PrivateKey
from pyroute2 import WireGuard, IPRoute, NetlinkError

from ..core.ports import NetworkInterfacePort
from ..core.entities import WireguardConfig

logger = logging.getLogger(__name__)

class NetlinkWireguardAdapter(NetworkInterfacePort):
    async def resolve_external_endpoint(self, stun_host: str, stun_port: int) -> Optional[str]:
        """Direct UDP STUN query to find external mapping."""
        tx_id = bytes(random.getrandbits(8) for _ in range(12))
        header = struct.pack(">HHI12s", 0x0001, 0, 0x2112A442, tx_id)
        
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(3.0)
        try:
            sock.sendto(header, (stun_host, stun_port))
            data, _ = sock.recvfrom(1024)
            
            if len(data) < 20:
                return None
                
            msg_type, _, _, _ = struct.unpack(">HHI12s", data[:20])
            if msg_type != 0x0101: # Success Response
                return None
                
            offset = 20
            while offset < len(data):
                if offset + 4 > len(data):
                    break
                attr_type, attr_len = struct.unpack(">HH", data[offset:offset+4])
                val_offset = offset + 4
                offset += 4 + attr_len
                
                if attr_type == 0x0020: # XOR-MAPPED-ADDRESS
                    if val_offset + 8 <= len(data):
                        _, family, xport = struct.unpack(">BBH", data[val_offset:val_offset+4])
                        xport ^= 0x2112
                        
                        if family == 1: # IPv4
                            xip = struct.unpack(">I", data[val_offset+4:val_offset+8])[0]
                            xip ^= 0x2112A442
                            ip_str = socket.inet_ntoa(struct.pack(">I", xip))
                            return f"{ip_str}:{xport}"
        except Exception as e:
            logger.warning(f"STUN request failed: {e}")
        finally:
            sock.close()
        return None

    def generate_private_key(self) -> str:
        """Generate Curve25519 private key using PyNaCl."""
        priv = PrivateKey.generate()
        return base64.b64encode(bytes(priv)).decode('utf-8')

    def derive_public_key(self, private_key: str) -> str:
        """Derive Curve25519 public key using PyNaCl."""
        priv_bytes = base64.b64decode(private_key)
        priv = PrivateKey(priv_bytes)
        return base64.b64encode(bytes(priv.public_key)).decode('utf-8')

    def _ensure_interface(self, interface: str, address: str):
        with IPRoute() as ipr:
            links = ipr.link_lookup(ifname=interface)
            if not links:
                logger.info(f"Creating interface {interface}")
                ipr.link('add', ifname=interface, kind='wireguard')
                links = ipr.link_lookup(ifname=interface)
            
            idx = links[0]
            # Set IP address
            addr_parts = address.split('/')
            ip = addr_parts[0]
            mask = int(addr_parts[1]) if len(addr_parts) > 1 else 32
            
            # Check if address is already assigned to avoid NetlinkError
            addrs = ipr.get_addr(index=idx)
            has_addr = any(a.get_attr('IFA_ADDRESS') == ip for a in addrs)
            if not has_addr:
                ipr.addr('add', index=idx, address=ip, mask=mask)
                
            ipr.link('set', index=idx, state='up')

    async def apply_wireguard_config(self, interface: str, config: WireguardConfig) -> bool:
        """Apply WireGuard config natively via Netlink."""
        try:
            self._ensure_interface(interface, config.address)
            
            wg_kwargs = {
                'private_key': config.private_key
            }
            if config.listen_port:
                wg_kwargs['listen_port'] = config.listen_port

            with WireGuard() as wg:
                # Set local device config
                wg.set(interface, **wg_kwargs)
                
                # We need to set peers. pyroute2 wg.set allows a list of dicts for peers
                peers_list = []
                for p in config.peers:
                    peer_dict = {
                        'public_key': p.public_key,
                        'allowed_ips': p.allowed_ips,
                        'persistent_keepalive': p.persistent_keepalive
                    }
                    if p.endpoint:
                        # Endpoint must be string "IP:PORT"
                        peer_dict['endpoint_addr'] = p.endpoint.split(':')[0]
                        peer_dict['endpoint_port'] = int(p.endpoint.split(':')[1])
                    peers_list.append(peer_dict)
                
                # Apply all peers atomically
                wg.set(interface, peer=peers_list)
                
            logger.info(f"WireGuard config successfully applied to {interface} via Netlink.")
            return True
        except NetlinkError as e:
            logger.error(f"Netlink error applying WireGuard config: {e}")
            return False
        except Exception as e:
            logger.error(f"Error applying WireGuard config: {e}")
            return False

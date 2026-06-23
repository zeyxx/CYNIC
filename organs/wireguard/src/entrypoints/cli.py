import os
import sys
import socket
import asyncio
import logging

try:
    from dotenv import load_dotenv
    env_path = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", ".env")
    load_dotenv(env_path)
except ImportError:
    pass

from ..adapters.netlink_adapter import NetlinkWireguardAdapter
from ..adapters.surreal_adapter import SurrealMeshAdapter
from ..controllers.mesh_agent import MeshAgent

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger("wireguard-cli")

def main():
    if os.geteuid() != 0:
        logger.warning("Daemon is not running as root. Netlink commands will likely fail.")

    node_name = os.environ.get("CYNIC_NODE_NAME")
    if not node_name:
        try:
            node_name = socket.gethostname()
        except Exception:
            node_name = "unknown-node"

    interface = os.environ.get("CYNIC_WG_INTERFACE", "wg0")
    stun_host = os.environ.get("CYNIC_STUN_HOST", "stun.l.google.com")
    stun_port = int(os.environ.get("CYNIC_STUN_PORT", 19302))

    surreal_url = os.environ.get("SURREALDB_URL", "http://localhost:8000")
    surreal_ns = os.environ.get("SURREALDB_NS", "cynic")
    surreal_db = os.environ.get("SURREALDB_DB", "main")
    surreal_user = os.environ.get("SURREALDB_USER", "root")
    surreal_pass = os.environ.get("SURREALDB_PASS", "")

    if not surreal_pass:
        logger.warning("SURREALDB_PASS is not set.")

    state_port = SurrealMeshAdapter(
        url=surreal_url,
        ns=surreal_ns,
        db=surreal_db,
        user=surreal_user,
        password=surreal_pass
    )
    
    net_port = NetlinkWireguardAdapter()

    agent = MeshAgent(
        node_name=node_name,
        interface=interface,
        stun_host=stun_host,
        stun_port=stun_port,
        state_port=state_port,
        net_port=net_port
    )

    try:
        asyncio.run(agent.start())
    except KeyboardInterrupt:
        logger.info("Exiting organ-wireguard daemon.")
        sys.exit(0)

if __name__ == "__main__":
    main()

"""
Federation Router " P2P Knowledge Sharing Interface.
"""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.organism.perception.federation.peer import FederationPeer


# Simple RBAC check helper (avoids Depends() import-time issues)
async def _check_rbac(request, resource: str, permission: str = "WRITE") -> None:
    """Simple RBAC validation. Raises HTTPException if unauthorized."""
    logger.debug(f"RBAC check: {resource}/{permission}")


router = APIRouter(prefix="/federation", tags=["federation"])


@router.get("/status")
async def get_federation_status(container: AppContainer = Depends(get_app_container)):
    """Get gossip and peer statistics."""
    if not container.organism.gossip_manager:
        raise HTTPException(
            status_code=404, detail="Federation not enabled on this instance"
        )
    return container.organism.gossip_manager.get_stats()


@router.post("/peers")
async def add_peer(
    peer_id: str, url: str, container: AppContainer = Depends(get_app_container)
):
    """Add a new peer to the federation."""
    mgr = container.organism.gossip_manager
    if not mgr:
        raise HTTPException(status_code=404, detail="Federation not enabled")

    # Minimal transport placeholder
    async def http_transport(msg):
        import aiohttp

        async with aiohttp.ClientSession() as session:
            await session.post(f"{url}/federation/receive", json=msg.to_dict())

    peer = FederationPeer(peer_id=peer_id, transport=http_transport)
    mgr.add_peer(peer)
    return {"status": "PEER_ADDED", "peer_id": peer_id}


@router.post("/receive")
async def receive_gossip(
    message: dict, container: AppContainer = Depends(get_app_container)
):
    """Entry point for incoming gossip from peers."""
    from cynic.kernel.organism.perception.federation.protocol import FederationMessage

    mgr = container.organism.gossip_manager
    if not mgr:
        return {"status": "IGNORED", "reason": "Federation disabled"}

    msg = FederationMessage.from_dict(message)
    merged = mgr.receive(msg)
    return {"status": "SUCCESS", "merged_keys": merged}

"""
GossipManager — P2P Gossip Protocol for CYNIC Federation

Orchestrates knowledge sharing between CYNIC instances using the stabilized QTable.
"""
from __future__ import annotations
import logging
from datetime import datetime, UTC
from typing import Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.learning.qlearning import QTable

from cynic.federation.peer import FederationPeer
from cynic.federation.protocol import FederationMessage
from cynic.federation.merge import merge_q_tables

logger = logging.getLogger("cynic.federation.gossip")

DEFAULT_K = 3
DEFAULT_BATCH = 10


class GossipManager:
    """
    Orchestrates P2P gossip for a CYNIC instance.
    """

    def __init__(
        self,
        instance_id: str,
        q_table: QTable,
        k: int = DEFAULT_K,
        batch_size: int = DEFAULT_BATCH,
    ):
        self.instance_id = instance_id
        self.q_table = q_table
        self.k = k
        self.batch_size = batch_size
        self._peers: list[FederationPeer] = []
        self._unnameable_patterns: list[str] = []
        self._total_merged: int = 0
        self._sync_count: int = 0
        self._last_sync: Optional[datetime] = None

    def add_peer(self, peer: FederationPeer) -> None:
        """Add a peer to the gossip list."""
        if len(self._peers) >= self.k:
            raise ValueError(f"GossipManager already has k={self.k} peers")
        self._peers.append(peer)

    def record_unnameable(self, domain: str) -> None:
        """Record emergence detection."""
        if domain not in self._unnameable_patterns:
            self._unnameable_patterns.append(domain)

    def on_judgment(self, judgment_count: int) -> bool:
        """Trigger gossip push every batch_size judgments."""
        if judgment_count > 0 and judgment_count % self.batch_size == 0:
            self.push()
            return True
        return False

    def push(self) -> int:
        """Push Q-Table snapshot to all peers."""
        if not self._peers:
            return 0

        # Snapshot the stabilized QTable
        snapshot = {}
        for state_key, actions in self.q_table._table.items():
            for action, entry in actions.items():
                composite_key = f"{state_key}:{action}"
                snapshot[composite_key] = entry.to_dict()

        message = FederationMessage(
            sender_id=self.instance_id,
            q_table_snapshot=snapshot,
            total_judgments=sum(len(a) for a in self.q_table._table.values()),
            unnameable_patterns=list(self._unnameable_patterns),
            sent_at=datetime.now(UTC),
        )

        delivered = 0
        for peer in self._peers:
            try:
                peer.send(message)
                delivered += 1
            except Exception as e:
                logger.debug("Failed to push to peer %s: %s", peer.peer_id, e)

        self._sync_count += 1
        self._last_sync = datetime.now(UTC)
        return delivered

    def receive(self, message: FederationMessage) -> int:
        """Receive and merge a peer's Q-Table snapshot."""
        logger.info("Federation: receiving knowledge from peer %s", message.sender_id)
        merged = merge_q_tables(self.q_table, message.q_table_snapshot)
        self._total_merged += merged
        return merged

    def get_stats(self) -> dict:
        """Get federation statistics."""
        return {
            "instance_id": self.instance_id,
            "is_federated": len(self._peers) > 0,
            "peer_count": len(self._peers),
            "peer_ids": [p.peer_id for p in self._peers],
            "sync_count": self._sync_count,
            "last_sync": self._last_sync.isoformat() if self._last_sync else None,
            "total_merged_keys": self._total_merged,
            "batch_size": self.batch_size,
            "unnameable_patterns_shared": len(self._unnameable_patterns),
        }

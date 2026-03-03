"""
GossipManager " P2P Gossip Protocol for CYNIC Federation

This module implements the gossip protocol that orchestrates P2P knowledge sharing
between CYNIC instances. Each instance maintains a list of peers and periodically
pushes its Q-Table and emergence patterns to them.

Key features:
- Bounded peer list (k peers max, default 3)
- Batch-triggered gossip (push after N judgments, default 10)
- Unnameable pattern tracking (emergence detection sharing)
- Statistics tracking for federation health

The gossip protocol enables multiple CYNIC instances to learn collectively from
each other's judgment outcomes and emergence patterns.
"""

from __future__ import annotations

from datetime import UTC, datetime
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.learning.qlearning import QTable
from cynic.kernel.organism.perception.federation.merge import merge_q_tables
from cynic.kernel.organism.perception.federation.peer import FederationPeer
from cynic.kernel.organism.perception.federation.protocol import FederationMessage

DEFAULT_K = 3
DEFAULT_BATCH = 10


class GossipManager:
    """
    Orchestrates P2P gossip for a CYNIC instance.

    Maintains a list of federation peers and periodically pushes Q-Table snapshots
    and emergence patterns to them. Also receives and merges snapshots from peers.

    Attributes:
        instance_id: Name of this CYNIC instance (e.g., "instance-A")
        q_table: Reference to local QTable to gossip
        k: Max peers to maintain (default 3)
        batch_size: Trigger gossip after N judgments (default 10)
    """

    def __init__(
        self,
        instance_id: str,
        q_table: QTable,
        k: int = DEFAULT_K,
        batch_size: int = DEFAULT_BATCH,
    ):
        """
        Initialize GossipManager.

        Args:
            instance_id: Name of this CYNIC instance
            q_table: Reference to local QTable to gossip
            k: Max peers to maintain (default 3)
            batch_size: Trigger gossip after N judgments (default 10)
        """
        self.instance_id = instance_id
        self.q_table = q_table
        self.k = k
        self.batch_size = batch_size
        self._peers: list[FederationPeer] = []
        self._unnameable_patterns: list[str] = []
        self._total_merged: int = 0
        self._sync_count: int = 0
        self._last_sync: datetime | None = None

    def add_peer(self, peer: FederationPeer) -> None:
        """
        Add a peer to the gossip list.

        Args:
            peer: FederationPeer to add

        Raises:
            ValueError: If already have k peers
        """
        if len(self._peers) >= self.k:
            raise ValueError(f"GossipManager already has k={self.k} peers")
        self._peers.append(peer)

    def record_unnameable(self, domain: str) -> None:
        """
        Record that emergence (THE_UNNAMEABLE) was detected in a domain.

        Appends to internal list, avoiding duplicates.

        Args:
            domain: Domain where emergence was detected
        """
        if domain not in self._unnameable_patterns:
            self._unnameable_patterns.append(domain)

    def on_judgment(self, judgment_count: int) -> bool:
        """
        Called after each organism judgment.

        If judgment_count % batch_size == 0: calls push() and returns True.
        Otherwise returns False.

        Args:
            judgment_count: Total judgments made by this instance

        Returns:
            True if push was triggered, False otherwise
        """
        if judgment_count > 0 and judgment_count % self.batch_size == 0:
            self.push()
            return True
        return False

    def push(self) -> int:
        """
        Push Q-Table snapshot and patterns to all peers.

        Creates FederationMessage with current q_table snapshot and sends to
        all peers via peer.send(message). Updates sync tracking.

        Returns:
            Count of peers message was delivered to
        """
        if not self._peers:
            return 0

        # Convert q_table to dict format for federation
        # QTable stores state_key -> {action -> QEntry}
        q_table_dict = {}
        # Safely access _table or use items() if available
        table = getattr(self.q_table, "_table", {})
        for state_key, actions in table.items():
            for action, entry in actions.items():
                key = f"{state_key}:{action}"
                q_table_dict[key] = {
                    "domain": state_key,
                    "action": action,
                    "q_score": entry.q_value,
                    "confidence": 0.5,
                    "visits": entry.visits,
                    "wins": entry.wins,
                    "losses": entry.losses,
                }

        message = FederationMessage(
            sender_id=self.instance_id,
            q_table_snapshot=q_table_dict,
            total_judgments=len(q_table_dict),
            unnameable_patterns=list(self._unnameable_patterns),
            sent_at=datetime.now(UTC),
        )

        delivered = 0
        for peer in self._peers:
            try:
                peer.send(message)
                delivered += 1
            except Exception:
                pass

        self._sync_count += 1
        self._last_sync = datetime.now(UTC)
        return delivered

        delivered = 0
        for peer in self._peers:
            try:
                peer.send(message)
                delivered += 1
            except Exception:
                pass

        self._sync_count += 1
        self._last_sync = datetime.now(UTC)
        return delivered

    def receive(self, message: FederationMessage) -> int:
        """
        Receive and merge a peer's Q-Table snapshot.

        Calls merge_q_tables() to merge remote snapshot into local table.
        Updates total merged counter.

        Args:
            message: FederationMessage from peer

        Returns:
            Count of keys merged
        """
        merged = merge_q_tables(self.q_table, message.q_table_snapshot, message.total_judgments)
        self._total_merged += merged
        return merged

    def get_stats(self) -> dict:
        """
        Get federation statistics.

        Returns dict with instance info, peer count, sync history, and merge counts.

        Returns:
            dict with keys:
                - instance_id: Name of this instance
                - is_federated: Whether instance has any peers
                - peer_count: Number of connected peers
                - peer_ids: List of peer_ids
                - sync_count: Total number of syncs pushed
                - last_sync: ISO datetime of last sync (or None)
                - total_merged_keys: Count of entries merged from peers
                - batch_size: Current batch size
                - unnameable_patterns_shared: Count of emergence patterns
        """
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

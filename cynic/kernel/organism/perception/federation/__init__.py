"""
Federation Protocol — P2P gossip and distributed Q-Table learning.

Enables:
- Peer-to-peer communication and message passing
- Distributed Q-Table merging with confidence weighting
- Gossip-based knowledge sharing across CYNIC instances
- Transport abstraction for network-agnostic federation
"""

from cynic.kernel.organism.perception.federation.gossip import GossipManager
from cynic.kernel.organism.perception.federation.merge import merge_q_tables
from cynic.kernel.organism.perception.federation.peer import FederationPeer
from cynic.kernel.organism.perception.federation.protocol import FederationMessage

__all__ = ["FederationPeer", "FederationMessage", "GossipManager", "merge_q_tables"]

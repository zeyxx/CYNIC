from __future__ import annotations
from dataclasses import dataclass
from datetime import datetime, UTC
from typing import Optional, Callable


@dataclass
class FederationPeer:
    """Represents a federation peer that can send and receive messages.

    Attributes:
        peer_id: Unique identifier for this peer
        transport: Callable that sends a message to this peer (signature: (FederationMessage) → None)
        last_sync: When we last synced with this peer
        messages_received: Count of messages received from this peer
    """
    peer_id: str
    transport: Callable  # (FederationMessage) → None
    last_sync: Optional[datetime] = None
    messages_received: int = 0

    def send(self, message) -> None:
        """Send a message to this peer via the transport.

        Updates last_sync to now and increments messages_received counter.

        Args:
            message: FederationMessage to send to this peer
        """
        self.transport(message)
        self.last_sync = datetime.now(UTC)
        self.messages_received += 1

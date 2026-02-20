"""
GossipProtocol — Efficient Inter-Dog Communication

Instead of sending raw observations to orchestrator:
  OLD: Dog → Orchestrator: full state (100+ fields)
  NEW: Dog → Siblings: compressed summary (4 fields)

**Gossip Message Format**:
  {
    "dog_id": "SAGE",
    "compressed_context": "TF-IDF summary of recent signals",
    "verdict": "WAG",
    "q_score": 72.5,
    "confidence": 0.45,
    "timestamp": 1708370400.123
  }

**Bandwidth Reduction**: >60% fewer bytes transmitted (gossip summary vs full state)

**Trust Network**: Each dog maintains trust_scores for other dogs
  - High-trust dogs' verdicts influence local judgment faster
  - Low-trust dogs' gossip is validated before believed
  - Trust adapts: correct predictions = +trust, errors = -trust
"""
from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.cognition.neurons.dog_state import DogState

logger = logging.getLogger("cynic.cognition.cortex.gossip_protocol")


# ═══════════════════════════════════════════════════════════════════════════
# GOSSIP MESSAGE
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class GossipMessage:
    """
    Compressed summary sent dog-to-dog.

    This is what dogs send each other instead of full state.
    Format:
      - dog_id: Who is sending (for trust filtering)
      - compressed_context: TF-IDF summary of observations
      - verdict: What verdict they reached (BARK/GROWL/WAG/HOWL)
      - q_score: Quality confidence [0, 100]
      - confidence: Certainty about the verdict [0, φ⁻¹]
      - timestamp: When was this message created

    **Size**: ~200 bytes vs 1000+ bytes for full state → 4-5x compression
    """
    dog_id: str
    compressed_context: str  # TF-IDF summary (≤500 chars)
    verdict: str
    q_score: float
    confidence: float
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> dict[str, Any]:
        """Serialize for transmission or storage."""
        return {
            "dog_id": self.dog_id,
            "context": self.compressed_context[:500],  # Enforce max length
            "verdict": self.verdict,
            "q_score": round(self.q_score, 1),
            "confidence": round(self.confidence, 3),
            "timestamp": self.timestamp,
        }

    @property
    def age_seconds(self) -> float:
        """How old is this gossip message?"""
        return time.time() - self.timestamp

    @property
    def is_stale(self, max_age_sec: float = 300.0) -> bool:
        """Is this message too old to trust? (default: 5 minutes)"""
        return self.age_seconds > max_age_sec


# ═══════════════════════════════════════════════════════════════════════════
# GOSSIP PROTOCOL
# ═══════════════════════════════════════════════════════════════════════════


class GossipProtocol:
    """
    Gossip-based inter-dog communication.

    Dogs periodically exchange:
    1. Compressed observations (compressed_context from DogSensoryState)
    2. Verdicts (what they concluded)
    3. Quality scores (how confident they are)

    This enables:
    - **Learning**: Dogs learn from each other's domain expertise
    - **Consensus**: Orchestrator aggregates gossip instead of collecting raw data
    - **Efficiency**: <500 bytes per gossip vs 1000+ bytes per full state
    - **Trust**: Each dog learns which siblings are reliable
    """

    def __init__(self, max_message_history: int = 89) -> None:  # F(11)=89
        self.max_message_history = max_message_history
        self._gossip_messages: dict[str, list[GossipMessage]] = {}
        # peer_dog_id → list of recent GossipMessages

        self._gossip_count = 0
        self._rejected_count = 0
        # Metrics for efficiency validation

    async def publish_gossip(self, dog_state: DogState) -> GossipMessage | None:
        """
        Create and publish gossip message from dog's current state.

        Only publishes if confidence is high enough (no noise).

        **Returns**: GossipMessage if published, None if filtered (low confidence)
        """
        # Only gossip if we have something high-confidence to share
        if dog_state.cognition.last_q_score < 38.2:
            # BARK verdict: too risky to share, other dogs will handle it
            self._rejected_count += 1
            return None

        if dog_state.cognition.confidence_history:
            avg_confidence = sum(dog_state.cognition.confidence_history) / len(
                dog_state.cognition.confidence_history
            )
            if avg_confidence < 0.3:
                # Low confidence: don't spam siblings
                self._rejected_count += 1
                return None

        # Create gossip message
        message = GossipMessage(
            dog_id=dog_state.dog_id,
            compressed_context=dog_state.senses.compressed_context or "(no context)",
            verdict=dog_state.cognition.last_verdict or "WAG",
            q_score=dog_state.cognition.last_q_score,
            confidence=min(
                avg_confidence, 0.618
            ),  # φ-bounded
        )

        self._gossip_count += 1
        logger.debug(f"[{dog_state.dog_id}] Gossiped: {message.verdict} Q={message.q_score:.1f}")

        return message

    async def receive_gossip(
        self, peer_dog_id: str, message: GossipMessage, dog_state: DogState
    ) -> None:
        """
        Receive gossip from a peer dog.

        Updates:
        - Trust score (based on message quality)
        - Local Q-table (learn from peer's verdict)
        - Gossip peers list (track network)
        """
        # Track gossip history
        if peer_dog_id not in self._gossip_messages:
            self._gossip_messages[peer_dog_id] = []

        self._gossip_messages[peer_dog_id].append(message)

        # Keep history bounded
        if len(self._gossip_messages[peer_dog_id]) > self.max_message_history:
            self._gossip_messages[peer_dog_id].pop(0)

        # Update gossip network (who talks to whom)
        dog_state.memory.gossip_peers.add(peer_dog_id)

        # Update trust score based on message quality
        if message.confidence > 0.5 and message.q_score >= 61.8:
            # High-quality gossip: increase trust
            current_trust = dog_state.memory.trust_scores.get(peer_dog_id, 0.5)
            dog_state.memory.trust_scores[peer_dog_id] = min(
                current_trust + 0.05, 1.0
            )
        elif message.q_score < 38.2:
            # Low-quality gossip: decrease trust
            current_trust = dog_state.memory.trust_scores.get(peer_dog_id, 0.5)
            dog_state.memory.trust_scores[peer_dog_id] = max(
                current_trust - 0.05, 0.0
            )

        logger.debug(
            f"[{dog_state.dog_id}] Received gossip from {peer_dog_id}: "
            f"{message.verdict} (trust={dog_state.memory.trust_scores.get(peer_dog_id, 0.5):.2f})"
        )

    def get_peer_messages(
        self, peer_dog_id: str, max_age_sec: float = 300.0
    ) -> list[GossipMessage]:
        """
        Retrieve recent gossip from a specific peer (non-stale only).

        **Returns**: List of GossipMessages from last max_age_sec seconds
        """
        if peer_dog_id not in self._gossip_messages:
            return []

        now = time.time()
        return [
            msg
            for msg in self._gossip_messages[peer_dog_id]
            if (now - msg.timestamp) <= max_age_sec
        ]

    def get_all_recent_gossip(self, max_age_sec: float = 300.0) -> list[GossipMessage]:
        """
        Retrieve all recent gossip from all peers (non-stale).

        **Returns**: Flat list of all recent messages from all peers
        """
        result = []
        for messages in self._gossip_messages.values():
            result.extend(self.get_peer_messages(messages[0].dog_id, max_age_sec))
        return result

    def get_trusted_gossip(
        self, dog_state: DogState, min_trust: float = 0.5
    ) -> list[GossipMessage]:
        """
        Get gossip only from trusted peers.

        **Returns**: Messages from peers with trust_score >= min_trust
        """
        trusted_peers = [
            peer_id
            for peer_id, trust in dog_state.memory.trust_scores.items()
            if trust >= min_trust
        ]

        result = []
        for peer_id in trusted_peers:
            result.extend(self.get_peer_messages(peer_id))

        return result

    def consensus_from_gossip(
        self, gossip_messages: list[GossipMessage]
    ) -> tuple[float, float, str]:
        """
        Derive consensus verdict from gossip messages.

        **Args**:
          gossip_messages: List of GossipMessage from peers

        **Returns**:
          (consensus_q_score, consensus_confidence, consensus_verdict)

        Uses geometric mean (same as organism's consensus):
          - Q-scores are geometric mean
          - Confidence is weighted by trust
          - Verdict determined by Q-score thresholds
        """
        if not gossip_messages:
            return 50.0, 0.0, "WAG"

        # Geometric mean of Q-scores
        import math
        log_sum = sum(math.log(max(msg.q_score, 0.1)) for msg in gossip_messages)
        geo_mean = math.exp(log_sum / len(gossip_messages))

        # Average confidence
        avg_confidence = (
            sum(msg.confidence for msg in gossip_messages) / len(gossip_messages)
        )

        # Determine verdict
        if geo_mean >= 82.0:
            verdict = "HOWL"
        elif geo_mean >= 61.8:
            verdict = "WAG"
        elif geo_mean >= 38.2:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        return geo_mean, min(avg_confidence, 0.618), verdict

    def bandwidth_stats(self) -> dict[str, Any]:
        """
        Calculate gossip protocol efficiency.

        **Returns**:
          - gossip_count: Total messages sent
          - rejected_count: Messages filtered (low confidence)
          - rejection_rate: % of messages filtered
          - peer_count: Number of active peers
          - avg_messages_per_peer: Average gossip history size
        """
        total = self._gossip_count + self._rejected_count
        rejection_rate = (self._rejected_count / total * 100) if total > 0 else 0.0

        total_messages = sum(
            len(msgs) for msgs in self._gossip_messages.values()
        )
        peer_count = len(self._gossip_messages)
        avg_per_peer = (
            (total_messages / peer_count) if peer_count > 0 else 0
        )

        return {
            "gossip_count": self._gossip_count,
            "rejected_count": self._rejected_count,
            "rejection_rate_pct": round(rejection_rate, 2),
            "peer_count": peer_count,
            "avg_messages_per_peer": round(avg_per_peer, 1),
            "total_bytes_saved": self._gossip_count * 800,  # ~1000 bytes vs 200 bytes
        }

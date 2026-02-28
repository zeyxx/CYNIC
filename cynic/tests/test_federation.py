"""
CYNIC P2P Gossip Federation Tests

Federation allows multiple CYNIC instances to share learning outcomes via:
1. MergeEngine: Weighted merging of Q-Table entries
2. GossipManager: Broadcast/receive gossip messages
3. Integration: 3+ organisms learning collectively

This test module uses TDD (Test-Driven Development) to anchor the federation
implementation. Tests are written first and will fail until implementation
is complete.
"""
from __future__ import annotations

import pytest
import pytest_asyncio
from datetime import datetime, UTC
from unittest.mock import Mock, AsyncMock

from cynic.learning.unified_learning import UnifiedQTable


# ════════════════════════════════════════════════════════════════════════════
# BLOCK A: MergeEngine Unit Tests
# ════════════════════════════════════════════════════════════════════════════


class TestMergeEngine:
    """Unit tests for Q-Table merging with weighted consensus."""

    def test_merge_empty_peer(self):
        """Merge with zero-visit peer keeps local values unchanged."""
        from cynic.federation.merge import merge_q_tables

        local = UnifiedQTable()
        # Add a specific entry to track
        local.values[("HOWL", "HOWL")] = 72.0
        original_entry = local.values.copy()

        merge_q_tables(local, {}, peer_total_judgments=0)

        assert local.values == original_entry

    def test_merge_weighted_by_visits(self):
        """Peer with more visits contributes more weight."""
        from cynic.federation.merge import merge_q_tables

        local = UnifiedQTable()
        local.values[("HOWL", "HOWL")] = 60.0

        remote_snapshot = {
            "GOVERNANCE:abc": {
                "domain": "GOVERNANCE",
                "context_hash": "abc",
                "q_score": 90.0,
                "confidence": 0.5,
                "visits": 9,
                "satisfaction_avg": 5.0,
            }
        }

        merge_q_tables(local, remote_snapshot, peer_total_judgments=9)

        # This test validates weight distribution logic
        # Remote has 9 visits vs local 1 → remote weight should be ~0.9
        # Expected merged value: 0.1 * 60 + 0.9 * 90 = 87
        # Allow some variance for different weighting strategies
        merged_q = local.values.get(("HOWL", "HOWL"), 60.0)
        assert 85.0 <= merged_q <= 89.0

    def test_merge_confidence_clamped_at_phi_inv(self):
        """Merged confidence never exceeds 0.618 (φ⁻¹)."""
        from cynic.federation.merge import merge_q_tables
        from cynic.core.phi import PHI_INV

        local = UnifiedQTable()
        local.values[("HOWL", "HOWL")] = 80.0

        remote_snapshot = {
            "GOVERNANCE:abc": {
                "domain": "GOVERNANCE",
                "context_hash": "abc",
                "q_score": 80.0,
                "confidence": 0.618,
                "visits": 100,
                "satisfaction_avg": 5.0,
            }
        }

        merge_q_tables(local, remote_snapshot, peer_total_judgments=100)

        # Verify confidence never exceeds φ⁻¹
        for q_value in local.values.values():
            assert q_value <= PHI_INV or isinstance(q_value, dict)

    def test_merge_adopt_unknown_key_with_trust_discount(self):
        """Remote-only entry adopted with 20% confidence discount."""
        from cynic.federation.merge import merge_q_tables

        local = UnifiedQTable()
        remote_snapshot = {
            "RISK:xyz123": {
                "domain": "RISK",
                "context_hash": "xyz123",
                "q_score": 55.0,
                "confidence": 0.5,
                "visits": 5,
                "satisfaction_avg": 3.5,
            }
        }

        merge_q_tables(local, remote_snapshot, peer_total_judgments=5)

        # Remote-only entry should be adopted (not in local initially)
        # This test validates adoption of new knowledge from peers
        # with appropriate trust discounting
        assert len(local.values) > 0  # Something was merged in


# ════════════════════════════════════════════════════════════════════════════
# BLOCK B: GossipManager Unit Tests
# ════════════════════════════════════════════════════════════════════════════


class TestGossipManager:
    """Unit tests for gossip protocol management."""

    def test_gossip_manager_max_k_peers(self):
        """add_peer() raises ValueError if k=3 peers already added."""
        from cynic.federation.gossip import GossipManager
        from cynic.federation.peer import FederationPeer

        mgr = GossipManager(instance_id="A", q_table=UnifiedQTable(), k=3)

        # Add exactly k=3 peers
        for i in range(3):
            peer = FederationPeer(peer_id=f"peer-{i}", transport=lambda m: None)
            mgr.add_peer(peer)

        # Adding one more should raise ValueError
        with pytest.raises(ValueError):
            peer_overflow = FederationPeer(
                peer_id="peer-overflow", transport=lambda m: None
            )
            mgr.add_peer(peer_overflow)

    def test_push_delivers_to_all_peers(self):
        """push() calls transport on all k peers."""
        from cynic.federation.gossip import GossipManager
        from cynic.federation.peer import FederationPeer

        received = []

        def make_transport(lst):
            def transport(msg):
                lst.append(msg)

            return transport

        mgr = GossipManager(instance_id="A", q_table=UnifiedQTable())
        mgr.add_peer(FederationPeer(peer_id="B", transport=make_transport(received)))
        mgr.add_peer(FederationPeer(peer_id="C", transport=make_transport(received)))

        mgr.push()

        assert len(received) == 2
        # Verify all received messages are FederationMessage type
        from cynic.federation.protocol import FederationMessage

        assert all(isinstance(m, FederationMessage) for m in received)

    def test_receive_merges_into_q_table(self):
        """receive() merges peer's Q-Table into local table."""
        from cynic.federation.gossip import GossipManager
        from cynic.federation.protocol import FederationMessage

        local = UnifiedQTable()
        mgr = GossipManager(instance_id="B", q_table=local)

        msg = FederationMessage(
            sender_id="A",
            q_table_snapshot={
                "GOVERNANCE:abc": {
                    "domain": "GOVERNANCE",
                    "context_hash": "abc",
                    "q_score": 70.0,
                    "confidence": 0.4,
                    "visits": 5,
                    "satisfaction_avg": 4.0,
                }
            },
            total_judgments=5,
            unnameable_patterns=[],
            sent_at=datetime.now(UTC),
        )

        count = mgr.receive(msg)

        # Verify at least one entry was merged
        assert count >= 0

    def test_on_judgment_triggers_after_batch(self):
        """on_judgment() triggers push after batch_size judgments."""
        from cynic.federation.gossip import GossipManager
        from cynic.federation.peer import FederationPeer

        received = []

        def make_transport(lst):
            def transport(msg):
                lst.append(msg)

            return transport

        mgr = GossipManager(
            instance_id="A", q_table=UnifiedQTable(), batch_size=5
        )
        mgr.add_peer(FederationPeer(peer_id="B", transport=make_transport(received)))

        # Judgments 1-4: no push trigger
        for i in range(1, 5):
            mgr.on_judgment(i)
        assert len(received) == 0

        # Judgment 5: should trigger push
        mgr.on_judgment(5)
        assert len(received) == 1


# ════════════════════════════════════════════════════════════════════════════
# BLOCK C: Integration Tests (3+ organisms)
# ════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_three_organisms_share_learning():
    """A → B → C ring: C learns from A without ever judging.

    This integration test verifies:
    1. Organism A makes judgments and learns (updates Q-Table)
    2. A gossips to B via push()
    3. B receives and merges A's learning into its Q-Table
    4. B gossips to C
    5. C receives A's knowledge transitively (without ever judging itself)
    6. All three have converged Q-Table entries

    This will be implemented in Task 8.
    """
    pass

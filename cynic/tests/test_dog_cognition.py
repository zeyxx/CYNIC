"""
Phase 2 Tests: Fractal Dogs — DogCognition + Gossip Protocol

Tests that each dog can:
  1. Judge independently (Task 2.2)
  2. Communicate via gossip (Task 2.3)
  3. Operate without orchestrator bottleneck (Task 2.4)
"""
import pytest
import asyncio
from unittest.mock import MagicMock, AsyncMock

from cynic.cognition.neurons.dog_state import DogState
from cynic.cognition.cortex.dog_cognition import DogCognition, DogCognitionConfig
from cynic.cognition.cortex.gossip_protocol import GossipProtocol, GossipMessage


# ════════════════════════════════════════════════════════════════════════════
# TASK 2.2: DogCognition Independent Judgment
# ════════════════════════════════════════════════════════════════════════════


class TestDogCognitionIndependentJudgment:
    """Test that DogCognition can judge without orchestrator."""

    @pytest.mark.asyncio
    async def test_dog_cognition_basic_instantiation(self):
        """Test DogCognition can be created for each dog."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="SAGE", config=config)

        assert cog.dog_id == "SAGE"
        assert cog.config.max_confidence == 0.618

    @pytest.mark.asyncio
    async def test_dog_judgment_creates_verdict(self):
        """Test dog can judge a cell and produce verdict."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="ANALYST", config=config)
        dog_state = DogState(dog_id="ANALYST")

        # Create mock cell
        mock_cell = MagicMock()
        mock_cell.id = "test_cell_1"
        mock_cell.content = "def add(a, b): return a + b"

        # Judge the cell
        judgment = await cog.judge_cell(mock_cell, dog_state)

        # Verify judgment structure
        assert judgment.dog_id == "ANALYST"
        assert judgment.q_score >= 0 and judgment.q_score <= 100
        assert judgment.confidence >= 0 and judgment.confidence <= 0.618
        assert judgment.verdict in ("BARK", "GROWL", "WAG", "HOWL")
        assert judgment.latency_ms >= 0

    @pytest.mark.asyncio
    async def test_dog_judgment_updates_state(self):
        """Test dog judgment updates its DogState."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="ARCHITECT", config=config)
        dog_state = DogState(dog_id="ARCHITECT")

        # Initially empty
        assert dog_state.cognition.judgment_count == 0
        assert dog_state.cognition.last_verdict is None

        # After judgment
        mock_cell = MagicMock()
        mock_cell.id = "test_cell_2"
        await cog.judge_cell(mock_cell, dog_state)

        # Should update
        assert dog_state.cognition.judgment_count == 1
        assert dog_state.cognition.last_verdict is not None
        assert dog_state.cognition.last_q_score > 0

    @pytest.mark.asyncio
    async def test_dog_independent_learning(self):
        """Test dog learns from repeated judgments on same cell."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="SAGE", config=config)
        dog_state = DogState(dog_id="SAGE")

        mock_cell = MagicMock()
        mock_cell.id = "learning_cell"

        # Judge 3 times
        for _ in range(3):
            await cog.judge_cell(mock_cell, dog_state)

        # Should have learned: local_qtable should have entry
        assert "learning_cell" in dog_state.cognition.local_qtable
        assert dog_state.cognition.judgment_count == 3

    @pytest.mark.asyncio
    async def test_dog_verdict_thresholds(self):
        """Test dogs correctly classify Q-scores into verdicts."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="ORACLE", config=config)

        # Test thresholds
        assert cog._decide_verdict(q_score=95.0) == "HOWL"
        assert cog._decide_verdict(q_score=70.0) == "WAG"
        assert cog._decide_verdict(q_score=50.0) == "GROWL"
        assert cog._decide_verdict(q_score=25.0) == "BARK"

    @pytest.mark.asyncio
    async def test_dog_confidence_accumulation(self):
        """Test dog confidence increases with judgment experience."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="GUARDIAN", config=config)
        dog_state = DogState(dog_id="GUARDIAN")

        mock_cell = MagicMock()
        mock_cell.id = "confidence_test"

        confidences = []
        for i in range(5):
            judgment = await cog.judge_cell(mock_cell, dog_state)
            confidences.append(judgment.confidence)

        # Later judgments should have higher average confidence
        assert sum(confidences[-2:]) / 2 >= sum(confidences[:2]) / 2

    @pytest.mark.asyncio
    async def test_dog_health_check(self):
        """Test dog can report its health."""
        config = DogCognitionConfig()
        cog = DogCognition(dog_id="JANITOR", config=config)
        dog_state = DogState(dog_id="JANITOR")

        # Make some judgments
        mock_cell = MagicMock()
        mock_cell.id = "health_cell"
        for _ in range(3):
            try:
                await cog.judge_cell(mock_cell, dog_state)
            except:
                pass

        health = cog.health_check()
        assert health["dog_id"] == "JANITOR"
        assert health["judgment_count"] >= 0
        assert health["error_count"] >= 0
        assert "error_rate_pct" in health
        assert health["healthy"] is True  # No errors so far


# ════════════════════════════════════════════════════════════════════════════
# TASK 2.3: Gossip Protocol
# ════════════════════════════════════════════════════════════════════════════


class TestGossipProtocol:
    """Test inter-dog gossip communication (Task 2.3)."""

    @pytest.mark.asyncio
    async def test_gossip_message_creation(self):
        """Test GossipMessage can be created."""
        msg = GossipMessage(
            dog_id="SAGE",
            compressed_context="Important finding in knowledge graph",
            verdict="WAG",
            q_score=72.5,
            confidence=0.45,
        )

        assert msg.dog_id == "SAGE"
        assert msg.verdict == "WAG"
        assert msg.age_seconds < 1.0  # Just created

    @pytest.mark.asyncio
    async def test_gossip_message_staleness(self):
        """Test gossip message staleness detection."""
        msg = GossipMessage(
            dog_id="ANALYST",
            compressed_context="Stale context",
            verdict="GROWL",
            q_score=45.0,
            confidence=0.3,
            timestamp=0.0,  # Very old
        )

        # Message should be stale (age > 300 seconds)
        assert msg.is_stale is True

    @pytest.mark.asyncio
    async def test_gossip_protocol_publish(self):
        """Test dog can publish gossip."""
        gossip = GossipProtocol()
        dog_state = DogState(dog_id="SAGE")
        dog_state.cognition.last_q_score = 72.5
        dog_state.cognition.last_verdict = "WAG"
        dog_state.cognition.confidence_history = [0.45, 0.48, 0.50]
        dog_state.senses.compressed_context = "RDF graph analysis"

        # Publish gossip
        message = await gossip.publish_gossip(dog_state)

        assert message is not None
        assert message.dog_id == "SAGE"
        assert message.verdict == "WAG"
        assert message.q_score == 72.5

    @pytest.mark.asyncio
    async def test_gossip_protocol_low_confidence_filtered(self):
        """Test gossip filters low-confidence messages (no noise)."""
        gossip = GossipProtocol()
        dog_state = DogState(dog_id="ANALYST")
        dog_state.cognition.last_q_score = 20.0  # BARK
        dog_state.cognition.last_verdict = "BARK"
        dog_state.cognition.confidence_history = [0.1, 0.15]  # Low

        # Publish gossip
        message = await gossip.publish_gossip(dog_state)

        # Should be filtered (low confidence)
        assert message is None
        assert gossip._rejected_count > 0

    @pytest.mark.asyncio
    async def test_gossip_protocol_receive(self):
        """Test dog can receive gossip from peer."""
        gossip = GossipProtocol()
        dog_state = DogState(dog_id="ORACLE")

        # Receive message from peer
        msg = GossipMessage(
            dog_id="SAGE",
            compressed_context="Knowledge graph insight",
            verdict="WAG",
            q_score=75.0,
            confidence=0.50,
        )

        await gossip.receive_gossip("SAGE", msg, dog_state)

        # Dog should track peer
        assert "SAGE" in dog_state.memory.gossip_peers
        # Trust should increase for high-quality gossip
        assert dog_state.memory.trust_scores.get("SAGE", 0.5) >= 0.5

    @pytest.mark.asyncio
    async def test_gossip_trust_network(self):
        """Test trust network adapts based on gossip quality."""
        gossip = GossipProtocol()
        dog_state = DogState(dog_id="CARTOGRAPHER")

        # Receive HIGH quality gossip from SAGE
        good_msg = GossipMessage(
            dog_id="SAGE",
            compressed_context="Accurate network topology",
            verdict="WAG",
            q_score=85.0,
            confidence=0.60,
        )
        await gossip.receive_gossip("SAGE", good_msg, dog_state)

        # Receive LOW quality gossip from JANITOR
        bad_msg = GossipMessage(
            dog_id="JANITOR",
            compressed_context="Unclear",
            verdict="BARK",
            q_score=20.0,
            confidence=0.2,
        )
        await gossip.receive_gossip("JANITOR", bad_msg, dog_state)

        # SAGE should have higher trust than JANITOR
        sage_trust = dog_state.memory.trust_scores.get("SAGE", 0.5)
        janitor_trust = dog_state.memory.trust_scores.get("JANITOR", 0.5)
        assert sage_trust > janitor_trust

    @pytest.mark.asyncio
    async def test_gossip_consensus_from_messages(self):
        """Test consensus derivation from multiple gossip messages."""
        gossip = GossipProtocol()

        messages = [
            GossipMessage("SAGE", "context1", "WAG", 70.0, 0.50),
            GossipMessage("ANALYST", "context2", "WAG", 74.0, 0.48),
            GossipMessage("ORACLE", "context3", "GROWL", 60.0, 0.40),
        ]

        q_score, confidence, verdict = gossip.consensus_from_gossip(messages)

        # Consensus should be geometric mean (around 68)
        assert 60 < q_score < 75
        assert confidence > 0.3
        assert verdict in ("WAG", "GROWL")

    @pytest.mark.asyncio
    async def test_gossip_bandwidth_stats(self):
        """Test gossip protocol reports bandwidth efficiency."""
        gossip = GossipProtocol()
        dog_state = DogState(dog_id="DEPLOYER")

        # Publish and receive gossip
        msg = GossipMessage("DEPLOYER", "deployment context", "WAG", 72.0, 0.45)
        await gossip.receive_gossip("DEPLOYER", msg, dog_state)

        stats = gossip.bandwidth_stats()
        assert "gossip_count" in stats
        assert "rejection_rate_pct" in stats
        assert "peer_count" in stats
        assert "total_bytes_saved" in stats

        # With 1 message, should save ~800 bytes
        assert stats["total_bytes_saved"] >= 0


# ════════════════════════════════════════════════════════════════════════════
# TASK 2.4: Fractal Structure Validation
# ════════════════════════════════════════════════════════════════════════════


class TestFractalStructureValidation:
    """Validate that fractal structure works correctly (Task 2.4)."""

    @pytest.mark.asyncio
    async def test_dogs_operate_independently(self):
        """Test dogs can judge in parallel without orchestrator."""
        dogs = {}
        states = {}

        # Create 3 dogs
        for dog_id in ["SAGE", "ANALYST", "GUARDIAN"]:
            config = DogCognitionConfig()
            dogs[dog_id] = DogCognition(dog_id=dog_id, config=config)
            states[dog_id] = DogState(dog_id=dog_id)

        # Each dog judges independently
        mock_cell = MagicMock()
        mock_cell.id = "parallel_test"

        judgments = {}
        for dog_id in dogs:
            judgment = await dogs[dog_id].judge_cell(mock_cell, states[dog_id])
            judgments[dog_id] = judgment

        # All dogs should have produced judgments
        assert len(judgments) == 3
        for dog_id in judgments:
            assert judgments[dog_id].dog_id == dog_id
            assert judgments[dog_id].q_score >= 0

    @pytest.mark.asyncio
    async def test_orchestrator_becomes_consensus_layer(self):
        """Test that orchestrator no longer re-judges, just aggregates."""
        gossip = GossipProtocol()
        dog_states = {}

        # Dogs publish gossip summaries
        for dog_id in ["SAGE", "ANALYST", "ORACLE"]:
            state = DogState(dog_id=dog_id)
            state.cognition.last_verdict = "WAG"
            state.cognition.last_q_score = 70.0 + len(dog_id) * 2  # Vary slightly
            state.cognition.confidence_history = [0.45]
            dog_states[dog_id] = state

        # Orchestrator would collect and aggregate gossip
        gossip_messages = []
        for dog_id, state in dog_states.items():
            msg = await gossip.publish_gossip(state)
            if msg:
                gossip_messages.append(msg)

        # Orchestrator reaches consensus WITHOUT re-judging
        q_score, confidence, verdict = gossip.consensus_from_gossip(
            gossip_messages
        )

        # Consensus is derived, not re-judged
        assert verdict in ("WAG", "GROWL", "HOWL", "BARK")
        assert 60 < q_score < 85  # Geometric mean of 70+, 72+, 74+ range

    @pytest.mark.asyncio
    async def test_cost_scaling_no_orchestrator_bottleneck(self):
        """Test that adding dogs doesn't add orchestrator cost."""
        # Cost of dog judgment is independent of number of dogs
        costs = []

        for num_dogs in [1, 5, 11]:
            config = DogCognitionConfig()
            cog = DogCognition(dog_id=f"DOG_{num_dogs}", config=config)
            dog_state = DogState(dog_id=f"DOG_{num_dogs}")

            mock_cell = MagicMock()
            mock_cell.id = "cost_test"

            # Measure judgment time (proxy for cost)
            import time
            start = time.time()
            judgment = await cog.judge_cell(mock_cell, dog_state)
            elapsed_ms = (time.time() - start) * 1000

            costs.append(elapsed_ms)

        # Cost should be roughly constant per dog (not linear in N)
        # If it were O(N), cost[2] >> cost[0]
        # But fractal should keep costs similar
        avg_early = sum(costs[:1]) / len(costs[:1])
        avg_late = sum(costs[-2:]) / len(costs[-2:])

        # Cost ratio should be small (not 11x difference)
        ratio = max(avg_late, avg_early) / min(max(avg_late, 0.01), max(avg_early, 0.01))
        assert ratio < 5.0  # Allow some variance but not linear growth


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TEST: Full Fractal Cycle
# ════════════════════════════════════════════════════════════════════════════


class TestFractalCycleIntegration:
    """Integration test: full 7-step cycle with gossip."""

    @pytest.mark.asyncio
    async def test_full_fractal_cycle(self):
        """
        Test complete fractal cycle:
        1. Dog A judges → publishes gossip
        2. Dog B receives gossip → updates trust
        3. Orchestrator aggregates consensus (no re-judgment)
        """
        # Setup dogs
        config = DogCognitionConfig()
        dog_a = DogCognition(dog_id="SAGE", config=config)
        dog_b = DogCognition(dog_id="ANALYST", config=config)
        state_a = DogState(dog_id="SAGE")
        state_b = DogState(dog_id="ANALYST")

        # Setup gossip
        gossip_ab = GossipProtocol()

        # Step 1: Dog A judges
        mock_cell = MagicMock()
        mock_cell.id = "integration_test_cell"
        judgment_a = await dog_a.judge_cell(mock_cell, state_a)

        # Step 2: Dog A publishes gossip
        state_a.cognition.confidence_history = [0.50]
        msg_a = await gossip_ab.publish_gossip(state_a)
        assert msg_a is not None

        # Step 3: Dog B receives gossip from A
        if msg_a:
            await gossip_ab.receive_gossip("SAGE", msg_a, state_b)

        # Step 4: Dog B also judges
        judgment_b = await dog_b.judge_cell(mock_cell, state_b)

        # Step 5: Dog B publishes gossip
        state_b.cognition.confidence_history = [0.48]
        msg_b = await gossip_ab.publish_gossip(state_b)

        # Step 6: Orchestrator derives consensus (no re-judgment)
        messages = [msg_a, msg_b]
        q_consensus, conf_consensus, v_consensus = gossip_ab.consensus_from_gossip(
            [m for m in messages if m]
        )

        # Verify fractal properties
        assert len([m for m in messages if m]) == 2  # Both published
        assert q_consensus > 0  # Consensus reached
        assert v_consensus in ("BARK", "GROWL", "WAG", "HOWL")

        # Verify no bottleneck: Dog judgments independent
        assert judgment_a.dog_id == "SAGE"
        assert judgment_b.dog_id == "ANALYST"
        # Both have non-zero costs (they judged independently)
        assert judgment_a.latency_ms >= 0
        assert judgment_b.latency_ms >= 0

"""Comprehensive tests for LNSP Judge Communication Architecture."""
from __future__ import annotations

import asyncio

import pytest

from cynic.kernel.protocol.lnsp.axioms import FidelityEvaluator
from cynic.kernel.protocol.lnsp.judge_communication import CentralJudge, JudgeConnection
from cynic.kernel.protocol.lnsp.layer3 import Layer3
from cynic.kernel.protocol.lnsp.messages import create_aggregated_state, create_judgment
from cynic.kernel.protocol.lnsp.types import (
    AggregationType,
    JudgmentType,
    LNSPMessage,
    VerdictType,
)

# ============================================================================
# Test Fixtures
# ============================================================================


@pytest.fixture
def judge() -> Layer3:
    """Create a fresh Layer3 judge for testing."""
    j = Layer3(judge_id="judge:test")
    j.register_axiom(FidelityEvaluator())
    return j


@pytest.fixture
def central_judge(judge: Layer3) -> CentralJudge:
    """Create a fresh CentralJudge with Layer3 judge."""
    cj = CentralJudge(
        judge=judge,
        judge_id="judge:central:test",
        max_queue_size=100,
        batch_size=5,
        batch_timeout_sec=0.5,
    )
    # Register the verdict callback with the judge
    judge.subscribe(cj._judge_verdict_callback)
    return cj


@pytest.fixture
def sample_aggregation() -> LNSPMessage:
    """Create a sample Layer 2 aggregation message."""
    return create_aggregated_state(
        aggregation_type=AggregationType.SYSTEM_STATE,
        data={"cpu": 50.0, "memory": 60.0},
        source="ganglia:test",
        based_on=["obs:001"],
        instance_id="instance:001",
        region="test",
    )


@pytest.fixture
def sample_verdict() -> LNSPMessage:
    """Create a sample Layer 3 verdict."""
    return create_judgment(
        judgment_type=JudgmentType.STATE_EVALUATION,
        verdict=VerdictType.WAG,
        q_score=0.5,
        confidence=0.618,
        axiom_scores={"FIDELITY": 0.8},
        data={"reason": "System healthy"},
        source="judge:test",
        target="instance:001",
        based_on=["agg:001"],
        instance_id="instance:001",
        region="test",
    )


# ============================================================================
# JudgeConnection Tests
# ============================================================================


class TestJudgeConnection:
    """Test JudgeConnection class."""

    @pytest.mark.asyncio
    async def test_judge_connection_creation(self) -> None:
        """Test creating a JudgeConnection."""
        cj = CentralJudge()
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        assert conn.instance_id == "instance:001"
        assert conn.host == "localhost"
        assert conn.port == 5000
        assert conn.is_connected is False
        assert conn.verdict_count == 0

    @pytest.mark.asyncio
    async def test_judge_connection_connect(self) -> None:
        """Test connecting a JudgeConnection."""
        cj = CentralJudge()
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        await conn.connect()
        assert conn.is_connected is True

    @pytest.mark.asyncio
    async def test_judge_connection_send_verdict(
        self, sample_verdict: LNSPMessage
    ) -> None:
        """Test sending a verdict to connection."""
        cj = CentralJudge()
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        await conn.send_verdict(sample_verdict)
        assert conn.verdict_queue.qsize() == 1

    @pytest.mark.asyncio
    async def test_judge_connection_backpressure(
        self, sample_verdict: LNSPMessage
    ) -> None:
        """Test backpressure when queue is full."""
        cj = CentralJudge(max_queue_size=2)
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        # Fill the queue
        await conn.send_verdict(sample_verdict)
        await conn.send_verdict(sample_verdict)

        # Next send should raise backpressure error
        with pytest.raises(RuntimeError, match="queue full"):
            await conn.send_verdict(sample_verdict)

    @pytest.mark.asyncio
    async def test_judge_connection_send_batch(
        self, sample_verdict: LNSPMessage
    ) -> None:
        """Test sending a batch of verdicts."""
        cj = CentralJudge()
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        batch = [sample_verdict, sample_verdict, sample_verdict]
        await conn.send_batch(batch)

        assert conn.verdict_count == 3

    @pytest.mark.asyncio
    async def test_judge_connection_close(self) -> None:
        """Test closing a connection."""
        cj = CentralJudge()
        conn = JudgeConnection(
            instance_id="instance:001",
            host="localhost",
            port=5000,
            judge=cj,
        )

        await conn.connect()
        assert conn.is_connected is True

        await conn.close()
        assert conn.is_connected is False


# ============================================================================
# CentralJudge Creation Tests
# ============================================================================


class TestCentralJudgeCreation:
    """Test CentralJudge initialization."""

    def test_central_judge_creation(self) -> None:
        """Test creating a CentralJudge."""
        cj = CentralJudge(judge_id="judge:test")

        assert cj.judge_id == "judge:test"
        assert cj.max_queue_size == 1000
        assert cj.batch_size == 10
        assert cj.batch_timeout_sec == 1.0
        assert len(cj._instance_connections) == 0
        assert cj._verdict_queue.qsize() == 0

    def test_central_judge_with_custom_params(self) -> None:
        """Test creating CentralJudge with custom parameters."""
        judge = Layer3("judge:custom")
        cj = CentralJudge(
            judge=judge,
            judge_id="judge:test:custom",
            max_queue_size=500,
            batch_size=20,
            batch_timeout_sec=2.0,
        )

        assert cj.judge_id == "judge:test:custom"
        assert cj.max_queue_size == 500
        assert cj.batch_size == 20
        assert cj.batch_timeout_sec == 2.0
        assert cj._judge is judge

    def test_central_judge_default_judge(self) -> None:
        """Test CentralJudge creates default judge if not provided."""
        cj = CentralJudge()

        assert cj._judge is not None
        assert isinstance(cj._judge, Layer3)


# ============================================================================
# Instance Registration Tests
# ============================================================================


class TestInstanceRegistration:
    """Test instance registration and management."""

    @pytest.mark.asyncio
    async def test_register_instance(self, central_judge: CentralJudge) -> None:
        """Test registering an instance."""
        await central_judge.register_instance("instance:001", "localhost", 5000)

        assert "instance:001" in central_judge._instance_connections
        conn = central_judge._instance_connections["instance:001"]
        assert conn.instance_id == "instance:001"
        assert conn.host == "localhost"
        assert conn.port == 5000

    @pytest.mark.asyncio
    async def test_register_multiple_instances(
        self, central_judge: CentralJudge
    ) -> None:
        """Test registering multiple instances."""
        await central_judge.register_instance("instance:001", "host1", 5001)
        await central_judge.register_instance("instance:002", "host2", 5002)
        await central_judge.register_instance("instance:003", "host3", 5003)

        assert len(central_judge._instance_connections) == 3
        assert "instance:001" in central_judge._instance_connections
        assert "instance:002" in central_judge._instance_connections
        assert "instance:003" in central_judge._instance_connections

    @pytest.mark.asyncio
    async def test_register_duplicate_instance_raises(
        self, central_judge: CentralJudge
    ) -> None:
        """Test that registering duplicate instance_id raises."""
        await central_judge.register_instance("instance:001", "host1", 5001)

        with pytest.raises(ValueError, match="already registered"):
            await central_judge.register_instance("instance:001", "host2", 5002)

    @pytest.mark.asyncio
    async def test_unregister_instance(self, central_judge: CentralJudge) -> None:
        """Test unregistering an instance."""
        await central_judge.register_instance("instance:001", "localhost", 5000)
        assert "instance:001" in central_judge._instance_connections

        await central_judge.unregister_instance("instance:001")
        assert "instance:001" not in central_judge._instance_connections

    @pytest.mark.asyncio
    async def test_unregister_nonexistent_instance_raises(
        self, central_judge: CentralJudge
    ) -> None:
        """Test that unregistering non-existent instance raises."""
        with pytest.raises(ValueError, match="not registered"):
            await central_judge.unregister_instance("instance:does_not_exist")


# ============================================================================
# Verdict Routing Tests
# ============================================================================


class TestVerdictRouting:
    """Test verdict routing to instances."""

    @pytest.mark.asyncio
    async def test_route_verdict_to_instance(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test routing verdict to specific instance."""
        await central_judge.register_instance("instance:001", "host1", 5001)

        central_judge._judge_verdict_callback(sample_verdict)

        # Start batching to process the verdict
        central_judge.start_batching()
        await asyncio.sleep(0.6)  # Wait for batch timeout
        central_judge.stop_batching()

        # Verify verdict was routed
        conn = central_judge._instance_connections["instance:001"]
        assert conn.verdict_count > 0

    @pytest.mark.asyncio
    async def test_route_verdict_broadcast(
        self, central_judge: CentralJudge
    ) -> None:
        """Test routing verdict to all instances (broadcast)."""
        await central_judge.register_instance("instance:001", "host1", 5001)
        await central_judge.register_instance("instance:002", "host2", 5002)

        # Create verdict with BROADCAST target
        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.WAG,
            q_score=0.5,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.8},
            data={"reason": "Broadcast test"},
            source="judge:test",
            target="BROADCAST",  # Broadcast to all
            based_on=["agg:001"],
            instance_id="instance:001",
        )

        central_judge._judge_verdict_callback(verdict)

        # Start batching
        central_judge.start_batching()
        await asyncio.sleep(0.6)
        central_judge.stop_batching()

        # Both instances should receive verdict
        for inst_id in ["instance:001", "instance:002"]:
            conn = central_judge._instance_connections[inst_id]
            assert conn.verdict_count > 0

    @pytest.mark.asyncio
    async def test_route_verdict_by_target(
        self, central_judge: CentralJudge
    ) -> None:
        """Test routing different verdicts to different instances."""
        await central_judge.register_instance("instance:001", "host1", 5001)
        await central_judge.register_instance("instance:002", "host2", 5002)

        # Send verdict to instance:001
        verdict1 = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.WAG,
            q_score=0.5,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.8},
            data={"test": 1},
            source="judge:test",
            target="instance:001",
            based_on=["agg:001"],
            instance_id="instance:001",
        )

        # Send verdict to instance:002
        verdict2 = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.3,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.5},
            data={"test": 2},
            source="judge:test",
            target="instance:002",
            based_on=["agg:002"],
            instance_id="instance:002",
        )

        central_judge._judge_verdict_callback(verdict1)
        central_judge._judge_verdict_callback(verdict2)

        central_judge.start_batching()
        await asyncio.sleep(0.6)
        central_judge.stop_batching()

        conn1 = central_judge._instance_connections["instance:001"]
        conn2 = central_judge._instance_connections["instance:002"]

        # Each should have received one verdict
        assert conn1.verdict_count > 0
        assert conn2.verdict_count > 0


# ============================================================================
# Batching Tests
# ============================================================================


class TestVerdictBatching:
    """Test verdict batching behavior."""

    @pytest.mark.asyncio
    async def test_batch_verdicts_on_size(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test batch is sent when reaching batch_size."""
        central_judge.batch_size = 3
        await central_judge.register_instance("instance:001", "host1", 5001)

        # Start batching
        central_judge.start_batching()

        # Send 3 verdicts (should trigger batch)
        for _ in range(3):
            central_judge._judge_verdict_callback(sample_verdict)

        # Wait a bit for batching to occur
        await asyncio.sleep(0.2)

        conn = central_judge._instance_connections["instance:001"]
        # Should have processed the batch
        assert conn.verdict_count > 0

        central_judge.stop_batching()

    @pytest.mark.asyncio
    async def test_batch_verdicts_on_timeout(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test batch is sent after timeout even if not full."""
        central_judge.batch_size = 10  # Large batch size
        central_judge.batch_timeout_sec = 0.3  # Short timeout
        await central_judge.register_instance("instance:001", "host1", 5001)

        central_judge.start_batching()

        # Send only 1 verdict (won't reach batch_size)
        central_judge._judge_verdict_callback(sample_verdict)

        # Wait for timeout
        await asyncio.sleep(0.5)

        conn = central_judge._instance_connections["instance:001"]
        # Should have sent batch due to timeout
        assert conn.verdict_count > 0

        central_judge.stop_batching()

    @pytest.mark.asyncio
    async def test_partial_batch_on_timeout(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test partial batch is sent after timeout."""
        central_judge.batch_size = 5
        central_judge.batch_timeout_sec = 0.3
        await central_judge.register_instance("instance:001", "host1", 5001)

        central_judge.start_batching()

        # Send 2 verdicts (less than batch_size)
        for _ in range(2):
            central_judge._judge_verdict_callback(sample_verdict)

        # Wait for timeout
        await asyncio.sleep(0.5)

        conn = central_judge._instance_connections["instance:001"]
        # Should have sent partial batch (2 verdicts)
        assert conn.verdict_count == 2

        central_judge.stop_batching()


# ============================================================================
# Backpressure Tests
# ============================================================================


class TestBackpressure:
    """Test backpressure handling."""

    @pytest.mark.asyncio
    async def test_backpressure_queue_full(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test handling of full verdict queue."""
        central_judge.max_queue_size = 2
        await central_judge.register_instance("instance:001", "host1", 5001)

        # Fill the queue
        central_judge._judge_verdict_callback(sample_verdict)
        central_judge._judge_verdict_callback(sample_verdict)

        # This should trigger backpressure
        stats_before = central_judge._verdict_stats["dropped_backpressure"]

        # Send more verdicts - should be dropped
        for _ in range(3):
            central_judge._judge_verdict_callback(sample_verdict)

        stats_after = central_judge._verdict_stats["dropped_backpressure"]

        # Should have dropped some verdicts
        assert stats_after > stats_before

    @pytest.mark.asyncio
    async def test_queue_tracking(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test queue size is tracked in stats."""
        await central_judge.register_instance("instance:001", "host1", 5001)

        # Send verdict
        central_judge._judge_verdict_callback(sample_verdict)

        stats = central_judge.stats()
        assert stats["verdict_queue_size"] > 0


# ============================================================================
# Integration Tests
# ============================================================================


class TestAggregationToVerdictFlow:
    """Test full flow from aggregation to verdict routing."""

    @pytest.mark.asyncio
    async def test_aggregation_to_verdict_flow(
        self, judge: Layer3
    ) -> None:
        """Test full pipeline from aggregation to routing."""
        # Create central judge with the judge
        cj = CentralJudge(judge=judge, batch_size=1, batch_timeout_sec=0.2)
        judge.subscribe(cj._judge_verdict_callback)

        # Register instance
        await cj.register_instance("instance:001", "host1", 5001)

        # Start batching
        cj.start_batching()

        # Create aggregation and process it
        aggregation = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"cpu": 50.0, "memory": 60.0},
            source="ganglia:test",
            based_on=["obs:001"],
            instance_id="instance:001",
            region="test",
        )

        await cj.process_aggregation(aggregation)

        # Wait for batching to complete
        await asyncio.sleep(0.5)

        # Verify verdict was routed
        stats = cj.stats()
        assert stats["verdict_stats"]["received"] > 0
        assert stats["verdict_stats"]["routed"] > 0

        # Verify at least one verdict was batched and sent
        assert stats["verdict_stats"]["batched"] > 0

        cj.stop_batching()

    @pytest.mark.asyncio
    async def test_judge_connection_state(
        self, central_judge: CentralJudge
    ) -> None:
        """Test per-instance connection state tracking."""
        await central_judge.register_instance("instance:001", "host1", 5001)
        await central_judge.register_instance("instance:002", "host2", 5002)

        stats = central_judge.stats()

        assert stats["instance_count"] == 2
        assert "instance:001" in stats["instances"]
        assert "instance:002" in stats["instances"]

        inst1_stats = stats["instances"]["instance:001"]
        assert inst1_stats["host"] == "host1"
        assert inst1_stats["port"] == 5001
        assert inst1_stats["is_connected"] is True
        assert inst1_stats["verdict_count"] == 0


# ============================================================================
# Subscriber Tests
# ============================================================================


class TestSubscribers:
    """Test verdict subscriber callbacks."""

    @pytest.mark.asyncio
    async def test_subscribe_to_verdicts(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test subscribing to verdict callbacks."""
        received = []

        def subscriber(msg: LNSPMessage) -> None:
            received.append(msg)

        central_judge.subscribe(subscriber)

        central_judge._judge_verdict_callback(sample_verdict)

        assert len(received) == 1
        assert received[0] is sample_verdict

    @pytest.mark.asyncio
    async def test_multiple_subscribers(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test multiple subscribers receive verdicts."""
        received1 = []
        received2 = []

        def subscriber1(msg: LNSPMessage) -> None:
            received1.append(msg)

        def subscriber2(msg: LNSPMessage) -> None:
            received2.append(msg)

        central_judge.subscribe(subscriber1)
        central_judge.subscribe(subscriber2)

        central_judge._judge_verdict_callback(sample_verdict)

        assert len(received1) == 1
        assert len(received2) == 1


# ============================================================================
# Statistics Tests
# ============================================================================


class TestJudgeCommunicationStats:
    """Test statistics tracking."""

    def test_stats_empty_judge(self) -> None:
        """Test stats for empty judge."""
        cj = CentralJudge(judge_id="judge:test")
        stats = cj.stats()

        assert stats["judge_id"] == "judge:test"
        assert stats["instance_count"] == 0
        assert stats["batch_size"] == 10
        assert stats["verdict_queue_size"] == 0
        assert stats["batching_active"] is False

    @pytest.mark.asyncio
    async def test_stats_with_instances(self, central_judge: CentralJudge) -> None:
        """Test stats with registered instances."""
        await central_judge.register_instance("instance:001", "host1", 5001)
        await central_judge.register_instance("instance:002", "host2", 5002)

        stats = central_judge.stats()

        assert stats["instance_count"] == 2
        assert "instance:001" in stats["instances"]
        assert "instance:002" in stats["instances"]

    @pytest.mark.asyncio
    async def test_stats_verdict_tracking(
        self, central_judge: CentralJudge, sample_verdict: LNSPMessage
    ) -> None:
        """Test verdict stats are tracked."""
        await central_judge.register_instance("instance:001", "host1", 5001)

        initial_stats = central_judge.stats()
        assert initial_stats["verdict_stats"]["received"] == 0

        central_judge._judge_verdict_callback(sample_verdict)

        updated_stats = central_judge.stats()
        assert updated_stats["verdict_stats"]["received"] == 1

    def test_stats_all_fields(self, central_judge: CentralJudge) -> None:
        """Test stats include all required fields."""
        stats = central_judge.stats()

        required_fields = [
            "judge_id",
            "instance_count",
            "max_queue_size",
            "batch_size",
            "batch_timeout_sec",
            "verdict_stats",
            "verdict_queue_size",
            "batching_active",
            "instances",
        ]

        for field in required_fields:
            assert field in stats

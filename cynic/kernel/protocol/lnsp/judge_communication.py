"""Judge Communication Architecture for LNSP " Verdict Routing

The Judge Communication component enables the central Judge to:
1. Receive de-duplicated/correlated Layer 2 state from Regional Coordinator
2. Emit verdicts through Layer 3 judge evaluation
3. Route verdicts back to correct instance handlers (Layer 4)
4. Handle batching and backpressure

Components:
- JudgeConnection: Manages outbound verdict routing to a single instance
- CentralJudge: Coordinates judge evaluation and verdict distribution
"""

from __future__ import annotations

import asyncio
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from .layer3 import Layer3
from .types import Layer, LNSPMessage


@dataclass
class JudgeConnection:
    """Manages outbound verdict routing to a single instance.

    Each instance connection tracks a destination where verdicts should be routed,
    including connection state, verdict queue, and delivery tracking.

    Attributes:
        instance_id: Unique identifier for the destination instance
        host: Hostname or IP address of the instance
        port: Port number for connection
        judge: Reference to parent CentralJudge
        verdict_queue: Queue of verdicts waiting to be sent
        is_connected: Whether connection is currently active
        verdict_count: Total verdicts sent to this instance
    """

    instance_id: str
    host: str
    port: int
    judge: CentralJudge
    verdict_queue: asyncio.Queue[LNSPMessage] = field(default_factory=asyncio.Queue)
    is_connected: bool = False
    verdict_count: int = 0

    async def connect(self) -> None:
        """Establish connection to instance handler.

        MVP: This is a placeholder. In a full implementation, this would
        establish an actual TCP socket connection to the instance.

        Future implementations would:
        - Create TCP socket to host:port
        - Perform handshake/auth
        - Set is_connected = True on success
        """
        # MVP: Treat as always connected for testing
        self.is_connected = True

    async def send_verdict(self, verdict: LNSPMessage) -> None:
        """Send a single verdict to this instance.

        Queues the verdict for batching and delivery. Handles backpressure
        by checking queue size before adding.

        Args:
            verdict: Layer 3 judgment message to send

        Raises:
            RuntimeError: If verdict queue is full (backpressure)
        """
        # Check queue size for backpressure
        if self.verdict_queue.qsize() >= self.judge.max_queue_size:
            raise RuntimeError(
                f"Verdict queue full for instance {self.instance_id} "
                f"(max {self.judge.max_queue_size})"
            )

        # Queue verdict for batching
        await self.verdict_queue.put(verdict)

    async def send_batch(self, verdicts: list[LNSPMessage]) -> None:
        """Send a batch of verdicts to this instance.

        In MVP, this just tracks the count. A full implementation would
        serialize the batch and send via socket.

        Args:
            verdicts: List of verdicts to send
        """
        # MVP: Track verdict delivery
        self.verdict_count += len(verdicts)

        # Future: Actually send via socket
        # await self._send_via_socket(verdicts)

    async def close(self) -> None:
        """Close connection to instance.

        MVP: Just marks as disconnected. Full implementation would
        close the TCP socket.
        """
        self.is_connected = False


@dataclass
class CentralJudge:
    """Central judge receiving aggregated state and routing verdicts.

    The CentralJudge sits between the Regional Coordinator (receives Layer 2
    aggregated state) and instance handlers (receives verdicts). It:
    1. Receives de-duplicated state from Regional Coordinator
    2. Passes to Layer3 judge for evaluation
    3. Judge emits verdicts via callback
    4. Batches verdicts based on size or timeout
    5. Routes to appropriate instance handlers

    Attributes:
        judge_id: Unique identifier for this judge
        max_queue_size: Maximum verdicts queued before backpressure
        batch_size: Number of verdicts to batch before sending
        batch_timeout_sec: Max time to wait before sending partial batch
        _judge: The actual Layer3 judge instance
        _instance_connections: Mapping of instance_id to JudgeConnection
        _verdict_queue: Queue of verdicts waiting to be batched
        _subscribers: List of callbacks for testing/monitoring
        _batching_task: Asyncio task for batching and sending
        _verdict_stats: Statistics about verdicts processed
    """

    judge_id: str = "judge:central"
    max_queue_size: int = 1000
    batch_size: int = 10
    batch_timeout_sec: float = 1.0

    # Internal state
    _judge: Layer3 = field(default_factory=lambda: Layer3("judge:default"))
    _instance_connections: dict[str, JudgeConnection] = field(default_factory=dict)
    _verdict_queue: asyncio.Queue[LNSPMessage] = field(default_factory=asyncio.Queue)
    _subscribers: list[Callable[[LNSPMessage], None]] = field(default_factory=list)
    _batching_task: asyncio.Task[None] | None = field(default=None)
    _verdict_stats: dict[str, int] = field(default_factory=dict)

    def __init__(
        self,
        judge: Layer3 | None = None,
        judge_id: str = "judge:central",
        max_queue_size: int = 1000,
        batch_size: int = 10,
        batch_timeout_sec: float = 1.0,
    ) -> None:
        """Initialize central judge with a Layer3 judge.

        Args:
            judge: Layer3 judge instance to use for evaluation
            judge_id: Unique identifier for this judge
            max_queue_size: Maximum verdicts in queue before backpressure
            batch_size: Number of verdicts to batch
            batch_timeout_sec: Timeout for partial batches
        """
        self.judge_id = judge_id
        self.max_queue_size = max_queue_size
        self.batch_size = batch_size
        self.batch_timeout_sec = batch_timeout_sec

        self._judge = judge if judge is not None else Layer3("judge:default")
        self._instance_connections = {}
        self._verdict_queue = asyncio.Queue()
        self._subscribers = []
        self._batching_task = None
        self._verdict_stats = {
            "received": 0,
            "routed": 0,
            "batched": 0,
            "dropped_backpressure": 0,
        }

    async def register_instance(self, instance_id: str, host: str, port: int) -> None:
        """Register a verdict destination instance.

        Creates a JudgeConnection for the instance and prepares it for
        verdict delivery.

        Args:
            instance_id: Unique identifier for the instance
            host: Hostname or IP address
            port: Port number for connection

        Raises:
            ValueError: If instance is already registered
        """
        if instance_id in self._instance_connections:
            raise ValueError(f"Instance {instance_id} already registered")

        connection = JudgeConnection(
            instance_id=instance_id,
            host=host,
            port=port,
            judge=self,
        )
        await connection.connect()
        self._instance_connections[instance_id] = connection

    async def unregister_instance(self, instance_id: str) -> None:
        """Unregister a verdict destination instance.

        Args:
            instance_id: Instance to unregister

        Raises:
            ValueError: If instance is not registered
        """
        if instance_id not in self._instance_connections:
            raise ValueError(f"Instance {instance_id} not registered")

        connection = self._instance_connections[instance_id]
        await connection.close()
        del self._instance_connections[instance_id]

    async def process_aggregation(self, msg: LNSPMessage) -> None:
        """Receive aggregated state from Regional Coordinator.

        Process:
        1. Verify message is Layer 2 (AGGREGATED)
        2. Pass to Layer3 judge for evaluation
        3. Judge emits verdicts via callback (registered in __post_init__)
        4. Verdicts are queued and batched for routing

        Args:
            msg: Layer 2 aggregated state message
        """
        # Verify this is a Layer 2 message
        if msg.header.layer != Layer.AGGREGATED:
            return

        # Pass to judge for evaluation
        # Judge will call _judge_verdict_callback for each verdict
        await self._judge.judge(msg)

    def _judge_verdict_callback(self, verdict: LNSPMessage) -> None:
        """Called when Judge emits a verdict (synchronous callback).

        This is registered as a subscriber to the Layer3 judge.
        When a judgment is emitted, this callback:
        1. Verifies it's a Layer 3 verdict
        2. Checks backpressure
        3. Queues verdict for routing (via asyncio)

        Args:
            verdict: Layer 3 judgment message from judge
        """
        # Verify this is a judgment/verdict
        if verdict.header.layer != Layer.JUDGMENT:
            return

        # Check backpressure
        if self._verdict_queue.qsize() >= self.max_queue_size:
            self._verdict_stats["dropped_backpressure"] += 1
            # In real system, might emit alert here
            return

        # Queue for batching and routing (synchronously queue without await)
        self._verdict_stats["received"] += 1
        try:
            self._verdict_queue.put_nowait(verdict)
        except asyncio.QueueFull:
            self._verdict_stats["dropped_backpressure"] += 1
            return

        # Notify subscribers (for monitoring/testing)
        for callback in self._subscribers:
            callback(verdict)

    def start_batching(self) -> None:
        """Start the verdict batching task.

        This should be called after the judge is set up but before
        processing messages. Creates and starts the batching coroutine.
        """
        if self._batching_task is None:
            self._batching_task = asyncio.create_task(self._batch_verdicts())

    def stop_batching(self) -> None:
        """Stop the verdict batching task.

        Cancels the batching task if it's running.
        """
        if self._batching_task is not None:
            self._batching_task.cancel()
            self._batching_task = None

    async def _batch_verdicts(self) -> None:
        """Batch verdicts and send to instances.

        Runs continuously, collecting verdicts into batches and sending to
        instances. Batching is triggered by either:
        - Reaching batch_size verdicts
        - Timeout of batch_timeout_sec elapsed

        This coroutine should be run as a background task via start_batching().
        """
        while True:
            try:
                batch: list[LNSPMessage] = []
                batch_deadline = time.time() + self.batch_timeout_sec

                # Collect verdicts until batch_size or timeout
                while len(batch) < self.batch_size:
                    time_remaining = batch_deadline - time.time()
                    if time_remaining <= 0:
                        # Timeout reached
                        break

                    try:
                        # Wait for next verdict with timeout
                        verdict = await asyncio.wait_for(
                            self._verdict_queue.get(),
                            timeout=time_remaining,
                        )
                        batch.append(verdict)
                    except TimeoutError:
                        # Batch timeout reached
                        break

                # Send batch if non-empty
                if batch:
                    await self._route_batch(batch)

            except asyncio.CancelledError:
                # Task cancelled, exit gracefully
                break
            except Exception:
                # Log error but continue batching
                # In production, might have proper logging here
                await asyncio.sleep(0.1)

    async def _route_batch(self, verdicts: list[LNSPMessage]) -> None:
        """Route a batch of verdicts to instances.

        Each verdict is routed to:
        - Instance specified in verdict.metadata.instance_id (primary method)
        - Specific instance if verdict.header.target is instance_id
        - All instances if verdict.header.target is "BROADCAST"
        - No routing if target doesn't match any instance

        Args:
            verdicts: List of verdicts to route
        """
        for verdict in verdicts:
            # Prefer routing by instance_id in metadata
            instance_id = verdict.metadata.instance_id
            target = verdict.header.target

            # Determine where to route this verdict
            target_instances: list[str] = []

            if target == "BROADCAST":
                # Route to all instances
                target_instances = list(self._instance_connections.keys())
            elif instance_id in self._instance_connections:
                # Route to the verdict's source instance
                target_instances = [instance_id]
            elif target and target in self._instance_connections:
                # Route to specific target if it's an instance_id
                target_instances = [target]

            # Send verdict to target instances
            for target_inst_id in target_instances:
                conn = self._instance_connections[target_inst_id]
                try:
                    await conn.send_verdict(verdict)
                except RuntimeError:
                    # Backpressure on this instance
                    self._verdict_stats["dropped_backpressure"] += 1

            self._verdict_stats["routed"] += 1

        # Send batches from each instance's queue
        for conn in self._instance_connections.values():
            batch = []
            while not conn.verdict_queue.empty():
                try:
                    batch.append(conn.verdict_queue.get_nowait())
                except asyncio.QueueEmpty:
                    break

            if batch:
                await conn.send_batch(batch)
                self._verdict_stats["batched"] += len(batch)

    def subscribe(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Subscribe to verdicts (for testing/monitoring).

        Adds a callback that will be invoked for each verdict received.

        Args:
            callback: Callable that takes an LNSPMessage verdict
        """
        self._subscribers.append(callback)

    def stats(self) -> dict[str, Any]:
        """Return judge communication statistics.

        Returns:
            Dict with keys:
                - judge_id: Judge identifier
                - instance_count: Number of registered instances
                - max_queue_size: Maximum queue size
                - batch_size: Verdicts per batch
                - batch_timeout_sec: Batch timeout
                - verdict_stats: Dict of verdict processing stats
                - instances: Dict mapping instance_id to connection stats
        """
        instance_stats = {}
        for inst_id, conn in self._instance_connections.items():
            instance_stats[inst_id] = {
                "host": conn.host,
                "port": conn.port,
                "is_connected": conn.is_connected,
                "verdict_count": conn.verdict_count,
                "queue_size": conn.verdict_queue.qsize(),
            }

        return {
            "judge_id": self.judge_id,
            "instance_count": len(self._instance_connections),
            "max_queue_size": self.max_queue_size,
            "batch_size": self.batch_size,
            "batch_timeout_sec": self.batch_timeout_sec,
            "verdict_stats": self._verdict_stats,
            "verdict_queue_size": self._verdict_queue.qsize(),
            "batching_active": self._batching_task is not None,
            "instances": instance_stats,
        }

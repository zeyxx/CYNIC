"""Layer 2: Aggregated State with Temporal Windows

Layer 2 aggregates raw Layer 1 observations into meaningful state abstractions
using temporal windows. It provides:

1. TemporalWindow for sliding window aggregation (5s, 60s, 5m, 1h)
2. Aggregator abstract base class for state synthesis
3. Layer2 manager coordinating aggregators and subscribers
4. Subscription pattern for Layer 3 Judge callbacks
5. Auto-expiration of old observations within windows
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from .types import LNSPMessage


@dataclass
class TemporalWindow:
    """A sliding temporal window for observations.

    Observations are kept within the window size and automatically expired
    when they become too old. Windows can be checked for aggregation readiness.

    Attributes:
        window_size_sec: Size of the temporal window in seconds
        observations: List of observations within the window
        last_aggregation_time: Unix timestamp of last aggregation completion
    """

    window_size_sec: float
    observations: list[LNSPMessage] = field(default_factory=list)
    last_aggregation_time: float = field(default_factory=time.time)

    def add(self, msg: LNSPMessage) -> None:
        """Add observation to window, auto-expiring old ones.

        Observations are kept only if they fall within the window size from now.
        Old observations are filtered out before adding the new one.

        Args:
            msg: Observation message to add
        """
        now = time.time()
        # Filter observations to keep only those within window
        self.observations = [
            obs for obs in self.observations if now - obs.header.timestamp <= self.window_size_sec
        ]
        # Add new observation
        self.observations.append(msg)

    def should_aggregate(self, interval_sec: float = 5.0) -> bool:
        """Check if enough time has passed since last aggregation.

        Returns True if at least interval_sec seconds have passed since the
        last successful aggregation.

        Args:
            interval_sec: Minimum seconds since last aggregation (default: 5.0)

        Returns:
            True if aggregation interval has elapsed, False otherwise
        """
        now = time.time()
        return now - self.last_aggregation_time >= interval_sec

    def reset(self) -> None:
        """Mark aggregation complete by updating last_aggregation_time.

        Called after successful aggregation to reset the timer for the next
        aggregation cycle.
        """
        self.last_aggregation_time = time.time()


class Aggregator(ABC):
    """Abstract base class for observation aggregation.

    Aggregators transform raw Layer 1 observations from a temporal window
    into synthesized Layer 2 aggregated state messages. Each aggregator
    has a unique ID and implements the aggregation logic.

    Attributes:
        aggregator_id: Unique identifier for this aggregator
    """

    def __init__(self, aggregator_id: str) -> None:
        """Initialize aggregator with ID.

        Args:
            aggregator_id: Unique identifier for this aggregator
        """
        self.aggregator_id = aggregator_id

    @abstractmethod
    async def aggregate(self, observations: list[LNSPMessage]) -> LNSPMessage | None:
        """Transform observations into aggregated state.

        Takes a list of raw observations from a temporal window and produces
        a single Layer 2 aggregated state message, or None if insufficient data.

        Args:
            observations: List of Layer 1 observations from the window

        Returns:
            Layer 2 aggregated state message, or None if insufficient data
        """
        pass


class Layer2:
    """Layer 2 manager for state aggregation.

    Layer2 manages temporal windows for each registered aggregator, processes
    incoming observations into windows, and calls aggregators to synthesize
    state. Aggregated messages are sent to Layer 3 Judge subscribers.

    Attributes:
        aggregators: Dict mapping aggregator_id to Aggregator instances
        windows: Dict mapping aggregator_id -> {window_size -> TemporalWindow}
        subscribers: List of Layer 3 callbacks to receive aggregated messages
    """

    def __init__(self) -> None:
        """Initialize Layer2 manager."""
        self.aggregators: dict[str, Aggregator] = {}
        self.windows: dict[str, dict[float, TemporalWindow]] = {}
        self.subscribers: list[Callable[[LNSPMessage], None]] = []

    def register_aggregator(self, aggregator: Aggregator, window_sizes: list[float]) -> None:
        """Register an aggregator with multiple temporal window sizes.

        Creates a TemporalWindow for each window size and registers them
        with the aggregator. Multiple window sizes allow multi-scale analysis.

        Args:
            aggregator: Aggregator instance to register
            window_sizes: List of window sizes in seconds (e.g., [5.0, 60.0])
        """
        self.aggregators[aggregator.aggregator_id] = aggregator
        self.windows[aggregator.aggregator_id] = {}

        for window_size in window_sizes:
            self.windows[aggregator.aggregator_id][window_size] = TemporalWindow(
                window_size_sec=window_size
            )

    def subscribe(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Subscribe a callback to receive aggregated state messages.

        Callbacks are called with each aggregated state message as it's
        produced. They are called synchronously during aggregate().

        Args:
            callback: Callable that takes an LNSPMessage aggregated state
        """
        self.subscribers.append(callback)

    async def process_observation(self, msg: LNSPMessage) -> None:
        """Route observation to all temporal windows.

        Takes a Layer 1 observation and adds it to all registered windows
        for all aggregators. This happens before aggregation.

        Args:
            msg: Layer 1 observation message to process
        """
        for aggregator_id in self.windows:
            for window in self.windows[aggregator_id].values():
                window.add(msg)

    async def aggregate(self) -> None:
        """Run aggregation cycle for all windows that are ready.

        For each aggregator and each window:
        1. Check if aggregation interval has elapsed
        2. Call aggregator.aggregate() with window observations
        3. If result is not None, call all subscribers
        4. Reset the window's aggregation timer

        This method should be called periodically in the event loop.
        """
        for aggregator_id, aggregator in self.aggregators.items():
            for _window_size, window in self.windows[aggregator_id].items():
                if window.should_aggregate():
                    # Call aggregator to synthesize state
                    aggregated_msg = await aggregator.aggregate(window.observations)

                    # Send to subscribers if we got a result
                    if aggregated_msg is not None:
                        for callback in self.subscribers:
                            callback(aggregated_msg)

                    # Mark aggregation complete
                    window.reset()

    def stats(self) -> dict[str, Any]:
        """Return current statistics about Layer2 state.

        Returns:
            Dict with keys:
                - aggregator_count: Number of registered aggregators
                - total_observations: Sum of observations across all windows
                - subscriber_count: Number of subscribed callbacks
        """
        total_obs = 0
        for aggregator_id in self.windows:
            for window in self.windows[aggregator_id].values():
                total_obs += len(window.observations)

        return {
            "aggregator_count": len(self.aggregators),
            "total_observations": total_obs,
            "subscriber_count": len(self.subscribers),
        }

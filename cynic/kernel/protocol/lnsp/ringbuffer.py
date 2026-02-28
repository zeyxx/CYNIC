"""Circular ringbuffer for overflow-safe message buffering.

The Ringbuffer provides a fixed-capacity circular buffer that automatically
drops the oldest item when new items are added to a full buffer. This provides
natural backpressure for high-frequency sensor streams.
"""
from __future__ import annotations

from typing import Generic, TypeVar

T = TypeVar("T")


class Ringbuffer(Generic[T]):
    """Circular buffer with fixed capacity and overflow-safe insertion.

    When the buffer is full and a new item is added, the oldest item is
    automatically dropped. This provides natural backpressure for streaming
    data sources.

    Type Parameters:
        T: The type of items stored in the buffer

    Attributes:
        capacity: Maximum number of items the buffer can hold
    """

    def __init__(self, capacity: int) -> None:
        """Initialize ringbuffer with fixed capacity.

        Args:
            capacity: Maximum number of items to store (must be > 0)

        Raises:
            ValueError: If capacity <= 0
        """
        if capacity <= 0:
            raise ValueError("Ringbuffer capacity must be > 0")

        self.capacity = capacity
        self._buffer: list[T | None] = [None] * capacity
        self._write_idx = 0  # Next position to write
        self._read_idx = 0  # Oldest position to read
        self._count = 0  # Number of items currently in buffer

    def put(self, item: T) -> None:
        """Add item to buffer, dropping oldest if full.

        If the buffer is at capacity, the oldest item is overwritten and
        its read position advanced, effectively dropping it.

        Args:
            item: Item to add to the buffer
        """
        self._buffer[self._write_idx] = item
        self._write_idx = (self._write_idx + 1) % self.capacity

        if self._count < self.capacity:
            self._count += 1
        else:
            # Buffer is full, advance read pointer (drop oldest)
            self._read_idx = (self._read_idx + 1) % self.capacity

    def get(self) -> T | None:
        """Remove and return oldest item from buffer.

        Returns:
            The oldest item in the buffer, or None if empty
        """
        if self._count == 0:
            return None

        item = self._buffer[self._read_idx]
        self._read_idx = (self._read_idx + 1) % self.capacity
        self._count -= 1
        return item

    def peek(self) -> T | None:
        """Return oldest item without removing it.

        Returns:
            The oldest item in the buffer, or None if empty
        """
        if self._count == 0:
            return None

        return self._buffer[self._read_idx]

    def is_empty(self) -> bool:
        """Check if buffer is empty.

        Returns:
            True if no items in buffer, False otherwise
        """
        return self._count == 0

    def size(self) -> int:
        """Return current number of items in buffer.

        Returns:
            Number of items currently stored (0 to capacity)
        """
        return self._count

    def is_full(self) -> bool:
        """Check if buffer is at capacity.

        Returns:
            True if buffer contains capacity items, False otherwise
        """
        return self._count == self.capacity

    def drain(self) -> list[T]:
        """Remove and return all items from buffer.

        Returns:
            List of all items in buffer (oldest to newest), empty list if empty
        """
        items: list[T] = []
        while not self.is_empty():
            item = self.get()
            assert item is not None  # Guaranteed by is_empty() check
            items.append(item)
        return items

"""Base types for all mirror sources."""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Iterator, Protocol


@dataclass
class Event:
    source: str
    event_type: str
    timestamp: str
    data: dict[str, Any] = field(default_factory=dict)


class Source(Protocol):
    @property
    def name(self) -> str: ...
    @property
    def current_offset(self) -> int: ...
    @property
    def error_count(self) -> int: ...
    def read_from(self, offset: int) -> Iterator[Event]: ...

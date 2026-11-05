"""Value objects for the X Social Graph bounded context.

Types derived from real X.com network capture (Playwright).
Source: x-traffic.db — /i/api/graphql/*/UsersByRestIds, HomeTimeline, etc.
"""

from dataclasses import dataclass
from datetime import datetime
from typing import NewType

Handle = NewType("Handle", str)
TweetId = NewType("TweetId", str)
UserId = NewType("UserId", str)


@dataclass(frozen=True)
class TweetText:
    """Immutable tweet text with invariant: max 280 chars."""
    value: str

    def __post_init__(self):
        if not self.value or len(self.value) > 280:
            raise ValueError(f"Tweet text must be 1-280 chars, got {len(self.value)}")

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class Timestamp:
    """Immutable UTC timestamp (from X API created_at field)."""
    value: datetime

    def __post_init__(self):
        if self.value.tzinfo is None:
            raise ValueError("Timestamp must be timezone-aware (UTC)")


@dataclass(frozen=True)
class Language:
    """Language code (e.g. 'en', 'fr', 'ja')."""
    value: str

    def __post_init__(self):
        if not self.value or len(self.value) > 5:
            raise ValueError(f"Invalid language code: {self.value}")

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class ScreenZone:
    """Screen quadrant for entropy privacy."""
    value: str  # TL, TR, BL, BR

    def __post_init__(self):
        if self.value not in ("TL", "TR", "BL", "BR"):
            raise ValueError(f"Invalid screen zone: {self.value}")
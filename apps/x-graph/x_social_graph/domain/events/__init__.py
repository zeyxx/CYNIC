"""Domain events for the X Social Graph bounded context."""

from dataclasses import dataclass
from datetime import datetime
from typing import Optional

from ..value_objects import UserId, TweetId, Handle


@dataclass(frozen=True)
class TweetCaptured:
    """Event: a tweet was captured from X traffic."""
    tweet_id: TweetId
    author_handle: Handle
    timestamp: datetime


@dataclass(frozen=True)
class DmSent:
    """Event: a DM was sent."""
    thread_id: str
    from_user: UserId
    to_user: UserId
    timestamp: datetime


@dataclass(frozen=True)
class PasscodeRequested:
    """Event: X requested DM passcode (4012)."""
    timestamp: datetime
    thread_id: Optional[str] = None


@dataclass(frozen=True)
class GraphUpdated:
    """Event: social graph was updated."""
    nodes_added: int
    edges_added: int
    timestamp: datetime

"""Entities for the X Social Graph bounded context."""

from dataclasses import dataclass
from datetime import datetime
from enum import Enum
from typing import Optional

from ..value_objects import UserId, TweetId


class InteractionType(Enum):
    REPLY = "reply"
    LIKE = "like"
    RETWEET = "retweet"
    MENTION = "mention"
    DM = "dm"
    FOLLOW = "follow"


@dataclass
class Interaction:
    """Entity: interaction between two users."""
    from_user: UserId
    to_user: UserId
    type: InteractionType
    tweet_id: Optional[TweetId] = None
    timestamp: Optional[datetime] = None

    def __post_init__(self):
        if self.timestamp is None:
            raise ValueError("Interaction requires a timestamp")


@dataclass
class DmThread:
    """Entity: DM conversation thread."""
    thread_id: str
    participants: list[UserId]
    last_message: str = ""
    last_timestamp: Optional[datetime] = None
    is_unlocked: bool = False

    def __post_init__(self):
        if len(self.participants) < 2:
            raise ValueError("DM thread requires at least 2 participants")

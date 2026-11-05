"""Aggregates for the X Social Graph bounded context.

Types derived from real X.com network capture (Playwright).
Source: x-traffic.db — /i/api/graphql/*/UsersByRestIds, HomeTimeline, etc.
"""

from dataclasses import dataclass, field
from datetime import datetime
from typing import List, Optional

from ..value_objects import Handle, TweetId, UserId, TweetText, Timestamp, Language


@dataclass
class User:
    """Aggregate root: X user (from UsersByRestIds GraphQL response)."""
    id: UserId
    handle: Handle
    name: str = ""
    bio: str = ""
    location: str = ""
    url: str = ""
    created_at: Optional[datetime] = None
    followers_count: int = 0
    following_count: int = 0
    favorites_count: int = 0
    tweet_count: int = 0
    verified: bool = False
    is_blue_verified: bool = False
    profile_image_shape: str = "Circle"
    last_seen: Optional[datetime] = None

    def __post_init__(self):
        if not self.handle:
            raise ValueError("Handle cannot be empty")


@dataclass
class Tweet:
    """Aggregate root: X tweet (from HomeTimeline GraphQL response)."""
    id: TweetId
    author_id: UserId
    author_handle: Handle
    text: TweetText
    timestamp: Timestamp
    likes: int = 0
    retweets: int = 0
    replies: int = 0
    quotes: int = 0
    bookmarks: int = 0
    views: int = 0
    is_retweet: bool = False
    is_reply: bool = False
    lang: str = ""
    has_media: bool = False
    is_dm: bool = False

    def engagement_score(self) -> float:
        return self.likes + self.retweets * 2 + self.replies * 3 + self.quotes * 2
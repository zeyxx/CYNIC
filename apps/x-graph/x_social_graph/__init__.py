"""X Social Graph bounded context."""

__version__ = "0.1.0"

from .domain.aggregates import User, Tweet
from .domain.entities import Interaction, DmThread, InteractionType
from .domain.value_objects import Handle, TweetText, Timestamp, ScreenZone
from .domain.events import TweetCaptured, DmSent, PasscodeRequested, GraphUpdated
from .ports.repositories import GraphRepository, TrafficRepository, EmbeddingRepository

__all__ = [
    "User", "Tweet",
    "Interaction", "DmThread", "InteractionType",
    "Handle", "TweetText", "Timestamp", "ScreenZone",
    "TweetCaptured", "DmSent", "PasscodeRequested", "GraphUpdated",
    "GraphRepository", "TrafficRepository", "EmbeddingRepository",
]
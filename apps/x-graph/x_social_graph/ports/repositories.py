"""Ports (interfaces) for the X Social Graph bounded context."""

from abc import ABC, abstractmethod
from typing import List, Optional

from ..domain.aggregates import User, Tweet
from ..domain.entities import Interaction, DmThread
from ..domain.value_objects import UserId, TweetId, Handle


class GraphRepository(ABC):
    """Port: social graph storage."""

    @abstractmethod
    def save_user(self, user: User) -> None: ...

    @abstractmethod
    def get_user(self, user_id: UserId) -> Optional[User]: ...

    @abstractmethod
    def get_user_by_handle(self, handle: Handle) -> Optional[User]: ...

    @abstractmethod
    def save_tweet(self, tweet: Tweet) -> None: ...

    @abstractmethod
    def get_tweet(self, tweet_id: TweetId) -> Optional[Tweet]: ...

    @abstractmethod
    def save_interaction(self, interaction: Interaction) -> None: ...

    @abstractmethod
    def get_interactions(self, user_id: UserId) -> List[Interaction]: ...

    @abstractmethod
    def save_dm_thread(self, thread: DmThread) -> None: ...

    @abstractmethod
    def get_dm_threads(self, user_id: UserId) -> List[DmThread]: ...


class TrafficRepository(ABC):
    """Port: raw traffic capture storage."""

    @abstractmethod
    def save_request(self, method: str, url: str, endpoint: str,
                     request_body: str, response_status: int,
                     response_body: str, duration_ms: float) -> None: ...

    @abstractmethod
    def get_requests(self, since: float, limit: int = 100) -> list: ...


class EmbeddingRepository(ABC):
    """Port: vector embedding storage for semantic search."""

    @abstractmethod
    def store_embedding(self, entity_id: str, text: str, vector: list[float]) -> None: ...

    @abstractmethod
    def search_similar(self, query_vector: list[float], top_k: int = 5) -> list: ...
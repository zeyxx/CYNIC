from abc import ABC, abstractmethod
from .entities import ContextBundle, ResolvedSocialSource, SocialContextItem, TwitterSurfaceState

class ITwitterPerceiver(ABC):
    @abstractmethod
    def perceive(self) -> TwitterSurfaceState:
        pass


class IContextRepository(ABC):
    @abstractmethod
    def add_item(self, item: SocialContextItem) -> SocialContextItem:
        pass

    @abstractmethod
    def list_items(self, query: str = "", limit: int = 20) -> list[SocialContextItem]:
        pass

    @abstractmethod
    def exists(self, item_id: str) -> bool:
        pass

    @abstractmethod
    def count_items(self) -> int:
        pass


class ISocialSourceResolver(ABC):
    @abstractmethod
    def resolve(self, url: str) -> ResolvedSocialSource:
        pass


class IContextExporter(ABC):
    @abstractmethod
    def export_bundle(self, bundle: ContextBundle) -> dict[str, str]:
        pass


class IStateStorage(ABC):
    @abstractmethod
    def save_state(self, state: TwitterSurfaceState) -> None:
        pass

from datetime import datetime, timezone

from .entities import ContextBundle, SocialContextItem, TwitterSurfaceState
from .ports import IContextExporter, IContextRepository, ISocialSourceResolver, IStateStorage, ITwitterPerceiver


class IngestContextItem:
    def __init__(self, repository: IContextRepository):
        self.repository = repository

    def execute(self, item: SocialContextItem) -> SocialContextItem:
        return self.repository.add_item(item)


class IngestSocialUrl:
    def __init__(self, resolver: ISocialSourceResolver, repository: IContextRepository):
        self.resolver = resolver
        self.repository = repository

    def execute(self, url: str, tags: list[str] | None = None) -> SocialContextItem:
        resolved = self.resolver.resolve(url)
        item = resolved.to_context_item()
        if tags:
            item.tags = sorted(set(item.tags + tags))
        return self.repository.add_item(item)


class ListContextItems:
    def __init__(self, repository: IContextRepository):
        self.repository = repository

    def execute(self, query: str = "", limit: int = 20) -> list[SocialContextItem]:
        return self.repository.list_items(query=query, limit=limit)


class ExportContextBundle:
    def __init__(self, repository: IContextRepository, exporter: IContextExporter):
        self.repository = repository
        self.exporter = exporter

    def execute(self, query: str = "", limit: int = 20) -> tuple[ContextBundle, dict[str, str]]:
        items = self.repository.list_items(query=query, limit=limit)
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        safe_query = "".join(ch if ch.isalnum() else "-" for ch in (query or "latest").lower()).strip("-")
        bundle = ContextBundle(
            bundle_id=f"{timestamp}-{safe_query or 'latest'}",
            generated_at=datetime.now(timezone.utc).isoformat(),
            query=query,
            limit=limit,
            items=items,
        )
        return bundle, self.exporter.export_bundle(bundle)


class PerceiveTwitterSurface:
    def __init__(self, perceiver: ITwitterPerceiver, storage: IStateStorage):
        self.perceiver = perceiver
        self.storage = storage

    def execute(self, save: bool = True) -> TwitterSurfaceState:
        state = self.perceiver.perceive()
        if save:
            self.storage.save_state(state)
        return state

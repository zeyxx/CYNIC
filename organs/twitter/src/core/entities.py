from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, List
from urllib.parse import urlparse


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


@dataclass
class Trend:
    keyword: str
    count: int


@dataclass
class SocialContextItem:
    item_id: str
    source: str
    source_type: str
    text: str
    url: str = ""
    title: str = ""
    author: str = ""
    tags: List[str] = field(default_factory=list)
    notes: str = ""
    created_at: str = field(default_factory=utc_now_iso)
    metadata: dict[str, Any] = field(default_factory=dict)

    def searchable_text(self) -> str:
        parts = [self.title, self.author, self.text, self.notes, " ".join(self.tags)]
        return " ".join(part for part in parts if part).lower()

    def compact_citation(self) -> str:
        if self.author and self.url:
            return f"{self.title or self.source_type} by @{self.author} ({self.url})"
        if self.url:
            host = urlparse(self.url).netloc or self.url
            return f"{self.title or self.source_type} ({host})"
        if self.author:
            return f"{self.title or self.source_type} by @{self.author}"
        return self.title or self.item_id


@dataclass
class ResolvedSocialSource:
    url: str
    source_type: str
    text: str
    author: str = ""
    title: str = ""
    tags: List[str] = field(default_factory=list)
    notes: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_context_item(self, source: str = "x-resolver") -> SocialContextItem:
        return SocialContextItem(
            item_id="",
            source=source,
            source_type=self.source_type,
            text=self.text,
            url=self.url,
            title=self.title,
            author=self.author,
            tags=self.tags,
            notes=self.notes,
            metadata=self.metadata,
        )


@dataclass
class ContextBundle:
    bundle_id: str
    generated_at: str
    query: str
    limit: int
    items: List[SocialContextItem] = field(default_factory=list)

    def to_markdown(self) -> str:
        lines = [
            f"# Twitter/X Context Bundle: {self.bundle_id}",
            "",
            f"- generated_at: {self.generated_at}",
            f"- query: {self.query or '*'}",
            f"- items: {len(self.items)}",
            "",
        ]
        for index, item in enumerate(self.items, start=1):
            lines.extend(
                [
                    f"## {index}. {item.compact_citation()}",
                    "",
                    f"- id: {item.item_id}",
                    f"- type: {item.source_type}",
                    f"- source: {item.source}",
                    f"- created_at: {item.created_at}",
                    f"- tags: {', '.join(item.tags) if item.tags else ''}",
                    "",
                    item.text.strip(),
                    "",
                ]
            )
            if item.notes:
                lines.extend(["Notes:", item.notes.strip(), ""])
        return "\n".join(lines).strip() + "\n"


@dataclass
class TwitterSurfaceState:
    total_tweets_captured: int
    total_verdicts_judged: int
    total_context_items: int = 0
    latest_context_bundle: str = ""
    latest_trends: List[Trend] = field(default_factory=list)
    proxy_status: str = "unknown"
    timestamp: str = ""

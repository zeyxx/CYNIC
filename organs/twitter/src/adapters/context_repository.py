import dataclasses
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable

from organs.contracts.src.surface_contracts import iter_json_objects, write_jsonl_file_atomic

from ..core.entities import SocialContextItem
from ..core.ports import IContextRepository


def read_jsonl(path: Path) -> Iterable[dict]:
    if not path.exists():
        return []
    with open(path, "r", encoding="utf-8") as handle:
        return list(iter_json_objects(handle))


def count_jsonl(path: Path) -> int:
    if not path.exists():
        return 0
    with open(path, "r", encoding="utf-8") as handle:
        return sum(1 for line in handle if line.strip())


def stable_item_id(item: SocialContextItem) -> str:
    if item.url:
        seed = "\n".join([item.source_type.strip().lower(), item.url.strip().lower()])
    else:
        seed = "\n".join(
            [
                item.source.strip().lower(),
                item.source_type.strip().lower(),
                item.author.strip().lower(),
                item.text.strip(),
            ]
        )
    return hashlib.sha256(seed.encode("utf-8")).hexdigest()[:16]


def item_from_dict(data: dict) -> SocialContextItem:
    return SocialContextItem(
        item_id=str(data.get("item_id") or data.get("id") or ""),
        source=str(data.get("source") or "manual"),
        source_type=str(data.get("source_type") or data.get("type") or "note"),
        text=str(data.get("text") or data.get("content") or ""),
        url=str(data.get("url") or ""),
        title=str(data.get("title") or ""),
        author=str(data.get("author") or data.get("author_screen_name") or ""),
        tags=list(data.get("tags") or []),
        notes=str(data.get("notes") or ""),
        created_at=str(data.get("created_at") or data.get("timestamp") or datetime.now(timezone.utc).isoformat()),
        metadata=dict(data.get("metadata") or {}),
    )


class JsonlContextRepository(IContextRepository):
    def __init__(self, dataset_file: Path):
        self.dataset_file = dataset_file

    def add_item(self, item: SocialContextItem) -> SocialContextItem:
        self.dataset_file.parent.mkdir(parents=True, exist_ok=True)
        if not item.item_id:
            item.item_id = stable_item_id(item)
        rows = list(read_jsonl(self.dataset_file))
        payload = dataclasses.asdict(item)
        updated = False
        for index, row in enumerate(rows):
            if str(row.get("item_id") or row.get("id") or "") == item.item_id:
                rows[index] = payload
                updated = True
                break
        if not updated:
            rows.append(payload)
        write_jsonl_file_atomic(self.dataset_file, rows, ensure_ascii=False)
        return item

    def list_items(self, query: str = "", limit: int = 20) -> list[SocialContextItem]:
        items = [item_from_dict(row) for row in read_jsonl(self.dataset_file)]
        query = query.strip().lower()
        if query:
            terms = [term for term in query.split() if term]
            items = [
                item for item in items
                if all(term in item.searchable_text() for term in terms)
            ]
        items.sort(key=lambda item: item.created_at, reverse=True)
        return items[:limit]

    def exists(self, item_id: str) -> bool:
        for row in read_jsonl(self.dataset_file):
            if str(row.get("item_id") or row.get("id") or "") == item_id:
                return True
        return False

    def count_items(self) -> int:
        return count_jsonl(self.dataset_file)

from __future__ import annotations

import json
from pathlib import Path
from typing import Iterable

from organs.contracts.src.surface_contracts import iter_json_objects

from src.core.entities import Opportunity


class JsonlOpportunityRepository:
    def __init__(self, path: Path) -> None:
        self.path = path

    def list(self) -> list[Opportunity]:
        opportunities: list[Opportunity] = []
        with self.path.open("r", encoding="utf-8") as handle:
            for raw in iter_json_objects(handle):
                opportunities.append(_to_opportunity(raw))
        return opportunities


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: Iterable[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")


def _to_opportunity(raw: dict) -> Opportunity:
    skills = raw.get("skills") or ()
    if isinstance(skills, str):
        skills = (skills,)
    else:
        skills = tuple(str(item) for item in skills)

    resources = raw.get("resources_detected") or ()
    if isinstance(resources, str):
        resources = (resources,)
    else:
        resources = tuple(str(item) for item in resources)

    reward = raw.get("reward_amount") or 0
    try:
        reward_amount = float(reward)
    except (TypeError, ValueError):
        reward_amount = 0.0

    return Opportunity(
        slug=str(raw.get("slug") or ""),
        title=str(raw.get("title") or ""),
        url=str(raw.get("url") or ""),
        deadline=raw.get("deadline"),
        reward_amount=reward_amount,
        reward_asset=str(raw.get("token") or ""),
        skills=tuple(skills),
        region=str(raw.get("region") or ""),
        sponsor=str(raw.get("sponsor") or ""),
        description_text=str(raw.get("description_text") or raw.get("description") or ""),
        resources=tuple(resources),
    )

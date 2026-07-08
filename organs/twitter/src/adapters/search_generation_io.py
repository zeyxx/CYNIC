"""Filesystem adapters for Hermes X search generation."""

from __future__ import annotations

import json
import logging
from datetime import datetime
from pathlib import Path

from organs.contracts.src.surface_contracts import iter_json_objects, json_object_or_default


logger = logging.getLogger("search-generator")


def load_skill_md(skill_path: Path) -> str:
    """Load SKILL.md content. Returns empty string if missing."""
    if not skill_path.exists():
        logger.info("SKILL.md not found at %s", skill_path)
        return ""
    try:
        return skill_path.read_text(encoding="utf-8")
    except IOError as exc:
        logger.warning("Failed to read SKILL.md: %s", exc)
        return ""


def load_curation_yield(organ_dir: Path) -> dict:
    """Load curation_yield.json for domain weights."""
    yield_path = organ_dir / "curation_yield.json"
    if not yield_path.exists():
        return {}
    try:
        return json_object_or_default(yield_path.read_text(encoding="utf-8"), {})
    except IOError as exc:
        logger.warning("Failed to load curation_yield.json: %s", exc)
        return {}


def load_existing_tasks(tasks_path: Path) -> list[dict]:
    """Load existing search tasks to avoid duplicates."""
    tasks = []
    if not tasks_path.exists():
        return tasks
    try:
        with tasks_path.open(encoding="utf-8") as handle:
            tasks.extend(iter_json_objects(handle))
    except IOError:
        pass
    return tasks


def write_tasks(tasks: list[dict], tasks_path: Path) -> int:
    """Write search tasks to search_tasks.jsonl in append mode."""
    written = 0
    try:
        with tasks_path.open("a", encoding="utf-8") as handle:
            for task in tasks:
                task["generated_at"] = datetime.now().isoformat()
                task["status"] = "pending"
                handle.write(json.dumps(task, default=str) + "\n")
                written += 1
    except IOError as exc:
        logger.error("Failed to write search tasks: %s", exc)
    return written

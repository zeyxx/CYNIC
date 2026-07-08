#!/usr/bin/env python3
"""Hermes X search generation entrypoint."""

from __future__ import annotations

import logging
import os
import sys
from pathlib import Path

from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR, SEARCH_TASKS, SKILL_MD
from organs.twitter.src.adapters.search_generation_io import (
    load_curation_yield,
    load_existing_tasks,
    load_skill_md,
    write_tasks,
)
from organs.twitter.src.core.search_generation import (
    SEED_QUERIES,
    apply_yield_weights,
    build_candidate_queries,
    deduplicate,
    extract_domains_from_skill,
    select_queries,
)

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
__version__ = "0.1.0"
MAX_TASKS = 20

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("search-generator")


def run_generation(organ_dir: Path, skill_path: Path, tasks_path: Path) -> int:
    logger.info("Search Generator v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)

    skill_text = load_skill_md(skill_path)
    yield_data = load_curation_yield(organ_dir)
    existing_tasks = load_existing_tasks(tasks_path)

    pending_count = len([task for task in existing_tasks if task.get("status") == "pending"])
    if pending_count >= MAX_TASKS:
        logger.info("Already %d pending tasks (max %d), skipping generation", pending_count, MAX_TASKS)
        return 0

    candidates = build_candidate_queries(skill_text, yield_data)
    logger.info("Extracted %d queries from SKILL.md", len(extract_domains_from_skill(skill_text)))

    new_queries = deduplicate(candidates, existing_tasks)
    logger.info("After dedup: %d new queries", len(new_queries))
    if not new_queries:
        logger.info("No new queries to generate")
        return 0

    selected = select_queries(new_queries, MAX_TASKS - pending_count)
    written = write_tasks(selected, tasks_path)
    logger.info("Wrote %d search tasks to %s", written, tasks_path)
    return 0


def main() -> int:
    organ_dir = Path(os.environ.get("X_ORGAN_DIR", HERMES_X_DIR))
    return run_generation(organ_dir, SKILL_MD, SEARCH_TASKS)


if __name__ == "__main__":
    sys.exit(main())

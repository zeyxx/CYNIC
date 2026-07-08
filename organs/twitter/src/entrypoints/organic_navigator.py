#!/usr/bin/env python3
"""Hermes organic navigator entrypoint."""

from __future__ import annotations

import argparse
import asyncio
import logging
import sys
from pathlib import Path

from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR, NAVIGATION_LOG, SKILL_MD
from organs.twitter.src.adapters.organic_navigation_io import (
    apply_yield_boost,
    get_cdp_endpoint,
    load_targets_from_skill,
    log_navigation,
)
from organs.twitter.src.adapters.organic_navigation_runtime import browse_page, run_navigation_cycle
from organs.twitter.src.core.organic_navigation import DEFAULT_TARGETS, UCB1Selector

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
__version__ = "0.1.0"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("navigator")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC Hermes Organic Navigator - UCB1-guided browsing")
    parser.add_argument(
        "--organ-dir",
        type=Path,
        default=HERMES_X_DIR,
        help="Organ directory (default: ~/.cynic/organs/hermes/x)",
    )
    parser.add_argument("--budget", type=int, default=5, help="Max navigation actions per cycle")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    organ_dir = args.organ_dir.expanduser()

    logger.info("Organic Navigator v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)
    logger.info("Budget: %d actions", args.budget)

    if not organ_dir.exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return 1

    targets = apply_yield_boost(load_targets_from_skill(SKILL_MD), organ_dir)
    logger.info("Loaded %d navigation targets", len(targets))

    selector = UCB1Selector(NAVIGATION_LOG)
    logger.info(
        "UCB1 state: %d total visits across %d domains",
        selector.total_visits,
        len(selector.visit_counts),
    )
    return asyncio.run(run_navigation_cycle(organ_dir, args.budget, targets, selector))


if __name__ == "__main__":
    sys.exit(main())

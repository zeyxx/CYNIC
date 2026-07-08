#!/usr/bin/env python3
"""
Gemini Learning from Observation Verdicts.

Thin invocation membrane for the organ-twitter verdict learning loop.
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
from organs.twitter.src.adapters.verdict_learning_io import (
    load_observation_verdicts,
    update_skill_md,
)
from organs.twitter.src.core.verdict_learning import extract_patterns

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
logger = logging.getLogger("gemini-learn")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Learn from observation verdicts and update SKILL.md")
    parser.add_argument(
        "--organ-dir",
        default=str(HERMES_X_DIR),
        help="Organ directory containing observation-verdicts/",
    )
    return parser


def configure_logging() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )


def run_cycle(organ_dir: str) -> int:
    resolved = str(Path(organ_dir).expanduser())
    if not Path(resolved).exists():
        logger.error("Organ directory not found: %s", resolved)
        return 1

    verdicts = load_observation_verdicts(resolved)
    if not verdicts:
        logger.info("no observation verdicts to learn from yet")
        return 0

    logger.info("analyzing %d observation verdict(s)", len(verdicts))
    patterns = extract_patterns(verdicts)

    if update_skill_md(resolved, patterns):
        logger.info("learning complete: SKILL.md updated")
        return 0

    logger.error("learning failed: could not update SKILL.md")
    return 1


def main() -> int:
    configure_logging()
    args = build_parser().parse_args()
    return run_cycle(args.organ_dir)


if __name__ == "__main__":
    sys.exit(main())

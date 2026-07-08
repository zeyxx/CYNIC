#!/usr/bin/env python3
"""Expired Hermes X data architecture audit entrypoint.

Status: EXPIRED_EXPERIMENT (2026-05-22)
"""

from __future__ import annotations

import logging
import sys

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
from organs.twitter.src.adapters.data_architecture_audit_runtime import (
    load_jsonl,
    main,
    test_1_pixel_to_tweet,
    test_2_search_to_results,
    test_3_observation_to_tweets,
)


logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Hermes X ingest daemon entrypoint."""

from __future__ import annotations

import sys

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.twitter.src.adapters import x_ingest_runtime as runtime
from organs.twitter.src.adapters.x_ingest_runtime import (
    ACCOUNT_ID,
    API_KEY,
    DEFAULT_DATASET,
    DEFAULT_ORGAN_DIR,
    DOMAIN,
    JUDGE_THRESHOLD,
    KERNEL_ADDR,
    ORGAN_NAME,
    emit_convergence,
    ingest_row,
    load_domains_config,
    load_env,
    load_narrative_mappings,
    main,
    post_judge,
    post_observe,
    tail_dataset,
)


def _transport():
    runtime.KERNEL_ADDR = globals()["KERNEL_ADDR"]
    runtime.API_KEY = globals()["API_KEY"]
    return runtime._transport()


if __name__ == "__main__":
    main()

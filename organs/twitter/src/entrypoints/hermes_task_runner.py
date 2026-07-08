#!/usr/bin/env python3
"""Hermes X task runner entrypoint."""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from organs.twitter.src.adapters import hermes_task_runner_runtime as runtime
from organs.twitter.src.adapters.hermes_task_runner_runtime import (
    API_KEY,
    ORGAN_DIR,
    POLL_INTERVAL,
    claim_task,
    execute_task,
    poll_tasks,
    release_task,
)


__version__ = runtime.VERSION
KERNEL_ADDR = runtime.KERNEL_ADDR
logger = logging.getLogger("hermes-task-runner")


def load_env():
    runtime.load_env()
    globals()["KERNEL_ADDR"] = runtime.KERNEL_ADDR
    globals()["API_KEY"] = runtime.API_KEY
    globals()["ORGAN_DIR"] = runtime.ORGAN_DIR


def complete_task(task_id: str, result: str | None = None, error: str | None = None) -> bool:
    runtime.KERNEL_ADDR = KERNEL_ADDR
    runtime.API_KEY = API_KEY
    return runtime.complete_task(task_id, result=result, error=error)


def post_heartbeat(kind: str, tasks_executed: int, tasks_failed: int):
    runtime.KERNEL_ADDR = KERNEL_ADDR
    runtime.API_KEY = API_KEY
    return runtime.post_heartbeat(kind, tasks_executed, tasks_failed)


def call_mcp_ts_introspect(node: str, service: str, port: int = 0) -> dict | None:
    return runtime.call_mcp_ts_introspect(node, service, port)


def probe_service(node: str, service: str, port: int = 0) -> dict:
    runtime.KERNEL_ADDR = KERNEL_ADDR
    runtime.API_KEY = API_KEY
    return runtime.probe_service(node, service, port)


def run(organ_dir: str, interval: float):
    return runtime.run(organ_dir, interval)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Hermes Task Runner - reads organ-local tasks")
    parser.add_argument("--organ-dir", default=ORGAN_DIR, help="Organ directory containing agent-tasks/")
    parser.add_argument("--interval", type=float, default=POLL_INTERVAL, help="Poll interval (seconds)")
    return parser


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()
    args = build_parser().parse_args()

    organ_dir = args.organ_dir or ORGAN_DIR
    if not organ_dir:
        logger.error("Organ directory not set. Use --organ-dir or set X_ORGAN_DIR")
        sys.exit(1)

    organ_dir = str(Path(organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error("Organ directory not found: %s", organ_dir)
        sys.exit(1)

    logger.info("Organ directory: %s", organ_dir)
    run(organ_dir, args.interval)


if __name__ == "__main__":
    main()

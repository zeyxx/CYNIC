"""Mirror Agent daemon — entry point."""
from __future__ import annotations

import logging
import os
import signal
import sys
from pathlib import Path

import requests

from . import __version__
from .coordinator import Coordinator

log = logging.getLogger("mirror-agent")


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format='{"ts":"%(asctime)s","level":"%(levelname)s","msg":"%(message)s"}',
    )
    kernel_addr = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
    api_key = os.environ.get("CYNIC_API_KEY", "")
    mirror_dir = Path.home() / ".cynic" / "organs" / "mirror"
    behavior_log = Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl"
    dataset_path = Path(os.environ.get(
        "X_DATASET_PATH",
        str(Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"),
    ))

    log.info("Mirror Agent v%s starting — mirror_dir=%s", __version__, mirror_dir)

    _emit_lifecycle(kernel_addr, api_key, "started", f"v{__version__}")

    coordinator = Coordinator(
        mirror_dir=mirror_dir,
        behavior_log=behavior_log,
        dataset_path=dataset_path,
        kernel_addr=kernel_addr,
        api_key=api_key,
    )

    def _shutdown(signum: int, frame: object) -> None:
        log.info("Received signal %d — shutting down", signum)
        coordinator.shutdown()
        _emit_lifecycle(kernel_addr, api_key, "stopped", f"v{__version__}")
        sys.exit(0)

    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)
    coordinator.run_forever()


def _emit_lifecycle(addr: str, key: str, status: str, version: str) -> None:
    try:
        requests.post(
            f"http://{addr}/observe",
            headers={"Authorization": f"Bearer {key}"},
            json={
                "tool": "mirror_lifecycle", "target": status,
                "domain": "mirror-lifecycle",
                "context": f"Mirror Agent {version} {status}",
                "agent_id": "mirror-agent",
                "tags": ["mirror-lifecycle"],
            },
            timeout=5,
        )
    except requests.RequestException:
        pass


if __name__ == "__main__":
    main()

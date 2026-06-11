# Tier 3
import argparse
import logging
import os
import sys
from pathlib import Path

from .ouroboros import OuroborosLifecycle
from .adapters.kernel_rest import KernelRestAdapter
from .adapters.hermes_kanban_executor import HermesKanbanAdapter

logger = logging.getLogger("organ_daemon")

def _csv_env(name: str) -> set[str]:
    raw = os.environ.get(name, "")
    return {part.strip() for part in raw.split(",") if part.strip()}

def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )

    parser = argparse.ArgumentParser(description="Ouroboros Organ Daemon")
    parser.add_argument("--organ-dir", required=True, help="Organ directory")
    parser.add_argument("--interval", type=float, default=30.0, help="Poll interval (seconds)")
    args = parser.parse_args()

    organ_dir = str(Path(args.organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error(f"Organ directory not found: {organ_dir}")
        sys.exit(1)

    kernel_addr = os.environ.get("CYNIC_REST_ADDR", "")
    if not kernel_addr:
        logger.error("CYNIC_REST_ADDR not set in environment.")
        sys.exit(1)

    kernel_auth_key = os.environ.get("CYNIC_API_KEY", "")
    domain_allowlist = _csv_env("HERMES_AGENT_DOMAIN_ALLOW")
    kind_allowlist = _csv_env("HERMES_AGENT_KIND_ALLOW")
    domain_denylist = _csv_env("HERMES_AGENT_DOMAIN_DENY")

    kernel = KernelRestAdapter(
        kernel_addr=kernel_addr,
        auth_key=kernel_auth_key,
        organ_dir=organ_dir,
        domain_allowlist=domain_allowlist,
        kind_allowlist=kind_allowlist,
        domain_denylist=domain_denylist
    )
    
    executor = HermesKanbanAdapter()
    
    lifecycle = OuroborosLifecycle(
        kernel=kernel,
        executor=executor,
        organ_dir=organ_dir,
        interval=args.interval
    )
    
    lifecycle.start()

if __name__ == "__main__":
    main()

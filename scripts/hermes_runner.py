#!/usr/bin/env python3
"""
CYNIC Hermes Runner — Hexagonal Composition Root.
Wires domain logic, adapters, and organ profiles.
"""

import argparse
import logging
import os
import sys
from pathlib import Path

# Add cynic-python to path to allow imports of inference_organ
sys.path.append(str(Path(__file__).parent.parent / "cynic-python"))

from inference_organ.app.runner import RunnerLoop
from inference_organ.profiles.x_organ import XOrganProfile
from inference_organ.adapters.repository import LocalFileTaskRepository
from inference_organ.adapters.executor import SubprocessAgentExecutor
from inference_organ.adapters.publisher import HttpEventPublisher
from inference_organ.adapters.prober import McpProbeAdapter

def load_env():
    """Load CYNIC environment variables."""
    config = {
        "kernel_url": os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030"),
        "api_key": os.environ.get("CYNIC_API_KEY", ""),
        "organ_dir_base": os.environ.get("CYNIC_ORGAN_DIR", str(Path.home() / ".cynic" / "organs" / "hermes")),
        "mcp_path": os.environ.get("TAILSCALE_MCP", "/home/user/Bureau/tailscale-mcp/tailscale-mcp")
    }
    
    # Fallback to .cynic-env if exists
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            if line.startswith("export "):
                line = line[7:]
            key, _, val = line.partition("=")
            val = val.strip().strip('"').strip("'")
            if key == "CYNIC_REST_ADDR": config["kernel_url"] = val
            elif key == "CYNIC_API_KEY": config["api_key"] = val
            elif key == "X_ORGAN_DIR": # Legacy fallback
                 config["organ_dir_base"] = str(Path(val).parent)
            elif key == "TAILSCALE_MCP": config["mcp_path"] = val
            
    return config

def main():
    parser = argparse.ArgumentParser(description="CYNIC Hermes Runner (Hexagonal)")
    parser.add_argument("--organ", default="x", help="Organ name (x, discord, etc.)")
    parser.add_argument("--interval", type=float, default=30.0, help="Poll interval in seconds")
    parser.add_argument("--verbose", action="store_true", help="Enable debug logging")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )

    env = load_env()
    
    # Ensure kernel_url has a protocol (default to http if missing)
    kernel_url = env["kernel_url"]
    if "://" not in kernel_url:
        kernel_url = f"http://{kernel_url}"
    
    # 1. Resolve Profile
    if args.organ == "x":
        profile = XOrganProfile()
    else:
        print(f"Error: Unsupported organ '{args.organ}'")
        sys.exit(1)

    # 2. Resolve Paths
    organ_dir = Path(env["organ_dir_base"]) / profile.name
    
    # 3. Instantiate Adapters
    repository = LocalFileTaskRepository(organ_dir=str(organ_dir))
    executor = SubprocessAgentExecutor(organ_dir=str(organ_dir))
    publisher = HttpEventPublisher(
        kernel_url=kernel_url,
        api_key=env["api_key"],
        agent_id=f"hermes-{profile.name}"
    )
    prober = McpProbeAdapter(mcp_path=env["mcp_path"])

    # 4. Wire into Runner Loop
    runner = RunnerLoop(
        organ_profile=profile,
        repository=repository,
        executor=executor,
        publisher=publisher,
        prober=prober,
        interval=args.interval
    )

    try:
        runner.start()
    except KeyboardInterrupt:
        runner.stop()

if __name__ == "__main__":
    main()

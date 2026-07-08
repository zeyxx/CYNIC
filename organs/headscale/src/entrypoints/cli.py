#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from organs.headscale.src.adapters.runtime_adapter import HeadscaleRuntimeAdapter
from organs.headscale.src.core.mesh_contract import HeadscaleContract


def main() -> int:
    parser = argparse.ArgumentParser(description="organ-headscale CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("contract", help="Print the declared headscale contract")
    subparsers.add_parser("status", help="Print the local headscale runtime view")
    args = parser.parse_args()

    if args.command == "contract":
        payload = HeadscaleContract().as_dict()
    else:
        payload = HeadscaleRuntimeAdapter().runtime_view().__dict__
        payload["organ"] = "organ-headscale"

    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

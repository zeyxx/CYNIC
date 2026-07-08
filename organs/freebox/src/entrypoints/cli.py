"""CLI entrypoint for the Freebox organ."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys

if __package__ in (None, ""):
    sys.path.append(str(Path(__file__).resolve().parents[4]))
    sys.path.append(str(Path(__file__).resolve().parents[2]))
    from src.adapters.freebox_http import FreeboxHttpAdapter
    from src.core.config import EdgeExposureEvidence, FreeboxConfig, PortForwardIntent
    from src.core.operations import FreeboxRuntime
    from src.core.ports import FileTokenStore
else:
    from ..adapters.freebox_http import FreeboxHttpAdapter
    from ..core.config import EdgeExposureEvidence, FreeboxConfig, PortForwardIntent
    from ..core.operations import FreeboxRuntime
    from ..core.ports import FileTokenStore


def _build_runtime() -> FreeboxRuntime:
    token_file = Path(os.environ.get("FREEBOX_TOKEN_FILE", "~/.freebox_token")).expanduser()
    config = FreeboxConfig(
        freebox_url=os.environ.get("FREEBOX_API_URL", "http://mafreebox.freebox.fr/api/v8"),
        token_file=token_file,
    )
    provider = FreeboxHttpAdapter(config.freebox_url)
    token_store = FileTokenStore(config.token_file)
    return FreeboxRuntime(
        provider=provider,
        token_store=token_store,
        config=config,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Freebox provider mutation helper.")
    subparsers = parser.add_subparsers(dest="command")
    subparsers.add_parser("status", help="Emit non-mutating Freebox organ status.")
    subparsers.add_parser("auth", help="Request and persist app authorization.")

    open_port = subparsers.add_parser("open-port", help="Open an explicitly approved port mapping.")
    open_port.add_argument("--lan-ip", required=True)
    open_port.add_argument("--wan-port-start", type=int, required=True)
    open_port.add_argument("--wan-port-end", type=int)
    open_port.add_argument("--lan-port", type=int, required=True)
    open_port.add_argument("--protocol", choices=("tcp", "udp"), default="tcp")
    open_port.add_argument("--src-ip", default="0.0.0.0")
    open_port.add_argument("--comment", default="CYNIC approved edge exposure")
    open_port.add_argument("--approve-router-mutation", action="store_true")
    open_port.add_argument("--registry-intent-id", default="")
    open_port.add_argument("--synapse-target", default="")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    runtime = _build_runtime()
    if args.command == "auth":
        token = runtime.authorize()
        return 0 if token is not None else 1

    if args.command == "open-port":
        intent = PortForwardIntent(
            lan_ip=args.lan_ip,
            wan_port_start=args.wan_port_start,
            wan_port_end=args.wan_port_end or args.wan_port_start,
            lan_port=args.lan_port,
            protocol=args.protocol,
            src_ip=args.src_ip,
            comment=args.comment,
        )
        evidence = EdgeExposureEvidence(
            registry_intent_id=args.registry_intent_id,
            synapse_target=args.synapse_target,
        )
        ok = runtime.run_bridge(intent, approved=args.approve_router_mutation, evidence=evidence)
        return 0 if ok else 1

    print(
        json.dumps(
            {
                "schema": "cynic.freebox.status.v1",
                "mutates": False,
                "commands": ["status", "auth", "open-port"],
                "router_mutation_requires": [
                    "open-port",
                    "--approve-router-mutation",
                    "--registry-intent-id",
                    "--synapse-target <name>.synapse.cynic",
                ],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
from __future__ import annotations

import argparse
import logging
import os
import sys

try:
    from organs.wireguard.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

try:
    from dotenv import load_dotenv

    load_dotenv(ROOT / ".env")
except ImportError:
    pass

from organs.wireguard.src.adapters.cli_runtime import (
    WIREGUARD_SURFACE_REGISTRY,
    run_daemon,
    run_dns_activation_status,
    run_dns_bundle,
    run_provider_fallback_hosts,
    run_provider_fallback_status,
    run_dns_plan,
    run_status,
    run_sync_dns,
    run_validate,
)


logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s")
logger = logging.getLogger("wireguard-cli")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-wireguard control surface")
    subparsers = parser.add_subparsers(dest="command")

    sync_dns = subparsers.add_parser("sync-dns", help="render CoreDNS synapse hosts from registry")
    sync_dns.add_argument("--registry", default="infra/registry.json")
    sync_dns.add_argument("--state-dir", default=os.environ.get("CYNIC_COREDNS_STATE_DIR", "/etc/coredns"))

    validate = subparsers.add_parser("validate", help="validate registry network policy without writing files")
    validate.add_argument("--registry", default="infra/registry.json")

    dns = subparsers.add_parser("dns", help="plan, bundle or inspect Synapse DNS activation")
    dns_subparsers = dns.add_subparsers(dest="dns_command", required=True)
    dns_subparsers.add_parser("plan")
    dns_bundle = dns_subparsers.add_parser("bundle")
    dns_bundle.add_argument("--registry", default="infra/registry.json")
    dns_bundle.add_argument("--output-dir", required=True)
    dns_status = dns_subparsers.add_parser("activation-status")
    dns_status.add_argument("--hosts-path", default="/etc/coredns/synapse.hosts")
    dns_status.add_argument("--corefile-path", default="/etc/coredns/Corefile")
    dns_status.add_argument("--service-path", default="/etc/systemd/system/coredns-synapse.service")
    dns_status.add_argument(
        "--resolved-dropin-path",
        default="/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
    )
    dns_status.add_argument("--resolve-dns", action="store_true")

    fallback_hosts = subparsers.add_parser(
        "fallback-hosts",
        help="render temporary provider-plane hosts bundle from registry network.local values",
    )
    fallback_hosts.add_argument("--registry", default="infra/registry.json")
    fallback_hosts.add_argument("--output-dir", required=True)
    fallback_status = subparsers.add_parser(
        "fallback-status",
        help="verify temporary provider-plane hosts and route resurrection path",
    )
    fallback_status.add_argument("--registry", default="infra/registry.json")
    fallback_status.add_argument("--hosts-path", default="/etc/hosts")
    fallback_status.add_argument("--check-connectivity", action="store_true")

    status = subparsers.add_parser("status", help="emit non-mutating Synapse mesh status")
    status.add_argument("--registry", default="infra/registry.json")
    status.add_argument("--resolve-dns", action="store_true")
    status.add_argument("--include-records", action="store_true")
    status.add_argument("--observe-runtime", action="store_true")
    subparsers.add_parser("daemon", help="run the WireGuard mesh control loop")
    return parser


def dispatch(args: argparse.Namespace, parser: argparse.ArgumentParser) -> int:
    command = args.command or "daemon"
    if command == "sync-dns":
        return run_sync_dns(args.registry, args.state_dir)
    if command == "validate":
        return run_validate(args.registry)
    if command == "dns":
        if args.dns_command == "plan":
            return run_dns_plan()
        if args.dns_command == "bundle":
            return run_dns_bundle(args.registry, args.output_dir)
        if args.dns_command == "activation-status":
            return run_dns_activation_status(
                args.hosts_path,
                args.corefile_path,
                args.service_path,
                args.resolved_dropin_path,
                args.resolve_dns,
            )
    if command == "fallback-hosts":
        return run_provider_fallback_hosts(args.registry, args.output_dir)
    if command == "fallback-status":
        return run_provider_fallback_status(args.registry, args.hosts_path, args.check_connectivity)
    if command == "status":
        return run_status(args.registry, args.resolve_dns, args.include_records, args.observe_runtime)
    if command == "daemon":
        if os.environ.get("CYNIC_ALLOW_LEGACY_WIREGUARD_DAEMON") != "1":
            print(
                "organ-wireguard daemon is legacy-blocked; use organ-fleet/cynic-node surfaces or set "
                "CYNIC_ALLOW_LEGACY_WIREGUARD_DAEMON=1 for breakglass compatibility.",
                file=sys.stderr,
            )
            return 2
        return run_daemon()
    parser.error(f"unknown command: {command}")
    return 2


def main() -> int:
    parser = build_parser()
    return dispatch(parser.parse_args(), parser)


if __name__ == "__main__":
    raise SystemExit(main())

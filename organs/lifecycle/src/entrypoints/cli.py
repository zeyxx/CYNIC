import argparse
import asyncio
import logging
import sys

try:
    from organs.lifecycle.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


if __package__ in {None, ""}:
    project_root = bootstrap_repo_path()
    if str(project_root) not in sys.path:
        sys.path.insert(0, str(project_root))

from organs.lifecycle.src.core.cli_runtime import (
    analyze_chain_command,
    convergence_dispatch_command,
    convergence_pack_command,
    destroy_command,
    mempool_fetch_command,
    provision_command,
    runtime_cycle_command,
)

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC organ-lifecycle CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)

    prov_parser = subparsers.add_parser("provision")
    prov_parser.add_argument("--target", required=True, help="Node role to provision")
    prov_parser.add_argument("--registry", required=True, help="Path to infra/registry.json")

    dest_parser = subparsers.add_parser("destroy")
    dest_parser.add_argument("--vmid", required=True, type=int)
    dest_parser.add_argument("--force", action="store_true")

    mempool_parser = subparsers.add_parser("mempool")
    mempool_sub = mempool_parser.add_subparsers(dest="mempool_cmd", required=True)
    mempool_sub.add_parser("fetch")

    analyze_parser = subparsers.add_parser("analyze-chain")
    analyze_parser.add_argument("--registry", required=True, help="Path to infra/registry.json")

    convergence_parser = subparsers.add_parser("convergence-pack")
    convergence_parser.add_argument("--registry", required=True, help="Path to infra/registry.json")

    convergence_dispatch_parser = subparsers.add_parser("convergence-dispatch")
    convergence_dispatch_parser.add_argument("--registry", required=True, help="Path to infra/registry.json")

    subparsers.add_parser("runtime-cycle")
    return parser


async def run_command(args) -> int:
    if args.command == "provision":
        return await provision_command(args)
    if args.command == "destroy":
        return await destroy_command(args)
    if args.command == "mempool" and args.mempool_cmd == "fetch":
        return await mempool_fetch_command(args)
    if args.command == "analyze-chain":
        return await analyze_chain_command(args)
    if args.command == "convergence-pack":
        return await convergence_pack_command(args)
    if args.command == "convergence-dispatch":
        return await convergence_dispatch_command(args)
    if args.command == "runtime-cycle":
        return await runtime_cycle_command(args)
    raise ValueError(f"unsupported command: {args.command}")


def main() -> int:
    return asyncio.run(run_command(build_parser().parse_args()))


if __name__ == "__main__":
    sys.exit(main())

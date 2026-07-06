"""Radar CLI — Scan targets and produce structured observations.

Usage:
    # Scan all configured targets
    python -m src.entrypoints.cli

    # Scan a single target
    python -m src.entrypoints.cli --target hermes-agent

    # JSON output for kernel ingestion
    python -m src.entrypoints.cli --json
"""

import argparse
import asyncio
import json
import sys

from src.core.scanner import DEFAULT_TARGETS, scan_target


async def main_async(target_name: str = None, json_output: bool = False, verbose: bool = False):
    """Main async entrypoint."""
    if target_name:
        targets = [DEFAULT_TARGETS[target_name]] if target_name in DEFAULT_TARGETS else []
        if not targets:
            print(f"Error: unknown target '{target_name}'", file=sys.stderr)
            sys.exit(1)
    else:
        targets = list(DEFAULT_TARGETS.values())

    results = []
    for target in targets:
        result = await scan_target(target, verbose=verbose)
        results.append(result)

    # Print results
    updates = [r for r in results if r["status"] == "update_available"]
    if updates:
        print(f"\n⚠ {len(updates)} update(s) available:")
        for u in updates:
            print(f"  • {u['target_id']}: {u['current_version']} → {u['latest_version']}")
            if u.get("release_info"):
                print(f"    URL: {u['release_info']['html_url']}")

    if json_output:
        if len(results) == 1:
            print(json.dumps(results[0], indent=2))
        else:
            print(json.dumps(results, indent=2))
    else:
        print(f"\n[*] Scan complete. {len(results)} target(s) checked.")
        for r in results:
            status_icon = "⚠" if r["status"] == "update_available" else "✓"
            print(f"  {status_icon} {r['target_id']}: {r['status']} (latency={r['latency_ms']}ms)")

    return results


def main():
    """CLI entrypoint."""
    parser = argparse.ArgumentParser(description="Radar — Scan ecosystem targets")
    parser.add_argument("--target", help="Scan a specific target by ID")
    parser.add_argument("--json", action="store_true", help="Output raw JSON")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    args = parser.parse_args()

    asyncio.run(main_async(
        target_name=args.target,
        json_output=args.json,
        verbose=args.verbose,
    ))


if __name__ == "__main__":
    main()

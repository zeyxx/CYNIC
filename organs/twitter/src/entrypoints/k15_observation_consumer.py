#!/usr/bin/env python3
"""K15 observation consumer entrypoint."""

from __future__ import annotations

import argparse
import sys

from organs.twitter.src.adapters import k15_observation_runtime
from organs.twitter.src.adapters.k15_observation_runtime import K15ObservationConsumer


requests = k15_observation_runtime.requests


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="K15 Observation Consumer")
    parser.add_argument("--kernel-url", default="", help="Kernel URL (or set CYNIC_REST_ADDR)")
    parser.add_argument("--api-key", default=None, help="API key (or $CYNIC_API_KEY)")
    parser.add_argument("--domain", default="twitter", help="Domain filter")
    parser.add_argument("--poll-interval", type=int, default=60, help="Poll interval in seconds")
    parser.add_argument("--max-iter", type=int, default=None, help="Max iterations for testing")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        consumer = K15ObservationConsumer(
            kernel_url=args.kernel_url,
            api_key=args.api_key,
            domain_filter=args.domain,
        )
    except RuntimeError as err:
        print(f"Configuration error: {err}", file=sys.stderr)
        print(
            "Set CYNIC_REST_ADDR (or --kernel-url), and set CYNIC_ALLOW_LOCAL_KERNEL=1 "
            "if local loopback is intended.",
            file=sys.stderr,
        )
        return 1

    consumer.run(poll_interval=args.poll_interval, max_iterations=args.max_iter)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

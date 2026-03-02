#!/usr/bin/env python3
"""
Profile TPS Baseline — Identify hot functions in event bus under load.

Uses Python's cProfile to analyze performance bottlenecks during high-frequency
event emission. Helps identify optimization opportunities for 10k TPS readiness.

Usage:
    python scripts/profile_tps_baseline.py [--output profile_results.prof]

Output:
    - profile_results.prof: Raw profiling data (binary format)
    - Console output: Top 30 functions by cumulative time
"""

from __future__ import annotations

import asyncio
import cProfile
import pstats
import sys
import argparse
import logging
from io import StringIO
from pathlib import Path

logger = logging.getLogger("profile_tps_baseline")


async def profile_event_emission(num_events: int = 5000) -> None:
    """
    Profile event emission under load.

    Emits 5000 events through the event bus and measures which functions
    consume the most time. Helps identify hot paths for optimization.

    Args:
        num_events: Number of events to emit for profiling
    """
    from cynic.kernel.organism.organism import awaken
    from cynic.kernel.core.event_bus import Event, CoreEvent

    logger.info(f"Profiling event emission: {num_events} events...")

    # Awaken organism
    organism = await awaken(db_pool=None)
    await organism.start()

    bus = organism.cognition.orchestrator.bus

    # Create events
    events = [
        Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            {"event_index": i},
            source="profile_tps"
        )
        for i in range(num_events)
    ]

    # Profile event emission
    logger.info(f"Starting profiling of {num_events} event emissions...")
    for event in events:
        await bus.emit(event)

    # Drain pending tasks
    await bus.drain(timeout=10.0)

    stats = bus.stats()
    logger.info(f"Emission complete: {stats['emitted']} events in {num_events} iterations")


def main():
    """Main entry point for profiling script."""
    parser = argparse.ArgumentParser(
        description="Profile TPS baseline — identify hot functions in event bus"
    )
    parser.add_argument(
        "--output",
        default="profile_results.prof",
        help="Output file for profiling data (default: profile_results.prof)"
    )
    parser.add_argument(
        "--num-events",
        type=int,
        default=5000,
        help="Number of events to emit for profiling (default: 5000)"
    )
    parser.add_argument(
        "--top",
        type=int,
        default=30,
        help="Number of top functions to display (default: 30)"
    )

    args = parser.parse_args()

    # Set up logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    )

    logger.info(f"TPS Baseline Profiling")
    logger.info(f"Output file: {args.output}")
    logger.info(f"Events to emit: {args.num_events}")
    logger.info(f"Top functions: {args.top}")

    # Create profiler
    profiler = cProfile.Profile()

    try:
        # Run profiled code
        logger.info("Starting profiler...")
        profiler.enable()

        # Run async event emission
        asyncio.run(profile_event_emission(num_events=args.num_events))

        profiler.disable()
        logger.info("Profiler complete")

    except Exception as e:
        logger.error(f"Profiling failed: {e}", exc_info=True)
        sys.exit(1)

    # Save profiling results
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    logger.info(f"Saving profiling data to {output_path}")
    profiler.dump_stats(str(output_path))

    # Print top functions
    logger.info(f"Top {args.top} functions by cumulative time:")
    print("\n" + "=" * 80)
    print(f"Top {args.top} Functions by Cumulative Time")
    print("=" * 80)

    stats = pstats.Stats(profiler, stream=sys.stdout)
    stats.strip_dirs()
    stats.sort_stats("cumulative")
    stats.print_stats(args.top)

    print("\n" + "=" * 80)
    print(f"Profile data saved to: {output_path}")
    print("=" * 80)


if __name__ == "__main__":
    main()

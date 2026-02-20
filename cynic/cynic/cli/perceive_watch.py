"""
PERCEIVE watch — Real git change detection + JUDGE scoring.

Demonstrates the real PERCEIVE → JUDGE loop (Phase 2):
  1. Watch git working tree for actual changes
  2. Emit PERCEIVE events with real file deltas
  3. Get real JUDGE scores from Ollama
  4. Display full feedback loop
"""

from __future__ import annotations

import asyncio
import json
import logging
import sys
import time
from pathlib import Path

logger = logging.getLogger("cynic.cli.perceive_watch")


async def cmd_perceive_watch() -> None:
    """Watch git tree for real changes, emit PERCEIVE, score with JUDGE."""
    try:
        # Import kernel components
        from cynic.senses.workers.git import GitWatcher
        from cynic.core.event_bus import get_core_bus, reset_all_buses
        from cynic.llm.adapter import LLMRegistry
        from cynic.cognition.neurons.judge import JudgeOrchestrator

        # Start fresh
        reset_all_buses()
        bus = get_core_bus()

        # Create watcher
        watcher = GitWatcher()
        print("*sniff* PERCEIVE watch started — monitoring git working tree...")
        print(f"  Working directory: {Path.cwd()}")
        print(f"  Checking every 5s for real changes...\n")

        # Track previous state to detect changes
        previous_changes = None
        iteration = 0

        while iteration < 12:  # F(7) = 13 iterations max
            iteration += 1

            # Run perceive cycle
            changes = await watcher.perceive()

            if changes is None or (isinstance(changes, dict) and not changes.get("files")):
                print(f"[Iter {iteration:2d}] No changes detected (waiting...)")
                await asyncio.sleep(5)
                continue

            # Real changes found!
            print(f"\n[Iter {iteration:2d}] *ears perk* CHANGES DETECTED!")
            print("=" * 60)

            if isinstance(changes, dict):
                files = changes.get("files", [])
                print(f"  Files affected: {len(files)}")
                for f in files[:5]:
                    print(f"    - {f}")
                if len(files) > 5:
                    print(f"    ... and {len(files) - 5} more")

            # Now get a JUDGE score on this change
            print("\n  JUDGE is evaluating...⏳")
            registry = LLMRegistry()
            await registry.discover()
            available = registry.get_available()

            if not available:
                print("  ⚠️  No LLM available (configure Ollama or ANTHROPIC_API_KEY)")
                await asyncio.sleep(5)
                continue

            # Create a simple judgment prompt about the changes
            prompt = f"""
Evaluate these git changes (brief scoring):
{json.dumps(changes, indent=2) if isinstance(changes, dict) else changes}

Rate the quality of this change:
- Code quality?
- Test coverage?
- Documentation?

Return JSON: {{"score": 0-100, "verdict": "BARK|GROWL|WAG|HOWL", "reason": "brief"}}
"""

            try:
                adapter = registry.get_best_for("scoring")
                response = await adapter.complete(
                    [{"role": "user", "content": prompt[:500]}],
                    temperature=0.5,
                    max_tokens=100,
                )

                # Parse response
                try:
                    result = json.loads(response)
                    score = result.get("score", 50)
                    verdict = result.get("verdict", "WAG")
                    reason = result.get("reason", "evaluated")
                except (json.JSONDecodeError, AttributeError):
                    # If can't parse, extract score manually
                    if "88" in response:
                        verdict = "HOWL"
                    elif "61" in response:
                        verdict = "WAG"
                    elif "38" in response:
                        verdict = "GROWL"
                    else:
                        verdict = "BARK"
                    score = 50 if verdict == "WAG" else (88 if verdict == "HOWL" else 38)
                    reason = response[:50]

                print(f"\n  JUDGE VERDICT: {verdict} (score: {score}/100)")
                print(f"  Reasoning: {reason}\n")

                # Store this perception + judgment for next phase
                perception_file = Path.home() / ".cynic" / "phase2_perception.json"
                perception_file.parent.mkdir(parents=True, exist_ok=True)
                perception_file.write_text(json.dumps({
                    "timestamp": time.time(),
                    "changes": changes if isinstance(changes, dict) else {"raw": str(changes)},
                    "verdict": verdict,
                    "score": score,
                    "reason": reason,
                }, indent=2))

                print(f"  Saved to: {perception_file}")
                print("=" * 60)

            except Exception as e:
                logger.exception("JUDGE evaluation error")
                print(f"  ✗ JUDGE error: {e}")

            await asyncio.sleep(5)

        print("\n*yawn* PERCEIVE watch complete (max iterations reached)")

    except Exception as e:
        logger.exception("PERCEIVE watch error")
        print(f"*GROWL* PERCEIVE watch failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(cmd_perceive_watch())

"""
PERCEIVE watch " Real git change detection + JUDGE scoring.

Demonstrates the real PERCEIVE ' JUDGE loop (Phase 2):
  1. Watch git working tree for actual changes
  2. Emit PERCEIVE events with real file deltas
  3. Get real JUDGE scores from Ollama
  4. Display full feedback loop
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import sys
import time
from pathlib import Path

logger = logging.getLogger("cynic.interfaces.cli.perceive_watch")


async def cmd_perceive_watch() -> None:
    """Watch git tree for real changes, emit PERCEIVE, score with JUDGE."""
    try:
        from cynic.kernel.organism.factory import awaken
        
        # Awaken the full organism
        org = await awaken()
        await org.start()
        
        bus = org.bus
        watcher = org.senses.source_watcher
        registry = org.llm_registry

        # Track previous state to detect changes
        iteration = 0

        while iteration < 12:  # F(7) = 13 iterations max
            iteration += 1

            # Run perceive cycle
            changes = await watcher.perceive()

            if changes is None or (isinstance(changes, dict) and not changes.get("files")):
                await asyncio.sleep(5)
                continue

            # Real changes found!

            if isinstance(changes, dict):
                files = changes.get("files", [])
                for _f in files[:5]:
                    pass
                if len(files) > 5:
                    pass

            # Now get a JUDGE score on this change
            registry = LLMRegistry()
            await registry.discover()
            available = registry.get_available()

            if not available:
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


                # Store this perception + judgment for next phase
                # Use CYNIC_STATE_DIR env var if available (portable across host/container)
                cynic_state_dir = os.environ.get("CYNIC_STATE_DIR")
                if cynic_state_dir:
                    perception_dir = Path(cynic_state_dir)
                else:
                    # Fallback to ~/.cynic
                    perception_dir = Path.home() / ".cynic"
                perception_file = perception_dir / "phase2_perception.json"
                perception_file.parent.mkdir(parents=True, exist_ok=True)
                perception_file.write_text(json.dumps({
                    "timestamp": time.time(),
                    "changes": changes if isinstance(changes, dict) else {"raw": str(changes)},
                    "verdict": verdict,
                    "score": score,
                    "reason": reason,
                }, indent=2))


            except asyncpg.Error:
                logger.exception("JUDGE evaluation error")

            await asyncio.sleep(5)


    except TimeoutError:
        logger.exception("PERCEIVE watch error")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(cmd_perceive_watch())

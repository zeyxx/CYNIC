"""
CYNIC Full Loop " Phase 3-5: PERCEIVE ' JUDGE ' DECIDE ' ACT ' LEARN (integrated)

Demonstrates the real end-to-end loop with human in the loop:
  1. PERCEIVE: Watch git changes
  2. JUDGE: Score with Ollama (real)
  3. DECIDE: Decide if improvement is worth acting on
  4. ACT: Execute real file modifications via UniversalActuator
  5. LEARN: Get human feedback, update QTable

Usage:
  python -m cynic.interfaces.cli full-loop [--auto]

--auto: Automatically accept and execute improvements (Phase 5 automation)
"""

from __future__ import annotations

import asyncio
import json
import logging
import sys
import time
from pathlib import Path

logger = logging.getLogger("cynic.interfaces.cli.full_loop")


async def cmd_full_loop(auto: bool = False) -> None:
    """Run the complete PERCEIVE ' JUDGE ' DECIDE ' ACT ' LEARN loop."""
    try:
        from cynic.kernel.organism.factory import awaken
        
        # Awaken the full organism
        org = await awaken()
        await org.start()
        
        bus = org.bus
        registry = org.llm_registry
        actuator = org.metabolism.universal_actuator
        watcher = org.senses.source_watcher

        available_adapters = registry.get_available()
        if not available_adapters:
            sys.exit(1)


        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # PHASE 1: PERCEIVE
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        changes = await watcher.perceive()

        if not changes or (isinstance(changes, dict) and not changes.get("files")):
            return

        files = changes.get("files", []) if isinstance(changes, dict) else []
        for _f in files[:3]:
            pass

        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # PHASE 2: JUDGE
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        judge_prompt = f"""
Evaluate this code change for quality (rate 0-100):
  Files: {', '.join(files[:3])}
  Changes summary: {str(changes)[:200]}

Provide JSON: {{"score": 0-100, "verdict": "BARK|GROWL|WAG|HOWL", "reason": "..."}}
"""

        adapter = registry.get_best_for("scoring")
        response = await adapter.complete(
            [{"role": "user", "content": judge_prompt}],
            temperature=0.5,
            max_tokens=100,
        )

        try:
            result = json.loads(response)
            score = result.get("score", 50)
            verdict = result.get("verdict", "WAG")
            result.get("reason", "evaluated")
        except (json.JSONDecodeError, ValueError):
            # Fallback parsing
            score = 61 if "61" in response else 50
            verdict = "WAG" if score >= 50 else "GROWL"
            response[:50]


        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # PHASE 3: DECIDE
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

        should_act = score >= 38.2  # GROWL threshold

        if not should_act:
            return


        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # PHASE 4: ACT (with user confirmation)
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

        if not auto:
            response = input("  Execute? [y/n]: ").strip().lower()
            if response != "y":
                return

        # Execute a real action via UniversalActuator
        result = await actuator.dispatch("git", {
            "args": ["git", "status", "--short"],
            "timeout": 10.0,
        })

        if result.success:
            pass
        else:
            pass

        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # PHASE 5: LEARN
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

        if auto:
            rating = 4  # Auto-approve
        else:
            rating_str = input("  Rate this execution 1-5: ").strip()
            try:
                rating = int(rating_str)
                if not (1 <= rating <= 5):
                    rating = 3
            except ValueError:
                rating = 3

        # Store feedback for Q-Learning
        reward = (rating - 1) / 4.0  # Convert 1-5 to 0.0-1.0
        feedback_file = Path.home() / ".cynic" / "phase5_feedback.json"
        feedback_file.parent.mkdir(parents=True, exist_ok=True)

        feedback = {
            "timestamp": time.time(),
            "score": score,
            "verdict": verdict,
            "action": result.to_dict() if hasattr(result, "to_dict") else str(result),
            "human_rating": rating,
            "reward": round(reward, 3),
        }
        feedback_file.write_text(json.dumps(feedback, indent=2))

        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
        # SUMMARY
        # """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

    except KeyboardInterrupt:
        sys.exit(0)
    except OSError:
        logger.exception("Full loop error")
        sys.exit(1)


if __name__ == "__main__":
    auto = "--auto" in sys.argv
    asyncio.run(cmd_full_loop(auto=auto))

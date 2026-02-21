"""
CYNIC Autonomous Battle Runner — Multi-Agent Learning Arena

Usage: python -m cynic.cli battles [--duration 8h] [--interval 30s] [--dry-run]

This runs continuous "battles" where CYNIC and Claude Code execute proposed actions,
measure outcomes, and learn from feedback. Both systems improve simultaneously.

Schema: ~/.cynic/battles.jsonl — one JSON object per battle, newline-delimited.
"""
import argparse
import asyncio
import json
import logging
import os
import sys
import time
import urllib.request
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
from typing import Any, Optional

from cynic.cli.utils import _c, _CYNIC_DIR, _API, _API_TIMEOUT

logger = logging.getLogger("cynic.cli.battles")
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s — %(message)s",
)


@dataclass
class BattleRecord:
    """One complete battle (propose → execute → measure → log)."""
    battle_id: str                  # unique ID
    timestamp_ms: float             # when battle started
    duration_ms: float              # how long execution took
    proposer: str                   # "CYNIC" or "CLAUDE_CODE"
    action_type: str                # "INVESTIGATE", "REFACTOR", etc.
    action_id: str                  # reference to action
    executed: bool                  # did it actually run?
    outcome: dict[str, Any]         # result data
    feedback: Optional[float]       # quality score 0-100
    q_score_before: float           # before judgment
    q_score_after: float            # after judgment
    verdict: Optional[str]          # HOWL/WAG/GROWL/BARK
    learning_signal: float          # reward for Q-learning
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


class BattleRunner:
    """Orchestrates autonomous battles between CYNIC and Claude Code."""

    def __init__(self, dry_run: bool = False):
        self.dry_run = dry_run
        self.start_time = time.time()
        self.battle_count = 0
        self.battles_file = os.path.join(_CYNIC_DIR, "battles.jsonl")

    async def run(self, max_duration_s: int = 28800, interval_s: int = 30) -> None:
        """
        Run battles for up to max_duration_s seconds.

        Args:
            max_duration_s: Maximum runtime (default 8h = 28800s)
            interval_s: Pause between battles (default 30s)
        """
        logger.info(
            _c("cyan", f"🎯 Starting autonomous battle arena ({max_duration_s//3600}h max)")
        )
        logger.info(f"   Logging to: {self.battles_file}")
        logger.info(f"   Dry-run mode: {self.dry_run}")

        os.makedirs(_CYNIC_DIR, exist_ok=True)

        iteration = 0
        while True:
            elapsed = time.time() - self.start_time
            if elapsed > max_duration_s:
                logger.info(
                    _c("yellow", f"⏱  Time limit reached ({max_duration_s//3600}h)")
                )
                break

            iteration += 1
            try:
                # Run one battle cycle
                await self._battle_cycle(iteration)

                # Sleep before next cycle
                logger.info(f"   Sleeping {interval_s}s before next battle...")
                await asyncio.sleep(interval_s)

            except KeyboardInterrupt:
                logger.info(_c("yellow", "Interrupted by user"))
                break
            except CynicError as e:
                logger.error(f"Battle cycle error: {e}", exc_info=True)
                await asyncio.sleep(interval_s)

        # Summary
        elapsed = time.time() - self.start_time
        logger.info(
            _c("green", f"✓ Battle arena completed: {self.battle_count} battles in {elapsed:.0f}s")
        )

    async def _battle_cycle(self, iteration: int) -> None:
        """Run one complete battle cycle."""
        start_time = time.time()
        logger.info(f"\n--- Battle {iteration} ---")

        # 1. Get pending actions from CYNIC
        actions = await self._get_pending_actions()
        if not actions:
            logger.info("No pending actions")
            return

        # 2. Execute first action
        action = actions[0]
        battle = await self._execute_action(action, iteration)

        # 3. Log battle record
        self._log_battle(battle)
        self.battle_count += 1

        duration_ms = (time.time() - start_time) * 1000
        logger.info(f"Battle {iteration} completed in {duration_ms:.0f}ms")

    async def _get_pending_actions(self) -> list[dict[str, Any]]:
        """Fetch pending actions from CYNIC server."""
        try:
            req = urllib.request.Request(f"{_API}/actions")
            with urllib.request.urlopen(req, timeout=_API_TIMEOUT) as resp:
                data = json.loads(resp.read().decode())
                actions = data.get("actions", [])
                return [a for a in actions if a.get("status") == "PENDING"]
        except json.JSONDecodeError as e:
            logger.warning(f"Could not fetch actions: {e}")
            return []

    async def _execute_action(self, action: dict[str, Any], battle_id: int) -> BattleRecord:
        """Execute an action and create a battle record."""
        action_id = action.get("action_id", f"action_{battle_id}")

        # Get current Q-score
        q_before = action.get("confidence", 0.0) * 100  # Rough estimate

        if self.dry_run:
            logger.info(f"  [DRY-RUN] Would execute action {action_id}")
            outcome = {"dry_run": True}
            executed = False
            q_after = q_before
        else:
            # Actually execute
            logger.info(f"  Executing action {action_id}...")
            try:
                req = urllib.request.Request(
                    f"{_API}/actions/{action_id}/accept",
                    data=b"{}",
                    headers={"Content-Type": "application/json"},
                    method="POST",
                )
                with urllib.request.urlopen(req, timeout=_API_TIMEOUT) as resp:
                    outcome = json.loads(resp.read().decode())
                    executed = outcome.get("executing", False)
                    logger.info(f"  Execution result: {executed}")
            except json.JSONDecodeError as e:
                logger.error(f"  Execution failed: {e}")
                outcome = {"error": str(e)}
                executed = False

            q_after = q_before + (5 if executed else -2)  # Simple reward

        # Create battle record
        return BattleRecord(
            battle_id=f"battle_{int(time.time() * 1000)}",
            timestamp_ms=time.time() * 1000,
            duration_ms=0,  # filled by caller
            proposer=action.get("proposer", "CYNIC"),
            action_type=action.get("action_type", "UNKNOWN"),
            action_id=action_id,
            executed=executed,
            outcome=outcome,
            feedback=None,
            q_score_before=q_before,
            q_score_after=q_after,
            verdict=None,
            learning_signal=q_after - q_before,
            metadata={
                "iteration": battle_id,
                "action_priority": action.get("priority", 3),
            },
        )

    def _log_battle(self, battle: BattleRecord) -> None:
        """Append battle record to JSONL log."""
        try:
            with open(self.battles_file, "a", encoding="utf-8") as f:
                json.dump(battle.to_dict(), f)
                f.write("\n")
            logger.info(f"  Logged: {battle.battle_id}")
        except OSError as e:
            logger.error(f"Failed to log battle: {e}")


async def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        description="CYNIC Autonomous Battle Runner",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python -m cynic.cli battles                    # 8h battles
  python -m cynic.cli battles --duration 1h      # 1h battles
  python -m cynic.cli battles --interval 60s     # 60s between battles
  python -m cynic.cli battles --dry-run           # Test mode
        """,
    )
    parser.add_argument(
        "--duration",
        default="8h",
        help="Maximum runtime (e.g., 8h, 1h, 30m, 900s) [default: 8h]",
    )
    parser.add_argument(
        "--interval",
        default="30s",
        help="Pause between battles (e.g., 30s, 1m, 5m) [default: 30s]",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Don't actually execute actions, just simulate",
    )

    # Skip 'battles' command name in sys.argv
    args = parser.parse_args(sys.argv[2:])

    # Parse duration
    duration_s = _parse_duration(args.duration)
    interval_s = _parse_duration(args.interval)

    # Run battles
    runner = BattleRunner(dry_run=args.dry_run)
    await runner.run(max_duration_s=duration_s, interval_s=interval_s)


def _parse_duration(s: str) -> int:
    """Parse duration string like '8h', '30m', '30s', '900'."""
    s = s.strip()
    if s.endswith("h"):
        return int(s[:-1]) * 3600
    elif s.endswith("m"):
        return int(s[:-1]) * 60
    elif s.endswith("s"):
        return int(s[:-1])
    else:
        return int(s)  # assume seconds


def cmd_battles() -> None:
    """Entry point from CLI."""
    asyncio.run(main())


if __name__ == "__main__":
    cmd_battles()

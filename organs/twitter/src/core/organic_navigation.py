"""Organic navigation target selection rules."""

from __future__ import annotations

import math
import random
from pathlib import Path

from organs.contracts.src.surface_contracts import iter_json_objects


DEFAULT_TARGETS = [
    {"url": "https://x.com/home", "domain": "timeline", "weight": 1.0},
    {"url": "https://x.com/search?q=solana&f=live", "domain": "solana", "weight": 0.9},
    {"url": "https://x.com/search?q=crypto+rug+pull&f=live", "domain": "security", "weight": 0.8},
    {"url": "https://x.com/search?q=defi+exploit&f=live", "domain": "security", "weight": 0.7},
    {"url": "https://x.com/search?q=solana+meme+coin&f=live", "domain": "memecoin", "weight": 0.6},
    {"url": "https://x.com/search?q=jupiter+dex&f=live", "domain": "dex", "weight": 0.5},
    {"url": "https://x.com/explore", "domain": "trending", "weight": 0.4},
    {"url": "https://x.com/cynicoracle", "domain": "cynicoracle", "weight": 0.3},
]


class UCB1Selector:
    """UCB1 bandit for balancing explore/exploit across navigation targets."""

    def __init__(self, nav_log_path: Path):
        self.nav_log = nav_log_path
        self.visit_counts: dict[str, int] = {}
        self.reward_sums: dict[str, float] = {}
        self.total_visits = 0
        self._load_history()

    def _load_history(self) -> None:
        """Load navigation history to initialize bandit state."""
        if not self.nav_log.exists():
            return
        try:
            with self.nav_log.open(encoding="utf-8") as handle:
                for entry in iter_json_objects(handle):
                    domain = entry.get("domain", "unknown")
                    reward = entry.get("reward", 0.0)
                    self.visit_counts[domain] = self.visit_counts.get(domain, 0) + 1
                    self.reward_sums[domain] = self.reward_sums.get(domain, 0.0) + reward
                    self.total_visits += 1
        except IOError:
            pass

    def select(self, targets: list[dict]) -> dict:
        """Select next target using UCB1."""
        if self.total_visits == 0:
            return random.choice(targets)

        best_score = -1.0
        best_target = targets[0]
        for target in targets:
            domain = target.get("domain", "unknown")
            visits = self.visit_counts.get(domain, 0)
            if visits == 0:
                return target

            avg_reward = self.reward_sums.get(domain, 0.0) / visits
            exploration = math.sqrt(2 * math.log(self.total_visits) / visits)
            ucb_score = (avg_reward + exploration) * target.get("weight", 1.0)
            if ucb_score > best_score:
                best_score = ucb_score
                best_target = target

        return best_target

    def update(self, domain: str, reward: float) -> None:
        """Update bandit with observed reward for a domain."""
        self.visit_counts[domain] = self.visit_counts.get(domain, 0) + 1
        self.reward_sums[domain] = self.reward_sums.get(domain, 0.0) + reward
        self.total_visits += 1

"""
CYNIC PerceiveWorkers package — backward-compatible re-exports.

All imports from `cynic.perceive.workers` continue to work unchanged.
"""
from cynic.perceive.workers.base import PerceiveWorker
from cynic.perceive.workers.git import GitWatcher
from cynic.perceive.workers.health import HealthWatcher
from cynic.perceive.workers.self_watcher import SelfWatcher
from cynic.perceive.workers.market import MarketWatcher
from cynic.perceive.workers.social import SocialWatcher
from cynic.perceive.workers.solana import SolanaWatcher
from cynic.perceive.workers.disk import DiskWatcher
from cynic.perceive.workers.memory import MemoryWatcher

__all__ = [
    "PerceiveWorker",
    "GitWatcher",
    "HealthWatcher",
    "SelfWatcher",
    "MarketWatcher",
    "SocialWatcher",
    "SolanaWatcher",
    "DiskWatcher",
    "MemoryWatcher",
]

"""
CYNIC PerceiveWorkers package — backward-compatible re-exports.

All imports from `cynic.perceive.workers` continue to work unchanged.
"""
from cynic.senses.workers.base import PerceiveWorker
from cynic.senses.workers.git import GitWatcher
from cynic.senses.workers.health import HealthWatcher
from cynic.senses.workers.self_watcher import SelfWatcher
from cynic.senses.workers.market import MarketWatcher
from cynic.senses.workers.social import SocialWatcher
from cynic.senses.workers.solana import SolanaWatcher
from cynic.senses.workers.disk import DiskWatcher
from cynic.senses.workers.memory import MemoryWatcher

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

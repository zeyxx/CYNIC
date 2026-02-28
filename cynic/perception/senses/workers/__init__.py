"""
CYNIC PerceiveWorkers package — backward-compatible re-exports.

All imports from `cynic.perceive.workers` continue to work unchanged.
"""
from cynic.perception.senses.workers.base import PerceiveWorker
from cynic.perception.senses.workers.git import GitWatcher
from cynic.perception.senses.workers.health import HealthWatcher
from cynic.perception.senses.workers.self_watcher import SelfWatcher
from cynic.perception.senses.workers.market import MarketWatcher
from cynic.perception.senses.workers.social import SocialWatcher
from cynic.perception.senses.workers.solana import SolanaWatcher
from cynic.perception.senses.workers.disk import DiskWatcher
from cynic.perception.senses.workers.memory import MemoryWatcher

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

"""
CYNIC PerceiveWorkers package — backward-compatible re-exports.

All imports from `cynic.perceive.workers` continue to work unchanged.
"""
from cynic.kernel.organism.perception.senses.workers.base import PerceiveWorker
from cynic.kernel.organism.perception.senses.workers.disk import DiskWatcher
from cynic.kernel.organism.perception.senses.workers.git import GitWatcher
from cynic.kernel.organism.perception.senses.workers.health import HealthWatcher
from cynic.kernel.organism.perception.senses.workers.market import MarketWatcher
from cynic.kernel.organism.perception.senses.workers.memory import MemoryWatcher
from cynic.kernel.organism.perception.senses.workers.self_watcher import SelfWatcher
from cynic.kernel.organism.perception.senses.workers.social import SocialWatcher
from cynic.kernel.organism.perception.senses.workers.solana import SolanaWatcher

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

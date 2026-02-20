"""
CYNIC Real-Time Topology System

Enables the organism to be conscious of its own architecture in real-time,
detecting and applying code changes without restart.

Layers:
  L1: SourceWatcher — File system monitoring
  L2: IncrementalTopologyBuilder — Change discovery
  L3: HotReloadCoordinator — Safe application
  L4: TopologyMirror — Continuous awareness

Events flow:
  SOURCE_CHANGED → TOPOLOGY_CHANGED → TOPOLOGY_APPLIED → TOPOLOGY_SNAPSHOT
                                   → TOPOLOGY_ROLLBACK (on failure)
"""

from cynic.core.topology.file_watcher import SourceWatcher
from cynic.core.topology.topology_builder import IncrementalTopologyBuilder
from cynic.core.topology.hot_reload import HotReloadCoordinator
from cynic.core.topology.topology_mirror import TopologyMirror
from cynic.core.topology.change_tracker import ChangeTracker
from cynic.core.topology.payloads import (
    SourceChangedPayload,
    TopologyChangedPayload,
    TopologyAppliedPayload,
    TopologyRollbackPayload,
    TopologySnapshotPayload,
    TopologyDelta,
)

__all__ = [
    "SourceWatcher",
    "IncrementalTopologyBuilder",
    "HotReloadCoordinator",
    "TopologyMirror",
    "ChangeTracker",
    "SourceChangedPayload",
    "TopologyChangedPayload",
    "TopologyAppliedPayload",
    "TopologyRollbackPayload",
    "TopologySnapshotPayload",
    "TopologyDelta",
]

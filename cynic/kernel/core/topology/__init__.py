"""
CYNIC Real-Time Topology System

Enables the organism to be conscious of its own architecture in real-time,
detecting and applying code changes without restart.

Layers:
  L1: SourceWatcher " File system monitoring
  L2: IncrementalTopologyBuilder " Change discovery
  L3: HotReloadCoordinator " Safe application
  L4: TopologyMirror " Continuous awareness

Events flow:
  SOURCE_CHANGED ' TOPOLOGY_CHANGED ' TOPOLOGY_APPLIED ' TOPOLOGY_SNAPSHOT
                                   ' TOPOLOGY_ROLLBACK (on failure)
"""

from cynic.kernel.core.topology.change_analyzer import ChangeAnalyzer
from cynic.kernel.core.topology.change_tracker import ChangeTracker
from cynic.kernel.core.topology.file_watcher import SourceWatcher
from cynic.kernel.core.topology.hot_reload import HotReloadCoordinator
from cynic.kernel.core.topology.payloads import (
    SourceChangedPayload,
    TopologyAppliedPayload,
    TopologyChangedPayload,
    TopologyDelta,
    TopologyRollbackPayload,
    TopologySnapshotPayload,
)
from cynic.kernel.core.topology.topology_builder import IncrementalTopologyBuilder
from cynic.kernel.core.topology.topology_mirror import TopologyMirror

__all__ = [
    "SourceWatcher",
    "IncrementalTopologyBuilder",
    "HotReloadCoordinator",
    "TopologyMirror",
    "ChangeTracker",
    "ChangeAnalyzer",
    "SourceChangedPayload",
    "TopologyChangedPayload",
    "TopologyAppliedPayload",
    "TopologyRollbackPayload",
    "TopologySnapshotPayload",
    "TopologyDelta",
]

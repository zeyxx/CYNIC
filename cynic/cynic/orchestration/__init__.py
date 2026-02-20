"""
CYNIC Self-Orchestration Layer

The organism manages itself: builds, deploys, monitors.
No external tools (n8n, Jenkins, etc.) — CYNIC is the orchestrator.

Three subsystems:
  1. docker — Build, deploy, health checks
  2. versioning — SemVer, releases, migrations
  3. monitor — Continuous health, alerts, metrics
"""
from .docker import DockerManager
from .versioning import VersionManager
from .monitor import HealthMonitor

__all__ = ["DockerManager", "VersionManager", "HealthMonitor"]

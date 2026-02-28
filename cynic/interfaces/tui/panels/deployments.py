"""
TUI Panel — Active Deployments

Shows real-time deployment status, logs, and results.
"""
from __future__ import annotations

from datetime import datetime
from typing import Optional

from rich.panel import Panel
from rich.table import Table
from rich.progress import Progress
from textual.widgets import Static

from cynic.orchestration.docker import BuildResult, DeployResult


class DeploymentsPanel(Static):
    """Shows active deployments and their status."""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.deployments: list[dict] = []

    def render(self) -> Panel:
        """Render deployment status."""
        content = self._render_deployment_table()
        return Panel(content, title="Active Deployments", border_style="cyan")

    def _render_deployment_table(self) -> str:
        """Create table of deployment status."""
        if not self.deployments:
            return "No active deployments.\nRun 'build' or 'deploy' to start."

        table = Table(title="Deployments")
        table.add_column("ID", style="cyan")
        table.add_column("Service", style="green")
        table.add_column("Status", style="yellow")
        table.add_column("Progress", style="magenta")
        table.add_column("Duration", style="blue")

        for dep in self.deployments:
            table.add_row(
                dep["id"],
                dep["service"],
                dep["status"],
                dep["progress"],
                dep["duration"],
            )

        return str(table)

    def add_deployment(self, service: str, deployment_type: str = "deploy") -> str:
        """
        Add a deployment to track.

        Returns:
            Deployment ID
        """
        dep_id = f"dep_{len(self.deployments):03d}"
        self.deployments.append({
            "id": dep_id,
            "service": service,
            "type": deployment_type,
            "status": "running",
            "progress": "0%",
            "duration": "0s",
            "started": datetime.now(),
        })
        return dep_id

    def update_deployment(self, dep_id: str, status: str, progress: int = 0) -> None:
        """Update deployment status."""
        for dep in self.deployments:
            if dep["id"] == dep_id:
                dep["status"] = status
                if progress > 0:
                    dep["progress"] = f"{progress}%"
                elapsed = (datetime.now() - dep["started"]).total_seconds()
                dep["duration"] = f"{elapsed:.0f}s"
                break

    def complete_deployment(self, dep_id: str, success: bool) -> None:
        """Mark deployment as complete."""
        for dep in self.deployments:
            if dep["id"] == dep_id:
                dep["status"] = "✓ Success" if success else "✗ Failed"
                elapsed = (datetime.now() - dep["started"]).total_seconds()
                dep["duration"] = f"{elapsed:.0f}s"
                break

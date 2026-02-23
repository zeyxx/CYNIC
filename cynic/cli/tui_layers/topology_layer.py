"""Layer 3: Architecture consciousness (L0 system)."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class TopologyLayer(Static):
    """Display architecture consciousness (source changes, convergence)."""

    source_changes = reactive(0, recompose=True)
    topology_deltas = reactive(0, recompose=True)
    convergence_verified = reactive(0, recompose=True)

    def render(self) -> RenderableType:
        """Render topology consciousness."""
        table = Table(title="CONVERGENCE STATE", show_header=True, padding=(0, 1))
        table.add_column("Metric", style="magenta")
        table.add_column("Value", justify="right", style="green")

        table.add_row("Source Changes", str(self.source_changes))
        table.add_row("Topology Deltas", str(self.topology_deltas))
        table.add_row("Verified", str(self.convergence_verified))

        return Panel(
            table,
            title="🏗️ TOPOLOGY CONSCIOUSNESS",
            border_style="magenta",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from topology snapshot."""
        topo = snapshot.get("topology_consciousness", {})
        self.source_changes = topo.get("source_changes_detected", 0)
        self.topology_deltas = topo.get("topology_deltas_computed", 0)
        conv = topo.get("convergence_validations", {})
        self.convergence_verified = conv.get("verified", 0)

"""Layer 1: Cross-bus ecosystem topology."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class EcosystemLayer(Static):
    """Display cross-bus event topology (CORE, AUTOMATION, AGENT)."""

    core_count = reactive(0, recompose=True)
    automation_count = reactive(0, recompose=True)
    agent_count = reactive(0, recompose=True)

    def render(self) -> RenderableType:
        """Render ecosystem topology."""
        table = Table(title="3 EVENT BUSES", show_header=True, padding=(0, 1))
        table.add_column("Bus", style="cyan")
        table.add_column("Events", justify="right", style="green")
        table.add_column("Status")

        table.add_row("CORE", str(self.core_count), "🟢")
        table.add_row("AUTOMATION", str(self.automation_count), "🟢")
        table.add_row("AGENT", str(self.agent_count), "🟢")

        return Panel(
            table,
            title="🔗 ECOSYSTEM TOPOLOGY",
            border_style="cyan",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from ecosystem snapshot."""
        self.core_count = len(snapshot.get("core_events", []))
        self.automation_count = len(snapshot.get("automation_events", []))
        self.agent_count = len(snapshot.get("agent_events", []))

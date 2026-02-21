"""Layer 6: Self-awareness (kernel mirror)."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class SelfAwareLayer(Static):
    """Display organism's self-awareness (kernel_mirror insights)."""

    observation_count = reactive(0, recompose=True)
    insight_count = reactive(0, recompose=True)
    proposal_count = reactive(0, recompose=True)

    def render(self) -> RenderableType:
        """Render self-awareness state."""
        table = Table(title="META-COGNITION", show_header=True, padding=(0, 1))
        table.add_column("Category", style="white")
        table.add_column("Count", justify="right", style="green")

        table.add_row("Observations 👀", str(self.observation_count))
        table.add_row("Insights 💡", str(self.insight_count))
        table.add_row("Proposals 🔧", str(self.proposal_count))

        return Panel(
            table,
            title="🪞 SELF-AWARENESS (Kernel Mirror)",
            border_style="white",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from self-awareness snapshot."""
        awareness = snapshot.get("self_awareness", {})
        self.observation_count = len(awareness.get("kernel_observations", []))
        self.insight_count = len(awareness.get("meta_insights", []))
        self.proposal_count = len(awareness.get("improvement_proposals", []))

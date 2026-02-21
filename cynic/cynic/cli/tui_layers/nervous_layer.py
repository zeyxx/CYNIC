"""Layer 4: Nervous system audit trail."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class NervousLayer(Static):
    """Display nervous system audit trail (events, decisions, loops)."""

    event_count = reactive(0, recompose=True)
    decision_count = reactive(0, recompose=True)
    loop_integrity = reactive(0, recompose=True)

    def render(self) -> RenderableType:
        """Render nervous system metrics."""
        table = Table(title="AUDIT TRAIL", show_header=True, padding=(0, 1))
        table.add_column("Component", style="blue")
        table.add_column("Count", justify="right", style="green")

        table.add_row("Events", str(self.event_count))
        table.add_row("Decisions", str(self.decision_count))
        table.add_row("Loop Checks", str(self.loop_integrity))

        return Panel(
            table,
            title="🧠 NERVOUS SYSTEM AUDIT",
            border_style="blue",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from nervous system snapshot."""
        audit = snapshot.get("nervous_system_audit", {})
        self.event_count = audit.get("event_count", 0)
        self.decision_count = audit.get("decision_count", 0)
        self.loop_integrity = len(audit.get("loop_integrity_checks", []))

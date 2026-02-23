"""Layer 5: Guardrail decisions."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class GuardrailsLayer(Static):
    """Display guardrail decisions (blocks, allows, approvals)."""

    allow_count = reactive(0, recompose=True)
    block_count = reactive(0, recompose=True)
    approval_count = reactive(0, recompose=True)

    def render(self) -> RenderableType:
        """Render guardrail decisions."""
        table = Table(title="GUARDIAN VERDICTS", show_header=True, padding=(0, 1))
        table.add_column("Decision", style="red")
        table.add_column("Count", justify="right", style="green")

        table.add_row("Allow ✅", str(self.allow_count))
        table.add_row("Block 🔴", str(self.block_count))
        table.add_row("Approval ⏳", str(self.approval_count))

        return Panel(
            table,
            title="🛡️ GUARDRAILS (Immune System)",
            border_style="red",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from guardrails snapshot."""
        decisions = snapshot.get("guardrail_decisions", [])
        self.allow_count = len([d for d in decisions if d.get("decision") == "allow"])
        self.block_count = len([d for d in decisions if d.get("decision") == "block"])
        approval = len([d for d in decisions if d.get("decision") == "require_approval"])
        self.approval_count = approval

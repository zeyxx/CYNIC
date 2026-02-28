"""Layer 2: Perceive worker activity."""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.reactive import reactive
from textual.widgets import Static
from rich.console import RenderableType


class PerceptionLayer(Static):
    """Display perceive worker status (what perceive workers have observed)."""

    git_status = reactive("idle", recompose=True)
    health_status = reactive("idle", recompose=True)
    market_status = reactive("idle", recompose=True)
    solana_status = reactive("idle", recompose=True)
    social_status = reactive("idle", recompose=True)

    def render(self) -> RenderableType:
        """Render perception sources."""
        table = Table(title="PERCEIVE WORKERS", show_header=True, padding=(0, 1))
        table.add_column("Worker", style="yellow")
        table.add_column("Status", style="green")

        table.add_row("GitWatcher", self.git_status)
        table.add_row("HealthWatcher", self.health_status)
        table.add_row("MarketWatcher", self.market_status)
        table.add_row("SolanaWatcher", self.solana_status)
        table.add_row("SocialWatcher", self.social_status)

        return Panel(
            table,
            title="👁️ PERCEPTION SOURCES",
            border_style="yellow",
            expand=False
        )

    def update_from_snapshot(self, snapshot: dict) -> None:
        """Update from perception snapshot."""
        sources = snapshot.get("perception_sources", {})
        self.git_status = sources.get("git", {}).get("status", "idle")
        self.health_status = sources.get("health", {}).get("status", "idle")
        self.market_status = sources.get("market", {}).get("status", "idle")
        self.solana_status = sources.get("solana", {}).get("status", "idle")
        self.social_status = sources.get("social", {}).get("status", "idle")

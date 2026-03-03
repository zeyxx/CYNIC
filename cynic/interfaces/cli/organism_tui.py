"""
CYNIC Organism TUI " Real-Time Visual Cortex.

High-fidelity terminal interface using the unified OrganismState and EventJournal.
"""

from __future__ import annotations

import asyncio
import logging
import time
from datetime import datetime
from typing import Any

from rich import box
from rich.console import Console, Group
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.progress import BarColumn, Progress, TextColumn
from rich.table import Table
from rich.text import Text

logger = logging.getLogger("cynic.interfaces.cli.tui")


class OrganismTUI:
    """Live Terminal UI for the unified CYNIC Organism."""

    def __init__(self, organism: Any) -> None:
        self.organism = organism
        self.console = Console()
        self._start_time = time.time()
        self._running = False
        self._event_buffer = []

    def create_layout(self) -> Layout:
        layout = Layout()
        layout.split_column(
            Layout(name="header", size=3),
            Layout(name="main", ratio=1),
            Layout(name="footer", size=3),
        )
        layout["main"].split_row(
            Layout(name="body", ratio=1),
            Layout(name="mind", ratio=2),
        )
        layout["mind"].split_column(
            Layout(name="axioms", ratio=1),
            Layout(name="dogs", ratio=1),
            Layout(name="stream", ratio=1),
        )
        return layout

    def get_header(self) -> Panel:
        uptime = str(
            datetime.fromtimestamp(time.time())
            - datetime.fromtimestamp(self._start_time)
        ).split(".")[0]
        grid = Table.grid(expand=True)
        grid.add_column(justify="left", ratio=1)
        grid.add_column(justify="center", ratio=1)
        grid.add_column(justify="right", ratio=1)

        grid.add_row(
            Text.from_markup("  | [bold blue]REALITY CONNECTED[/] "),
            Text("UNIFIED ORGANISM V5.0", style="bold white"),
            Text(f"UPTIME: {uptime} ", style="bold green"),
        )
        return Panel(grid, style="white on blue")

    async def get_body_panel(self) -> Panel:
        """Somatic Body metrics."""
        progress = Progress(
            TextColumn("{task.description}"),
            BarColumn(bar_width=None),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            expand=True,
        )

        try:
            stats = await self.organism.state.get_stats()
            cpu = stats.get("machine_cpu", 0.0)
            ram = stats.get("machine_ram", 0.0)

            progress.add_task("[cyan]CPU", completed=cpu)
            progress.add_task("[magenta]RAM", completed=ram)

            content = Group(
                progress,
                Text(""),
                Text(f"TOTAL CYCLES: {stats['cycles']['total']}", style="bold yellow"),
                Text(f"REFLEX: {stats['cycles']['reflex']}"),
                Text(f"MACRO: {stats['cycles']['macro']}"),
                Text(
                    f"CONSCIOUSNESS: {stats['consciousness_level']}", style="bold green"
                ),
            )
        except Exception:
            content = Text("Waiting for pulse...")

        return Panel(content, title="[bold]SOMATIC BODY[/bold]", border_style="cyan")

    def get_axiom_panel(self) -> Panel:
        """Moral Hypercube metrics."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("AXIOM")
        table.add_column("SCORE", justify="right")
        table.add_column("STATUS")

        # In a real cycle, these come from AxiomMonitor
        axioms = [
            ("FIDELITY", 61.8),
            ("PHI", 61.8),
            ("VERIFY", 100.0),
            ("CULTURE", 38.2),
            ("BURN", 61.8),
        ]
        for name, score in axioms:
            status = "WAG" if score >= 61.8 else "GROWL"
            color = "green" if status == "WAG" else "yellow"
            table.add_row(name, f"{score:.1f}", f"[{color}]{status}[/]")

        return Panel(
            table, title="[bold]MORAL HYPERCUBE[/bold]", border_style="magenta"
        )

    def get_dogs_panel(self) -> Panel:
        """The 11 Dogs status."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("DOG")
        table.add_column("SEFIROT")
        table.add_column("SKILL")
        table.add_column("Q", justify="right")

        try:
            dogs = self.organism.cognition.orchestrator.dogs
            for dog_id, dog in list(dogs.items())[:6]:  # Show first 6
                table.add_row(
                    dog_id,
                    dog.soul.sefirot,
                    dog.soul.expertise_plugin or "None",
                    "0.50",
                )
        except Exception:
            table.add_row("Loading Dogs...", "", "", "")

        return Panel(
            table, title="[bold]SEFIROTIC EXPERTISE[/bold]", border_style="green"
        )

    def get_stream_panel(self) -> Panel:
        """Nervous Pulse (Event Journal)."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("TIME", style="dim")
        table.add_column("EVENT")
        table.add_column("SOURCE")

        # Read from real internal event buffer
        events = list(self._event_buffer[-8:])
        for ev in reversed(events):
            table.add_row(ev["time"], ev["topic"], ev["source"])

        return Panel(table, title="[bold]NERVOUS PULSE[/bold]", border_style="yellow")

    async def render(self) -> Layout:
        layout = self.create_layout()
        try:
            layout["header"].update(self.get_header())
            layout["body"].update(await self.get_body_panel())
            layout["axioms"].update(self.get_axiom_panel())
            layout["dogs"].update(self.get_dogs_panel())
            layout["stream"].update(self.get_stream_panel())
            layout["footer"].update(
                Panel(Text("Press Ctrl+C to detach consciousness", justify="center"))
            )
        except Exception as e:
            layout["main"].update(Panel(f"[bold red]Render Error:[/] {e}"))
        return layout

    async def run(self) -> None:
        self._running = True

        # Subscribe to ALL events to populate the Nervous Pulse panel
        bus = get_core_bus("DEFAULT")

        async def _on_any_event(event):
            try:
                t = datetime.now().strftime("%H:%M:%S")
                # Format topic for better visibility
                topic = event.topic.replace("core.", "")
                source = event.source[:15]
                self._event_buffer.append({"time": t, "topic": topic, "source": source})
                if len(self._event_buffer) > 50:
                    self._event_buffer.pop(0)
            except Exception as _e:
                logger.debug(f"Silenced: {_e}")

        bus.on("*", _on_any_event)

        try:
            with Live(await self.render(), refresh_per_second=4, screen=True) as live:
                while self._running:
                    live.update(await self.render())
                    await asyncio.sleep(0.25)
        except asyncio.CancelledError:
            pass
        finally:
            self._running = False
            bus.off("*", _on_any_event)

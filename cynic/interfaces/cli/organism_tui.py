"""
CYNIC Organism TUI — Real-Time Visual Cortex.

High-fidelity terminal interface using the unified OrganismState.
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

from cynic.kernel.core.phi import MAX_Q_SCORE

logger = logging.getLogger("cynic.interfaces.cli.tui")

class OrganismTUI:
    """Live Terminal UI for the unified CYNIC Organism."""

    def __init__(self, organism: Any) -> None:
        self.organism = organism
        self.console = Console()
        self._start_time = time.time()
        self._running = False

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
        uptime = str(datetime.fromtimestamp(time.time()) - datetime.fromtimestamp(self._start_time)).split(".")[0]
        grid = Table.grid(expand=True)
        grid.add_column(justify="left", ratio=1)
        grid.add_column(justify="center", ratio=1)
        grid.add_column(justify="right", ratio=1)
        
        grid.add_row(
            Text.from_markup(" κυνικός | [bold blue]REALITY CONNECTED[/] "),
            Text("UNIFIED ORGANISM V5.0", style="bold white"),
            Text(f"UPTIME: {uptime} ", style="bold green"),
        )
        return Panel(grid, style="white on blue")

    def get_body_panel(self) -> Panel:
        """Somatic Body metrics."""
        progress = Progress(
            TextColumn("{task.description}"),
            BarColumn(bar_width=None),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            expand=True
        )
        
        try:
            stats = self.organism.state.get_stats()
            # In a real environment, HardwareBody updates these
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
                Text(f"CONSCIOUSNESS: {stats['consciousness_level']}", style="bold green")
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
        
        # Simulated/Fetched Axioms
        axioms = [("FIDELITY", 61.8), ("PHI", 61.8), ("VERIFY", 100.0), ("CULTURE", 38.2), ("BURN", 61.8)]
        for name, score in axioms:
            status = "WAG" if score >= 61.8 else "GROWL"
            color = "green" if status == "WAG" else "yellow"
            table.add_row(name, f"{score:.1f}", f"[{color}]{status}[/]")

        return Panel(table, title="[bold]MORAL HYPERCUBE[/bold]", border_style="magenta")

    def get_dogs_panel(self) -> Panel:
        """The 11 Dogs status."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("DOG")
        table.add_column("SEFIROT")
        table.add_column("SKILL")
        
        try:
            dogs = self.organism.cognition.orchestrator.dogs
            for dog_id, dog in list(dogs.items())[:6]: # Show first 6 for space
                table.add_row(dog_id, dog.soul.sefirot, dog.soul.expertise_plugin or "None")
        except Exception:
            table.add_row("Loading Dogs...", "", "")

        return Panel(table, title="[bold]SEFIROTIC EXPERTISE[/bold]", border_style="green")

    def get_stream_panel(self) -> Panel:
        """Recent Judgments."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("ID", style="dim")
        table.add_column("REALITY")
        table.add_column("VERDICT")
        table.add_column("Q-SCORE", justify="right")

        try:
            recent = self.organism.state.get_recent_judgments(limit=5)
            for j in recent:
                color = "green" if j.verdict in ("HOWL", "WAG") else "red"
                table.add_row(j.judgment_id[:8], j.reality, f"[{color}]{j.verdict}[/]", f"{j.q_score:.1f}")
        except Exception:
            pass

        return Panel(table, title="[bold]CONSCIOUSNESS STREAM[/bold]", border_style="yellow")

    def render(self) -> Layout:
        layout = self.create_layout()
        layout["header"].update(self.get_header())
        layout["body"].update(self.get_body_panel())
        layout["axioms"].update(self.get_axiom_panel())
        layout["dogs"].update(self.get_dogs_panel())
        layout["stream"].update(self.get_stream_panel())
        layout["footer"].update(Panel(Text("CYNIC: Reality is a choice. Efficiency is a law. PHI is the scale.", justify="center")))
        return layout

    async def run(self) -> None:
        self._running = True
        try:
            with Live(self.render(), refresh_per_second=2, screen=True) as live:
                while self._running:
                    live.update(self.render())
                    await asyncio.sleep(0.5)
        except asyncio.CancelledError:
            pass
        finally:
            self._running = False

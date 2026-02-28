"""
Organism TUI — The 'Skin' of CYNIC.
Visual Cortex for monitoring the embodied organism in real-time.

Features:
- Somatic Dashboard (Hardware health)
- Axiom Hypercube (Moral health)
- Consciousness Feed (Recent events)
"""

from __future__ import annotations

import asyncio
import time
from datetime import datetime
from typing import Any, Dict, List, Optional

from rich.console import Console, Group
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.progress import BarColumn, Progress, TextColumn
from rich.table import Table
from rich.text import Text
from rich import box

from cynic.kernel.core.phi import PHI_INV, PHI

class OrganismTUI:
    """Live Terminal UI for CYNIC Embodiment."""

    def __init__(self, organism: Any) -> None:
        self.organism = organism
        self.console = Console()
        self._start_time = time.time()
        self._recent_events: List[Dict[str, Any]] = []
        self._max_events = 10

    def create_layout(self) -> Layout:
        """Define the UI grid."""
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
            Layout(name="stream", ratio=1),
        )
        return layout

    def get_header(self) -> Panel:
        """Create the top status bar."""
        uptime = str(datetime.fromtimestamp(time.time()) - datetime.fromtimestamp(self._start_time)).split(".")[0]
        grid = Table.grid(expand=True)
        grid.add_column(justify="left", ratio=1)
        grid.add_column(justify="center", ratio=1)
        grid.add_column(justify="right", ratio=1)
        
        grid.add_row(
            Text(" κυνικός | CYNIC ORGANISM ", style="bold magenta"),
            Text(f"V4.0 INFINITE & PERFECT", style="bold white"),
            Text(f"UPTIME: {uptime} ", style="bold green"),
        )
        return Panel(grid, style="white on blue")

    def get_body_panel(self) -> Panel:
        """Display hardware metrics (Somatic Sensation)."""
        # Try to get state from organism body
        state = None
        cost = 1.0
        
        if self.organism and hasattr(self.organism, "metabolism"):
            body = getattr(self.organism.metabolism, "body", None)
            state = getattr(body, "_last_state", None)
            if body:
                cost = body.get_metabolic_cost()
        
        progress = Progress(
            TextColumn("{task.description}"),
            BarColumn(bar_width=None),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            expand=True
        )
        
        if state:
            progress.add_task("[cyan]CPU", completed=state.cpu_percent)
            progress.add_task("[magenta]RAM", completed=state.ram_percent)
            progress.add_task("[yellow]DSK", completed=state.disk_usage)
            
            status = "HEALTHY" if state.cpu_percent < 70 else "STRESSED"
            color = "green" if status == "HEALTHY" else "red"
            
            temp = f"{state.cpu_temp:.1f}°C" if state.cpu_temp else "N/A"
            charging = "⚡" if state.is_charging else "🔋"
        else:
            progress.add_task("[white]Waiting for pulse...", completed=0)
            status, color, temp, charging = "SLEEPING", "white", "N/A", "?"

        content = Group(
            progress,
            Text(""),
            Text(f"STATUS: {status}", style=f"bold {color}"),
            Text(f"TEMP: {temp}"),
            Text(f"POWER: {charging}"),
            Text(f"METABOLIC COST: {cost:.3f}x")
        )
        
        return Panel(content, title="[bold]SOMATIC BODY[/bold]", border_style="cyan")

    def get_axiom_panel(self) -> Panel:
        """Display Core Axiom scores."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("AXIOM", style="bold")
        table.add_column("SCORE", justify="right")
        table.add_column("STATUS")

        # In a real scenario, fetch from organism state
        # For now, base Phi-aligned defaults
        axioms = {
            "FIDELITY": 61.8,
            "PHI": 61.8,
            "VERIFY": 61.8,
            "CULTURE": 61.8,
            "BURN": 61.8
        }
        
        if self.organism and hasattr(self.organism, "state"):
            # Future: pull real scores from self.organism.state.get_stats()
            pass

        for ax, score in axioms.items():
            status = "WAG" if score >= 61.8 else "BARK"
            color = "green" if status == "WAG" else "red"
            table.add_row(ax, f"{score:.1f}", f"[{color}]{status}[/]")

        return Panel(table, title="[bold]MORAL HYPERCUBE[/bold]", border_style="magenta")

    def get_stream_panel(self) -> Panel:
        """Display last events."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("TIME", style="dim")
        table.add_column("TOPIC")
        table.add_column("EVENT")

        # Fetch from internal buffer
        for ev in reversed(self._recent_events[-10:]):
            table.add_row(ev["time"], ev["topic"], ev["msg"])

        return Panel(table, title="[bold]CONSCIOUSNESS STREAM[/bold]", border_style="yellow")

    async def run(self) -> None:
        """Launch the live display."""
        layout = self.create_layout()
        
        # Subscribe to bus to capture events for the stream
        from cynic.kernel.core.event_bus import get_core_bus, CoreEvent
        bus = get_core_bus()
        
        async def _capture(event):
            t = datetime.now().strftime("%H:%M:%S")
            msg = str(event.payload)[:50] + "..." if event.payload else ""
            self._recent_events.append({"time": t, "topic": event.topic, "msg": msg})
            if len(self._recent_events) > self._max_events:
                self._recent_events.pop(0)

        bus.on("*", _capture)

        with Live(layout, refresh_per_second=4, screen=True) as live:
            while True:
                layout["header"].update(self.get_header())
                layout["body"].update(self.get_body_panel())
                layout["axioms"].update(self.get_axiom_panel())
                layout["stream"].update(self.get_stream_panel())
                layout["footer"].update(Panel(Text("Press Ctrl+C to detach consciousness", justify="center")))
                await asyncio.sleep(0.25)

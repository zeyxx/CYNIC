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
        
        # Connection status
        from cynic.kernel.observability.symbiotic_state_manager import _INSTANCE
        conx = "[bold red]OFFLINE[/]"
        if _INSTANCE:
            if _INSTANCE.remote_mode:
                conx = f"[bold green]κ-NET {getattr(_INSTANCE.knet, 'status', 'CONNECTED')}[/]"
            elif _INSTANCE._organism:
                conx = "[bold blue]LOCAL[/]"

        grid = Table.grid(expand=True)
        grid.add_column(justify="left", ratio=1)
        grid.add_column(justify="center", ratio=1)
        grid.add_column(justify="right", ratio=1)
        
        grid.add_row(
            Text.from_markup(f" κυνικός | {conx} "),
            Text(f"V4.0 INFINITE & PERFECT", style="bold white"),
            Text(f"UPTIME: {uptime} ", style="bold green"),
        )
        return Panel(grid, style="white on blue")

    def get_body_panel(self) -> Panel:
        """Display hardware metrics (Somatic Sensation)."""
        # 1. Try to get state from live organism object (Local)
        state_data = None
        cost = 1.0
        
        if self.organism and hasattr(self.organism, "metabolism"):
            body = getattr(self.organism.metabolism, "body", None)
            local_state = getattr(body, "_last_state", None)
            if local_state:
                state_data = {
                    "cpu": local_state.cpu_percent,
                    "ram": local_state.ram_percent,
                    "disk": local_state.disk_usage,
                    "temp": local_state.cpu_temp,
                    "charging": local_state.is_charging,
                    "status": "HEALTHY" if local_state.cpu_percent < 70 else "STRESSED"
                }
                cost = body.get_metabolic_cost()
        
        # 2. Fallback: Fetch from SymbioticStateManager (supports Remote/Docker)
        if not state_data:
            from cynic.kernel.observability.symbiotic_state_manager import _INSTANCE
            if _INSTANCE and _INSTANCE._last_snapshot:
                snap = _INSTANCE._last_snapshot
                state_data = {
                    "cpu": snap.machine_resources.get("cpu", 0.0),
                    "ram": snap.machine_resources.get("ram", 0.0),
                    "disk": snap.machine_resources.get("disk", 0.0),
                    "temp": None,
                    "charging": True,
                    "status": "REMOTE" if _INSTANCE.remote_mode else "SLEEPING"
                }

        progress = Progress(
            TextColumn("{task.description}"),
            BarColumn(bar_width=None),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            expand=True
        )
        
        if state_data:
            progress.add_task("[cyan]CPU", completed=state_data["cpu"])
            progress.add_task("[magenta]RAM", completed=state_data["ram"])
            progress.add_task("[yellow]DSK", completed=state_data["disk"])
            
            status = state_data["status"]
            color = "green" if status in ["HEALTHY", "REMOTE"] else "red"
            temp = f"{state_data['temp']:.1f}°C" if state_data['temp'] else "N/A"
            charging = "⚡" if state_data["charging"] else "🔋"
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
        """Display real Core Axiom scores from snapshot."""
        table = Table(box=box.SIMPLE, expand=True)
        table.add_column("AXIOM", style="bold")
        table.add_column("SCORE", justify="right")
        table.add_column("STATUS")

        # 1. Base axioms
        axioms = ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]
        scores = {ax: 61.8 for ax in axioms} # Default fallback
        
        # 2. Try to fetch real scores from snapshot
        from cynic.kernel.observability.symbiotic_state_manager import _INSTANCE
        if _INSTANCE and _INSTANCE._last_pulse_data:
            mind = _INSTANCE._last_pulse_data.get("mind", {})
            real_scores = mind.get("axiom_scores", {})
            if real_scores:
                for ax in axioms:
                    if ax in real_scores:
                        scores[ax] = real_scores[ax]

        for ax in axioms:
            score = scores[ax]
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
                # Refresh global state snapshot (triggers remote detection)
                from cynic.kernel.observability.symbiotic_state_manager import get_current_state
                await get_current_state()
                
                layout["header"].update(self.get_header())
                layout["body"].update(self.get_body_panel())
                layout["axioms"].update(self.get_axiom_panel())
                layout["stream"].update(self.get_stream_panel())
                layout["footer"].update(Panel(Text("Press Ctrl+C to detach consciousness", justify="center")))
                await asyncio.sleep(0.25)

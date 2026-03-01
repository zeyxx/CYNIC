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
    """Live Terminal UI for CYNIC Embodiment."""

    def __init__(self, organism: Any) -> None:
        self.organism = organism
        self.console = Console()
        self._start_time = time.time()
        self._recent_events: list[dict[str, Any]] = []
        self._max_events = 10
        self._running = False

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
        try:
            if _INSTANCE:
                if _INSTANCE.remote_mode:
                    conx = f"[bold green]κ-NET {getattr(_INSTANCE.knet, 'status', 'CONNECTED')}[/]"
                elif _INSTANCE._organism:
                    conx = "[bold blue]LOCAL[/]"
        except Exception:
            conx = "[bold yellow]UNKNOWN[/]"

        grid = Table.grid(expand=True)
        grid.add_column(justify="left", ratio=1)
        grid.add_column(justify="center", ratio=1)
        grid.add_column(justify="right", ratio=1)
        
        grid.add_row(
            Text.from_markup(f" κυνικός | {conx} "),
            Text("V4.0 INFINITE & PERFECT", style="bold white"),
            Text(f"UPTIME: {uptime} ", style="bold green"),
        )
        return Panel(grid, style="white on blue")

    def get_body_panel(self) -> Panel:
        """Display hardware metrics (Somatic Sensation)."""
        # 1. Try to get state from live organism object (Local)
        state_data = None
        cost = 1.0
        
        try:
            if self.organism and hasattr(self.organism, "metabolism"):
                body = getattr(self.organism.metabolism, "body", None)
                local_state = getattr(body, "_last_state", None)
                if local_state:
                    state_data = {
                        "cpu": getattr(local_state, "cpu_percent", 0.0),
                        "ram": getattr(local_state, "ram_percent", 0.0),
                        "disk": getattr(local_state, "disk_usage", 0.0),
                        "temp": getattr(local_state, "cpu_temp", None),
                        "charging": getattr(local_state, "is_charging", True),
                        "status": "HEALTHY" if getattr(local_state, "cpu_percent", 0.0) < 70 else "STRESSED"
                    }
                    cost = body.get_metabolic_cost()
        except Exception as e:
            logger.debug(f"TUI Local Body fetch error: {e}")
        
        # 2. Fallback: Fetch from SymbioticStateManager (supports Remote/Docker)
        if not state_data:
            try:
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
            except Exception:
                pass

        progress = Progress(
            TextColumn("{task.description}"),
            BarColumn(bar_width=None),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            expand=True
        )
        
        if state_data:
            progress.add_task("[cyan]CPU", completed=state_data.get("cpu", 0))
            progress.add_task("[magenta]RAM", completed=state_data.get("ram", 0))
            progress.add_task("[yellow]DSK", completed=state_data.get("disk", 0))
            
            status = state_data.get("status", "UNKNOWN")
            color = "green" if status in ["HEALTHY", "REMOTE"] else "red"
            temp_val = state_data.get('temp')
            temp = f"{temp_val:.1f}°C" if temp_val else "N/A"
            charging = "⚡" if state_data.get("charging") else "🔋"
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
        try:
            from cynic.kernel.observability.symbiotic_state_manager import _INSTANCE
            if _INSTANCE and _INSTANCE._last_pulse_data:
                mind = _INSTANCE._last_pulse_data.get("mind", {})
                real_scores = mind.get("axiom_scores", {})
                if real_scores:
                    for ax in axioms:
                        if ax in real_scores:
                            scores[ax] = real_scores[ax]
        except Exception:
            pass

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

        # Fetch from internal buffer (thread-safe copy)
        events = list(self._recent_events[-10:])
        for ev in reversed(events):
            table.add_row(ev["time"], ev["topic"], ev["msg"])

        return Panel(table, title="[bold]CONSCIOUSNESS STREAM[/bold]", border_style="yellow")

    def render(self) -> Layout:
        """Produce a single renderable snapshot of the TUI."""
        layout = self.create_layout()
        try:
            layout["header"].update(self.get_header())
            layout["body"].update(self.get_body_panel())
            layout["axioms"].update(self.get_axiom_panel())
            layout["stream"].update(self.get_stream_panel())
            layout["footer"].update(Panel(Text("Press Ctrl+C to detach consciousness", justify="center")))
        except Exception as e:
            # Emergency render if components fail
            layout["main"].update(Panel(f"[bold red]Render Error:[/] {e}"))
        return layout

    async def run(self) -> None:
        """Launch the live display."""
        if self._running:
            return
        self._running = True
        
        # Subscribe to bus to capture events for the stream
        from cynic.kernel.core.event_bus import get_core_bus
        bus = get_core_bus()
        
        async def _capture(event):
            try:
                t = datetime.now().strftime("%H:%M:%S")
                payload_str = str(event.payload)
                msg = payload_str[:50] + "..." if len(payload_str) > 50 else payload_str
                self._recent_events.append({"time": t, "topic": event.topic, "msg": msg})
                if len(self._recent_events) > self._max_events:
                    self._recent_events.pop(0)
            except Exception:
                pass

        bus.on("*", _capture)

        try:
            with Live(self.render(), refresh_per_second=4, screen=True) as live:
                while self._running:
                    try:
                        # Refresh global state snapshot (triggers remote detection)
                        from cynic.kernel.observability.symbiotic_state_manager import (
                            get_current_state,
                        )
                        await get_current_state()
                        
                        # Update the live display with a fresh render
                        live.update(self.render())
                        
                    except asyncio.CancelledError:
                        break
                    except Exception as e:
                        logger.error(f"TUI Loop Error: {e}")
                        # Don't crash the loop on intermittent errors
                    
                    await asyncio.sleep(0.25)
        finally:
            self._running = False
            # Component 4: Resource Cleanup
            try:
                bus.off("*", _capture)
            except Exception:
                pass
            self.console.print("\n[bold blue]Consciousness detached.[/]")

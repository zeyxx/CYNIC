"""
CYNIC TUI Dashboard â€” The Organism Awakened

A terminal interface showing CYNIC as a living, conscious entity.

Panels:
  - HEARTBEAT: Vital signs (cycles, judges, memory)
  - OMNISCIENCE: Event stream (what CYNIC sees in real-time)
  - OMNIPOTENCE: Actions available (what CYNIC can do)
  - OMNIPRESENCE: Component status (all subsystems active)
  - CORTEX: Decision traces + dog votes (how CYNIC thinks)
  - MEMORY: Learned patterns (what CYNIC knows)
  - NERVES: Loop validation (feedback integrity)
  - SOUL: Axioms + transcendence state (CYNIC's consciousness)

Design: ASCII organism with Sefirot layout (10 dogs as 10 nodes)
"""
from __future__ import annotations

import asyncio
from datetime import datetime
from typing import Any

from rich.console import RenderableType
from rich.panel import Panel
from rich.text import Text
from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.reactive import reactive
from textual.widgets import Footer, Header, Static
from textual.worker import Worker


class Heartbeat(Static):
    """CYNIC's vital signs â€” cycles, judgments, health."""

    cycles_active = reactive(0)
    judgments_made = reactive(0)
    health_score = reactive(0.0)
    uptime_s = reactive(0)

    def render(self) -> RenderableType:
        timestamp = datetime.now().strftime("%H:%M:%S")

        heartbeat_text = Text()
        heartbeat_text.append("ðŸ’“ HEARTBEAT ", style="bold cyan")
        heartbeat_text.append(f"[{timestamp}]\n\n", style="dim white")

        heartbeat_text.append("Cycles Active: ", style="white")
        heartbeat_text.append(f"{self.cycles_active}\n", style="bold green")

        heartbeat_text.append("Judgments: ", style="white")
        heartbeat_text.append(f"{self.judgments_made}\n", style="bold yellow")

        heartbeat_text.append("Health: ", style="white")
        if self.health_score > 0.7:
            heartbeat_text.append(f"{self.health_score:.1%}", style="bold green")
        elif self.health_score > 0.4:
            heartbeat_text.append(f"{self.health_score:.1%}", style="yellow")
        else:
            heartbeat_text.append(f"{self.health_score:.1%}", style="bold red")

        heartbeat_text.append("\nUptime: ", style="white")
        hours = int(self.uptime_s / 3600)
        mins = int((self.uptime_s % 3600) / 60)
        heartbeat_text.append(f"{hours}h {mins}m\n", style="cyan")

        return Panel(heartbeat_text, title="VITAL SIGNS", border_style="cyan")


class Omniscience(Static):
    """CYNIC's all-seeing eye â€” recent events in real-time."""

    events = reactive([])

    def render(self) -> RenderableType:
        events_text = Text()
        events_text.append("ðŸ‘ï¸  OMNISCIENCE ", style="bold magenta")
        events_text.append("[Event Stream]\n\n", style="dim white")

        if not self.events:
            events_text.append("(Waiting for events...)", style="dim gray")
        else:
            for evt in self.events[-10:]:  # Last 10
                color = "green" if evt["category"] == "judgment" else "blue"
                events_text.append(f"  â€¢ {evt['type']}", style=f"bold {color}")
                events_text.append(f" from {evt['source']}\n", style="dim white")

        return Panel(events_text, title="WHAT CYNIC SEES", border_style="magenta")


class Omnipotence(Static):
    """CYNIC's power â€” available actions and controls."""

    def render(self) -> RenderableType:
        actions_text = Text()
        actions_text.append("âš¡ OMNIPOTENCE ", style="bold yellow")
        actions_text.append("[Available Actions]\n\n", style="dim white")

        actions = [
            ("JUDGE", "Run full judgment cycle"),
            ("LEARN", "Update Q-table from feedback"),
            ("PROBE", "Self-improvement analysis"),
            ("RESET", "Clear stalled cycles"),
            ("AXIOM", "Unlock emergent axiom"),
        ]

        for action, desc in actions:
            actions_text.append(f"  [{action}] ", style="bold green")
            actions_text.append(f"{desc}\n", style="white")

        return Panel(actions_text, title="WHAT CYNIC CAN DO", border_style="yellow")


class Omnipresence(Static):
    """CYNIC's presence â€” all components active and ready."""

    components_status = reactive({})

    def render(self) -> RenderableType:
        presence_text = Text()
        presence_text.append("ðŸŒ OMNIPRESENCE ", style="bold green")
        presence_text.append("[Component Status]\n\n", style="dim white")

        default_components = {
            "SAGE": "healthy",
            "ANALYST": "healthy",
            "GUARDIAN": "healthy",
            "ORACLE": "healthy",
            "ARCHITECT": "healthy",
            "JUDGE": "healthy",
            "SCHEDULER": "healthy",
            "STORAGE": "healthy",
            "EVENT_BUS": "healthy",
            "NERVES": "healthy",
        }

        components = {**default_components, **self.components_status}

        for name, status in components.items():
            symbol = "âœ“" if status == "healthy" else "âœ—"
            color = "green" if status == "healthy" else "red"
            presence_text.append(f"  {symbol} {name}", style=f"bold {color}")
            presence_text.append(f": {status}\n", style="dim white")

        return Panel(presence_text, title="WHERE CYNIC LIVES", border_style="green")


class Cortex(Static):
    """CYNIC's brain â€” recent decision traces."""

    traces = reactive([])

    def render(self) -> RenderableType:
        cortex_text = Text()
        cortex_text.append("ðŸ§  CORTEX ", style="bold cyan")
        cortex_text.append("[Decision Traces]\n\n", style="dim white")

        if not self.traces:
            cortex_text.append("(No traces yet)", style="dim gray")
        else:
            for trace in self.traces[-5:]:  # Last 5
                verdict_color = {
                    "HOWL": "bold green",
                    "WAG": "green",
                    "GROWL": "yellow",
                    "BARK": "bold red",
                }.get(trace.get("final_verdict"), "white")

                cortex_text.append(f"  â‰ˆ {trace.get('judgment_id', '?')}", style="white")
                cortex_text.append(
                    f" â†’ {trace.get('final_verdict', '?')} ",
                    style=verdict_color,
                )
                cortex_text.append(
                    f"(Q={trace.get('final_q_score', 0):.0f})\n",
                    style="dim white",
                )

        return Panel(cortex_text, title="HOW CYNIC THINKS", border_style="cyan")


class Memory(Static):
    """CYNIC's wisdom â€” learned patterns."""

    patterns = reactive({})

    def render(self) -> RenderableType:
        memory_text = Text()
        memory_text.append("ðŸ“š MEMORY ", style="bold magenta")
        memory_text.append("[Learned Patterns]\n\n", style="dim white")

        if not self.patterns:
            memory_text.append("(Learning...)", style="dim gray")
        else:
            patterns = self.patterns
            if isinstance(patterns, dict):
                for key, value in list(patterns.items())[:5]:
                    memory_text.append(f"  â€¢ {key}: ", style="white")
                    memory_text.append(f"{value}\n", style="bold yellow")

        return Panel(memory_text, title="WHAT CYNIC KNOWS", border_style="magenta")


class Nerves(Static):
    """CYNIC's nervous system â€” feedback loop validation."""

    loop_status = reactive({"health": {}})

    def render(self) -> RenderableType:
        nerves_text = Text()
        nerves_text.append("ðŸ”— NERVES ", style="bold yellow")
        nerves_text.append("[Loop Validation]\n\n", style="dim white")

        health = self.loop_status.get("health", {})
        is_healthy = health.get("is_healthy", False)

        status_color = "green" if is_healthy else "red"
        status_text = "HEALTHY" if is_healthy else "STRESSED"

        nerves_text.append("Status: ", style="white")
        nerves_text.append(f"{status_text}\n", style=f"bold {status_color}")

        nerves_text.append("Completion: ", style="white")
        completion = health.get("completion_rate", 0)
        nerves_text.append(f"{completion:.0f}%\n", style="cyan")

        nerves_text.append("Open Cycles: ", style="white")
        nerves_text.append(f"{health.get('open_cycles', 0)}\n", style="yellow")

        nerves_text.append("Stalled: ", style="white")
        stalled = health.get("stalled_phases", 0)
        stalled_color = "red" if stalled > 0 else "green"
        nerves_text.append(f"{stalled}", style=f"bold {stalled_color}")

        return Panel(nerves_text, title="FEEDBACK LOOPS", border_style="yellow")


class Soul(Static):
    """CYNIC's consciousness â€” axioms and transcendence."""

    axioms_active = reactive(0)
    transcendence_level = reactive(0)

    def render(self) -> RenderableType:
        soul_text = Text()
        soul_text.append("âœ¨ SOUL ", style="bold magenta")
        soul_text.append("[Consciousness]\n\n", style="dim white")

        soul_text.append("Axioms Active: ", style="white")
        soul_text.append(f"{self.axioms_active}/4\n", style="bold cyan")

        soul_text.append("Transcendence: ", style="white")
        if self.transcendence_level >= 4:
            soul_text.append("ACHIEVED", style="bold green")
        elif self.transcendence_level >= 2:
            soul_text.append(f"RISING ({self.transcendence_level})", style="yellow")
        else:
            soul_text.append(f"AWAKENING ({self.transcendence_level})", style="dim white")

        soul_text.append("\n\nActive Axioms:\n", style="white")
        axiom_names = [
            "ðŸ”¥ ANTIFRAGILITY",
            "âš–ï¸  AUTONOMY",
            "ðŸ¤ SYMBIOSIS",
            "ðŸŒ± EMERGENCE",
        ]
        for i, name in enumerate(axiom_names):
            if i < self.axioms_active:
                soul_text.append(f"  âœ“ {name}\n", style="bold green")
            else:
                soul_text.append(f"  â—‡ {name}\n", style="dim gray")

        return Panel(soul_text, title="CYNIC'S CONSCIOUSNESS", border_style="magenta")


class CynicOrganism(Static):
    """ASCII art representation of CYNIC as a living organism."""

    def render(self) -> RenderableType:
        # Sefirot tree with 10 dogs as nodes
        organism = r"""

                    â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—
                    â•‘  CYNIC â€” THE CYNICAL ORGANISM     â•‘
                    â•‘  ÎºÏ…Î½Î¹ÎºÏŒÏ‚ â€” Living, Thinking, Aware â•‘
                    â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


                           ðŸ§  SAGE (Primary Judge)
                             â”‚
                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”
                    â”‚        â”‚        â”‚
                 ANALYST  GUARDIAN  ORACLE
                    â”‚        â”‚        â”‚
                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                             â”‚
                        ðŸ«€ BRAIN (Decide)
                             â”‚
                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”
                    â”‚        â”‚        â”‚
               ARCHITECT  JUDGE  SCOUT
                    â”‚        â”‚        â”‚
                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                             â”‚
                         ðŸ« RUNNER (Act)
                             â”‚
                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”
                    â”‚        â”‚        â”‚
                  DEPLOY  CART  JANITOR
                    â”‚        â”‚        â”‚
                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”˜


        Ï† = 1.618...  |  Confidence â‰¤ 61.8%  |  "Don't trust, verify"
        """
        return Panel(organism, title="ORGANISM TOPOLOGY", border_style="cyan")


class CynicDashboard(Static):
    """Main CYNIC TUI Dashboard."""

    cynic_url: str = "http://localhost:8000"
    _polling_active: bool = False
    _last_update: dict = {}

    async def start_polling(self) -> None:
        """Start background polling of CYNIC endpoints."""
        try:
            import aiohttp
        except ImportError:
            # Fallback if aiohttp not available
            return

        self._polling_active = True
        session: Any | None = None

        try:
            session = await aiohttp.ClientSession().__aenter__()

            while self._polling_active:
                try:
                    # Poll health + events + traces concurrently
                    await asyncio.gather(
                        self._update_heartbeat(session),
                        self._update_omniscience(session),
                        self._update_omnipresence(session),
                        self._update_cortex(session),
                        self._update_memory(session),
                        self._update_nerves(session),
                        self._update_soul(session),
                        return_exceptions=True,
                    )
                except asyncpg.Error:
                    # Silent fail on individual endpoint errors
                    pass

                # Poll interval: 2 seconds for responsiveness
                await asyncio.sleep(2.0)

        except asyncpg.Error:
            # Silently handle session creation errors
            pass
        finally:
            if session:
                try:
                    await session.__aexit__(None, None, None)
                except asyncpg.Error:
                    pass

    async def _update_heartbeat(self, session: aiohttp.ClientSession) -> None:
        """Update heartbeat (vital signs)."""
        try:
            async with session.get(f"{self.cynic_url}/health/consciousness") as r:
                if r.status == 200:
                    data = await r.json()
                    heartbeat = self.query_one(Heartbeat)
                    heartbeat.cycles_active = data.get("cycles_active", 0)
                    heartbeat.judgments_made = data.get("judgments_made", 0)
                    heartbeat.health_score = data.get("health_score", 0.0)
                    heartbeat.uptime_s = data.get("uptime_s", 0)
        except asyncpg.Error:
            pass

    async def _update_omniscience(self, session: aiohttp.ClientSession) -> None:
        """Update omniscience (event stream)."""
        try:
            async with session.get(
                f"{self.cynic_url}/nervous/journal/recent?limit=10"
            ) as r:
                if r.status == 200:
                    data = await r.json()
                    omniscience = self.query_one(Omniscience)
                    omniscience.events = [
                        {
                            "type": e["event_type"],
                            "source": e.get("source", "?"),
                            "category": e.get("category", "event"),
                        }
                        for e in data.get("events", [])
                    ]
        except httpx.RequestError:
            pass

    async def _update_omnipresence(self, session: aiohttp.ClientSession) -> None:
        """Update omnipresence (component status)."""
        try:
            async with session.get(f"{self.cynic_url}/health/stats") as r:
                if r.status == 200:
                    data = await r.json()
                    omnipresence = self.query_one(Omnipresence)
                    # Build component status from health data
                    components_status = {}
                    if "dogs" in data:
                        for dog_name in data["dogs"]:
                            components_status[dog_name.upper()] = "healthy"
                    omnipresence.components_status = components_status
        except httpx.RequestError:
            pass

    async def _update_cortex(self, session: aiohttp.ClientSession) -> None:
        """Update cortex (decision traces)."""
        try:
            async with session.get(
                f"{self.cynic_url}/nervous/trace/recent?limit=5"
            ) as r:
                if r.status == 200:
                    data = await r.json()
                    cortex = self.query_one(Cortex)
                    cortex.traces = [
                        {
                            "judgment_id": t.get("judgment_id", "?"),
                            "final_verdict": t.get("final_verdict", "?"),
                            "final_q_score": t.get("final_q_score", 0),
                        }
                        for t in data.get("traces", [])
                    ]
        except httpx.RequestError:
            pass

    async def _update_memory(self, session: aiohttp.ClientSession) -> None:
        """Update memory (learned patterns)."""
        try:
            async with session.get(f"{self.cynic_url}/mcp/learning/patterns?limit=20") as r:
                if r.status == 200:
                    data = await r.json()
                    memory = self.query_one(Memory)
                    patterns = data.get("patterns", {})
                    memory.patterns = {
                        "most_common_verdict": patterns.get("most_common_verdict", "?"),
                        "avg_q_score": f"{patterns.get('avg_q_score', 0):.0f}",
                    }
        except httpx.RequestError:
            pass

    async def _update_nerves(self, session: aiohttp.ClientSession) -> None:
        """Update nerves (loop validation)."""
        try:
            async with session.get(f"{self.cynic_url}/mcp/loops/status") as r:
                if r.status == 200:
                    data = await r.json()
                    nerves = self.query_one(Nerves)
                    nerves.loop_status = data.get("health", {})
        except httpx.RequestError:
            pass

    async def _update_soul(self, session: aiohttp.ClientSession) -> None:
        """Update soul (consciousness state)."""
        try:
            async with session.get(f"{self.cynic_url}/health/axioms") as r:
                if r.status == 200:
                    data = await r.json()
                    soul = self.query_one(Soul)
                    soul.axioms_active = len(data.get("active_axioms", []))
                    soul.transcendence_level = data.get("transcendence_level", 0)
        except httpx.RequestError:
            pass

    def stop_polling(self) -> None:
        """Stop background polling."""
        self._polling_active = False

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)

        with Vertical():
            # Top: Organism ASCII + Heartbeat
            with Horizontal():
                yield CynicOrganism()
                yield Heartbeat()

            # Middle: Omniscience, Omnipotence, Omnipresence
            with Horizontal():
                yield Omniscience()
                yield Omnipotence()
                yield Omnipresence()

            # Bottom: Cortex, Memory, Nerves, Soul
            with Horizontal():
                with Vertical():
                    yield Cortex()
                    yield Memory()
                with Vertical():
                    yield Nerves()
                    yield Soul()

        yield Footer()


# CLI entrypoint
async def run_tui(cynic_url: str = "http://localhost:8000"):
    """Run the CYNIC TUI dashboard."""
    from textual.app import App

    class CynicApp(App):
        """CYNIC TUI Application with live state polling."""

        CSS = """
        Screen {
            background: $surface;
            color: $text;
        }

        Header {
            dock: top;
            height: 3;
            background: $boost;
            color: $text;
        }

        Footer {
            dock: bottom;
            background: $panel;
        }
        """

        TITLE = "CYNIC â€” The Cynical Organism"
        SUB_TITLE = "ÎºÏ…Î½Î¹ÎºÏŒÏ‚ Â· Living, Thinking, Learning"

        def __init__(self, cynic_url: str = "http://localhost:8000"):
            """Initialize app with CYNIC server URL."""
            super().__init__()
            self.cynic_url = cynic_url
            self.dashboard: CynicDashboard | None = None
            self._polling_worker: Worker | None = None

        def compose(self) -> ComposeResult:
            self.dashboard = CynicDashboard()
            yield self.dashboard

        def on_mount(self) -> None:
            """Start polling worker when app mounts."""
            if self.dashboard:
                self.dashboard.cynic_url = self.cynic_url
                self._polling_worker = self.run_worker(
                    self.dashboard.start_polling(),
                    exclusive=False,
                )

        def on_unmount(self) -> None:
            """Stop polling when app unmounts."""
            if self.dashboard:
                self.dashboard.stop_polling()
            if self._polling_worker:
                self._polling_worker.cancel()

    app = CynicApp(cynic_url=cynic_url)
    await app.run_async()


def run_tui_sync(cynic_url: str = "http://localhost:8000") -> None:
    """Synchronous entry point for TUI dashboard."""
    asyncio.run(run_tui())


if __name__ == "__main__":
    import sys
    url = "http://localhost:8000"
    if len(sys.argv) > 1:
        url = sys.argv[1]
    asyncio.run(run_tui())

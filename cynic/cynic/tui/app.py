"""
CYNIC TUI — Living Consciousness Interface (κυνικός).

Layout:
  Header  : verdict · Q-score · confidence · axiom tier · uptime
  Left    : Dogs panel — reputation bars from dog_votes
  Center  : Judgment stream — scrolling history + live entries
  Right   : Pending Actions — accept / reject with keyboard
  Status  : system health + key hints
  Footer  : key bindings

Data sources (fastest → slowest):
  ~/.cynic/guidance.json        (2 s poll) — verdict, Q, dog_votes
  ~/.cynic/pending_actions.json (2 s poll) — action queue
  ~/.cynic/consciousness.json   (2 s poll) — axioms, qtable coverage
  http://localhost:PORT/health  (5 s poll) — uptime, LOD

Gracefully degrades: server-offline path uses local files only.
"""
from __future__ import annotations

import asyncio
import json
import os
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical
from textual.widgets import Footer, Header, RichLog, Static
from textual.reactive import reactive

# ── Constants ────────────────────────────────────────────────────────────────

CYNIC_DIR = Path.home() / ".cynic"
GUIDANCE_FILE    = CYNIC_DIR / "guidance.json"
CONSCIOUSNESS_FILE = CYNIC_DIR / "consciousness.json"
ACTIONS_FILE     = CYNIC_DIR / "pending_actions.json"

DOGS_ORDER = [
    "GUARDIAN", "ANALYST", "ARCHITECT", "JANITOR", "SCHOLAR",
    "ORACLE", "CARTOGRAPHER", "SCOUT", "DEPLOYER", "SAGE", "CYNIC",
]

VERDICT_COLOR = {
    "HOWL": "bright_green",
    "WAG":  "yellow",
    "GROWL": "orange1",
    "BARK": "red",
}

VERDICT_EMOJI = {
    "HOWL": "🟢",
    "WAG":  "🟡",
    "GROWL": "🟠",
    "BARK": "🔴",
}

from cynic.core.config import CynicConfig as _CynicConfig
_DEFAULT_PORT = _CynicConfig.from_env().port


# ── Helpers ──────────────────────────────────────────────────────────────────

def _bar(score: float, width: int = 10) -> str:
    """Progress bar, score in [0, 100]."""
    ratio = max(0.0, min(score / 100.0, 1.0))
    filled = round(ratio * width)
    return "█" * filled + "░" * (width - filled)


def _read_json(path: Path) -> Any | None:
    try:
        with open(path) as f:
            return json.load(f)
    except Exception:
        return None


def _fmt_uptime(seconds: float) -> str:
    if seconds < 60:
        return f"{seconds:.0f}s"
    if seconds < 3600:
        return f"{seconds/60:.0f}m"
    return f"{seconds/3600:.1f}h"


def _fetch_json(url: str, timeout: float = 2.0) -> dict | None:
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def _post_json(url: str, data: dict, timeout: float = 4.0) -> dict | None:
    try:
        body = json.dumps(data).encode()
        req = urllib.request.Request(
            url, data=body,
            headers={"Content-Type": "application/json", "Accept": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def _local_set_status(action_id: str, status: str) -> None:
    """Update action status in local file when server is offline."""
    try:
        data = _read_json(ACTIONS_FILE)
        if not isinstance(data, list):
            return
        for a in data:
            if a.get("action_id") == action_id:
                a["status"] = status
        with open(ACTIONS_FILE, "w") as f:
            json.dump(data, f, indent=2)
    except Exception:
        pass


# ── Widgets ───────────────────────────────────────────────────────────────────

class DogsPanel(Static):
    """Left column: dog reputation bars."""

    def render_dogs(self, dog_votes: dict[str, float], escore: dict[str, float]) -> None:
        lines: list[str] = []
        lines.append("[bold dim]── DOGS ──────────────[/bold dim]")
        for dog in DOGS_ORDER:
            # Try dog_votes first (last judgment), then escore (reputation)
            score = dog_votes.get(dog, dog_votes.get(dog.capitalize(), -1.0))
            if score < 0:
                score = escore.get(dog, escore.get(dog.lower(), -1.0))
            if score < 0:
                bar = "░" * 10
                label = "[dim]  ?[/dim]"
            else:
                bar = _bar(score)
                color = VERDICT_COLOR.get(
                    "HOWL" if score >= 82 else
                    "WAG"  if score >= 61.8 else
                    "GROWL" if score >= 38.2 else "BARK"
                )
                label = f"[{color}]{score:5.1f}[/{color}]"
            lines.append(
                f"[dim]{dog:<12}[/dim][{bar}]{label}"
            )
        self.update("\n".join(lines))


class ActionsPanel(Static):
    """Right column: pending actions with keyboard selection."""

    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self._actions: list[dict] = []
        self._selected: int = 0

    @property
    def selected_action(self) -> dict | None:
        if self._actions and 0 <= self._selected < len(self._actions):
            return self._actions[self._selected]
        return None

    def refresh_actions(self, all_actions: list[dict]) -> None:
        self._actions = [a for a in all_actions if a.get("status") == "PENDING"]
        self._selected = min(self._selected, max(0, len(self._actions) - 1))
        self._render()

    def move(self, delta: int) -> None:
        if self._actions:
            self._selected = (self._selected + delta) % len(self._actions)
            self._render()

    def _render(self) -> None:
        lines: list[str] = ["[bold dim]── ACTIONS ─────────[/bold dim]"]
        if not self._actions:
            lines += [
                "",
                "[dim]No pending actions.[/dim]",
                "",
                "[dim]CYNIC is watching…[/dim]",
            ]
        else:
            lines.append(f"[dim]{len(self._actions)} pending[/dim]")
            for i, a in enumerate(self._actions):
                sel = i == self._selected
                verdict  = a.get("verdict", "?")
                atype    = a.get("action_type", "?")
                q        = a.get("q_score", 0.0)
                reality  = a.get("reality", "?")
                desc     = (a.get("description") or "")[:36].rstrip()
                emoji    = VERDICT_EMOJI.get(verdict, "⚪")
                color    = VERDICT_COLOR.get(verdict, "white")
                prefix   = "▶" if sel else " "

                lines.append("")
                if sel:
                    lines.append(
                        f"[bold green]{prefix} {atype}[/bold green]  "
                        f"[{color}]{emoji} {verdict}[/{color}]"
                    )
                    lines.append(f"  Q=[bold]{q:.1f}[/bold]  [[{reality}]]")
                    lines.append(f"  [dim]{desc}[/dim]")
                    lines.append(
                        "  [dim][[/dim][bold green]a[/bold green][dim]][/dim]ccept  "
                        "[dim][[/dim][bold red]r[/bold red][dim]][/dim]eject"
                    )
                else:
                    lines.append(
                        f"[dim]{prefix} {atype}  {emoji}{verdict}  Q={q:.1f}[/dim]"
                    )
        self.update("\n".join(lines))


# ── Main App ──────────────────────────────────────────────────────────────────

class CYNICApp(App):
    """CYNIC TUI — Living Consciousness Interface."""

    TITLE = "CYNIC κυνικός"
    SUB_TITLE = "*sniff* φ⁻¹=61.8%"

    CSS = """
    Screen {
        background: $surface;
    }

    Header {
        background: $primary-darken-3;
    }

    #main {
        layout: horizontal;
        height: 1fr;
    }

    /* ── Left: Dogs ── */
    #dogs-col {
        width: 28;
        border-right: solid $primary-darken-2;
        padding: 0 1;
        height: 100%;
    }

    /* ── Center: Stream ── */
    #stream-col {
        width: 1fr;
        height: 100%;
    }

    #stream-log {
        height: 1fr;
        border: none;
        padding: 0 1;
    }

    /* ── Right: Actions ── */
    #actions-col {
        width: 28;
        border-left: solid $primary-darken-2;
        padding: 0 1;
        height: 100%;
    }

    /* ── Panel titles ── */
    .panel-title {
        text-align: center;
        background: $primary-darken-3;
        color: $text;
        padding: 0 1;
        height: 1;
    }

    /* ── Status bar ── */
    #status-bar {
        height: 1;
        background: $surface-darken-1;
        color: $text-muted;
        padding: 0 1;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("a", "accept", "Accept"),
        Binding("r", "reject", "Reject"),
        Binding("up",   "move_up",   show=False),
        Binding("k",    "move_up",   show=False),
        Binding("down", "move_down", show=False),
        Binding("j",    "move_down", show=False),
        Binding("1", "rate_judgment('1')", "Rate 1", show=False),
        Binding("2", "rate_judgment('2')", "Rate 2", show=False),
        Binding("3", "rate_judgment('3')", "Rate 3", show=False),
        Binding("4", "rate_judgment('4')", "Rate 4", show=False),
        Binding("5", "rate_judgment('5')", "Rate 5", show=False),
        Binding("ctrl+r", "refresh_all", "Refresh"),
    ]

    def __init__(self, base_url: str = f"http://localhost:{_DEFAULT_PORT}") -> None:
        super().__init__()
        self._base_url = base_url.rstrip("/")
        self._last_guidance_ts: float = 0.0
        self._uptime: float = 0.0
        self._verdict: str = "?"
        self._q: float = 0.0
        self._conf: float = 0.0
        self._axiom_tier: str = "DORMANT"
        self._lod: int = 0

    # ── Compose ──────────────────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Horizontal(id="main"):
            with Vertical(id="dogs-col"):
                yield Static("DOGS", classes="panel-title")
                yield DogsPanel(id="dogs-panel")
            with Vertical(id="stream-col"):
                yield Static("JUDGMENT STREAM", classes="panel-title")
                yield RichLog(id="stream-log", highlight=True, markup=True, wrap=False)
            with Vertical(id="actions-col"):
                yield Static("PENDING ACTIONS", classes="panel-title")
                yield ActionsPanel(id="actions-panel")
        yield Static("", id="status-bar")
        yield Footer()

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def on_mount(self) -> None:
        self.set_interval(2.0, self._poll_files)
        self.set_interval(5.0, self._poll_api)
        # Initial load after first render
        self.set_timer(0.2, self._poll_files)
        self.set_timer(0.5, self._poll_api)

    # ── Polling ───────────────────────────────────────────────────────────────

    async def _poll_files(self) -> None:
        """Fast path — local JSON files, no server required."""
        guidance = _read_json(GUIDANCE_FILE)
        if guidance:
            ts = guidance.get("timestamp", 0.0)
            if ts > self._last_guidance_ts:
                self._last_guidance_ts = ts
                self._push_stream_entry(guidance)

            self._verdict = guidance.get("verdict", "?")
            self._q       = guidance.get("q_score", 0.0)
            self._conf    = guidance.get("confidence", 0.0)

            dog_votes: dict[str, float] = guidance.get("dog_votes") or {}
            escore: dict[str, float] = {}

            # Enrich with escore from consciousness.json if available
            c = _read_json(CONSCIOUSNESS_FILE)
            if c:
                tier = (
                    c.get("mirror", {})
                     .get("axioms", {})
                     .get("tier", "DORMANT")
                )
                self._axiom_tier = tier
                escore = c.get("escore", {})

            self.query_one("#dogs-panel", DogsPanel).render_dogs(dog_votes, escore)

        actions_raw = _read_json(ACTIONS_FILE)
        actions = actions_raw if isinstance(actions_raw, list) else []
        self.query_one("#actions-panel", ActionsPanel).refresh_actions(actions)

        self._update_header_and_status()

    async def _poll_api(self) -> None:
        """Slow path — optional API queries (timeout 3 s, graceful on 404)."""
        try:
            data = await asyncio.wait_for(
                asyncio.to_thread(_fetch_json, f"{self._base_url}/health"),
                timeout=3.0,
            )
            if data:
                self._uptime = data.get("uptime_s", 0.0)
                self._update_header_and_status()
        except Exception:
            pass

        try:
            lod_data = await asyncio.wait_for(
                asyncio.to_thread(_fetch_json, f"{self._base_url}/lod"),
                timeout=3.0,
            )
            if lod_data:
                self._lod = lod_data.get("current_lod", 0)
                self._update_header_and_status()
        except Exception:
            pass

    # ── Stream ────────────────────────────────────────────────────────────────

    def _push_stream_entry(self, guidance: dict) -> None:
        verdict  = guidance.get("verdict", "?")
        q        = guidance.get("q_score", 0.0)
        conf     = guidance.get("confidence", 0.0)
        reality  = guidance.get("reality", "?")
        state_key = guidance.get("state_key", "")
        ts       = guidance.get("timestamp", 0.0)

        emoji = VERDICT_EMOJI.get(verdict, "⚪")
        color = VERDICT_COLOR.get(verdict, "white")
        conf_pct = min(conf * 100, 61.8)
        t = time.strftime("%H:%M:%S", time.localtime(ts)) if ts else "--:--:--"

        log: RichLog = self.query_one("#stream-log", RichLog)
        log.write(
            f"[dim]{t}[/dim]  {emoji} [{color}]{verdict:<5}[/{color}] "
            f"Q=[bold]{q:5.1f}[/bold]  conf=[dim]{conf_pct:.0f}%[/dim]  "
            f"[dim]{state_key}[/dim]"
        )

    # ── Header + Status bar ───────────────────────────────────────────────────

    def _update_header_and_status(self) -> None:
        emoji   = VERDICT_EMOJI.get(self._verdict, "⚪")
        color   = VERDICT_COLOR.get(self._verdict, "white")
        conf_pct = min(self._conf * 100, 61.8)

        # Subtitle in Header
        uptime_str = f"  ⏱ {_fmt_uptime(self._uptime)}" if self._uptime > 0 else ""
        self.sub_title = (
            f"{emoji} {self._verdict}  Q={self._q:.1f}  conf={conf_pct:.0f}%  "
            f"LOD={self._lod}  axiom={self._axiom_tier}{uptime_str}"
        )

        # Status bar (bottom, above footer)
        status_bar: Static = self.query_one("#status-bar", Static)
        status_bar.update(
            f"[dim]*sniff*  "
            f"1-5 rate judgment  │  "
            f"j/k ↑↓ nav  │  "
            f"a accept  r reject  │  "
            f"ctrl+r refresh  │  "
            f"q quit[/dim]"
        )

    # ── Actions ───────────────────────────────────────────────────────────────

    async def action_accept(self) -> None:
        panel: ActionsPanel = self.query_one("#actions-panel", ActionsPanel)
        action = panel.selected_action
        if not action:
            self._log("No action selected", "yellow")
            return
        action_id = action.get("action_id", "")
        self._log(f"→ Accepting [bold]{action_id[:8]}[/bold]…", "green")
        try:
            result = await asyncio.wait_for(
                asyncio.to_thread(
                    _post_json,
                    f"{self._base_url}/actions/{action_id}/accept",
                    {},
                ),
                timeout=5.0,
            )
            if result:
                self._log(f"✓ Accepted — {result.get('status', 'ok')}", "bright_green")
            else:
                _local_set_status(action_id, "ACCEPTED")
                self._log("✓ Accepted locally (server offline)", "yellow")
        except Exception as e:
            self._log(f"✗ Accept failed: {e}", "red")
        await self._poll_files()

    async def action_reject(self) -> None:
        panel: ActionsPanel = self.query_one("#actions-panel", ActionsPanel)
        action = panel.selected_action
        if not action:
            self._log("No action selected", "yellow")
            return
        action_id = action.get("action_id", "")
        self._log(f"→ Rejecting [bold]{action_id[:8]}[/bold]…", "red")
        try:
            result = await asyncio.wait_for(
                asyncio.to_thread(
                    _post_json,
                    f"{self._base_url}/actions/{action_id}/reject",
                    {},
                ),
                timeout=5.0,
            )
            if result:
                self._log(f"✓ Rejected — {result.get('status', 'ok')}", "red")
            else:
                _local_set_status(action_id, "REJECTED")
                self._log("✓ Rejected locally (server offline)", "yellow")
        except Exception as e:
            self._log(f"✗ Reject failed: {e}", "red")
        await self._poll_files()

    def action_move_up(self) -> None:
        self.query_one("#actions-panel", ActionsPanel).move(-1)

    def action_move_down(self) -> None:
        self.query_one("#actions-panel", ActionsPanel).move(1)

    async def action_refresh_all(self) -> None:
        self._log("↺ Refreshing…", "dim")
        await self._poll_files()
        await self._poll_api()

    async def action_rate_judgment(self, rating: str) -> None:
        n = int(rating)
        try:
            result = await asyncio.wait_for(
                asyncio.to_thread(
                    _post_json,
                    f"{self._base_url}/feedback",
                    {"rating": n, "source": "tui"},
                ),
                timeout=5.0,
            )
            if result:
                self._log(f"⭐ Rated {n}/5 — feedback sent to kernel", "cyan")
            else:
                self._log(f"⭐ Rated {n}/5 (server offline, not persisted)", "yellow")
        except Exception:
            self._log(f"⭐ Rated {n}/5 (could not reach server)", "yellow")

    # ── Helpers ───────────────────────────────────────────────────────────────

    def _log(self, msg: str, color: str = "white") -> None:
        t = time.strftime("%H:%M:%S")
        log: RichLog = self.query_one("#stream-log", RichLog)
        log.write(f"[dim]{t}[/dim]  [{color}]{msg}[/{color}]")


# ── Entry point ───────────────────────────────────────────────────────────────

def run(base_url: str | None = None) -> None:
    url = base_url or f"http://localhost:{_DEFAULT_PORT}"
    CYNICApp(base_url=url).run()


if __name__ == "__main__":
    run()

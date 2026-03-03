"""
TUI Panel " CYNIC Orchestration Control

From the TUI, manage:
- Docker builds & deployments
- Service health monitoring
- Release management
- Version bumping

This IS the command center for CYNIC self-orchestration.
"""
from __future__ import annotations

from rich.panel import Panel
from rich.table import Table
from textual.containers import Container
from textual.reactive import reactive
from textual.widgets import Static

from cynic.orchestration import DockerManager, HealthMonitor, VersionManager


class OrchestrationPanel(Container):
    """TUI panel for CYNIC orchestration."""

    # State
    docker_manager = reactive(None)
    version_manager = reactive(None)
    health_monitor = reactive(None)
    current_section = reactive("overview")  # overview, docker, versions, health

    def __init__(self):
        super().__init__()
        self.docker_manager = DockerManager()
        self.version_manager = VersionManager()
        self.health_monitor = HealthMonitor()

    def render(self) -> Panel:
        """Render the orchestration panel."""
        if self.current_section == "overview":
            return self._render_overview()
        elif self.current_section == "docker":
            return self._render_docker()
        elif self.current_section == "versions":
            return self._render_versions()
        elif self.current_section == "health":
            return self._render_health()
        else:
            return Panel("Unknown section", title="Orchestration")

    def _render_overview(self) -> Panel:
        """Overview of all systems."""
        content = """
"-
'           CYNIC Self-Orchestration Console                    '
'                                                               '
'  [D] Docker     - Build & deploy services                    '
'  [V] Versions   - Manage releases & migrations               '
'  [H] Health     - Monitor services & alerts                  '
'  [S] Status     - Quick overview of all systems              '


Current Status:
"""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
Kernel:         Ready (port 8000)
PostgreSQL:     Ready (port 5433)
Ollama:         Ready (port 11434)

Last Deploy:    2026-02-20 14:32:15 UTC
Last Build:     v1.0.0 (2026-02-20 14:31:00 UTC)
Version:        1.0.0
"""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

Commands:
  [1] Start services        [6] View logs
  [2] Stop services         [7] Health check
  [3] Build image           [8] Bump version
  [4] Deploy to staging     [9] Create release
  [5] Deploy to prod        [0] Help

*sniff* Ready to orchestrate. Choose your path:
"""
        return Panel(content, title="CYNIC Orchestration", border_style="cyan")

    def _render_docker(self) -> Panel:
        """Docker management section."""
        table = Table(title="Docker Services")
        table.add_column("Service", style="cyan")
        table.add_column("Status", style="green")
        table.add_column("Port", style="yellow")
        table.add_column("Uptime", style="magenta")

        # These would be populated from actual container data
        table.add_row("cynic-kernel", " Running", "8000", "2h 34m")
        table.add_row("postgres-py", " Running", "5433", "2h 34m")
        table.add_row("ollama", " Running", "11434", "2h 34m")

        content = f""""
"-
'                    Docker Management                         '


{table}

Last Actions:
"""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
Build: {self.docker_manager.last_build().image if self.docker_manager.last_build() else "None"}
Deploy: {self.docker_manager.last_deploy().timestamp if self.docker_manager.last_deploy() else "None"}

Commands:
  [B] Build image      [D] Deploy dev
  [S] Stop services    [P] Deploy prod
  [L] View logs        [H] Health check
"""
        return Panel(content, title="Docker", border_style="blue")

    def _render_versions(self) -> Panel:
        """Version management section."""
        current = self.version_manager.get_current()

        content = f""""
"-
'                  Version Management                          '


Current Version: {current}
""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
Docker Image:   cynic-kernel:{current}
Status:         Stable
Last Release:   2026-02-20
Next Release:   Scheduled for 2026-02-27

Release History:
  v1.0.0    "  2026-02-20  Init: Full Python kernel, SurrealDB
  v0.9.0    "  2026-02-19  Beta: Learning loops + Axiom unlock

Commands:
  [1] Bump patch (1.0.0 ' 1.0.1)
  [2] Bump minor (1.0.0 ' 1.1.0)
  [3] Bump major (1.0.0 ' 2.0.0)
  [4] Create release
  [5] View changelog
"""
        return Panel(content, title="Versions", border_style="green")

    def _render_health(self) -> Panel:
        """Health monitoring section."""
        table = Table(title="Service Health")
        table.add_column("Service", style="cyan")
        table.add_column("Status", style="green")
        table.add_column("CPU", style="yellow")
        table.add_column("Memory", style="magenta")
        table.add_column("Alerts", style="red")

        table.add_row("cynic-kernel", " Healthy", "2.3%", "342 MB", "0")
        table.add_row("postgres-py", " Healthy", "1.1%", "128 MB", "0")
        table.add_row("ollama", " Healthy", "0.0%", "4.2 GB", "0")

        content = f""""
"-
'               Health Monitoring & Alerts                     '


{table}

Active Alerts: None
""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""

Commands:
  [1] Full health report  [4] Configure thresholds
  [2] View metrics         [5] Clear alerts
  [3] Start monitoring     [6] Stop monitoring
"""
        return Panel(content, title="Health", border_style="yellow")

    def handle_command(self, key: str) -> None:
        """Handle keyboard commands."""
        if key == "d":
            self.current_section = "docker"
        elif key == "v":
            self.current_section = "versions"
        elif key == "h":
            self.current_section = "health"
        elif key == "s":
            self.current_section = "overview"


class OrchestrationWidget(Static):
    """Textual widget wrapper for orchestration panel."""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.panel = OrchestrationPanel()

    def render(self) -> Panel:
        return self.panel.render()

    def on_key(self, event) -> None:
        """Handle keyboard input."""
        self.panel.handle_command(event.key)
        self.refresh()